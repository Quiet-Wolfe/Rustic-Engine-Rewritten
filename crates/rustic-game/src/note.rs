//! Note and lane types, plus chart → gameplay conversion.
//!
//! Lanes use the four base-FNF directions. Sustain length is in mixer
//! samples so judgment math stays in audio-domain time.
//!
//! ref: 50fccded:source/PlayState.hx:1010-1018  // strumTime, lane mod 4, mustHit flip
//! ref: 50fccded:source/PlayState.hx:1028        // sustainLength = songNotes[2]

use rustic_asset::ChartNote;
use rustic_core::ids::NoteId;
use rustic_core::time::Samples;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Lane {
    Left = 0,
    Down = 1,
    Up = 2,
    Right = 3,
}

impl Lane {
    /// Lane is the chart's `noteData = sectionNotes[1] % 4`.
    /// ref: 50fccded:source/PlayState.hx:1012
    pub fn from_raw(raw: u8) -> Lane {
        match raw % 4 {
            0 => Lane::Left,
            1 => Lane::Down,
            2 => Lane::Up,
            _ => Lane::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Note {
    pub id: NoteId,
    pub lane: Lane,
    /// Time at which the note should be hit, in mixer samples relative to
    /// the song origin.
    pub hit_at: Samples,
    /// Sustain duration in samples. Zero for tap notes.
    pub sustain_samples: i64,
    /// Whose side the note belongs to. `false` = player, `true` = opponent.
    pub opponent: bool,
}

/// Convert a chart's notes into gameplay `Note`s, materialising sample
/// positions from the chart's millisecond domain. Sustain segments are not
/// expanded into per-step children — that's a render/scoring concern that
/// lives outside the bare chart shape.
pub fn notes_from_chart<'a>(
    chart_notes: impl IntoIterator<Item = &'a ChartNote>,
    sample_rate: u32,
) -> Vec<Note> {
    let scale = sample_rate as f64 / 1000.0;
    chart_notes
        .into_iter()
        .enumerate()
        .map(|(i, n)| Note {
            id: NoteId::new(i as u32),
            lane: Lane::from_raw(n.raw_lane),
            hit_at: Samples((n.time_ms * scale).round() as i64),
            sustain_samples: (n.sustain_ms * scale).round() as i64,
            opponent: !n.is_player,
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rustic_asset::ParsedSong;

    #[test]
    fn lane_is_raw_modulo_four() {
        assert_eq!(Lane::from_raw(0), Lane::Left);
        assert_eq!(Lane::from_raw(1), Lane::Down);
        assert_eq!(Lane::from_raw(2), Lane::Up);
        assert_eq!(Lane::from_raw(3), Lane::Right);
        assert_eq!(Lane::from_raw(4), Lane::Left);
        assert_eq!(Lane::from_raw(7), Lane::Right);
    }

    #[test]
    fn chart_to_notes_converts_ms_to_samples() {
        // 1000ms @ 48kHz → 48000 samples.
        const CHART: &str = r#"{
            "song": {
                "song": "X",
                "bpm": 100.0,
                "notes": [
                    {"mustHitSection": true, "lengthInSteps": 16,
                     "sectionNotes": [[1000.0, 0, 0], [2000.0, 5, 250]]}
                ]
            }
        }"#;
        let parsed = ParsedSong::parse(CHART.as_bytes()).unwrap();
        let notes = notes_from_chart(parsed.chart.notes.iter(), 48_000);
        assert_eq!(notes.len(), 2);

        // First note: 1000ms, lane 0 (player owner because mustHit=true).
        assert_eq!(notes[0].lane, Lane::Left);
        assert_eq!(notes[0].hit_at, Samples(48_000));
        assert_eq!(notes[0].sustain_samples, 0);
        assert!(!notes[0].opponent);

        // Second note: 2000ms, raw lane 5 → mod 4 = Down,
        // is_player=false (mustHit=true & lane>3 → opponent).
        assert_eq!(notes[1].lane, Lane::Down);
        assert_eq!(notes[1].hit_at, Samples(96_000));
        assert_eq!(notes[1].sustain_samples, 12_000);
        assert!(notes[1].opponent);
    }
}
