//! Adobe Animate atlas data shapes. Phase 7 fills in the parser and the
//! per-frame draw call expansion.

use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Frame {
    pub name: String,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub size: Vec2,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct Sprite {
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Atlas {
    pub sprites: Vec<Sprite>,
}
