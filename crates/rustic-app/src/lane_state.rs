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

#[derive(Debug, Default, Clone)]
pub struct ActiveHolds {
    ends_at: [Option<Samples>; 4],
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

    pub fn hold_confirm(&mut self, lane: Lane, cursor: Samples, duration: Samples) {
        let index = lane_index(lane);
        if !self.is_confirming(lane, cursor) {
            self.confirm_started[index] = cursor;
        }
        let until = Samples(cursor.0 + duration.0.max(0));
        self.confirm_until[index] = self.confirm_until[index].max(until);
    }

    pub fn play_static(&mut self, lane: Lane) {
        let index = lane_index(lane);
        self.confirm_started[index] = Samples(0);
        self.confirm_until[index] = Samples(0);
    }

    pub fn play_press(&mut self, lane: Lane, started_at: Samples) {
        let index = lane_index(lane);
        if self.pressed[index] {
            self.pressed_started[index] = started_at;
        }
        self.play_static(lane);
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

impl ActiveHolds {
    pub fn start(&mut self, lane: Lane, hold_end_at: Samples, cursor: Samples) {
        if hold_end_at > cursor {
            self.ends_at[lane_index(lane)] = Some(hold_end_at);
        }
    }

    pub fn release(&mut self, lane: Lane, cursor: Samples) -> Option<Samples> {
        let active = self.ends_at[lane_index(lane)].take()?;
        (active > cursor).then_some(active)
    }

    pub fn active_lanes(&self, cursor: Samples) -> impl Iterator<Item = Lane> + '_ {
        LANES
            .iter()
            .copied()
            .enumerate()
            .filter(move |(idx, _)| self.ends_at[*idx].is_some_and(|end| end > cursor))
            .map(|(_, lane)| lane)
    }

    pub fn complete_elapsed(&mut self, cursor: Samples) -> Vec<Lane> {
        let mut completed = Vec::new();
        for (idx, lane) in LANES.iter().copied().enumerate() {
            if self.ends_at[idx].is_some_and(|end| end <= cursor) {
                self.ends_at[idx] = None;
                completed.push(lane);
            }
        }
        completed
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

    #[test]
    fn hold_confirm_extends_without_restarting_active_confirm() {
        let mut held = HeldLanes::default();
        held.confirm(Lane::Left, Samples(50), Samples(100));
        held.hold_confirm(Lane::Left, Samples(80), Samples(100));

        assert_eq!(
            held.receptor_state(Lane::Left, Samples(120)),
            ReceptorState::Confirm {
                started_at: Samples(50)
            }
        );
        assert!(held.is_confirming(Lane::Left, Samples(180)));
        assert!(!held.is_confirming(Lane::Left, Samples(181)));
    }

    #[test]
    fn release_forces_receptor_back_to_static() {
        let mut held = HeldLanes::default();
        held.apply(&event_at(
            InputAction::LaneLeft,
            InputState::Pressed,
            Samples(25),
        ));
        held.confirm(Lane::Left, Samples(50), Samples(100));
        held.apply(&event_at(
            InputAction::LaneLeft,
            InputState::Released,
            Samples(60),
        ));
        held.play_static(Lane::Left);

        assert_eq!(
            held.receptor_state(Lane::Left, Samples(61)),
            ReceptorState::Static
        );
    }

    #[test]
    fn completed_hold_restarts_press_if_key_is_still_down() {
        let mut held = HeldLanes::default();
        held.apply(&event_at(
            InputAction::LaneLeft,
            InputState::Pressed,
            Samples(25),
        ));
        held.confirm(Lane::Left, Samples(50), Samples(100));
        held.play_press(Lane::Left, Samples(80));

        assert_eq!(
            held.receptor_state(Lane::Left, Samples(81)),
            ReceptorState::Pressed {
                started_at: Samples(80)
            }
        );
    }

    #[test]
    fn active_holds_report_drops_and_completions_once() {
        let mut holds = ActiveHolds::default();
        holds.start(Lane::Left, Samples(200), Samples(100));

        let lanes: Vec<_> = holds.active_lanes(Samples(150)).collect();
        assert_eq!(lanes, vec![Lane::Left]);
        assert_eq!(holds.release(Lane::Left, Samples(160)), Some(Samples(200)));
        assert_eq!(holds.release(Lane::Left, Samples(170)), None);

        holds.start(Lane::Down, Samples(300), Samples(200));
        assert_eq!(holds.complete_elapsed(Samples(299)), Vec::<Lane>::new());
        assert_eq!(holds.complete_elapsed(Samples(300)), vec![Lane::Down]);
        assert_eq!(holds.complete_elapsed(Samples(301)), Vec::<Lane>::new());
    }
}
