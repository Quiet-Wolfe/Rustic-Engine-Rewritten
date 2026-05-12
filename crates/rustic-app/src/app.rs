use crate::active_holds::ActiveHolds;
// LINT-ALLOW: long-file app owns prototype runtime wiring until screen modules split.
use crate::app_runtime::create_runtime as create_app_runtime;
use crate::app_text::preview_text_commands;
use crate::app_types::{AppOptions, Runtime};
use crate::audio_fallback::open_audio_output_or_fallback;
use crate::bitmap_text_assets::BitmapTextSkin;
use crate::boot::{init_logging, install_panic_hook};
use crate::camera_events::{apply_camera_event, focus_initial_camera as focus_initial};
use crate::camera_fx::CameraFx;
use crate::character_anim::CharacterAnimState;
use crate::countdown_assets::{countdown_start_cursor, CountdownSkin};
use crate::countdown_audio::CountdownAudio;
use crate::game_over::GameOverState;
use crate::hold_cover_assets::{HoldCoverSkin, HoldCovers};
use crate::hud_assets::HudSkin;
use crate::hud_bop::health_icon_scale;
use crate::input_bridge::{build_event, map_key};
use crate::lane_state::{lane_for_action, AutoReceptors, HeldLanes};
use crate::miss_note_audio::{play_miss_note_or_warn as play_miss_sfx, MissNoteKind};
use crate::note_assets::{confirm_duration_or_default, NoteSkin};
use crate::note_splash_assets::{NoteSplashSkin, NoteSplashes};
use crate::popup_assets::{PopupSkin, ScorePopups};
use crate::preview_song::PreviewSelection;
use crate::scene_assets::{
    load_preview_play_state_for, load_preview_scene_for, CameraFocusPoints, CharacterSet,
    LoadedScene,
};
use crate::screen::ScreenStack;
use crate::song_audio::{load_preview_stems_for, play_sample_rate, set_vocals_gain};
use crate::stage_object_assets::StagePropSet;
use crate::title_assets::TitleScreenAssets;
use anyhow::Result;
use rustic_asset::ChartEventKind;
use rustic_audio::{AudioOutput, SharedMixer};
use rustic_core::ids::AssetId;
use rustic_core::input::{InputAction, InputState, NormalizedInputEvent};
use rustic_core::time::Samples;
use rustic_game::{Judgment, Lane, PlayState};
use rustic_render::{CameraRegistry, RenderCommandList, SpriteBatcher, TextCommandList, Texture};
use std::collections::HashMap;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};

mod debug_overlay;
mod redraw;
mod title_flow;

use title_flow::AppMode;

struct App {
    options: AppOptions,
    boot_instant: Instant,
    mixer: SharedMixer,
    audio_output: Option<AudioOutput>,
    cameras: CameraRegistry,
    camera_fx: CameraFx,
    camera_focus: CameraFocusPoints,
    base_camera_zoom: f32,
    static_cmds: RenderCommandList,
    stage_props: StagePropSet,
    cmds: RenderCommandList,
    text_cmds: TextCommandList,
    atlases: HashMap<AssetId, Texture>,
    characters: Option<CharacterSet>,
    bitmap_text_skin: Option<BitmapTextSkin>,
    character_anim: CharacterAnimState,
    note_skin: Option<NoteSkin>,
    note_splash_skin: Option<NoteSplashSkin>,
    hold_cover_skin: Option<HoldCoverSkin>,
    hud_skin: Option<HudSkin>,
    popup_skin: Option<PopupSkin>,
    countdown_skin: Option<CountdownSkin>,
    countdown_audio: CountdownAudio,
    score_popups: ScorePopups,
    note_splashes: NoteSplashes,
    hold_covers: HoldCovers,
    active_holds: ActiveHolds,
    held_lanes: HeldLanes,
    opponent_receptors: AutoReceptors,
    preview_selection: PreviewSelection,
    mode: AppMode,
    title_assets: Option<TitleScreenAssets>,
    title_start: Instant,
    play_state: Option<PlayState>,
    song_start: Instant,
    song_start_cursor: Samples,
    song_started: bool,
    game_over: Option<GameOverState>,
    debug_overlay: bool,
    last_frame_at: Instant,
    debug_fps: f32,
    batcher: SpriteBatcher,
    screens: ScreenStack,
    runtime: Option<Runtime>,
}

impl App {
    fn new(options: AppOptions) -> Self {
        let now = Instant::now();
        let (audio_output, mixer) = open_audio_output_or_fallback();
        Self {
            options,
            boot_instant: now,
            mixer,
            audio_output,
            cameras: CameraRegistry::with_default_fnf(),
            camera_fx: CameraFx::default(),
            camera_focus: CameraFocusPoints::default(),
            base_camera_zoom: 1.0,
            static_cmds: RenderCommandList::new(),
            stage_props: StagePropSet::default(),
            cmds: RenderCommandList::new(),
            text_cmds: TextCommandList::new(),
            atlases: HashMap::new(),
            characters: None,
            bitmap_text_skin: None,
            character_anim: CharacterAnimState::default(),
            note_skin: None,
            note_splash_skin: None,
            hold_cover_skin: None,
            hud_skin: None,
            popup_skin: None,
            countdown_skin: None,
            countdown_audio: CountdownAudio::default(),
            score_popups: ScorePopups::default(),
            note_splashes: NoteSplashes::default(),
            hold_covers: HoldCovers::default(),
            active_holds: ActiveHolds::default(),
            held_lanes: HeldLanes::default(),
            opponent_receptors: AutoReceptors::default(),
            preview_selection: PreviewSelection::from_env(),
            mode: AppMode::Title,
            title_assets: None,
            title_start: now,
            play_state: None,
            song_start: now,
            song_start_cursor: Samples(0),
            song_started: false,
            game_over: None,
            debug_overlay: false,
            last_frame_at: now,
            debug_fps: 0.0,
            batcher: SpriteBatcher::new(),
            screens: ScreenStack::new(),
            runtime: None,
        }
    }
    fn create_runtime(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        self.runtime = Some(create_app_runtime(&self.options, event_loop)?);
        self.load_title_screen();
        Ok(())
    }
    fn apply_scene(&mut self, scene: LoadedScene) {
        let anim_timings = scene
            .characters
            .as_ref()
            .map(CharacterSet::anim_timings)
            .unwrap_or_default();
        self.cmds = scene.commands;
        self.static_cmds = self.cmds.clone();
        self.stage_props = scene.stage_props;
        self.atlases = scene.textures;
        self.characters = scene.characters;
        self.character_anim.set_timings(anim_timings);
        self.bitmap_text_skin = scene.bitmap_text_skin;
        self.note_skin = scene.note_skin;
        self.note_splash_skin = scene.note_splash_skin;
        self.hold_cover_skin = scene.hold_cover_skin;
        self.hud_skin = scene.hud_skin;
        self.popup_skin = scene.popup_skin;
        self.countdown_skin = scene.countdown_skin;
        self.countdown_audio = CountdownAudio::load_default_or_warn(self.audio_output.is_some());
        self.camera_focus = scene.camera_focus;
        self.base_camera_zoom = scene.camera_zoom;
        self.camera_fx.reset(&mut self.cameras, scene.camera_zoom);
        focus_initial(&mut self.cameras, &mut self.camera_fx, self.camera_focus);
    }
    fn load_selected_scene(&mut self) {
        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        let scene = load_preview_scene_for(
            &runtime.rs.device,
            &runtime.rs.queue,
            self.preview_selection,
        );
        match scene {
            Ok(scene) => self.apply_scene(scene),
            Err(e) => tracing::warn!(target: "rustic.asset", "preview scene unavailable: {e:#}"),
        }
    }
    fn load_selected_song(&mut self) {
        self.update_window_title();
        self.load_selected_scene();
        let sample_rate = play_sample_rate(&self.mixer);
        match load_preview_play_state_for(self.preview_selection, sample_rate) {
            Ok(play_state) => {
                tracing::info!(
                    target: "rustic",
                    "loading {} ({})",
                    self.preview_selection.song.display_name(),
                    self.preview_selection.difficulty.as_str()
                );
                let bpm = play_state.bpm;
                self.play_state = Some(play_state);
                self.reset_song_runtime(bpm);
                self.load_selected_stems();
                self.rebuild_frame_commands();
            }
            Err(e) => tracing::warn!(target: "rustic.asset", "preview chart unavailable: {e:#}"),
        }
    }
    fn update_window_title(&self) {
        if let Some(runtime) = self.runtime.as_ref() {
            runtime.window.set_title(&format!(
                "{} - {} [{}]",
                self.options.title,
                self.preview_selection.song.display_name(),
                self.preview_selection.difficulty.as_str()
            ));
        }
    }
    fn load_selected_stems(&mut self) {
        if self.audio_output.is_none() {
            tracing::warn!(target: "rustic.audio", "preview stems skipped: no audio output");
            return;
        }
        if let Err(e) = load_preview_stems_for(&self.mixer, self.preview_selection, Samples(0)) {
            tracing::warn!(target: "rustic.audio", "preview stems unavailable: {e:#}");
        } else if let Err(e) = self.mixer.edit(|mixer| {
            mixer.pause();
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "pause countdown audio: {e:#}");
        }
    }
    fn reset_song_runtime(&mut self, bpm: f64) {
        self.song_start_cursor = countdown_start_cursor(play_sample_rate(&self.mixer), bpm);
        self.song_start = Instant::now();
        self.song_started = false;
        self.game_over = None;
        self.countdown_audio.reset();
        self.character_anim.reset_song();
        (
            self.score_popups,
            self.note_splashes,
            self.hold_covers,
            self.active_holds,
            self.held_lanes,
            self.opponent_receptors,
        ) = Default::default();
        self.camera_fx
            .reset(&mut self.cameras, self.base_camera_zoom);
        focus_initial(&mut self.cameras, &mut self.camera_fx, self.camera_focus);
        set_vocals_gain(&self.mixer, 1.0);
    }
    fn handle_preview_selection_input(&mut self, action: InputAction) -> bool {
        self.preview_selection = match action {
            InputAction::UiLeft => self.preview_selection.next_song(),
            InputAction::UiRight => self.preview_selection.next_difficulty(),
            _ => return false,
        };
        if self.mode == AppMode::Title {
            self.update_window_title();
        } else {
            self.load_selected_song();
        }
        true
    }
    fn rebuild_frame_commands(&mut self) {
        if self.mode == AppMode::Title {
            self.rebuild_title_commands();
            return;
        }
        if self.mode == AppMode::SongSelect {
            self.rebuild_song_select_commands();
            return;
        }
        self.text_cmds = preview_text_commands(self.preview_selection);
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = if let Some(game_over) = self.game_over {
            game_over.cursor(sample_rate)
        } else {
            self.advance_song_clock()
        };
        self.append_debug_overlay_commands(cursor, sample_rate);
        if self.game_over.is_some() {
            self.rebuild_game_over_commands(cursor, sample_rate);
            return;
        }
        let mut opponent_hits = Vec::new();
        let mut song_events = Vec::new();
        let mut bpm = None;
        let mut late_misses = Vec::new();
        let mut dead = false;
        let confirm_duration = confirm_duration_or_default(self.note_skin.as_ref(), sample_rate);
        let hold_ticks = self.active_holds.score_ticks(cursor);
        self.opponent_receptors.update(cursor, confirm_duration);
        for lane in self.active_holds.complete_elapsed(cursor) {
            if self.held_lanes.is_held(lane) {
                self.held_lanes.complete_hold(lane);
            } else {
                self.held_lanes.play_static(lane);
            }
        }
        let active_hold_lanes: Vec<_> = self.active_holds.active_lanes(cursor).collect();
        for lane in active_hold_lanes {
            self.held_lanes.hold_confirm(lane, cursor, confirm_duration);
        }
        if let Some(play_state) = self.play_state.as_mut() {
            for tick in hold_ticks {
                play_state.register_hold_tick(tick.elapsed_samples, sample_rate);
            }
            song_events = play_state.resolve_song_events(cursor);
            opponent_hits = play_state.resolve_opponent_notes(cursor);
            let held_lanes: Vec<_> = self.held_lanes.active_lanes().collect();
            for lane in held_lanes {
                if !self.active_holds.active_lanes(cursor).any(|l| l == lane) {
                    if let Some((note_id, hold_end_at)) =
                        play_state.pickup_hold_in_lane(cursor, lane, sample_rate)
                    {
                        self.active_holds.start(lane, hold_end_at, cursor, note_id);
                        self.held_lanes.hold_confirm(lane, cursor, confirm_duration);
                        self.hold_covers.start(lane, cursor, hold_end_at);
                    }
                }
            }
            late_misses = play_state.expire_late_notes(cursor, sample_rate);
            dead = play_state.is_dead();
            bpm = Some(play_state.bpm);
        }
        if dead {
            self.enter_game_over(cursor);
            self.rebuild_game_over_commands(cursor, sample_rate);
            return;
        }
        if let Some(bpm) = bpm {
            let anim = &mut self.character_anim;
            let had_late_misses = !late_misses.is_empty();
            for miss in late_misses {
                anim.player_note_miss(miss.lane, cursor, sample_rate, bpm);
                anim.girlfriend_combo_drop(miss.combo_count, cursor);
                play_miss_sfx(&self.mixer, cursor, MissNoteKind::Scoreable);
            }
            if had_late_misses {
                set_vocals_gain(&self.mixer, 0.0);
            }
            let had_opponent_hits = !opponent_hits.is_empty();
            let opponent_receptors = &mut self.opponent_receptors;
            for hit in opponent_hits {
                opponent_receptors.confirm(hit.lane, cursor, confirm_duration);
                anim.opponent_note_hit(hit.lane, cursor, sample_rate, bpm);
                if let Some(hold_end_at) = hit.hold_end_at {
                    opponent_receptors.start_hold(hit.lane, hold_end_at, cursor, hit.note_id);
                    self.hold_covers
                        .start_opponent(hit.lane, cursor, hold_end_at);
                }
            }
            if had_opponent_hits {
                self.camera_fx.enable_zooming();
                set_vocals_gain(&self.mixer, 1.0);
            }
            for event in song_events {
                self.apply_song_event(&event.kind, cursor, sample_rate, bpm);
            }
            self.character_anim.update(
                cursor,
                sample_rate,
                bpm,
                self.held_lanes.active_lanes().next().is_some(),
            );
            if !self.song_started {
                self.countdown_audio
                    .tick_or_warn(&self.mixer, cursor, sample_rate, bpm);
            }
            self.camera_fx
                .update(&mut self.cameras, cursor, sample_rate, bpm);
        }
        self.cmds = self.static_cmds.clone();
        for cmd in self.stage_props.commands(cursor, sample_rate) {
            self.cmds.push(cmd);
        }
        if let Some(characters) = &self.characters {
            for cmd in characters.commands(self.character_anim.poses(), cursor, sample_rate) {
                self.cmds.push(cmd);
            }
        }
        let (Some(play_state), Some(note_skin)) = (&self.play_state, &self.note_skin) else {
            return;
        };
        for view in play_state.hold_trail_views(cursor, sample_rate, |lane, opp| {
            if opp {
                true
            } else {
                self.held_lanes.is_held(lane)
            }
        }) {
            if view.head_resolved && !view.opponent && !self.held_lanes.is_held(view.lane) {
                continue;
            }
            for cmd in note_skin.hold_trail_commands(&view) {
                if cmd.world_pos.y + cmd.size.y >= -200.0 {
                    self.cmds.push(cmd);
                }
            }
        }
        for cmd in note_skin.receptor_commands(cursor, sample_rate, |player, lane| match player {
            1 => self.held_lanes.receptor_state(lane, cursor),
            _ => self.opponent_receptors.receptor_state(lane, cursor),
        }) {
            self.cmds.push(cmd);
        }
        for view in play_state.note_views(cursor, sample_rate) {
            if view.is_sustain {
                continue;
            }
            let cmd = note_skin.command_for_view(&view);
            if cmd.world_pos.y + cmd.size.y >= -200.0 {
                self.cmds.push(cmd);
            }
        }
        if let Some(note_splash_skin) = &self.note_splash_skin {
            for cmd in self
                .note_splashes
                .commands(note_splash_skin, cursor, sample_rate)
            {
                self.cmds.push(cmd);
            }
        }
        if let Some(hold_cover_skin) = &self.hold_cover_skin {
            for cmd in self
                .hold_covers
                .commands(hold_cover_skin, cursor, sample_rate)
            {
                self.cmds.push(cmd);
            }
        }
        if let Some(hud_skin) = &self.hud_skin {
            for cmd in hud_skin.commands_with_icon_scale(
                play_state.health,
                health_icon_scale(cursor, sample_rate, play_state.bpm),
            ) {
                self.cmds.push(cmd);
            }
        }
        if let Some(bitmap_text_skin) = &self.bitmap_text_skin {
            for cmd in bitmap_text_skin.score_text_commands(play_state.score) {
                self.cmds.push(cmd);
            }
        }
        if let Some(popup_skin) = &self.popup_skin {
            for cmd in self
                .score_popups
                .commands(popup_skin, cursor, sample_rate, play_state.bpm)
            {
                self.cmds.push(cmd);
            }
        }
        if let Some(countdown_skin) = &self.countdown_skin {
            for cmd in countdown_skin.commands(cursor, sample_rate, play_state.bpm) {
                self.cmds.push(cmd);
            }
        }
    }
    fn apply_song_event(
        &mut self,
        kind: &ChartEventKind,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
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
        if let ChartEventKind::PlayAnimation {
            target,
            animation,
            force,
        } = kind
        {
            self.character_anim
                .play_chart_animation(target, animation, cursor, *force);
        }
    }
    fn handle_gameplay_input(&mut self, event: &NormalizedInputEvent, already_held: bool) {
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
                should_enter_game_over = true;
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
                    anim.player_note_hit(lane, cursor, sample_rate, play_state.bpm);
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
                should_enter_game_over = play_state.is_dead();
            }
        }
        if restore_vocals {
            set_vocals_gain(&self.mixer, 1.0);
        }
        if should_enter_game_over {
            self.enter_game_over(cursor);
        }
    }
    fn register_hold_drop(
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
        let remaining_samples = hold_end_at.0.saturating_sub(cursor.0);
        let Some(drop) = play_state.register_hold_drop(note_id, remaining_samples, sample_rate)
        else {
            return;
        };
        let anim = &mut self.character_anim;
        anim.player_note_miss(lane, cursor, sample_rate, play_state.bpm);
        anim.girlfriend_combo_drop(drop.combo_count, cursor);
        self.score_popups
            .push(Judgment::Miss, drop.combo_popup, cursor);
        set_vocals_gain(&self.mixer, 0.0);
        play_miss_sfx(&self.mixer, cursor, MissNoteKind::Scoreable);
    }
    fn register_hold_tick(&mut self, elapsed_samples: i64) {
        let sample_rate = play_sample_rate(&self.mixer);
        if let Some(play_state) = self.play_state.as_mut() {
            play_state.register_hold_tick(elapsed_samples, sample_rate);
        }
    }
    fn restart_song_after_game_over(&mut self) {
        // ref: bdedc0aa:source/funkin/play/GameOverSubState.hx:409-424
        if self.game_over.take().is_none() {
            return;
        }
        if let Some(play_state) = self.play_state.as_mut() {
            play_state.restart();
        }
        let bpm = self.play_state.as_ref().map_or(100.0, |state| state.bpm);
        self.reset_song_runtime(bpm);
        if let Err(e) = self.mixer.edit(|mixer| {
            mixer.seek(Samples(0))?;
            mixer.pause();
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "reset game over audio: {e:#}");
        }
    }
    fn advance_song_clock(&mut self) -> Samples {
        if !self.song_started {
            let elapsed = self.song_start.elapsed().as_secs_f64();
            let elapsed_samples =
                (elapsed * f64::from(play_sample_rate(&self.mixer))).round() as i64;
            let cursor = Samples(self.song_start_cursor.0 + elapsed_samples);
            if cursor.0 < 0 {
                return cursor;
            }
            self.song_started = true;
            self.song_start = Instant::now();
            if let Err(e) = self.mixer.edit(|mixer| {
                mixer.seek(Samples(0))?;
                mixer.resume();
                Ok(())
            }) {
                tracing::warn!(target: "rustic.audio", "start countdown audio: {e:#}");
            }
            return Samples(0);
        }

        if self.audio_output.is_some() {
            return self.mixer.sample_cursor();
        }
        let elapsed = self.song_start.elapsed().as_secs_f64();
        let elapsed_samples = (elapsed * f64::from(play_sample_rate(&self.mixer))).round() as i64;
        Samples(elapsed_samples)
    }
    fn enter_game_over(&mut self, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/PlayState.hx:1441-1472
        if self.game_over.is_some() {
            return;
        }
        let sample_rate = play_sample_rate(&self.mixer);
        let loop_after = self
            .characters
            .as_ref()
            .and_then(|characters| characters.player_animation_duration("firstDeath", sample_rate))
            .unwrap_or(Samples(i64::from(sample_rate)));
        self.character_anim.player_first_death(cursor);
        if let Some(characters) = &self.characters {
            let (target, zoom) = characters.player_game_over_camera(self.base_camera_zoom);
            self.camera_fx
                .focus_game_over_camera(&mut self.cameras, target, zoom);
        }
        (
            self.held_lanes,
            self.opponent_receptors,
            self.hold_covers,
            self.active_holds,
        ) = Default::default();
        set_vocals_gain(&self.mixer, 0.0);
        if let Err(e) = self.mixer.edit(|mixer| {
            mixer.pause();
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "pause game over audio: {e:#}");
        }
        self.game_over = Some(GameOverState::new(cursor, loop_after));
    }
    fn rebuild_game_over_commands(&mut self, cursor: Samples, sample_rate: u32) {
        let Some(game_over) = self.game_over.as_mut() else {
            return;
        };
        if let Some(loop_at) = game_over.start_loop_if_due(cursor) {
            self.character_anim.player_death_loop(loop_at);
        }
        self.camera_fx
            .update(&mut self.cameras, cursor, sample_rate, 100.0);

        self.cmds.clear();
        if let Some(characters) = &self.characters {
            for cmd in
                characters.player_commands(self.character_anim.poses().player, cursor, sample_rate)
            {
                self.cmds.push(cmd);
            }
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.runtime.is_some() {
            return;
        }
        if let Err(e) = self.create_runtime(event_loop) {
            tracing::error!(target: "rustic", "failed to bring up renderer: {e:#}");
            event_loop.exit();
        }
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.handle_resize(size.width, size.height),
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(action) = map_key(event.physical_key) {
                    let song_cursor = self.input_cursor();
                    let evt = build_event(action, event.state, self.boot_instant, song_cursor);
                    let already_held = event.repeat
                        || lane_for_action(action)
                            .map(|lane| self.held_lanes.is_held(lane))
                            .unwrap_or(false);
                    if event.state == ElementState::Pressed && action == InputAction::Debug {
                        self.toggle_debug_overlay();
                        return;
                    }
                    if self.handle_mode_input(action, event.state, event_loop) {
                        return;
                    }
                    if event.state == ElementState::Pressed
                        && self.handle_preview_selection_input(action)
                    {
                        return;
                    }
                    if event.state == ElementState::Pressed
                        && action == InputAction::Confirm
                        && self.game_over.is_some()
                    {
                        self.restart_song_after_game_over();
                    }
                    if event.state == ElementState::Pressed
                        && action == InputAction::Back
                        && self.game_over.is_some()
                    {
                        self.enter_song_select();
                        return;
                    }
                    self.held_lanes.apply(&evt);
                    if event.state == ElementState::Released {
                        if let Some(lane) = lane_for_action(action) {
                            if let Some(release) = self.active_holds.release(lane, song_cursor) {
                                self.register_hold_tick(release.elapsed_samples);
                                if release.hold_end_at > song_cursor {
                                    self.register_hold_drop(
                                        lane,
                                        song_cursor,
                                        release.hold_end_at,
                                        release.note_id,
                                    );
                                }
                            }
                            self.hold_covers.end(lane, song_cursor);
                            self.held_lanes.play_static(lane);
                        }
                    }
                    self.screens.input(&evt);
                    self.handle_gameplay_input(&evt, already_held);
                    if event.state == ElementState::Pressed && action == InputAction::Back {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                if let Some(rt) = self.runtime.as_ref() {
                    rt.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

pub fn run(options: AppOptions) -> Result<()> {
    init_logging();
    install_panic_hook();
    tracing::info!(target: "rustic", "starting RusticV3 ({}x{})", options.width, options.height);

    let event_loop = EventLoop::new()?;
    let mut app = App::new(options);
    event_loop.run_app(&mut app)?;
    Ok(())
}
