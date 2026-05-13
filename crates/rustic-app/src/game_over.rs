use rustic_core::time::Samples;
use std::time::{Duration, Instant};

const CONFIRM_RESTART_DELAY: Duration = Duration::from_millis(1_030);

#[derive(Debug, Clone, Copy)]
pub(crate) struct GameOverState {
    song_cursor: Samples,
    animation_started: Instant,
    loop_at: Samples,
    loop_started: bool,
}

impl GameOverState {
    pub(crate) fn new(song_cursor: Samples, loop_after: Samples) -> Self {
        Self {
            song_cursor,
            animation_started: Instant::now(),
            loop_at: Samples(song_cursor.0 + loop_after.0),
            loop_started: false,
        }
    }

    pub(crate) fn cursor(self, sample_rate: u32) -> Samples {
        let elapsed = self.animation_started.elapsed().as_secs_f64();
        let elapsed_samples = (elapsed * f64::from(sample_rate.max(1))).round() as i64;
        Samples(self.song_cursor.0 + elapsed_samples)
    }

    pub(crate) fn start_loop_if_due(&mut self, cursor: Samples) -> Option<Samples> {
        if self.loop_started || cursor < self.loop_at {
            None
        } else {
            self.loop_started = true;
            Some(self.loop_at)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GameOverRestart {
    started_at: Instant,
    delay: Duration,
}

impl GameOverRestart {
    pub(crate) fn new() -> Self {
        // ref: bdedc0aa:source/funkin/play/GameOverSubState.hx:382-387
        Self {
            started_at: Instant::now(),
            delay: CONFIRM_RESTART_DELAY,
        }
    }

    pub(crate) fn is_due(self) -> bool {
        self.started_at.elapsed() >= self.delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn death_loop_starts_once_after_first_death_duration() {
        let mut state = GameOverState::new(Samples(1_000), Samples(500));
        assert_eq!(state.start_loop_if_due(Samples(1_499)), None);
        assert_eq!(
            state.start_loop_if_due(Samples(1_500)),
            Some(Samples(1_500))
        );
        assert_eq!(state.start_loop_if_due(Samples(2_000)), None);
    }

    #[test]
    fn confirm_restart_delay_uses_og_end_music_timer() {
        assert_eq!(CONFIRM_RESTART_DELAY, Duration::from_millis(1_030));
    }
}
