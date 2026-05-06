//! `PlayState` — headless gameplay state.
//!
//! See `PLAN.md` Section 9. This file holds gameplay numbers only — score,
//! combo, health, judgment counters. Render/audio side-effects belong to
//! `rustic-app`. Scoring math lives in `scoring.rs` next door.
//!
//! ref: bdedc0aa:source/funkin/util/Constants.hx:436-447
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:3292-3321

use crate::judgment::JudgmentWindows;
use crate::note::{notes_from_chart, Note};
use rustic_asset::ParsedSong;
use rustic_core::ids::{NoteId, SongId};
use serde::{Deserialize, Serialize};

/// Initial player health. Bar UI shows 50% at this value (range 0..2).
/// ref: bdedc0aa:source/funkin/util/Constants.hx:441
pub const INITIAL_HEALTH: f32 = 1.0;
/// Max player health. Above this it clamps.
/// ref: bdedc0aa:source/funkin/util/Constants.hx:436
pub const MAX_HEALTH: f32 = 2.0;
/// Game over when health reaches or drops below this.
/// ref: bdedc0aa:source/funkin/util/Constants.hx:447
pub const DEATH_HEALTH: f32 = 0.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PlayState {
    pub song: Option<SongId>,
    /// Chart BPM, used by beat/hold-timer behavior.
    #[serde(default = "default_bpm")]
    pub bpm: f64,
    /// Chart scroll speed (`SONG.speed`) used by the OG note-y formula.
    #[serde(default = "default_scroll_speed")]
    pub scroll_speed: f32,
    pub notes: Vec<Note>,
    /// IDs of notes that have already been resolved (hit or expired). Kept
    /// separately from `notes` so the chart stays immutable and rewind
    /// only needs to replay the resolved set.
    pub resolved_notes: Vec<NoteId>,
    /// Hit window in milliseconds. The serialized name stays generic so old
    /// prototype saves can fall back cleanly.
    pub windows: JudgmentWindowsSerde,
    pub score: i64,
    pub combo: u32,
    pub max_combo: u32,
    pub health: f32,
    pub sicks: u32,
    pub goods: u32,
    pub bads: u32,
    pub shits: u32,
    pub misses: u32,
}

impl Default for PlayState {
    fn default() -> Self {
        Self {
            song: None,
            bpm: default_bpm(),
            scroll_speed: default_scroll_speed(),
            notes: Vec::new(),
            resolved_notes: Vec::new(),
            windows: JudgmentWindows::base_fnf().into(),
            score: 0,
            combo: 0,
            max_combo: 0,
            health: INITIAL_HEALTH,
            sicks: 0,
            goods: 0,
            bads: 0,
            shits: 0,
            misses: 0,
        }
    }
}

impl PlayState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_chart(song: SongId, parsed: &ParsedSong, sample_rate: u32) -> Self {
        let mut state = Self::default();
        state.load_chart(song, parsed, sample_rate);
        state
    }

    pub fn load_chart(&mut self, song: SongId, parsed: &ParsedSong, sample_rate: u32) {
        let notes = notes_from_chart(parsed.chart.notes.iter(), sample_rate, parsed.chart.bpm);
        *self = Self {
            song: Some(song),
            bpm: parsed.chart.bpm,
            scroll_speed: parsed.chart.speed as f32,
            notes,
            ..Self::default()
        };
    }

    /// Total non-miss judgments — used for accuracy and UI counters.
    pub fn total_hits(&self) -> u32 {
        self.sicks + self.goods + self.bads + self.shits
    }

    /// True when health has dropped to the death threshold.
    pub fn is_dead(&self) -> bool {
        self.health <= DEATH_HEALTH
    }
}

pub(crate) fn default_scroll_speed() -> f32 {
    1.0
}

pub(crate) fn default_bpm() -> f64 {
    100.0
}

/// Serde-friendly wrapper because `JudgmentWindows` is `non_exhaustive`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct JudgmentWindowsSerde {
    #[serde(alias = "safe_zone_ms")]
    pub hit_window_ms: f64,
}

impl From<JudgmentWindows> for JudgmentWindowsSerde {
    fn from(w: JudgmentWindows) -> Self {
        Self {
            hit_window_ms: w.hit_window_ms.0,
        }
    }
}

impl From<JudgmentWindowsSerde> for JudgmentWindows {
    fn from(w: JudgmentWindowsSerde) -> Self {
        // Default to base FNF if the persisted value is zero (legacy save).
        let z = if w.hit_window_ms > 0.0 {
            w.hit_window_ms
        } else {
            JudgmentWindows::DEFAULT_HIT_WINDOW_MS
        };
        let mut out = JudgmentWindows::base_fnf();
        out.hit_window_ms = rustic_core::time::Milliseconds(z);
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rustic_core::time::Samples;

    #[test]
    fn defaults_match_fnf() {
        let s = PlayState::new();
        assert_eq!(s.score, 0);
        assert_eq!(s.combo, 0);
        assert_eq!(s.max_combo, 0);
        assert_eq!(s.bpm, 100.0);
        assert_eq!(s.scroll_speed, 1.0);
        assert!((s.health - INITIAL_HEALTH).abs() < 1e-6);
        assert!(!s.is_dead());
    }

    #[test]
    fn windows_round_trip_through_serde_wrapper() {
        let serde: JudgmentWindowsSerde = JudgmentWindows::base_fnf().into();
        let back: JudgmentWindows = serde.into();
        assert!((back.hit_window_ms.0 - JudgmentWindows::DEFAULT_HIT_WINDOW_MS).abs() < 1e-9);
    }

    #[test]
    fn windows_zero_falls_back_to_fnf_default() {
        let zero = JudgmentWindowsSerde { hit_window_ms: 0.0 };
        let back: JudgmentWindows = zero.into();
        assert!((back.hit_window_ms.0 - JudgmentWindows::DEFAULT_HIT_WINDOW_MS).abs() < 1e-9);
    }

    #[test]
    fn load_chart_sets_song_notes_and_fresh_counters() {
        const CHART: &str = r#"{
            "song": {
                "song": "Bopeebo",
                "bpm": 100.0,
                "speed": 1.4,
                "notes": [
                    {"mustHitSection": true, "lengthInSteps": 16,
                     "sectionNotes": [[1000.0, 0, 0], [2000.0, 5, 250]]}
                ]
            }
        }"#;
        let parsed = ParsedSong::parse(CHART.as_bytes()).unwrap();
        let mut state = PlayState::new();
        state.score = 1234;
        state.combo = 12;
        state.health = 0.25;

        state.load_chart(SongId::new(7), &parsed, 48_000);

        assert_eq!(state.song, Some(SongId::new(7)));
        assert_eq!(state.bpm, 100.0);
        assert_eq!(state.scroll_speed, 1.4);
        assert_eq!(state.notes.len(), 3);
        assert_eq!(state.notes[0].hit_at, Samples(48_000));
        assert_eq!(state.notes[1].sustain_samples, 12_000);
        assert!(state.notes[2].is_sustain);
        assert_eq!(state.score, 0);
        assert_eq!(state.combo, 0);
        assert!((state.health - INITIAL_HEALTH).abs() < 1e-6);
    }
}
