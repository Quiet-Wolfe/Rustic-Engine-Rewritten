//! winit -> `NormalizedInputEvent`. See `PLAN.md` Section 10.
//!
//! Wall-clock and audio-cursor stamps are captured at platform-event
//! receipt. Wall clock is from `std::time::Instant`; the audio cursor is
//! sampled from the mixer (passed in by the caller, who owns it).

use rustic_audio::Mixer;
use rustic_core::input::{InputAction, InputState, NormalizedInputEvent};
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};

/// Default base-FNF binding: arrow keys for lanes, enter/escape for
/// confirm/back. Future settings will let users rebind.
pub fn map_key(key: PhysicalKey) -> Option<InputAction> {
    let PhysicalKey::Code(code) = key else {
        return None;
    };
    Some(match code {
        KeyCode::ArrowLeft | KeyCode::KeyA => InputAction::LaneLeft,
        KeyCode::ArrowDown | KeyCode::KeyS => InputAction::LaneDown,
        KeyCode::ArrowUp | KeyCode::KeyW => InputAction::LaneUp,
        KeyCode::ArrowRight | KeyCode::KeyD => InputAction::LaneRight,
        KeyCode::Enter | KeyCode::Space => InputAction::Confirm,
        KeyCode::Escape => InputAction::Back,
        KeyCode::F3 => InputAction::Debug,
        KeyCode::F5 => InputAction::Reset,
        KeyCode::Pause => InputAction::Pause,
        _ => return None,
    })
}

pub fn build_event(
    action: InputAction,
    state: ElementState,
    boot_instant: std::time::Instant,
    mixer: &Mixer,
) -> NormalizedInputEvent {
    let wall_clock_ns = boot_instant.elapsed().as_nanos() as u64;
    let state = match state {
        ElementState::Pressed => InputState::Pressed,
        ElementState::Released => InputState::Released,
    };
    NormalizedInputEvent::new(action, state, wall_clock_ns, mixer.sample_cursor())
}
