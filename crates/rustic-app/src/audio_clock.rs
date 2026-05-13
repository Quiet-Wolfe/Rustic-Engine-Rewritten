//! Gameplay clock guard for opened audio streams that stop advancing.

use rustic_core::time::Samples;
use std::time::{Duration, Instant};

const AUDIO_CURSOR_STALL_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Debug, Clone, Copy)]
pub(crate) struct AudioClockFallback {
    last_cursor: Samples,
    last_wall: Instant,
    fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AudioClockDecision {
    Audio(Samples),
    SwitchToWall(Samples),
    Wall,
}

impl AudioClockFallback {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            last_cursor: Samples(0),
            last_wall: now,
            fallback: false,
        }
    }

    pub(crate) fn reset(&mut self, now: Instant) {
        *self = Self::new(now);
    }

    pub(crate) fn observe(&mut self, cursor: Samples, now: Instant) -> AudioClockDecision {
        if self.fallback {
            return AudioClockDecision::Wall;
        }
        if cursor > self.last_cursor {
            self.last_cursor = cursor;
            self.last_wall = now;
            return AudioClockDecision::Audio(cursor);
        }
        if now.duration_since(self.last_wall) < AUDIO_CURSOR_STALL_TIMEOUT {
            return AudioClockDecision::Audio(cursor);
        }
        self.fallback = true;
        AudioClockDecision::SwitchToWall(cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advancing_audio_cursor_keeps_audio_clock() {
        let now = Instant::now();
        let mut clock = AudioClockFallback::new(now);

        assert_eq!(
            clock.observe(Samples(1), now + Duration::from_millis(800)),
            AudioClockDecision::Audio(Samples(1))
        );
    }

    #[test]
    fn stalled_audio_cursor_switches_to_wall_clock_once() {
        let now = Instant::now();
        let mut clock = AudioClockFallback::new(now);

        assert_eq!(
            clock.observe(Samples(0), now + Duration::from_millis(749)),
            AudioClockDecision::Audio(Samples(0))
        );
        assert_eq!(
            clock.observe(Samples(0), now + Duration::from_millis(750)),
            AudioClockDecision::SwitchToWall(Samples(0))
        );
        assert_eq!(
            clock.observe(Samples(10), now + Duration::from_millis(760)),
            AudioClockDecision::Wall
        );
    }
}
