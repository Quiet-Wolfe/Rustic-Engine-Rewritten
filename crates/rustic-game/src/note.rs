//! Note and lane types, plus chart → gameplay conversion.
//!
//! Lanes use the four base-FNF directions. Sustain length is in mixer
//! samples so judgment math stays in audio-domain time.
//!
//! ref: 50fccded:source/PlayState.hx:1010-1018  // strumTime, lane mod 4, mustHit flip
//! ref: 50fccded:source/PlayState.hx:1028        // sustainLength = songNotes[2]
//! ref: 50fccded:source/PlayState.hx:1031-1042   // sustain child note expansion

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
    /// True for generated sustain child notes.
    pub is_sustain: bool,
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
    bpm: f64,
) -> Vec<Note> {
    let scale = sample_rate as f64 / 1000.0;
    let step_crochet_ms = 15_000.0 / bpm;
    let mut expanded = Vec::new();

    for n in chart_notes {
        let lane = Lane::from_raw(n.raw_lane);
        let opponent = !n.is_player;
        expanded.push(ExpandedNote {
            time_ms: n.time_ms,
            lane,
            sustain_ms: n.sustain_ms,
            is_sustain: false,
            opponent,
        });

        // ref: 50fccded:source/PlayState.hx:1031-1042 — floor
        // sustainLength / stepCrochet and place child notes one step
        // after the head note.
        let sustain_steps = (n.sustain_ms / step_crochet_ms).floor() as u32;
        for i in 0..sustain_steps {
            expanded.push(ExpandedNote {
                time_ms: n.time_ms + step_crochet_ms * (i + 1) as f64,
                lane,
                sustain_ms: 0.0,
                is_sustain: true,
                opponent,
            });
        }
    }

    expanded.sort_by(|a, b| {
        a.time_ms
            .partial_cmp(&b.time_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    expanded
        .into_iter()
        .enumerate()
        .map(|(i, n)| Note {
            id: NoteId::new(i as u32),
            lane: n.lane,
            hit_at: Samples((n.time_ms * scale).round() as i64),
            sustain_samples: (n.sustain_ms * scale).round() as i64,
            is_sustain: n.is_sustain,
            opponent: n.opponent,
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct ExpandedNote {
    time_ms: f64,
    lane: Lane,
    sustain_ms: f64,
    is_sustain: bool,
    opponent: bool,
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
        let notes = notes_from_chart(parsed.chart.notes.iter(), 48_000, parsed.chart.bpm);
        assert_eq!(notes.len(), 3);

        // First note: 1000ms, lane 0 (player owner because mustHit=true).
        assert_eq!(notes[0].lane, Lane::Left);
        assert_eq!(notes[0].hit_at, Samples(48_000));
        assert_eq!(notes[0].sustain_samples, 0);
        assert!(!notes[0].is_sustain);
        assert!(!notes[0].opponent);

        // Second note: 2000ms, raw lane 5 → mod 4 = Down,
        // is_player=false (mustHit=true & lane>3 → opponent).
        assert_eq!(notes[1].lane, Lane::Down);
        assert_eq!(notes[1].hit_at, Samples(96_000));
        assert_eq!(notes[1].sustain_samples, 12_000);
        assert!(!notes[1].is_sustain);
        assert!(notes[1].opponent);

        // 250ms sustain at 100 BPM produces one child at stepCrochet
        // (150ms) after the head.
        assert_eq!(notes[2].lane, Lane::Down);
        assert_eq!(notes[2].hit_at, Samples(103_200));
        assert!(notes[2].is_sustain);
        assert!(notes[2].opponent);
    }
}
