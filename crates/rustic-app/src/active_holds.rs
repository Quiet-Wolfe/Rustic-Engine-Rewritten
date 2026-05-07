//! App-owned active hold-note state and score cursors.

use rustic_core::time::Samples;
use rustic_game::Lane;

const LANES: [Lane; 4] = [Lane::Left, Lane::Down, Lane::Up, Lane::Right];

#[derive(Debug, Default, Clone)]
pub struct ActiveHolds {
    active: [Option<ActiveHold>; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActiveHold {
    end_at: Samples,
    scored_until: Samples,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoldTick {
    pub lane: Lane,
    pub elapsed_samples: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoldRelease {
    pub hold_end_at: Samples,
    pub elapsed_samples: i64,
}

impl ActiveHolds {
    pub fn start(&mut self, lane: Lane, hold_end_at: Samples, cursor: Samples) {
        if hold_end_at > cursor {
            self.active[lane_index(lane)] = Some(ActiveHold {
                end_at: hold_end_at,
                scored_until: cursor,
            });
        }
    }

    pub fn release(&mut self, lane: Lane, cursor: Samples) -> Option<HoldRelease> {
        let active = self.active[lane_index(lane)].take()?;
        Some(HoldRelease {
            hold_end_at: active.end_at,
            elapsed_samples: elapsed_until(active, cursor),
        })
    }

    pub fn active_lanes(&self, cursor: Samples) -> impl Iterator<Item = Lane> + '_ {
        LANES
            .iter()
            .copied()
            .enumerate()
            .filter(move |(idx, _)| self.active[*idx].is_some_and(|hold| hold.end_at > cursor))
            .map(|(_, lane)| lane)
    }

    pub fn score_ticks(&mut self, cursor: Samples) -> Vec<HoldTick> {
        let mut ticks = Vec::new();
        for (idx, lane) in LANES.iter().copied().enumerate() {
            let Some(active) = self.active[idx].as_mut() else {
                continue;
            };
            let until = active.end_at.min(cursor);
            let elapsed_samples = until.0.saturating_sub(active.scored_until.0);
            active.scored_until = until;
            if elapsed_samples > 0 {
                ticks.push(HoldTick {
                    lane,
                    elapsed_samples,
                });
            }
        }
        ticks
    }

    pub fn complete_elapsed(&mut self, cursor: Samples) -> Vec<Lane> {
        let mut completed = Vec::new();
        for (idx, lane) in LANES.iter().copied().enumerate() {
            if self.active[idx].is_some_and(|hold| hold.end_at <= cursor) {
                self.active[idx] = None;
                completed.push(lane);
            }
        }
        completed
    }
}

fn elapsed_until(active: ActiveHold, cursor: Samples) -> i64 {
    active
        .end_at
        .min(cursor)
        .0
        .saturating_sub(active.scored_until.0)
}

fn lane_index(lane: Lane) -> usize {
    match lane {
        Lane::Left => 0,
        Lane::Down => 1,
        Lane::Up => 2,
        Lane::Right => 3,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hold_release(end_at: i64, elapsed_samples: i64) -> Option<HoldRelease> {
        Some(HoldRelease {
            hold_end_at: Samples(end_at),
            elapsed_samples,
        })
    }

    fn hold_tick(lane: Lane, elapsed_samples: i64) -> HoldTick {
        HoldTick {
            lane,
            elapsed_samples,
        }
    }

    #[test]
    fn active_holds_report_drops_and_completions_once() {
        let mut holds = ActiveHolds::default();
        holds.start(Lane::Left, Samples(200), Samples(100));

        let lanes: Vec<_> = holds.active_lanes(Samples(150)).collect();
        assert_eq!(lanes, vec![Lane::Left]);
        assert_eq!(
            holds.release(Lane::Left, Samples(160)),
            hold_release(200, 60)
        );
        assert_eq!(holds.release(Lane::Left, Samples(170)), None);

        holds.start(Lane::Down, Samples(300), Samples(200));
        assert_eq!(holds.complete_elapsed(Samples(299)), Vec::<Lane>::new());
        assert_eq!(holds.complete_elapsed(Samples(300)), vec![Lane::Down]);
        assert_eq!(holds.complete_elapsed(Samples(301)), Vec::<Lane>::new());
    }

    #[test]
    fn active_holds_score_elapsed_time_until_release_or_completion() {
        let mut holds = ActiveHolds::default();
        holds.start(Lane::Left, Samples(200), Samples(100));

        assert_eq!(
            holds.score_ticks(Samples(140)),
            vec![hold_tick(Lane::Left, 40)]
        );
        assert_eq!(
            holds.release(Lane::Left, Samples(180)),
            hold_release(200, 40)
        );

        holds.start(Lane::Down, Samples(300), Samples(200));
        assert_eq!(
            holds.score_ticks(Samples(350)),
            vec![hold_tick(Lane::Down, 100)]
        );
        assert_eq!(holds.complete_elapsed(Samples(350)), vec![Lane::Down]);
    }
}
