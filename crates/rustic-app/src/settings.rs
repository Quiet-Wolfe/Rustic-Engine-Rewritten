//! Settings loader skeleton. See `PLAN.md` Section 12.
//!
//! Atomic writes (write to temp + rename), `.bad` rename on corrupt parse,
//! and binary save migrations land alongside Phase 6/8 once the screens
//! that drive them exist.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Settings {
    pub render: RenderSettings,
    pub audio: AudioSettings,
    pub input: InputSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
#[non_exhaustive]
pub struct RenderSettings {
    /// Optional backend override, e.g. "vulkan", "dx12". `None` lets
    /// `wgpu::Backends::PRIMARY` choose. Debug-only.
    pub backend_override: Option<String>,
    /// Whether to surface `wgpu` adapter/backend/limits in the F3 overlay.
    pub debug_show_adapter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
#[non_exhaustive]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
#[non_exhaustive]
pub struct InputSettings {
    /// Touch enable for Android. See `PLAN.md` Section 10.
    pub touch_enabled: bool,
    pub touch_lane_height_px: u32,
    pub touch_opacity: f32,
}
