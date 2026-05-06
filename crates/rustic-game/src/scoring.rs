//! Scoring + input-matching impl block for `PlayState`.
//!
//! Lives next to `state.rs` to keep the data definition file lean while
//! still centralising the FNF gameplay math under one type.
//!
//! ref: 50fccded:source/PlayState.hx:1684-1716   // popUpScore
//! ref: 50fccded:source/PlayState.hx:1574-1577   // late note health penalty
//! ref: 50fccded:source/PlayState.hx:2028-2042   // noteMiss
//! ref: 50fccded:source/PlayState.hx:1961-1977   // held sustain goodNoteHit
//! ref: 50fccded:source/PlayState.hx:2096-2139   // goodNoteHit
//! ref: 50fccded:source/Note.hx:172-184          // canBeHit / tooLate
// LINT-ALLOW: long-file scoring implementation plus focused unit tests

use crate::judgment::{
    health_delta, late_note_health_delta, score_value, Judgment, JudgmentWindows,
};
use crate::note::Lane;
use crate::state::{PlayState, MAX_HEALTH};
use rustic_core::input::NormalizedInputEvent;
use rustic_core::time::Samples;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct HitOutcome {
    pub judgment: Judgment,
    pub is_sustain: bool,
}

impl PlayState {
    /// Resolve a player-side note hit at a given absolute timing diff and
    /// fold the result into the gameplay counters.
    pub fn register_hit(&mut self, abs_diff_ms: f64) -> Judgment {
        let windows: JudgmentWindows = self.windows.into();
        let j = windows.judge(abs_diff_ms);
        self.score += score_value(j);
        self.combo += 1;
        if self.combo > self.max_combo {
            self.max_combo = self.combo;
        }
        match j {
            Judgment::Sick => self.sicks += 1,
            Judgment::Good => self.goods += 1,
            Judgment::Bad => self.bads += 1,
            Judgment::Shit => self.shits += 1,
            Judgment::Miss => self.misses += 1,
        }
        self.apply_health(health_delta(j));
        j
    }

    /// Sustain child notes do not call `popUpScore` and do not increment
    /// combo in base FNF, but they do receive the normal hit health.
    /// ref: 50fccded:source/PlayState.hx:2098-2139
    pub fn register_sustain_hit(&mut self) {
        self.apply_health(health_delta(Judgment::Sick));
    }

    /// Player pressed an empty lane or the wrong lane. This mirrors
    /// `noteMiss`; late unhit notes use `register_late_note_miss`.
    /// ref: 50fccded:source/PlayState.hx:2028-2042
    pub fn register_miss(&mut self) {
        self.score += score_value(Judgment::Miss);
        self.combo = 0;
        self.misses += 1;
        self.apply_health(health_delta(Judgment::Miss));
    }

    /// Player-side note became too late/unhit. Base FNF's offscreen note
    /// path only reduces health and mutes vocals; score/combo are not
    /// touched by this branch.
    /// ref: 50fccded:source/PlayState.hx:1574-1577
    pub fn register_late_note_miss(&mut self) {
        self.misses += 1;
        self.apply_health(late_note_health_delta());
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
        let safe_zone = JudgmentWindows::from(self.windows).safe_zone_ms.0;
        let cursor = event.audio_sample_cursor_at_receive;
        let ms_per_sample = 1000.0 / sample_rate as f64;

        let mut best: Option<(usize, f64)> = None;
        for (i, n) in self.notes.iter().enumerate() {
            if n.opponent || n.lane != lane {
                continue;
            }
            if self.resolved_notes.contains(&n.id) {
                continue;
            }
            let abs = ((n.hit_at.0 - cursor.0) as f64 * ms_per_sample).abs();
            if abs > safe_zone {
                continue;
            }
            if best.map(|(_, b)| abs < b).unwrap_or(true) {
                best = Some((i, abs));
            }
        }

        let (idx, abs_diff_ms) = best?;
        let id = self.notes[idx].id;
        let is_sustain = self.notes[idx].is_sustain;
        self.resolved_notes.push(id);
        if is_sustain {
            self.register_sustain_hit();
            Some(HitOutcome {
                judgment: Judgment::Sick,
                is_sustain,
            })
        } else {
            Some(HitOutcome {
                judgment: self.register_hit(abs_diff_ms),
                is_sustain,
            })
        }
    }

    /// Resolve player-side sustain child notes while a lane is held. This
    /// mirrors the per-frame held-input path in `PlayState.update`.
    /// ref: 50fccded:source/PlayState.hx:1961-1977
    pub fn resolve_held_sustains_in_lane(
        &mut self,
        cursor: Samples,
        lane: Lane,
        sample_rate: u32,
    ) -> u32 {
        let safe_zone = JudgmentWindows::from(self.windows).safe_zone_ms.0;
        let ms_per_sample = 1000.0 / sample_rate as f64;
        let mut hits = Vec::new();

        for n in &self.notes {
            if n.opponent || !n.is_sustain || n.lane != lane {
                continue;
            }
            if self.resolved_notes.contains(&n.id) {
                continue;
            }
            let diff_ms = (n.hit_at.0 - cursor.0) as f64 * ms_per_sample;
            if diff_ms > -safe_zone && diff_ms < safe_zone * 0.5 {
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

    /// Mark every player-side note whose hit_at is more than `safe_zone_ms`
    /// behind `cursor` as a miss. Returns the count of newly missed notes.
    pub fn expire_late_notes(&mut self, cursor: Samples, sample_rate: u32) -> u32 {
        let safe_zone = JudgmentWindows::from(self.windows).safe_zone_ms.0;
        let ms_per_sample = 1000.0 / sample_rate as f64;
        let mut newly_missed = Vec::new();
        for n in &self.notes {
            if n.opponent || self.resolved_notes.contains(&n.id) {
                continue;
            }
            let diff_ms = (n.hit_at.0 - cursor.0) as f64 * ms_per_sample;
            if diff_ms < -safe_zone {
                newly_missed.push(n.id);
            }
        }
        let count = newly_missed.len() as u32;
        for id in newly_missed {
            self.resolved_notes.push(id);
            self.register_late_note_miss();
        }
        count
    }

    pub(crate) fn apply_health(&mut self, delta: f32) {
        let next = self.health + delta;
        // Upstream caps at 2.0 from above; below 0 is allowed and triggers
        // game over via `is_dead`.
        self.health = if next > MAX_HEALTH { MAX_HEALTH } else { next };
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
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
            opponent: false,
        });
    }

    #[test]
    fn perfect_hit_awards_sick_and_350() {
        let mut s = PlayState::new();
        let j = s.register_hit(0.0);
        assert_eq!(j, Judgment::Sick);
        assert_eq!(s.score, 350);
        assert_eq!(s.combo, 1);
        assert_eq!(s.sicks, 1);
        assert!((s.health - (INITIAL_HEALTH + 0.023)).abs() < 1e-5);
    }

    #[test]
    fn miss_resets_combo_and_takes_health() {
        let mut s = PlayState::new();
        s.register_hit(0.0);
        s.register_hit(0.0);
        assert_eq!(s.combo, 2);
        s.register_miss();
        assert_eq!(s.combo, 0);
        assert_eq!(s.max_combo, 2);
        assert_eq!(s.misses, 1);
        assert_eq!(s.score, 690);
    }

    #[test]
    fn health_caps_at_two() {
        let mut s = PlayState::new();
        s.health = 1.99;
        s.register_hit(0.0);
        assert!((s.health - MAX_HEALTH).abs() < 1e-6);
    }

    #[test]
    fn enough_misses_kill_player() {
        let mut s = PlayState::new();
        // INITIAL_HEALTH=1.0, miss = -0.04 → 25 misses to reach 0.0.
        for _ in 0..25 {
            s.register_miss();
        }
        assert!(s.is_dead());
    }

    #[test]
    fn late_note_miss_uses_og_health_penalty_without_score_or_combo_reset() {
        let mut s = PlayState::new();
        s.score = 1_000;
        s.combo = 7;

        s.register_late_note_miss();

        assert_eq!(s.score, 1_000);
        assert_eq!(s.combo, 7);
        assert_eq!(s.misses, 1);
        assert!((s.health - (INITIAL_HEALTH - 0.0475)).abs() < 1e-6);
    }

    #[test]
    fn ratings_match_fnf_thresholds() {
        let mut s = PlayState::new();
        // 0.2 * 166.667 ≈ 33.333; just over → Good (200).
        assert_eq!(s.register_hit(34.0), Judgment::Good);
        assert_eq!(s.score, 200);
        // 0.75 * 166.667 ≈ 125; just over → Bad (100).
        assert_eq!(s.register_hit(126.0), Judgment::Bad);
        assert_eq!(s.score, 300);
        // 0.9 * 166.667 = 150; just over → Shit (50).
        assert_eq!(s.register_hit(151.0), Judgment::Shit);
        assert_eq!(s.score, 350);
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
    fn input_offset_in_samples_resolves_to_correct_rating() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        // 4800 samples late = 100ms @ 48kHz → Good.
        let j = s.try_hit_in_lane(&input_at(48_000 + 4_800), Lane::Left, 48_000);
        assert_eq!(j.map(|outcome| outcome.judgment), Some(Judgment::Good));
    }

    #[test]
    fn input_outside_safe_zone_is_no_hit() {
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
            opponent: false,
        });
        s.notes.push(Note {
            id: NoteId::new(1),
            lane: Lane::Left,
            hit_at: Samples(48_000),
            sustain_samples: 0,
            is_sustain: false,
            opponent: true,
        });
        let j = s.try_hit_in_lane(&input_at(48_000), Lane::Left, 48_000);
        assert_eq!(j, None);
    }

    #[test]
    fn sustain_hit_adds_health_without_score_or_combo() {
        let mut s = PlayState::new();
        add_sustain(&mut s, 0, Lane::Left, 48_000);

        let j = s.try_hit_in_lane(&input_at(48_000), Lane::Left, 48_000);

        assert_eq!(
            j,
            Some(HitOutcome {
                judgment: Judgment::Sick,
                is_sustain: true,
            })
        );
        assert_eq!(s.score, 0);
        assert_eq!(s.combo, 0);
        assert_eq!(s.sicks, 0);
        assert!(s.resolved_notes.contains(&NoteId::new(0)));
        assert!((s.health - (INITIAL_HEALTH + 0.023)).abs() < 1e-5);
    }

    #[test]
    fn held_lane_resolves_sustain_children_in_fnf_window() {
        let mut s = PlayState::new();
        add_sustain(&mut s, 0, Lane::Left, 48_000);
        add_sustain(&mut s, 1, Lane::Left, 62_400);
        add_note(&mut s, 2, Lane::Left, 48_000);

        let count = s.resolve_held_sustains_in_lane(Samples(48_000), Lane::Left, 48_000);

        assert_eq!(count, 1);
        assert!(s.resolved_notes.contains(&NoteId::new(0)));
        assert!(!s.resolved_notes.contains(&NoteId::new(1)));
        assert!(!s.resolved_notes.contains(&NoteId::new(2)));
        assert_eq!(s.score, 0);
        assert_eq!(s.combo, 0);
    }

    #[test]
    fn expire_late_notes_misses_unhit_player_notes_past_safe_zone() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        add_note(&mut s, 1, Lane::Left, 48_000 + 96_000);
        s.score = 500;
        s.combo = 3;
        let cursor = Samples(48_000 + 9_000);
        let missed = s.expire_late_notes(cursor, 48_000);
        assert_eq!(missed, 1);
        assert!(s.resolved_notes.contains(&NoteId::new(0)));
        assert_eq!(s.misses, 1);
        assert_eq!(s.score, 500);
        assert_eq!(s.combo, 3);
        assert!((s.health - (INITIAL_HEALTH - 0.0475)).abs() < 1e-6);
    }
}
