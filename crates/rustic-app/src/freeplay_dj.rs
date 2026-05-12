//! Freeplay DJ sprite from Funkin' v0.8.5.
//!
//! Phase 2 covers the BF DJ idle frame-label loop. Confirm/intro/fistPump,
//! AFK easter egg, and the multi-style (Sparrow / Packer / MultiSparrow) DJ
//! paths come later - for now we hard-code BF's Adobe Animate atlas at
//! `images/freeplay/freeplay-boyfriend/`.
//!
//! ref: bdedc0aa:source/funkin/ui/freeplay/dj/BaseFreeplayDJ.hx
//! ref: bdedc0aa:source/funkin/ui/freeplay/dj/AnimateAtlasFreeplayDJ.hx
//! ref: bdedc0aa:assets/preload/data/players/bf.json

use crate::asset_roots::baked_assets_root;
use anyhow::{Context, Result};
use rustic_asset::{
    load_animate_animation, load_animate_spritemap, load_png, AnimateAnimation, AnimateAtlas,
    AnimateDrawPart, AssetPath, OverlayResolver,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, Texture};

// ref: bdedc0aa:source/funkin/ui/freeplay/dj/BaseFreeplayDJ.hx:221-233 (resetPosition non-widescreen branch)
const DJ_FPS: u16 = 24;
const DJ_IDLE_LABEL: &str = "Idle";

/// Logical paths to assets the BF DJ depends on.
pub const REQUIRED_DJ_ASSETS: &[&str] = &[
    "images/freeplay/freeplay-boyfriend/Animation.json",
    "images/freeplay/freeplay-boyfriend/spritemap1.json",
    "images/freeplay/freeplay-boyfriend/spritemap1.png",
];

#[derive(Debug)]
pub struct FreeplayDJ {
    texture_id: AssetId,
    animation: AnimateAnimation,
    atlas: AnimateAtlas,
    idle_frame_count: u32,
    pub texture: Option<Texture>,
}

impl FreeplayDJ {
    pub fn commands(&self, cursor: Samples, sample_rate: u32) -> Vec<DrawCommand> {
        let frame_offset = label_frame_index(cursor, sample_rate, DJ_FPS, self.idle_frame_count);
        let Ok(parts) = self
            .animation
            .flatten_label_frame(DJ_IDLE_LABEL, frame_offset)
        else {
            return Vec::new();
        };
        parts
            .iter()
            .filter_map(|part| self.command_for_part(part))
            .collect()
    }

    fn command_for_part(&self, part: &AnimateDrawPart) -> Option<DrawCommand> {
        let frame = self.atlas.frame(&part.frame_name)?;
        let mut cmd = DrawCommand::sprite(
            self.texture_id,
            glam::vec2(0.0, 0.0),
            glam::vec2(frame.size.x, frame.size.y),
        );
        cmd.camera = CameraId(1);
        cmd.layer = RenderLayer::Characters;
        cmd.z = 100;
        cmd.pivot = glam::Vec2::ZERO;
        cmd.filter = FilterMode::Linear;
        cmd.affine = part.matrix;
        cmd.uv_min = frame.uv_min;
        cmd.uv_max = frame.uv_max;
        cmd.uv_rotated = frame.rotated;
        cmd.color = glam::Vec4::from_array(part.color);
        cmd.color_offset = glam::Vec4::from_array(part.color_offset);
        Some(cmd)
    }

    pub fn take_texture(&mut self) -> Option<(AssetId, Texture)> {
        self.texture.take().map(|tex| (self.texture_id, tex))
    }
}

pub fn load_freeplay_dj(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<FreeplayDJ> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let animation_path = AssetPath::new("images/freeplay/freeplay-boyfriend/Animation.json")?;
    let spritemap_path = AssetPath::new("images/freeplay/freeplay-boyfriend/spritemap1.json")?;
    let texture_path = AssetPath::new("images/freeplay/freeplay-boyfriend/spritemap1.png")?;
    let animation = load_animate_animation(&resolver, &animation_path)
        .with_context(|| format!("load {animation_path}"))?;
    let atlas = load_animate_spritemap(&resolver, &spritemap_path)
        .with_context(|| format!("load {spritemap_path}"))?;
    let image =
        load_png(&resolver, &texture_path).with_context(|| format!("load {texture_path}"))?;
    let texture_id = asset_id_for_path(&texture_path);
    let texture = Texture::from_png_image(
        device,
        queue,
        &image,
        FilterMode::Linear,
        Some(texture_path.as_str()),
    );
    let idle_frame_count = animation
        .label(DJ_IDLE_LABEL)
        .map(|label| label.duration.max(1))
        .unwrap_or(1);
    Ok(FreeplayDJ {
        texture_id,
        animation,
        atlas,
        idle_frame_count,
        texture: Some(texture),
    })
}

fn label_frame_index(cursor: Samples, sample_rate: u32, fps: u16, frame_count: u32) -> u32 {
    if frame_count <= 1 {
        return 0;
    }
    let elapsed = cursor.0.max(0) as u128;
    let frame = (elapsed * u128::from(fps) / u128::from(sample_rate.max(1))) as u32;
    frame % frame_count.max(1)
}

fn asset_id_for_path(path: &AssetPath) -> AssetId {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in path.as_str().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    AssetId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_frame_index_loops() {
        assert_eq!(label_frame_index(Samples(0), 48_000, 24, 10), 0);
        // 12 frames * (48000/24) samples = 24000 samples per frame
        assert_eq!(label_frame_index(Samples(2_000), 48_000, 24, 10), 1);
        assert_eq!(label_frame_index(Samples(2_000 * 10), 48_000, 24, 10), 0);
    }

    /// Locks the DJ source asset inventory.
    #[test]
    fn required_assets_present() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest_dir
            .parent()
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| manifest_dir.to_path_buf());
        let source_root = workspace.join("assets/source");
        let mut missing = Vec::new();
        for logical in REQUIRED_DJ_ASSETS {
            let path = source_root.join(logical);
            if !path.exists() {
                missing.push(path.display().to_string());
            }
        }
        assert!(
            missing.is_empty(),
            "freeplay DJ assets missing - DO NOT DELETE these files, they are required for the OG-fidelity DJ:\n{}",
            missing.join("\n"),
        );
    }
}
