//! App-owned lane hold state for gameplay input.

use rustic_core::input::{InputAction, InputState, NormalizedInputEvent};
use rustic_core::time::Samples;
use rustic_game::Lane;

const LANES: [Lane; 4] = [Lane::Left, Lane::Down, Lane::Up, Lane::Right];

#[derive(Debug, Default, Clone)]
pub struct HeldLanes {
    pressed: [bool; 4],
    pressed_started: [Samples; 4],
    confirm_started: [Samples; 4],
    confirm_until: [Samples; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceptorState {
    Static,
    Pressed { started_at: Samples },
    Confirm { started_at: Samples },
}

impl HeldLanes {
    pub fn apply(&mut self, event: &NormalizedInputEvent) -> Option<Lane> {
        let lane = lane_for_action(event.action)?;
        let index = lane_index(lane);
        self.pressed[index] = event.state == InputState::Pressed;
        if event.state == InputState::Pressed {
            self.pressed_started[index] = event.audio_sample_cursor_at_receive;
        }
        Some(lane)
    }

    pub fn active_lanes(&self) -> impl Iterator<Item = Lane> + '_ {
        LANES
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(idx, lane)| self.pressed[idx].then_some(lane))
    }

    pub fn is_held(&self, lane: Lane) -> bool {
        self.pressed[lane_index(lane)]
    }

    pub fn confirm(&mut self, lane: Lane, started_at: Samples, duration: Samples) {
        let index = lane_index(lane);
        self.confirm_started[index] = started_at;
        self.confirm_until[index] = Samples(started_at.0 + duration.0.max(0));
    }

    pub fn is_confirming(&self, lane: Lane, cursor: Samples) -> bool {
        let until = self.confirm_until[lane_index(lane)];
        until.0 > 0 && until >= cursor
    }

    pub fn receptor_state(&self, lane: Lane, cursor: Samples) -> ReceptorState {
        let index = lane_index(lane);
        if self.is_confirming(lane, cursor) {
            ReceptorState::Confirm {
                started_at: self.confirm_started[index],
            }
        } else if self.pressed[index] {
            ReceptorState::Pressed {
                started_at: self.pressed_started[index],
            }
        } else {
            ReceptorState::Static
        }
    }
}

pub fn lane_for_action(action: InputAction) -> Option<Lane> {
    match action {
        InputAction::LaneLeft => Some(Lane::Left),
        InputAction::LaneDown => Some(Lane::Down),
        InputAction::LaneUp => Some(Lane::Up),
        InputAction::LaneRight => Some(Lane::Right),
        _ => None,
    }
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

    fn event_at(action: InputAction, state: InputState, cursor: Samples) -> NormalizedInputEvent {
        NormalizedInputEvent::new(action, state, 0, cursor)
    }

    fn event(action: InputAction, state: InputState) -> NormalizedInputEvent {
        event_at(action, state, Samples(0))
    }

    #[test]
    fn lane_events_update_held_state() {
        let mut held = HeldLanes::default();

        assert_eq!(
            held.apply(&event(InputAction::LaneLeft, InputState::Pressed)),
            Some(Lane::Left)
        );
        assert!(held.is_held(Lane::Left));

        held.apply(&event(InputAction::LaneLeft, InputState::Released));
        assert!(!held.is_held(Lane::Left));
    }

    #[test]
    fn non_lane_events_do_not_change_held_state() {
        let mut held = HeldLanes::default();

        assert_eq!(
            held.apply(&event(InputAction::Confirm, InputState::Pressed)),
            None
        );
        assert!(held.active_lanes().next().is_none());
    }

    #[test]
    fn active_lanes_preserve_fnf_lane_order() {
        let mut held = HeldLanes::default();
        held.apply(&event(InputAction::LaneRight, InputState::Pressed));
        held.apply(&event(InputAction::LaneDown, InputState::Pressed));

        let lanes: Vec<_> = held.active_lanes().collect();
        assert_eq!(lanes, vec![Lane::Down, Lane::Right]);
    }

    #[test]
    fn confirm_window_is_sample_cursor_based() {
        let mut held = HeldLanes::default();
        held.confirm(Lane::Up, Samples(100), Samples(100));

        assert!(held.is_confirming(Lane::Up, Samples(199)));
        assert!(held.is_confirming(Lane::Up, Samples(200)));
        assert!(!held.is_confirming(Lane::Up, Samples(201)));
    }

    #[test]
    fn receptor_state_tracks_press_and_confirm_starts() {
        let mut held = HeldLanes::default();
        held.apply(&event_at(
            InputAction::LaneLeft,
            InputState::Pressed,
            Samples(25),
        ));
        assert_eq!(
            held.receptor_state(Lane::Left, Samples(30)),
            ReceptorState::Pressed {
                started_at: Samples(25)
            }
        );

        held.confirm(Lane::Left, Samples(50), Samples(10));
        assert_eq!(
            held.receptor_state(Lane::Left, Samples(55)),
            ReceptorState::Confirm {
                started_at: Samples(50)
            }
        );
        assert_eq!(
            held.receptor_state(Lane::Left, Samples(61)),
            ReceptorState::Pressed {
                started_at: Samples(25)
            }
        );
    }
}
