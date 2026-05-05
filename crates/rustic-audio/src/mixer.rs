//! Mixer skeleton. The real `cpal` mixer lands in Phase 5.

use rustic_core::time::Samples;

#[derive(Debug, Default)]
pub struct Mixer {
    sample_cursor: Samples,
    sample_rate: u32,
}

impl Mixer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_cursor: Samples(0),
            sample_rate,
        }
    }

    #[inline]
    pub fn sample_cursor(&self) -> Samples {
        self.sample_cursor
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Advance the authoritative sample cursor. The real mixer advances
    /// from the audio device callback; this stub exists so unit tests can
    /// drive deterministic time during gameplay porting.
    pub fn advance_for_test(&mut self, frames: i64) {
        self.sample_cursor = Samples(self.sample_cursor.0 + frames);
    }

    pub fn seek_for_test(&mut self, position: Samples) {
        self.sample_cursor = position;
    }
}
