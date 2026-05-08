//! Render command list. The game emits commands; the renderer consumes
//! them. See `PLAN.md` Section 7.

use crate::filter::FilterMode;
use glam::{Vec2, Vec4};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;

/// World-space sprite draw. The renderer batches these into instance
/// buffers grouped by atlas + sampler.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DrawCommand {
    pub camera: CameraId,
    pub layer: RenderLayer,
    /// Local z used for sub-layer ordering.
    pub z: i32,
    pub texture: AssetId,
    pub filter: FilterMode,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub uv_rotated: bool,
    pub world_pos: Vec2,
    pub size: Vec2,
    /// Pivot in unit-quad space (0..1). (0.5, 0.5) centers the sprite.
    pub pivot: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
    pub scroll_factor: Vec2,
    /// Extra local affine transform `[a, b, c, d, tx, ty]` applied after
    /// pivot/scale/rotation and before camera projection.
    pub affine: [f32; 6],
    pub color: Vec4,
}

impl DrawCommand {
    pub const IDENTITY_AFFINE: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

    pub fn sprite(texture: AssetId, world_pos: Vec2, size: Vec2) -> Self {
        Self {
            camera: CameraId(0),
            layer: RenderLayer::Stage,
            z: 0,
            texture,
            filter: FilterMode::Linear,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
            uv_rotated: false,
            world_pos,
            size,
            pivot: Vec2::splat(0.5),
            scale: Vec2::ONE,
            rotation: 0.0,
            scroll_factor: Vec2::ONE,
            affine: Self::IDENTITY_AFFINE,
            color: Vec4::ONE,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct RenderCommandList {
    commands: Vec<DrawCommand>,
}

impl RenderCommandList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, cmd: DrawCommand) {
        self.commands.push(cmd);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &DrawCommand> {
        self.commands.iter()
    }

    pub fn as_slice(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
