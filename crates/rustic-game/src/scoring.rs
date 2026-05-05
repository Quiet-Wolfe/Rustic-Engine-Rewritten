//! Scoring + input-matching impl block for `PlayState`.
//!
//! Lives next to `state.rs` to keep the data definition file lean while
//! still centralising the FNF gameplay math under one type.
//!
//! ref: 50fccded:source/PlayState.hx:1684-1716   // popUpScore
//! ref: 50fccded:source/PlayState.hx:2028-2042   // noteMiss
//! ref: 50fccded:source/PlayState.hx:2096-2109   // goodNoteHit
//! ref: 50fccded:source/Note.hx:172-184          // canBeHit / tooLate

use crate::judgment::{health_delta, score_value, Judgment, JudgmentWindows};
use crate::note::Lane;
use crate::state::{PlayState, MAX_HEALTH};
use rustic_core::input::NormalizedInputEvent;
use rustic_core::time::Samples;

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

    /// Player either pressed an empty lane or let a note pass beyond the
    /// safe zone.
    pub fn register_miss(&mut self) {
        self.score += score_value(Judgment::Miss);
        self.combo = 0;
        self.misses += 1;
        self.apply_health(health_delta(Judgment::Miss));
    }

    /// Try to consume a player keypress against the closest unresolved
    /// player-side note in `lane`. `event.audio_sample_cursor_at_receive`
    /// is the authoritative timestamp (PLAN.md Section 8/10).
    pub fn try_hit_in_lane(
        &mut self,
        event: &NormalizedInputEvent,
        lane: Lane,
        sample_rate: u32,
    ) -> Option<Judgment> {
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
        self.resolved_notes.push(id);
        Some(self.register_hit(abs_diff_ms))
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
            self.register_miss();
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
        assert_eq!(j, Some(Judgment::Sick));
        assert_eq!(s.resolved_notes.len(), 1);
    }

    #[test]
    fn input_offset_in_samples_resolves_to_correct_rating() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        // 4800 samples late = 100ms @ 48kHz → Good.
        let j = s.try_hit_in_lane(&input_at(48_000 + 4_800), Lane::Left, 48_000);
        assert_eq!(j, Some(Judgment::Good));
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
            opponent: false,
        });
        s.notes.push(Note {
            id: NoteId::new(1),
            lane: Lane::Left,
            hit_at: Samples(48_000),
            sustain_samples: 0,
            opponent: true,
        });
        let j = s.try_hit_in_lane(&input_at(48_000), Lane::Left, 48_000);
        assert_eq!(j, None);
    }

    #[test]
    fn expire_late_notes_misses_unhit_player_notes_past_safe_zone() {
        let mut s = PlayState::new();
        add_note(&mut s, 0, Lane::Left, 48_000);
        add_note(&mut s, 1, Lane::Left, 48_000 + 96_000);
        let cursor = Samples(48_000 + 9_000);
        let missed = s.expire_late_notes(cursor, 48_000);
        assert_eq!(missed, 1);
        assert!(s.resolved_notes.contains(&NoteId::new(0)));
        assert_eq!(s.misses, 1);
    }
}
