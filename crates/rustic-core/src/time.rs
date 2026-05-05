//! Time types shared across crates. See `PLAN.md` Sections 3 and 8.
//!
//! Wall-clock and audio-cursor time are intentionally distinct types so
//! gameplay code cannot accidentally mix them.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Seconds(pub f64);

impl Seconds {
    #[inline]
    pub const fn new(s: f64) -> Self {
        Self(s)
    }
    #[inline]
    pub fn as_milliseconds(self) -> Milliseconds {
        Milliseconds(self.0 * 1000.0)
    }
    #[inline]
    pub fn as_samples(self, sample_rate: u32) -> Samples {
        Samples((self.0 * sample_rate as f64) as i64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Milliseconds(pub f64);

impl Milliseconds {
    #[inline]
    pub const fn new(ms: f64) -> Self {
        Self(ms)
    }
    #[inline]
    pub fn as_seconds(self) -> Seconds {
        Seconds(self.0 / 1000.0)
    }
}

/// Audio-domain integer sample cursor. The mixer owns this; gameplay reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Samples(pub i64);

impl Samples {
    #[inline]
    pub const fn new(n: i64) -> Self {
        Self(n)
    }
    #[inline]
    pub fn as_seconds(self, sample_rate: u32) -> Seconds {
        Seconds(self.0 as f64 / sample_rate as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Bpm(pub f64);

impl Bpm {
    #[inline]
    pub const fn new(bpm: f64) -> Self {
        Self(bpm)
    }
    #[inline]
    pub fn beat_seconds(self) -> Seconds {
        Seconds(60.0 / self.0)
    }
    #[inline]
    pub fn step_seconds(self) -> Seconds {
        Seconds(60.0 / (self.0 * 4.0))
    }
}

/// Whole-beat position relative to a song origin.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Beat(pub f64);

/// Sixteenth-note step position. FNF charts are step-quantized.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Step(pub f64);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn seconds_to_samples_round_trip() {
        let s = Seconds(1.5);
        let n = s.as_samples(48_000);
        assert_eq!(n, Samples(72_000));
        let back = n.as_seconds(48_000);
        assert!((back.0 - 1.5).abs() < 1e-9);
    }

    #[test]
    fn bpm_step_is_quarter_beat() {
        let bpm = Bpm(120.0);
        let beat = bpm.beat_seconds();
        let step = bpm.step_seconds();
        assert!((beat.0 - 0.5).abs() < 1e-9);
        assert!((step.0 - 0.125).abs() < 1e-9);
    }
}
