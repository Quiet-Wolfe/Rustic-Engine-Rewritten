//! App-owned lane hold state for gameplay input.

use crate::active_holds::ActiveHolds;
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
    hold_confirming: [bool; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceptorState {
    Static,
    Pressed { started_at: Samples },
    Confirm { started_at: Samples, hold: bool },
}

#[derive(Debug, Default, Clone)]
pub struct AutoReceptors {
    holds: ActiveHolds,
    lanes: HeldLanes,
}

impl AutoReceptors {
    pub fn update(&mut self, cursor: Samples, confirm_duration: Samples) {
        for lane in self.holds.complete_elapsed(cursor) {
            self.lanes.play_static(lane);
        }
        let active_lanes: Vec<_> = self.holds.active_lanes(cursor).collect();
        for lane in active_lanes {
            self.lanes.hold_confirm(lane, cursor, confirm_duration);
        }
    }

    pub fn confirm(&mut self, lane: Lane, cursor: Samples, confirm_duration: Samples) {
        self.lanes.confirm(lane, cursor, confirm_duration);
    }

    pub fn start_hold(
        &mut self,
        lane: Lane,
        hold_end_at: Samples,
        cursor: Samples,
        note_id: rustic_core::ids::NoteId,
    ) {
        self.holds.start(lane, hold_end_at, cursor, note_id);
    }

    pub fn receptor_state(&self, lane: Lane, cursor: Samples) -> ReceptorState {
        self.lanes.receptor_state(lane, cursor)
    }
}

impl HeldLanes {
    pub fn apply(&mut self, event: &NormalizedInputEvent) -> Option<Lane> {
        let lane = lane_for_action(event.action)?;
        let index = lane_index(lane);
        let was_pressed = self.pressed[index];
        self.pressed[index] = event.state == InputState::Pressed;
        if event.state == InputState::Pressed && !was_pressed {
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
        self.hold_confirming[index] = false;
    }

    pub fn hold_confirm(&mut self, lane: Lane, cursor: Samples, duration: Samples) {
        let index = lane_index(lane);
        if !self.is_confirming(lane, cursor) {
            self.confirm_started[index] = cursor;
        }
        self.hold_confirming[index] = true;
        let until = Samples(cursor.0 + duration.0.max(0));
        self.confirm_until[index] = self.confirm_until[index].max(until);
    }

    pub fn complete_hold(&mut self, lane: Lane) {
        let index = lane_index(lane);
        self.hold_confirming[index] = false;
    }

    pub fn play_static(&mut self, lane: Lane) {
        let index = lane_index(lane);
        self.confirm_started[index] = Samples(0);
        self.confirm_until[index] = Samples(0);
        self.hold_confirming[index] = false;
    }

    pub fn play_press(&mut self, lane: Lane, started_at: Samples) {
        let index = lane_index(lane);
        if self.pressed[index] {
            self.pressed_started[index] = started_at;
        }
        self.play_static(lane);
    }

    pub fn is_confirming(&self, lane: Lane, cursor: Samples) -> bool {
        let index = lane_index(lane);
        let until = self.confirm_until[index];
        if until.0 == 0 {
            return false;
        }
        until >= cursor || self.pressed[index]
    }

    pub fn receptor_state(&self, lane: Lane, cursor: Samples) -> ReceptorState {
        let index = lane_index(lane);
        if self.is_confirming(lane, cursor) {
            ReceptorState::Confirm {
                started_at: self.confirm_started[index],
                hold: self.hold_confirming[index],
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
                started_at: Samples(50),
                hold: false
            }
        );
        assert_eq!(
            held.receptor_state(Lane::Left, Samples(61)),
            ReceptorState::Confirm {
                started_at: Samples(50),
                hold: false
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
                started_at: Samples(50),
                hold: true
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
    fn repeated_press_does_not_restart_pressed_animation() {
        let mut held = HeldLanes::default();
        held.apply(&event_at(
            InputAction::LaneLeft,
            InputState::Pressed,
            Samples(25),
        ));
        held.apply(&event_at(
            InputAction::LaneLeft,
            InputState::Pressed,
            Samples(80),
        ));

        assert_eq!(
            held.receptor_state(Lane::Left, Samples(81)),
            ReceptorState::Pressed {
                started_at: Samples(25)
            }
        );
    }

    #[test]
    fn auto_receptors_confirm_then_hold_until_completion() {
        let mut receptors = AutoReceptors::default();
        receptors.confirm(Lane::Left, Samples(10), Samples(100));
        assert_eq!(
            receptors.receptor_state(Lane::Left, Samples(20)),
            ReceptorState::Confirm {
                started_at: Samples(10),
                hold: false
            }
        );

        receptors.start_hold(
            Lane::Left,
            Samples(500),
            Samples(10),
            rustic_core::ids::NoteId::new(0),
        );
        receptors.update(Samples(200), Samples(100));
        assert_eq!(
            receptors.receptor_state(Lane::Left, Samples(200)),
            ReceptorState::Confirm {
                started_at: Samples(200),
                hold: true
            }
        );

        receptors.update(Samples(500), Samples(100));
        assert_eq!(
            receptors.receptor_state(Lane::Left, Samples(500)),
            ReceptorState::Static
        );
    }
}
