//! winit -> `NormalizedInputEvent`. See `PLAN.md` Section 10.
//!
//! Wall-clock and audio-cursor stamps are captured at platform-event
//! receipt. Wall clock is from `std::time::Instant`; the song cursor is
//! sampled by the app because countdown time can be negative before the
//! mixer starts.

use rustic_core::input::{InputAction, InputState, NormalizedInputEvent};
use rustic_core::time::Samples;
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
        KeyCode::KeyZ | KeyCode::Enter | KeyCode::Space => InputAction::Confirm,
        KeyCode::KeyX | KeyCode::Backspace | KeyCode::Escape => InputAction::Back,
        KeyCode::KeyP => InputAction::Pause,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => InputAction::UiPauseScroll,
        KeyCode::F3 => InputAction::Debug,
        KeyCode::KeyR | KeyCode::F5 => InputAction::Reset,
        KeyCode::KeyQ | KeyCode::F6 => InputAction::UiLeft,
        KeyCode::KeyE | KeyCode::F7 => InputAction::UiRight,
        KeyCode::Tab => InputAction::UiSelect,
        KeyCode::Pause => InputAction::Pause,
        _ => return None,
    })
}

pub fn build_event(
    action: InputAction,
    state: ElementState,
    boot_instant: std::time::Instant,
    song_cursor: Samples,
) -> NormalizedInputEvent {
    let wall_clock_ns = boot_instant.elapsed().as_nanos() as u64;
    let state = match state {
        ElementState::Pressed => InputState::Pressed,
        ElementState::Released => InputState::Released,
    };
    NormalizedInputEvent::new(action, state, wall_clock_ns, song_cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_keys_drive_preview_selection_actions() {
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::KeyQ)),
            Some(InputAction::UiLeft)
        );
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::F6)),
            Some(InputAction::UiLeft)
        );
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::KeyE)),
            Some(InputAction::UiRight)
        );
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::F7)),
            Some(InputAction::UiRight)
        );
    }

    #[test]
    fn p_key_drives_pause_action() {
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::KeyP)),
            Some(InputAction::Pause)
        );
    }

    #[test]
    fn tab_drives_ui_select_action() {
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::Tab)),
            Some(InputAction::UiSelect)
        );
    }

    #[test]
    fn shift_drives_menu_pause_scroll_action() {
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::ShiftLeft)),
            Some(InputAction::UiPauseScroll)
        );
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::ShiftRight)),
            Some(InputAction::UiPauseScroll)
        );
    }

    #[test]
    fn desktop_default_action_keys_match_supported_funkin_binds() {
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::KeyZ)),
            Some(InputAction::Confirm)
        );
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::KeyX)),
            Some(InputAction::Back)
        );
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::Backspace)),
            Some(InputAction::Back)
        );
        assert_eq!(
            map_key(PhysicalKey::Code(KeyCode::KeyR)),
            Some(InputAction::Reset)
        );
    }
}
