//! Conductor: derives song position from the mixer sample cursor.
//! See `PLAN.md` Section 8.

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

#[derive(Debug)]
pub struct Conductor {
    bpm: Bpm,
    sample_rate: u32,
    /// Sample at which the song origin (beat 0) sits in the mixer cursor.
    origin_sample: Samples,
}

impl Conductor {
    pub fn new(bpm: Bpm, sample_rate: u32) -> Self {
        Self {
            bpm,
            sample_rate,
            origin_sample: Samples(0),
        }
    }

    pub fn set_origin(&mut self, origin: Samples) {
        self.origin_sample = origin;
    }

    pub fn snapshot(&self, mixer_cursor: Samples) -> ConductorState {
        let song_samples = mixer_cursor.0 - self.origin_sample.0;
        let position_s = song_samples as f64 / self.sample_rate as f64;
        let beat = position_s / self.bpm.beat_seconds().0;
        let step = position_s / self.bpm.step_seconds().0;
        ConductorState {
            sample_cursor: mixer_cursor,
            sample_rate: self.sample_rate,
            bpm: self.bpm,
            position: Seconds(position_s),
            beat: Beat(beat),
            step: Step(step),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

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
}
