use anyhow::{Context, Result};
use rustic_asset::{
    load_animate_animation, load_animate_spritemap, load_png, AnimateAnimation, AnimateAtlas,
    AnimateDrawPart, AssetPath, OverlayResolver,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const STARS_LABEL: &str = "diff stars";
const STARS_FPS: u16 = 24;
const STARS_BLOCK_FRAMES: u32 = 100;
const STARS_ZERO_FRAME: u32 = 1500;
const STARS_POS: glam::Vec2 = glam::vec2(1280.0 - 330.0, 209.0);

#[derive(Debug)]
pub(super) struct FreeplayDifficultyStars {
    texture_id: AssetId,
    atlas: AnimateAtlas,
    animation: AnimateAnimation,
}

impl FreeplayDifficultyStars {
    pub(super) fn commands(
        &self,
        rating: u8,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        let frame = difficulty_frame(rating, cursor, sample_rate);
        self.animation
            .flatten_label_frame(STARS_LABEL, frame)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|part| self.command_for_part(&part))
            .collect()
    }

    fn command_for_part(&self, part: &AnimateDrawPart) -> Option<DrawCommand> {
        let frame = self.atlas.frame(&part.frame_name)?;
        let mut cmd = DrawCommand::sprite(self.texture_id, STARS_POS, frame.size);
        cmd.camera = CameraId(1);
        cmd.layer = RenderLayer::Overlay;
        cmd.z = 317;
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
}

pub(super) fn load_freeplay_difficulty_stars(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<FreeplayDifficultyStars> {
    let animation_path = AssetPath::new("images/freeplay/freeplayStars/Animation.json")?;
    let spritemap_path = AssetPath::new("images/freeplay/freeplayStars/spritemap1.json")?;
    let texture_path = AssetPath::new("images/freeplay/freeplayStars/spritemap1.png")?;
    let animation = load_animate_animation(resolver, &animation_path)
        .with_context(|| format!("load {animation_path}"))?;
    let atlas = load_animate_spritemap(resolver, &spritemap_path)
        .with_context(|| format!("load {spritemap_path}"))?;
    let image =
        load_png(resolver, &texture_path).with_context(|| format!("load {texture_path}"))?;
    let texture_id = asset_id_for_path(&texture_path);
    textures.insert(
        texture_id,
        Texture::from_png_image(
            device,
            queue,
            &image,
            FilterMode::Linear,
            Some(texture_path.as_str()),
        ),
    );
    Ok(FreeplayDifficultyStars {
        texture_id,
        atlas,
        animation,
    })
}

fn difficulty_frame(rating: u8, cursor: Samples, sample_rate: u32) -> u32 {
    if rating == 0 {
        return STARS_ZERO_FRAME;
    }
    let block = u32::from(rating.clamp(1, 15) - 1);
    let elapsed = cursor.0.max(0) as u128;
    let frame = (elapsed * u128::from(STARS_FPS) / u128::from(sample_rate.max(1))) as u32;
    block * STARS_BLOCK_FRAMES + frame % STARS_BLOCK_FRAMES
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
    fn difficulty_rating_selects_hundred_frame_blocks() {
        assert_eq!(difficulty_frame(0, Samples(0), 48_000), STARS_ZERO_FRAME);
        assert_eq!(difficulty_frame(1, Samples(0), 48_000), 0);
        assert_eq!(difficulty_frame(8, Samples(0), 48_000), 700);
        assert_eq!(difficulty_frame(99, Samples(0), 48_000), 1400);
    }

    #[test]
    fn difficulty_rating_loops_inside_selected_block() {
        assert_eq!(difficulty_frame(2, Samples(0), 48_000), 100);
        assert_eq!(difficulty_frame(2, Samples(2_000), 48_000), 101);
        assert_eq!(difficulty_frame(2, Samples(2_000 * 100), 48_000), 100);
    }
}
