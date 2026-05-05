//! Input enums and normalized input events. See `PLAN.md` Section 10.

use crate::time::Samples;
use serde::{Deserialize, Serialize};

/// Coarse semantic action a binding produces. Lane bindings are explicit
/// per-direction so gameplay never branches on raw key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputAction {
    LaneLeft,
    LaneDown,
    LaneUp,
    LaneRight,
    UiUp,
    UiDown,
    UiLeft,
    UiRight,
    Confirm,
    Back,
    Pause,
    Reset,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputState {
    Pressed,
    Released,
}

/// Wall-clock + audio-cursor stamped input event. Gameplay timing math uses
/// `audio_sample_cursor_at_receive`. Wall clock is preserved for diagnostics
/// and menu interactions.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct NormalizedInputEvent {
    pub action: InputAction,
    pub state: InputState,
    /// Monotonic wall-clock timestamp captured at platform-event receipt.
    pub wall_clock_ns: u64,
    /// Audio sample cursor sampled from the mixer at platform-event receipt.
    pub audio_sample_cursor_at_receive: Samples,
}

impl NormalizedInputEvent {
    pub fn new(
        action: InputAction,
        state: InputState,
        wall_clock_ns: u64,
        audio_sample_cursor_at_receive: Samples,
    ) -> Self {
        Self {
            action,
            state,
            wall_clock_ns,
            audio_sample_cursor_at_receive,
        }
    }
}
