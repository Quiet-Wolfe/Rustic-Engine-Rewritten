use super::App;
use crate::camera_events::apply_camera_event;
use crate::gameplay_note_sfx::{play_note_kind_hit_sfx_or_warn, play_note_kind_miss_sfx_or_warn};
use crate::hud_assets::HealthIconEvent;
use crate::lane_state::lane_for_action;
use crate::miss_note_audio::{play_miss_note_or_warn as play_miss_sfx, MissNoteKind};
use crate::note_assets::confirm_duration_or_default;
use crate::song_audio::{play_sample_rate, set_vocals_gain};
use crate::stage_sfx::play_sserafim_event_sound_or_warn;
use rustic_asset::ChartEventKind;
use rustic_core::input::{InputAction, InputState, NormalizedInputEvent};
use rustic_core::time::Samples;
use rustic_game::{Judgment, Lane};

impl App {
    pub(super) fn apply_song_event(
        &mut self,
        kind: &ChartEventKind,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        if !self.options_preferences.camera_zooms
            && matches!(
                kind,
                ChartEventKind::ZoomCamera { .. } | ChartEventKind::SetCameraBop { .. }
            )
        {
            return;
        }
        if apply_camera_event(
            &mut self.cameras,
            &mut self.camera_fx,
            self.camera_focus,
            kind,
            cursor,
            sample_rate,
            bpm,
        ) {
            return;
        }
        if self.sserafim_stage.active() {
            play_sserafim_event_sound_or_warn(&self.mixer, kind);
        }
        if self.sserafim_stage.apply_event(kind, cursor) {
            return;
        }
        if let ChartEventKind::PlayAnimation {
            target,
            animation,
            force,
        } = kind
        {
            self.character_anim
                .play_chart_animation(target, animation, cursor, *force);
        } else if let ChartEventKind::SetHealthIcon {
            target,
            id,
            scale,
            flip_x,
            is_pixel,
            offset_x,
            offset_y,
        } = kind
        {
            if let Some(hud) = self.hud_skin.as_mut() {
                hud.set_health_icon(
                    id,
                    HealthIconEvent {
                        target: *target,
                        scale: *scale,
                        flip_x: *flip_x,
                        is_pixel: *is_pixel,
                        offset: glam::vec2(*offset_x, *offset_y),
                    },
                );
            }
        }
    }

    pub(super) fn handle_gameplay_input(
        &mut self,
        event: &NormalizedInputEvent,
        already_held: bool,
    ) {
        if event.state != InputState::Pressed {
            return;
        }
        let cursor = event.audio_sample_cursor_at_receive;
        if self.game_over.is_some() {
            return;
        }
        let sample_rate = play_sample_rate(&self.mixer);
        let confirm_duration = confirm_duration_or_default(self.note_skin.as_ref(), sample_rate);
        let gameplay_event =
            NormalizedInputEvent::new(event.action, event.state, event.wall_clock_ns, cursor);
        let mut restore_vocals = false;
        let should_enter_game_over;
        {
            let Some(play_state) = self.play_state.as_mut() else {
                return;
            };
            if event.action == InputAction::Reset {
                // ref: bdedc0aa:source/funkin/play/PlayState.hx:1243-1258
                play_state.health = 0.0;
                should_enter_game_over = !self.practice_mode;
            } else {
                let Some(lane) = lane_for_action(event.action) else {
                    return;
                };
                if already_held {
                    return;
                }
                let anim = &mut self.character_anim;
                if let Some(outcome) =
                    play_state.try_hit_in_lane(&gameplay_event, lane, sample_rate)
                {
                    self.held_lanes.confirm(lane, cursor, confirm_duration);
                    anim.player_note_hit_kind(
                        lane,
                        outcome.kind,
                        cursor,
                        sample_rate,
                        play_state.bpm,
                    );
                    if let Some(kind) = outcome.kind {
                        play_note_kind_hit_sfx_or_warn(&self.mixer, cursor, kind);
                    }
                    let combo_count = outcome.combo_count;
                    anim.girlfriend_note_hit(outcome.judgment, combo_count, cursor);
                    restore_vocals = true;
                    if !outcome.is_sustain {
                        self.score_popups
                            .push(outcome.judgment, outcome.combo_popup, cursor);
                        if outcome.judgment == Judgment::Sick {
                            self.note_splashes.push(lane, cursor);
                        }
                        if let Some(hold_end_at) = outcome.hold_end_at {
                            self.active_holds
                                .start(lane, hold_end_at, cursor, outcome.note_id);
                            self.hold_covers.start(lane, cursor, hold_end_at);
                        }
                    }
                } else {
                    play_state.register_ghost_miss();
                    anim.player_note_miss(lane, cursor, sample_rate, play_state.bpm);
                    play_miss_sfx(&self.mixer, cursor, MissNoteKind::Ghost);
                }
                should_enter_game_over = play_state.is_dead() && !self.practice_mode;
            }
        }
        if restore_vocals {
            set_vocals_gain(&self.mixer, 1.0);
        }
        if should_enter_game_over {
            self.enter_game_over(cursor);
        }
    }

    pub(super) fn register_hold_drop(
        &mut self,
        lane: Lane,
        cursor: Samples,
        hold_end_at: Samples,
        note_id: rustic_core::ids::NoteId,
    ) {
        let sample_rate = play_sample_rate(&self.mixer);
        let Some(play_state) = self.play_state.as_mut() else {
            return;
        };
        let kind = play_state
            .notes
            .iter()
            .find(|note| note.id == note_id)
            .and_then(|note| note.kind);
        let remaining_samples = hold_end_at.0.saturating_sub(cursor.0);
        let Some(drop) = play_state.register_hold_drop(note_id, remaining_samples, sample_rate)
        else {
            return;
        };
        let anim = &mut self.character_anim;
        anim.player_note_miss_kind(lane, kind, cursor, sample_rate, play_state.bpm);
        if let Some(kind) = kind {
            play_note_kind_miss_sfx_or_warn(&self.mixer, kind);
        }
        anim.girlfriend_combo_drop(drop.combo_count, cursor);
        self.score_popups
            .push(Judgment::Miss, drop.combo_popup, cursor);
        set_vocals_gain(&self.mixer, 0.0);
        play_miss_sfx(&self.mixer, cursor, MissNoteKind::Scoreable);
    }

    pub(super) fn register_hold_tick(&mut self, elapsed_samples: i64) {
        let sample_rate = play_sample_rate(&self.mixer);
        if let Some(play_state) = self.play_state.as_mut() {
            play_state.register_hold_tick(elapsed_samples, sample_rate);
        }
    }
}
