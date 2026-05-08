//! Conductor: derives song position from the mixer sample cursor.
//! See `PLAN.md` Section 8.
//!
//! ref: bdedc0aa:source/funkin/Conductor.hx:437-484

use rustic_asset::Chart;
use rustic_core::time::{Beat, Bpm, Samples, Seconds, Step};

#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct ConductorState {
    pub sample_cursor: Samples,
    pub sample_rate: u32,
    pub bpm: Bpm,
    pub position: Seconds,
    pub beat: Beat,
    pub step: Step,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct BpmChangeEvent {
    pub step_time: i64,
    pub song_time: Seconds,
    pub bpm: Bpm,
}

#[derive(Debug)]
pub struct Conductor {
    bpm: Bpm,
    sample_rate: u32,
    /// Sample at which the song origin (beat 0) sits in the mixer cursor.
    origin_sample: Samples,
    bpm_changes: Vec<BpmChangeEvent>,
}

impl Conductor {
    pub fn new(bpm: Bpm, sample_rate: u32) -> Self {
        Self {
            bpm,
            sample_rate: sample_rate.max(1),
            origin_sample: Samples(0),
            bpm_changes: Vec::new(),
        }
    }

    pub fn from_chart(chart: &Chart, sample_rate: u32) -> Self {
        let mut conductor = Self::new(Bpm(chart.bpm), sample_rate);
        conductor.bpm_changes = map_bpm_changes(chart);
        conductor
    }

    pub fn set_origin(&mut self, origin: Samples) {
        self.origin_sample = origin;
    }

    pub fn bpm_changes(&self) -> &[BpmChangeEvent] {
        &self.bpm_changes
    }

    pub fn snapshot(&self, mixer_cursor: Samples) -> ConductorState {
        let song_samples = mixer_cursor.0 - self.origin_sample.0;
        let position_s = song_samples as f64 / self.sample_rate as f64;
        let last_change = self.change_at(Seconds(position_s));
        let step = last_change.step_time as f64
            + ((position_s - last_change.song_time.0) / last_change.bpm.step_seconds().0);
        let beat = step / 4.0;
        ConductorState {
            sample_cursor: mixer_cursor,
            sample_rate: self.sample_rate,
            bpm: last_change.bpm,
            position: Seconds(position_s),
            beat: Beat(beat),
            step: Step(step),
        }
    }

    fn change_at(&self, position: Seconds) -> BpmChangeEvent {
        let mut last = BpmChangeEvent {
            step_time: 0,
            song_time: Seconds(0.0),
            bpm: self.bpm,
        };
        for change in &self.bpm_changes {
            if position.0 >= change.song_time.0 {
                last = *change;
            }
        }
        last
    }
}

pub fn map_bpm_changes(chart: &Chart) -> Vec<BpmChangeEvent> {
    // Legacy expanded-chart compatibility. v0.8.5 charts carry time changes
    // in metadata; this path keeps imported 0.x sections coherent until the
    // runtime chart model is fully v-slice-native.
    let mut changes = Vec::new();
    let mut cur_bpm = chart.bpm;
    let mut total_steps: i64 = 0;
    let mut total_pos_ms = 0.0;

    for section in &chart.sections {
        if let Some(next_bpm) = section.bpm_change {
            if next_bpm != cur_bpm {
                cur_bpm = next_bpm;
                changes.push(BpmChangeEvent {
                    step_time: total_steps,
                    song_time: Seconds(total_pos_ms / 1000.0),
                    bpm: Bpm(cur_bpm),
                });
            }
        }

        let step_ms = (60.0 / cur_bpm) * 1000.0 / 4.0;
        total_steps += i64::from(section.length_in_steps);
        total_pos_ms += step_ms * section.length_in_steps as f64;
    }

    changes
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rustic_asset::ParsedSong;

    #[test]
    fn beat_from_sample_cursor_at_120_bpm() {
        let conductor = Conductor::new(Bpm(120.0), 48_000);
        // 24_000 samples == 0.5 s == 1 beat at 120 BPM
        let snap = conductor.snapshot(Samples(24_000));
        assert!((snap.beat.0 - 1.0).abs() < 1e-9);
        assert!((snap.position.0 - 0.5).abs() < 1e-9);
    }

    #[test]
    fn origin_offset_subtracts_correctly() {
        let mut conductor = Conductor::new(Bpm(120.0), 48_000);
        conductor.set_origin(Samples(48_000));
        let snap = conductor.snapshot(Samples(48_000));
        assert!(snap.position.0.abs() < 1e-9);
    }

    #[test]
    fn bpm_change_map_matches_og_section_walk() {
        let parsed = ParsedSong::parse(BPM_CHANGE_CHART.as_bytes()).unwrap();
        let changes = map_bpm_changes(&parsed.chart);

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].step_time, 16);
        assert!((changes[0].song_time.0 - 2.0).abs() < 1e-9);
        assert_eq!(changes[0].bpm, Bpm(180.0));
        assert_eq!(changes[1].step_time, 24);
        assert!((changes[1].song_time.0 - 2.666_666_666_666_666_5).abs() < 1e-9);
        assert_eq!(changes[1].bpm, Bpm(90.0));
    }

    #[test]
    fn snapshot_uses_last_bpm_change_for_step_and_beat() {
        let parsed = ParsedSong::parse(BPM_CHANGE_CHART.as_bytes()).unwrap();
        let conductor = Conductor::from_chart(&parsed.chart, 1_000);

        let snap = conductor.snapshot(Samples(2_250));

        assert_eq!(snap.bpm, Bpm(180.0));
        assert!((snap.step.0 - 19.0).abs() < 1e-9);
        assert!((snap.beat.0 - 4.75).abs() < 1e-9);
    }

    const BPM_CHANGE_CHART: &str = r#"{
        "song": {
            "song": "BpmChange",
            "bpm": 120,
            "notes": [
                {"lengthInSteps": 16, "changeBPM": false, "sectionNotes": []},
                {"lengthInSteps": 8, "changeBPM": true, "bpm": 180, "sectionNotes": []},
                {"lengthInSteps": 16, "changeBPM": true, "bpm": 90, "sectionNotes": []}
            ]
        }
    }"#;
}
