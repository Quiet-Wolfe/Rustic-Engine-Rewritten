//! App-owned lane hold state for gameplay input.

use rustic_core::input::{InputAction, InputState, NormalizedInputEvent};
use rustic_game::Lane;

const LANES: [Lane; 4] = [Lane::Left, Lane::Down, Lane::Up, Lane::Right];

#[derive(Debug, Default, Clone)]
pub struct HeldLanes {
    pressed: [bool; 4],
}

impl HeldLanes {
    pub fn apply(&mut self, event: &NormalizedInputEvent) -> Option<Lane> {
        let lane = lane_for_action(event.action)?;
        self.pressed[lane_index(lane)] = event.state == InputState::Pressed;
        Some(lane)
    }

    pub fn active_lanes(&self) -> impl Iterator<Item = Lane> + '_ {
        LANES
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(idx, lane)| self.pressed[idx].then_some(lane))
    }

    #[cfg(test)]
    fn is_held(&self, lane: Lane) -> bool {
        self.pressed[lane_index(lane)]
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
    use rustic_core::time::Samples;

    fn event(action: InputAction, state: InputState) -> NormalizedInputEvent {
        NormalizedInputEvent::new(action, state, 0, Samples(0))
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
}
