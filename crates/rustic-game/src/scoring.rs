//! Scoring + input-matching impl block for `PlayState`.
//!
//! Lives next to `state.rs` to keep the data definition file lean while
//! still centralising the FNF gameplay math under one type.
//!
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:2990-3048
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:3065-3128
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:3135-3198
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:3292-3351
//! ref: bdedc0aa:source/funkin/util/GRhythmUtil.hx:54-111
// LINT-ALLOW: long-file scoring implementation plus focused unit tests

use crate::judgment::{
    combo_breaks, ghost_miss_health_delta, ghost_miss_score_delta, health_delta,
    hold_drop_score_delta, note_miss_score_delta, score_for_timing, Judgment, JudgmentWindows,
    HEALTH_HOLD_BONUS_PER_SECOND, SCORE_HOLD_BONUS_PER_SECOND,
};
use crate::note::Lane;
use crate::state::{PlayState, MAX_HEALTH};
use rustic_core::input::NormalizedInputEvent;
use rustic_core::time::Samples;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct HitOutcome {
    pub note_id: rustic_core::ids::NoteId,
    pub judgment: Judgment,
    pub is_sustain: bool,
    pub hold_end_at: Option<Samples>,
    pub combo_break: bool,
    pub combo_count: u32,
    pub combo_popup: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct HoldDropOutcome {
    pub score_delta: i64,
    pub combo_count: u32,
    pub combo_popup: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LateMissOutcome {
    pub lane: Lane,
    pub combo_count: u32,
}

#[derive(Debug, Clone, Copy)]
struct TimedHitRegistration {
    judgment: Judgment,
    combo_break: bool,
    combo_count: u32,
    combo_popup: Option<u32>,
}

impl PlayState {
    /// Resolve a player-side note hit at a given absolute timing diff and
    /// fold the result into the gameplay counters.
    pub fn register_hit(&mut self, abs_diff_ms: f64) -> Judgment {
        self.register_timed_hit(abs_diff_ms).judgment
    }

    fn register_timed_hit(&mut self, abs_diff_ms: f64) -> TimedHitRegistration {
        let windows: JudgmentWindows = self.windows.into();
        let judgment = windows.judge(abs_diff_ms);
        if judgment == Judgment::Miss {
            self.register_note_miss();
            return TimedHitRegistration {
                judgment,
                combo_break: true,
                combo_count: 0,
                combo_popup: None,
            };
        }

        let previous_combo = self.combo;
        let combo_count = previous_combo + 1;
        self.score += score_for_timing(abs_diff_ms);
        self.add_judgment_tally(judgment);
        self.apply_health(health_delta(judgment));

        let combo_break = combo_breaks(judgment);
        if combo_break {
            self.combo = 0;
        } else {
            self.combo += 1;
            if self.combo > self.max_combo {
                self.max_combo = self.combo;
            }
        }

        let combo_popup = if combo_break {
            (previous_combo >= 10).then_some(0)
        } else {
            (self.combo >= 10).then_some(self.combo)
        };

        TimedHitRegistration {
            judgment,
            combo_break,
            combo_count,
            combo_popup,
        }
    }

    fn add_judgment_tally(&mut self, judgment: Judgment) {
        match judgment {
            Judgment::Sick => self.sicks += 1,
            Judgment::Good => self.goods += 1,
            Judgment::Bad => self.bads += 1,
            Judgment::Shit => self.shits += 1,
            Judgment::Miss => self.misses += 1,
        }
    }

    /// Sustain child notes are still represented by the prototype's expanded
    /// chart model, but v0.8.5 hold score/health comes from `register_hold_tick`.
    pub fn register_sustain_hit(&mut self) {}

    /// Player pressed an empty lane or the wrong lane. v0.8.5 treats this as
    /// a ghost miss: score/health penalty, no combo break and no miss tally.
    pub fn register_ghost_miss(&mut self) {
        self.score += ghost_miss_score_delta();
        self.apply_health(ghost_miss_health_delta());
    }

    /// Player-side scoreable note left the hit window unhit.
    pub fn register_note_miss(&mut self) {
        self.score += note_miss_score_delta();
        self.combo = 0;
        self.misses += 1;
        self.apply_health(health_delta(Judgment::Miss));
    }

    /// Player released an already-hit hold before completion. v0.8.5 applies
    /// one remaining-duration score penalty and breaks combo without adding a
    /// miss tally.
    pub fn register_hold_drop(
        &mut self,
        note_id: rustic_core::ids::NoteId,
        remaining_samples: i64,
        sample_rate: u32,
    ) -> Option<HoldDropOutcome> {
        self.dropped_holds.insert(note_id, remaining_samples.max(0));
        let remaining_ms = remaining_samples.max(0) as f64 * 1000.0 / f64::from(sample_rate.max(1));
        let delta = hold_drop_score_delta(remaining_ms)?;
        let previous_combo = self.combo;
        self.score += delta;
        self.combo = 0;
        Some(HoldDropOutcome {
            score_delta: delta,
            combo_count: previous_combo,
            combo_popup: (previous_combo >= 10).then_some(0),
        })
    }

    /// v0.8.5 gives score and health every frame while a hit hold note still
    /// has remaining length.
    pub fn register_hold_tick(&mut self, elapsed_samples: i64, sample_rate: u32) -> i64 {
        if elapsed_samples <= 0 {
            return 0;
        }
        let elapsed_secs = elapsed_samples as f64 / f64::from(sample_rate.max(1));
        self.apply_health(HEALTH_HOLD_BONUS_PER_SECOND * elapsed_secs as f32);

        self.hold_score_carry += SCORE_HOLD_BONUS_PER_SECOND * elapsed_secs;
        let delta = self.hold_score_carry.trunc() as i64;
        self.hold_score_carry -= delta as f64;
        self.score += delta;
        delta
    }

    /// Try to consume a player keypress against the closest unresolved
    /// player-side note in `lane`. `event.audio_sample_cursor_at_receive`
    /// is the authoritative timestamp (PLAN.md Section 8/10).
    pub fn try_hit_in_lane(
        &mut self,
        event: &NormalizedInputEvent,
        lane: Lane,
        sample_rate: u32,
    ) -> Option<HitOutcome> {
        let hit_window = JudgmentWindows::from(self.windows).hit_window_ms.0;
        let cursor = event.audio_sample_cursor_at_receive;
        let ms_per_sample = 1000.0 / sample_rate as f64;

        let mut best: Option<(usize, f64)> = None;
        for (i, n) in self.notes.iter().enumerate() {
            if n.opponent || n.is_sustain || n.lane != lane {
                continue;
            }
            if self.resolved_notes.contains(&n.id) {
                continue;
            }
            let abs = ((n.hit_at.0 - cursor.0) as f64 * ms_per_sample).abs();
            if abs > hit_window {
                continue;
            }
            if best.map(|(_, b)| abs < b).unwrap_or(true) {
                best = Some((i, abs));
            }
        }

        let (idx, abs_diff_ms) = best?;
        let id = self.notes[idx].id;
        let hold_end_at = (self.notes[idx].sustain_samples > 0).then_some(Samples(
            self.notes[idx].hit_at.0 + self.notes[idx].sustain_samples,
        ));
        self.resolved_notes.push(id);

        let hit = self.register_timed_hit(abs_diff_ms);
        Some(HitOutcome {
            note_id: id,
            judgment: hit.judgment,
            is_sustain: false,
            hold_end_at,
            combo_break: hit.combo_break,
            combo_count: hit.combo_count,
            combo_popup: hit.combo_popup,
        })
    }

    /// Resolve generated player-side sustain children while a hit hold lane is
    /// held. They are bookkeeping only; active hold ticks own score/health.
    pub fn resolve_held_sustains_in_lane(
        &mut self,
        cursor: Samples,
        lane: Lane,
        sample_rate: u32,
    ) -> u32 {
        let hit_window = JudgmentWindows::from(self.windows).hit_window_ms.0;
        let ms_per_sample = 1000.0 / sample_rate as f64;
        let mut hits = Vec::new();

        for n in &self.notes {
            if n.opponent || !n.is_sustain || n.lane != lane {
                continue;
            }
            if self.resolved_notes.contains(&n.id) {
                continue;
            }
            if !self.is_active_sustain_child(n.id) {
                continue;
            }
            let diff_ms = (n.hit_at.0 - cursor.0) as f64 * ms_per_sample;
            if diff_ms > -hit_window && diff_ms < hit_window * 0.5 {
                hits.push(n.id);
            }
        }

        let count = hits.len() as u32;
        for id in hits {
            self.resolved_notes.push(id);
            self.register_sustain_hit();
        }
        count
    }

    /// Mark every player-side tap note whose hit_at is more than the hit
    /// window behind `cursor` as a miss. Sustain children are an internal
    /// trail approximation in this prototype; dropped holds are scored once
    /// by remaining duration, not as a miss per child.
    pub fn expire_late_notes(&mut self, cursor: Samples, sample_rate: u32) -> Vec<LateMissOutcome> {
        let hit_window = JudgmentWindows::from(self.windows).hit_window_ms.0;
        let ms_per_sample = 1000.0 / sample_rate as f64;
        let mut newly_missed = Vec::new();
        let mut expired_sustains = Vec::new();
        for n in &self.notes {
            if n.opponent || self.resolved_notes.contains(&n.id) {
                continue;
            }
            let diff_ms = (n.hit_at.0 - cursor.0) as f64 * ms_per_sample;
            if diff_ms < -hit_window {
                if n.is_sustain {
                    expired_sustains.push(n.id);
                } else {
                    newly_missed.push((n.id, n.lane));
                }
            }
        }
        self.resolved_notes.extend(expired_sustains);
        let mut outcomes = Vec::with_capacity(newly_missed.len());
        for (id, lane) in newly_missed {
            let combo_count = self.combo;
            self.resolved_notes.push(id);
            self.register_note_miss();
            outcomes.push(LateMissOutcome { lane, combo_count });
        }
        outcomes
    }

    pub(crate) fn apply_health(&mut self, delta: f32) {
        let next = self.health + delta;
        // Upstream caps at max health from above; below min health is allowed
        // and triggers game over via `is_dead`.
        self.health = if next > MAX_HEALTH { MAX_HEALTH } else { next };
    }

    fn is_active_sustain_child(&self, sustain_id: rustic_core::ids::NoteId) -> bool {
        let Some(child) = self.notes.iter().find(|note| note.id == sustain_id) else {
            return false;
        };
        self.notes.iter().any(|head| {
            !head.opponent
                && !head.is_sustain
                && head.lane == child.lane
                && head.sustain_samples > 0
                && self.resolved_notes.contains(&head.id)
                && child.hit_at > head.hit_at
                && child.hit_at.0 <= head.hit_at.0 + head.sustain_samples
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::judgment::score_for_timing;
    use crate::note::Note;
    use crate::state::INITIAL_HEALTH;
    use rustic_core::ids::NoteId;
    use rustic_core::input::{InputAction, InputState};

    fn input_at(cursor_samples: i64) -> NormalizedInputEvent {
        NormalizedInputEvent::new(
            InputAction::LaneLeft,
            InputState::Pressed,
            0,
            Samples(cursor_samples),
        )
    }

    fn add_note(s: &mut PlayState, id: u32, lane: Lane, hit_at_samples: i64) {
        s.notes.push(Note {
            id: NoteId::new(id),
            lane,
            hit_at: Samples(hit_at_samples),
            sustain_samples: 0,
            is_sustain: false,
            is_sustain_end: false,
            opponent: false,
        });
    }

    fn add_sustain(s: &mut PlayState, id: u32, lane: Lane, hit_at_samples: i64) {
        s.notes.push(Note {
            id: NoteId::new(id),
            lane,
            hit_at: Samples(hit_at_samples),
            sustain_samples: 0,
            is_sustain: true,
            is_sustain_end: true,
            opponent: false,
        });
    }

    fn add_hold_head(
        s: &mut PlayState,
        id: u32,
        lane: Lane,
        hit_at_samples: i64,
        sustain_samples: i64,
    ) {
        s.notes.push(Note {
            id: NoteId::new(id),
            lane,
            hit_at: Samples(hit_at_samples),
            sustain_samples,
            is_sustain: false,
            is_sustain_end: false,
            opponent: false,
        });
    }

    #[test]
    fn perfect_hit_awards_sick_and_max_score() {
        let mut s = PlayState::new();
        let j = s.register_hit(0.0);
        assert_eq!(j, Judgment::Sick);
        assert_eq!(s.score, 500);
        assert_eq!(s.combo, 1);
        assert_eq!(s.sicks, 1);
        assert!((s.health - (INITIAL_HEALTH + 0.03)).abs() < 1e-5);
    }

    #[test]
    fn note_miss_resets_combo_and_takes_health() {
        let mut s = PlayState::new();
        s.register_hit(0.0);
        s.register_hit(0.0);
        assert_eq!(s.combo, 2);
        s.register_note_miss();
        assert_eq!(s.combo, 0);
        assert_eq!(s.max_combo, 2);
        assert_eq!(s.misses, 1);
        assert_eq!(s.score, 900);
    }

    #[test]
    fn hold_drop_penalty_breaks_combo_without_miss_tally() {
        let mut s = PlayState::new();
        s.combo = 7;
        s.score = 500;

        let drop = s.register_hold_drop(NoteId::new(0), 48_000, 48_000);

        assert_eq!(drop.map(|drop| drop.score_delta), Some(-125));
        assert_eq!(s.score, 375);
        assert_eq!(s.combo, 0);
        assert_eq!(s.misses, 0);
    }

    #[test]
    fn tiny_hold_drop_has_no_penalty() {
        let mut s = PlayState::new();
        s.combo = 7;
        s.score = 500;

        let delta = s.register_hold_drop(NoteId::new(0), 7_680, 48_000);

        assert_eq!(delta, None);
        assert_eq!(s.score, 500);
        assert_eq!(s.combo, 7);
    }

    #[test]
    fn hold_drop_reports_zero_combo_popup_when_breaking_double_digit_combo() {
        let mut s = PlayState::new();
        s.combo = 10;

        let drop = s.register_hold_drop(NoteId::new(0), 48_000, 48_000);

        assert_eq!(drop.and_then(|drop| drop.combo_popup), Some(0));
    }

    #[test]
    fn hold_drop_reports_previous_combo_for_character_events() {
        let mut s = PlayState::new();
        s.combo = 70;

        let drop = s.register_hold_drop(NoteId::new(0), 48_000, 48_000);

        assert_eq!(drop.map(|drop| drop.combo_count), Some(70));
    }

    #[test]
    fn hold_tick_adds_continuous_score_and_health_with_fractional_carry() {
        let mut s = PlayState::new();

        let first = s.register_hold_tick(1_600, 48_000);
        let second = s.register_hold_tick(1_600, 48_000);
        let rest = s.register_hold_tick(44_800, 48_000);

        assert_eq!(first, 8);
        assert_eq!(second, 8);
        assert_eq!(rest, 234);
        assert_eq!(s.score, 250);
        assert!(s.hold_score_carry.abs() < 1e-9);
        assert!((s.health - (INITIAL_HEALTH + 0.12)).abs() < 1e-6);
        assert_eq!(s.combo, 0);
        assert_eq!(s.misses, 0);
    }

    #[test]
    fn ghost_miss_does_not_break_combo_or_increment_misses() {
        let mut s = PlayState::new();
        s.register_hit(0.0);
        s.register_ghost_miss();
        assert_eq!(s.combo, 1);
        assert_eq!(s.misses, 0);
        assert_eq!(s.score, 490);
        assert!((s.health - (INITIAL_HEALTH + 0.03 - 0.08)).abs() < 1e-6);
    }

    #[test]
    fn health_caps_at_two() {
        let mut s = PlayState::new();
        s.health = 1.99;
        s.register_hit(0.0);
        assert!((s.health - MAX_HEALTH).abs() < 1e-6);
    }

    #[test]
    fn enough_note_misses_kill_player() {
        let mut s = PlayState::new();
        for _ in 0..13 {
            s.register_note_miss();
        }
        assert!(s.is_dead());
    }

    #[test]
    fn late_note_miss_uses_note_miss_score_health_and_combo_break() {
        let mut s = PlayState::new();
        s.score = 1_000;
        s.combo = 7;

        s.register_note_miss();

        assert_eq!(s.score, 900);
        assert_eq!(s.combo, 0);
        assert_eq!(s.misses, 1);
        assert!((s.health - (INITIAL_HEALTH - 0.08)).abs() < 1e-6);
    }

    #[test]
    fn ratings_and_score_match_pbot1_thresholds() {
        let mut s = PlayState::new();
        assert_eq!(s.register_hit(46.0), Judgment::Good);
        assert_eq!(s.score, score_for_timing(46.0));
        assert_eq!(s.register_hit(91.0), Judgment::Bad);
        assert_eq!(s.score, score_for_timing(46.0) + score_for_timing(91.0));
        assert_eq!(s.combo, 0);
        assert_eq!(s.register_hit(136.0), Judgment::Shit);
        assert_eq!(
            s.score,
            score_for_timing(46.0) + score_for_timing(91.0) + score_for_timing(136.0)
        );
    }

    #[test]
    fn input_cursor_drives_judgment() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        let j = s.try_hit_in_lane(&input_at(48_000), Lane::Left, 48_000);
        assert_eq!(j.map(|outcome| outcome.judgment), Some(Judgment::Sick));
        assert_eq!(s.resolved_notes.len(), 1);
    }

    #[test]
    fn hit_outcome_reports_next_combo_for_character_events() {
        let mut s = PlayState::new();
        s.combo = 49;
        add_note(&mut s, 0, Lane::Left, 48_000);

        let j = s.try_hit_in_lane(&input_at(48_000), Lane::Left, 48_000);

        assert_eq!(j.map(|outcome| outcome.combo_count), Some(50));
    }

    #[test]
    fn hold_head_hit_reports_hold_end_cursor() {
        let mut s = PlayState::new();
        add_hold_head(&mut s, 0, Lane::Left, 48_000, 24_000);

        let j = s.try_hit_in_lane(&input_at(48_000), Lane::Left, 48_000);

        assert_eq!(
            j.and_then(|outcome| outcome.hold_end_at),
            Some(Samples(72_000))
        );
    }

    #[test]
    fn input_offset_in_samples_resolves_to_correct_rating() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        // 4800 samples late = 100ms @ 48kHz -> Bad in PBOT1.
        let j = s.try_hit_in_lane(&input_at(48_000 + 4_800), Lane::Left, 48_000);
        assert_eq!(j.map(|outcome| outcome.judgment), Some(Judgment::Bad));
    }

    #[test]
    fn input_outside_hit_window_is_no_hit() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        let j = s.try_hit_in_lane(&input_at(48_000 - 9_600), Lane::Left, 48_000);
        assert_eq!(j, None);
        assert_eq!(s.resolved_notes.len(), 0);
    }

    #[test]
    fn input_picks_closest_note_in_lane() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        add_note(&mut s, 1, Lane::Left, 48_000 + 4_800);
        s.try_hit_in_lane(&input_at(48_050), Lane::Left, 48_000);
        assert!(s.resolved_notes.contains(&NoteId::new(0)));
        assert!(!s.resolved_notes.contains(&NoteId::new(1)));
    }

    #[test]
    fn input_ignores_other_lanes_and_opponent_notes() {
        let mut s = PlayState::new();
        s.notes.push(Note {
            id: NoteId::new(0),
            lane: Lane::Right,
            hit_at: Samples(48_000),
            sustain_samples: 0,
            is_sustain: false,
            is_sustain_end: false,
            opponent: false,
        });
        s.notes.push(Note {
            id: NoteId::new(1),
            lane: Lane::Left,
            hit_at: Samples(48_000),
            sustain_samples: 0,
            is_sustain: false,
            is_sustain_end: false,
            opponent: true,
        });
        let j = s.try_hit_in_lane(&input_at(48_000), Lane::Left, 48_000);
        assert_eq!(j, None);
    }

    #[test]
    fn direct_input_does_not_score_sustain_children() {
        let mut s = PlayState::new();
        add_sustain(&mut s, 0, Lane::Left, 48_000);

        let j = s.try_hit_in_lane(&input_at(48_000), Lane::Left, 48_000);

        assert_eq!(j, None);
        assert_eq!(s.score, 0);
        assert_eq!(s.combo, 0);
        assert_eq!(s.sicks, 0);
        assert!(!s.resolved_notes.contains(&NoteId::new(0)));
        assert!((s.health - INITIAL_HEALTH).abs() < 1e-5);
    }

    #[test]
    fn held_lane_resolves_sustain_children_after_head_hit() {
        let mut s = PlayState::new();
        add_hold_head(&mut s, 0, Lane::Left, 48_000, 48_000);
        add_sustain(&mut s, 1, Lane::Left, 62_400);
        add_sustain(&mut s, 2, Lane::Left, 76_800);
        s.resolved_notes.push(NoteId::new(0));

        let count = s.resolve_held_sustains_in_lane(Samples(62_400), Lane::Left, 48_000);

        assert_eq!(count, 1);
        assert!(s.resolved_notes.contains(&NoteId::new(1)));
        assert!(!s.resolved_notes.contains(&NoteId::new(2)));
        assert_eq!(s.score, 0);
        assert_eq!(s.combo, 0);
        assert!((s.health - INITIAL_HEALTH).abs() < 1e-6);
    }

    #[test]
    fn held_lane_does_not_resolve_sustain_children_when_head_was_not_hit() {
        let mut s = PlayState::new();
        add_hold_head(&mut s, 0, Lane::Left, 48_000, 48_000);
        add_sustain(&mut s, 1, Lane::Left, 62_400);

        let count = s.resolve_held_sustains_in_lane(Samples(62_400), Lane::Left, 48_000);

        assert_eq!(count, 0);
        assert!(!s.resolved_notes.contains(&NoteId::new(1)));
        assert_eq!(s.score, 0);
        assert!((s.health - INITIAL_HEALTH).abs() < 1e-6);
    }

    #[test]
    fn expire_late_notes_misses_unhit_player_notes_past_hit_window() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        add_note(&mut s, 1, Lane::Left, 48_000 + 96_000);
        s.score = 500;
        s.combo = 3;
        let cursor = Samples(48_000 + 8_000);
        let missed = s.expire_late_notes(cursor, 48_000);
        assert_eq!(missed.len(), 1);
        assert_eq!(missed[0].lane, Lane::Left);
        assert_eq!(missed[0].combo_count, 3);
        assert!(s.resolved_notes.contains(&NoteId::new(0)));
        assert_eq!(s.misses, 1);
        assert_eq!(s.score, 400);
        assert_eq!(s.combo, 0);
        assert!((s.health - (INITIAL_HEALTH - 0.08)).abs() < 1e-6);
    }

    #[test]
    fn expire_late_notes_reports_combo_after_prior_same_frame_miss() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        add_note(&mut s, 1, Lane::Right, 48_000 + 1);
        s.combo = 12;

        let missed = s.expire_late_notes(Samples(48_000 + 8_000), 48_000);

        assert_eq!(
            missed,
            vec![
                LateMissOutcome {
                    lane: Lane::Left,
                    combo_count: 12,
                },
                LateMissOutcome {
                    lane: Lane::Right,
                    combo_count: 0,
                },
            ]
        );
        assert_eq!(s.combo, 0);
        assert_eq!(s.misses, 2);
    }

    #[test]
    fn expire_late_notes_silently_resolves_sustain_children() {
        let mut s = PlayState::new();
        add_sustain(&mut s, 0, Lane::Left, 48_000);
        s.score = 500;
        s.combo = 3;

        let missed = s.expire_late_notes(Samples(48_000 + 8_000), 48_000);

        assert!(missed.is_empty());
        assert!(s.resolved_notes.contains(&NoteId::new(0)));
        assert_eq!(s.misses, 0);
        assert_eq!(s.score, 500);
        assert_eq!(s.combo, 3);
        assert!((s.health - INITIAL_HEALTH).abs() < 1e-6);
    }
}
