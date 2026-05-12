//! Title-screen asset wiring from Funkin' v0.8.5.
//!
//! ref: bdedc0aa:source/funkin/ui/title/TitleState.hx:72-121,380-397
// LINT-ALLOW: long-file title screen Sparrow and Animate wiring stay together.

use crate::asset_roots::baked_assets_root;
use anyhow::{Context, Result};
use rustic_asset::{
    load_animate_animation, load_animate_spritemap, load_png, load_sparrow, AnimateAnimation,
    AnimateAtlas, AssetPath, OverlayResolver, SparrowFrame,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;

const TITLE_ANIMATION_FPS: u16 = 24;
const TITLE_BPM: f64 = 102.0;
const LOGO_POS: glam::Vec2 = glam::vec2(-150.0, -100.0);
const GF_POS: glam::Vec2 = glam::vec2(1280.0 * 0.4, 720.0 * 0.07);
const PROMPT_POS: glam::Vec2 = glam::vec2(682.75, 720.0 * 0.8);

#[derive(Debug)]
pub struct TitleScreenAssets {
    logo: SparrowClip,
    girlfriend: GirlfriendDanceClip,
    prompt: AnimateClip,
    pub textures: HashMap<AssetId, Texture>,
}

impl TitleScreenAssets {
    pub fn commands(&self, cursor: Samples, sample_rate: u32) -> RenderCommandList {
        let mut commands = RenderCommandList::new();
        if let Some(cmd) =
            self.logo
                .command(cursor, sample_rate, current_beat_start(cursor, sample_rate))
        {
            commands.push(cmd);
        }
        if let Some(cmd) = self.girlfriend.command(cursor, sample_rate) {
            commands.push(cmd);
        }
        for cmd in self.prompt.commands(cursor, sample_rate) {
            commands.push(cmd);
        }
        commands
    }
}

#[derive(Debug, Clone)]
struct SparrowClip {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frames: Vec<SparrowFrame>,
    position: glam::Vec2,
    z: i32,
}

impl SparrowClip {
    fn command(
        &self,
        cursor: Samples,
        sample_rate: u32,
        started_at: Samples,
    ) -> Option<DrawCommand> {
        let frame = self.frames.get(animation_frame_index(
            cursor,
            sample_rate,
            started_at,
            TITLE_ANIMATION_FPS,
            self.frames.len(),
            false,
        ))?;
        Some(sparrow_command(
            self.texture_id,
            self.texture_width,
            self.texture_height,
            frame,
            self.position,
            self.z,
        ))
    }
}

#[derive(Debug, Clone)]
struct GirlfriendDanceClip {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    dance_left: Vec<SparrowFrame>,
    dance_right: Vec<SparrowFrame>,
    position: glam::Vec2,
}

impl GirlfriendDanceClip {
    fn command(&self, cursor: Samples, sample_rate: u32) -> Option<DrawCommand> {
        let beat = title_beat(cursor, sample_rate).max(1);
        let frames = if beat % 2 == 0 {
            &self.dance_left
        } else {
            &self.dance_right
        };
        let frame = frames.get(animation_frame_index(
            cursor,
            sample_rate,
            current_beat_start(cursor, sample_rate),
            TITLE_ANIMATION_FPS,
            frames.len(),
            false,
        ))?;
        Some(sparrow_command(
            self.texture_id,
            self.texture_width,
            self.texture_height,
            frame,
            self.position,
            1,
        ))
    }
}

#[derive(Debug, Clone)]
struct AnimateClip {
    texture_id: AssetId,
    atlas: AnimateAtlas,
    animation: AnimateAnimation,
    label: &'static str,
    position: glam::Vec2,
    z: i32,
}

impl AnimateClip {
    fn commands(&self, cursor: Samples, sample_rate: u32) -> Vec<DrawCommand> {
        let Some(label) = self.animation.label(self.label) else {
            return Vec::new();
        };
        let frame = animation_frame_index(
            cursor,
            sample_rate,
            Samples(0),
            TITLE_ANIMATION_FPS,
            label.duration as usize,
            true,
        ) as u32;
        self.animation
            .flatten_label_frame(self.label, frame)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|part| {
                let frame = self.atlas.frame(&part.frame_name)?;
                let mut cmd = DrawCommand::sprite(self.texture_id, self.position, frame.size);
                cmd.camera = CameraId(1);
                cmd.layer = RenderLayer::Overlay;
                cmd.z = self.z;
                cmd.pivot = glam::Vec2::ZERO;
                cmd.filter = FilterMode::Linear;
                cmd.affine = part.matrix;
                cmd.uv_min = frame.uv_min;
                cmd.uv_max = frame.uv_max;
                cmd.uv_rotated = frame.rotated;
                cmd.color = glam::Vec4::from_array(part.color);
                cmd.color_offset = glam::Vec4::from_array(part.color_offset);
                Some(cmd)
            })
            .collect()
    }
}

pub fn load_title_screen_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<TitleScreenAssets> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let mut textures = HashMap::new();
    let logo = load_sparrow_clip(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/logoBumpin.xml",
        "logo bumpin",
        LOGO_POS,
        0,
    )?;
    let gf = load_girlfriend_clip(device, queue, &resolver, &mut textures)?;
    let prompt = load_animate_clip(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/title-screen-text",
        "Idle",
        PROMPT_POS,
        2,
    )?;
    Ok(TitleScreenAssets {
        logo,
        girlfriend: gf,
        prompt,
        textures,
    })
}

fn load_sparrow_clip(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    atlas_path: &str,
    prefix: &str,
    position: glam::Vec2,
    z: i32,
) -> Result<SparrowClip> {
    let atlas_path = AssetPath::new(atlas_path)?;
    let atlas = load_sparrow(resolver, &atlas_path)?;
    let (texture_id, texture_width, texture_height) =
        load_sparrow_texture(device, queue, resolver, textures, &atlas_path, &atlas)?;
    let frames = atlas
        .animation_frames(prefix, &[])
        .into_iter()
        .cloned()
        .collect();
    Ok(SparrowClip {
        texture_id,
        texture_width,
        texture_height,
        frames,
        position,
        z,
    })
}

fn load_girlfriend_clip(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<GirlfriendDanceClip> {
    let atlas_path = AssetPath::new("images/gfDanceTitle.xml")?;
    let atlas = load_sparrow(resolver, &atlas_path)?;
    let (texture_id, texture_width, texture_height) =
        load_sparrow_texture(device, queue, resolver, textures, &atlas_path, &atlas)?;
    let dance_left = atlas
        .animation_frames(
            "gfDance",
            &[30, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
        )
        .into_iter()
        .cloned()
        .collect();
    let dance_right = atlas
        .animation_frames(
            "gfDance",
            &[15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29],
        )
        .into_iter()
        .cloned()
        .collect();
    Ok(GirlfriendDanceClip {
        texture_id,
        texture_width,
        texture_height,
        dance_left,
        dance_right,
        position: GF_POS,
    })
}

fn load_animate_clip(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    asset_dir: &str,
    label: &'static str,
    position: glam::Vec2,
    z: i32,
) -> Result<AnimateClip> {
    let animation_path = AssetPath::new(format!("{asset_dir}/Animation.json"))?;
    let spritemap_path = AssetPath::new(format!("{asset_dir}/spritemap1.json"))?;
    let texture_path = AssetPath::new(format!("{asset_dir}/spritemap1.png"))?;
    let animation = load_animate_animation(resolver, &animation_path)?;
    let atlas = load_animate_spritemap(resolver, &spritemap_path)?;
    let image = load_png(resolver, &texture_path)?;
    let texture_id = asset_id_for_path(&texture_path);
    let texture = Texture::from_png_image(
        device,
        queue,
        &image,
        FilterMode::Linear,
        Some(texture_path.as_str()),
    );
    textures.insert(texture_id, texture);
    Ok(AnimateClip {
        texture_id,
        atlas,
        animation,
        label,
        position,
        z,
    })
}

fn load_sparrow_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    atlas_path: &AssetPath,
    atlas: &rustic_asset::SparrowAtlas,
) -> Result<(AssetId, u32, u32)> {
    let texture_path = if atlas.image_path.contains('/') {
        AssetPath::new(atlas.image_path.clone())?
    } else {
        atlas_path.sibling(&atlas.image_path)?
    };
    let image =
        load_png(resolver, &texture_path).with_context(|| format!("load {texture_path}"))?;
    let texture_id = asset_id_for_path(&texture_path);
    let (width, height) = (image.width, image.height);
    let texture = Texture::from_png_image(
        device,
        queue,
        &image,
        FilterMode::Linear,
        Some(texture_path.as_str()),
    );
    textures.insert(texture_id, texture);
    Ok((texture_id, width, height))
}

fn sparrow_command(
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frame: &SparrowFrame,
    position: glam::Vec2,
    z: i32,
) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(
        texture_id,
        position - glam::vec2(frame.frame_x as f32, frame.frame_y as f32),
        frame_draw_size(frame),
    );
    cmd.camera = CameraId(1);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Linear;
    (cmd.uv_min, cmd.uv_max) = frame_uv(frame, texture_width, texture_height);
    cmd.uv_rotated = frame.rotated;
    cmd
}

fn animation_frame_index(
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
    fps: u16,
    frame_count: usize,
    looped: bool,
) -> usize {
    if frame_count <= 1 {
        return 0;
    }
    let elapsed = cursor.0.saturating_sub(started_at.0).max(0) as u128;
    let index = (elapsed * u128::from(fps) / u128::from(sample_rate.max(1))) as usize;
    if looped {
        index % frame_count
    } else {
        index.min(frame_count - 1)
    }
}

fn title_beat(cursor: Samples, sample_rate: u32) -> i64 {
    let beat_samples = f64::from(sample_rate.max(1)) * 60.0 / TITLE_BPM;
    (cursor.0.max(0) as f64 / beat_samples).floor() as i64
}

fn current_beat_start(cursor: Samples, sample_rate: u32) -> Samples {
    let beat_samples = f64::from(sample_rate.max(1)) * 60.0 / TITLE_BPM;
    Samples((title_beat(cursor, sample_rate) as f64 * beat_samples).round() as i64)
}

fn frame_draw_size(frame: &SparrowFrame) -> glam::Vec2 {
    if frame.rotated {
        glam::vec2(frame.height as f32, frame.width as f32)
    } else {
        glam::vec2(frame.width as f32, frame.height as f32)
    }
}

fn frame_uv(
    frame: &SparrowFrame,
    texture_width: u32,
    texture_height: u32,
) -> (glam::Vec2, glam::Vec2) {
    let width = texture_width.max(1) as f32;
    let height = texture_height.max(1) as f32;
    (
        glam::vec2(frame.x as f32 / width, frame.y as f32 / height),
        glam::vec2(
            (frame.x as f32 + frame.width as f32) / width,
            (frame.y as f32 + frame.height as f32) / height,
        ),
    )
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
    fn title_beat_uses_menu_bpm() {
        assert_eq!(title_beat(Samples(0), 48_000), 0);
        assert_eq!(title_beat(Samples(48_000), 48_000), 1);
    }

    #[test]
    fn animation_frame_index_clamps_or_wraps() {
        assert_eq!(
            animation_frame_index(Samples(0), 48_000, Samples(0), 24, 3, false),
            0
        );
        assert_eq!(
            animation_frame_index(Samples(96_000), 48_000, Samples(0), 24, 3, false),
            2
        );
        assert_eq!(
            animation_frame_index(Samples(96_000), 48_000, Samples(0), 24, 3, true),
            0
        );
    }
}
