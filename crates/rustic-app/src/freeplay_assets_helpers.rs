use super::{
    CAPSULE_BASE_X, CAPSULE_BASE_Y, CAPSULE_BEAT_FPS, CAPSULE_BEAT_XFRAMES, CAPSULE_FRAME_HEIGHT,
    CAPSULE_FRAME_WIDTH, CAPSULE_REAL_SCALED, CAPSULE_SIN_AMPLITUDE, CAPSULE_SPACING_PAD,
    PINKBACK_TARGET_HEIGHT, WHITE_TEXTURE_ID,
};
// LINT-ALLOW: long-file freeplay helper math, asset inventory, and regression guards stay together.
use anyhow::{Context, Result};
use rustic_asset::{
    load_png, load_sparrow, AssetPath, OverlayResolver, SparrowAtlas, SparrowFrame,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

#[derive(Debug)]
pub(super) struct StaticTexture {
    pub(super) texture_id: AssetId,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) filter: FilterMode,
}

impl StaticTexture {
    pub(super) fn command(
        &self,
        pos: glam::Vec2,
        color: glam::Vec4,
        z: i32,
        draw_size: glam::Vec2,
    ) -> DrawCommand {
        self.command_on_layer(pos, color, z, draw_size, RenderLayer::Overlay)
    }

    pub(super) fn background_command(
        &self,
        pos: glam::Vec2,
        color: glam::Vec4,
        z: i32,
        draw_size: glam::Vec2,
    ) -> DrawCommand {
        self.command_on_layer(pos, color, z, draw_size, RenderLayer::Background)
    }

    fn command_on_layer(
        &self,
        pos: glam::Vec2,
        color: glam::Vec4,
        z: i32,
        draw_size: glam::Vec2,
        layer: RenderLayer,
    ) -> DrawCommand {
        let mut cmd = DrawCommand::sprite(self.texture_id, pos, draw_size);
        cmd.camera = CameraId(1);
        cmd.layer = layer;
        cmd.z = z;
        cmd.pivot = glam::Vec2::ZERO;
        cmd.filter = self.filter;
        cmd.color = color;
        cmd
    }
}

#[derive(Debug)]
pub(super) struct SparrowAtlasHandle {
    pub(super) texture_id: AssetId,
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) fn capsule_position(offset: f32) -> glam::Vec2 {
    let capsule_height_scaled = CAPSULE_FRAME_HEIGHT * CAPSULE_REAL_SCALED;
    let y = offset * (capsule_height_scaled + CAPSULE_SPACING_PAD) + CAPSULE_BASE_Y;
    let x = CAPSULE_BASE_X + CAPSULE_SIN_AMPLITUDE * offset.sin();
    glam::vec2(x, y)
}

pub(super) fn bg_image_scale(bg: &StaticTexture) -> f32 {
    PINKBACK_TARGET_HEIGHT / bg.height.max(1) as f32
}

/// Width of the solid yellow rectangle behind BF. The triangle extension
/// starts at the right edge of this rectangle.
pub(super) const PINKBACK_RECT_WIDTH: f32 = 360.0;

/// Width of the right-triangle extension drawn past the rectangle.
/// The triangle is narrow at the top (single point) and widens to its full
/// width at the bottom — the combined back forms a trapezoid that narrows
/// upward (slope ~20° from vertical instead of the prior ~42°).
pub(super) const PINKBACK_TRIANGLE_WIDTH: f32 = 240.0;

/// Vertical extent of the yellow back (top of orange bar).
pub(super) const PINKBACK_RECT_HEIGHT: f32 = 645.0;

pub(super) const PINKBACK_LOGICAL_WIDTH: f32 = 900.0;

pub(super) fn capsule_text_offset() -> glam::Vec2 {
    // ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:200
    glam::vec2(
        CAPSULE_FRAME_WIDTH * 0.26 * CAPSULE_REAL_SCALED,
        40.0 * CAPSULE_REAL_SCALED,
    )
}

/// Compute the one-shot capsule jump-in scale. OG runs the xFrames
/// sequence ONCE on `doJumpIn = true` at 24fps and then locks at
/// `[1.0, 1.0]` for the rest of the screen's life; it is NOT a
/// per-beat bop, so we anchor the playback at the enter cursor and
/// hold the resting scale after the seven frames elapse.
///
/// ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:603,663-691
pub(super) fn capsule_beat_scale(
    cursor: Samples,
    enter_started_at: Samples,
    sample_rate: u32,
) -> (f32, f32) {
    let elapsed = cursor.0.saturating_sub(enter_started_at.0).max(0) as f64;
    let frame =
        (elapsed * f64::from(CAPSULE_BEAT_FPS) / f64::from(sample_rate.max(1))).floor() as usize;
    if frame >= CAPSULE_BEAT_XFRAMES.len() {
        return (1.0, 1.0);
    }
    let sx = CAPSULE_BEAT_XFRAMES[frame];
    (sx, 1.0 / sx.max(0.0001))
}

pub(super) fn digit_frames(atlas: &SparrowAtlas) -> [Option<SparrowFrame>; 10] {
    // ref: bdedc0aa:assets/preload/images/freeplay/freeplayCapsule/bignumbers.xml
    const NAMES: [&str; 10] = [
        "ZERO0000",
        "ONE0000",
        "TWO0000",
        "THREE0000",
        "FOUR0000",
        "FIVE0000",
        "SIX0000",
        "SEVEN0000",
        "EIGHT0000",
        "NINE0000",
    ];
    let mut out: [Option<SparrowFrame>; 10] = Default::default();
    for (idx, name) in NAMES.iter().enumerate() {
        out[idx] = atlas.frames.iter().find(|f| f.name == *name).cloned();
    }
    out
}

pub(super) fn load_static_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    path: &str,
    filter: FilterMode,
) -> Result<StaticTexture> {
    let path = AssetPath::new(path)?;
    let image = load_png(resolver, &path).with_context(|| format!("load {path}"))?;
    let texture_id = asset_id_for_path(&path);
    let (width, height) = (image.width, image.height);
    textures.insert(
        texture_id,
        Texture::from_png_image(device, queue, &image, filter, Some(path.as_str())),
    );
    Ok(StaticTexture {
        texture_id,
        width,
        height,
        filter,
    })
}

pub(super) fn load_sparrow_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    xml_path: &str,
) -> Result<(SparrowAtlasHandle, SparrowAtlas)> {
    let xml_path = AssetPath::new(xml_path)?;
    let atlas = load_sparrow(resolver, &xml_path).with_context(|| format!("load {xml_path}"))?;
    let texture_path = xml_path.sibling(&atlas.image_path)?;
    let image =
        load_png(resolver, &texture_path).with_context(|| format!("load {texture_path}"))?;
    let texture_id = asset_id_for_path(&texture_path);
    let (width, height) = (image.width, image.height);
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
    Ok((
        SparrowAtlasHandle {
            texture_id,
            width,
            height,
        },
        atlas,
    ))
}

pub(super) fn clone_frames(atlas: &SparrowAtlas, prefix: &str) -> Vec<SparrowFrame> {
    atlas
        .animation_frames(prefix, &[])
        .into_iter()
        .cloned()
        .collect()
}

pub(super) fn frame_for_cursor(
    frames: &[SparrowFrame],
    cursor: Samples,
    sample_rate: u32,
    fps: u16,
    looped: bool,
) -> Option<&SparrowFrame> {
    if frames.is_empty() {
        return None;
    }
    let elapsed = cursor.0.max(0) as u128;
    let index = (elapsed * u128::from(fps) / u128::from(sample_rate.max(1))) as usize;
    let frame_count = frames.len();
    let index = if looped {
        index % frame_count
    } else {
        index.min(frame_count - 1)
    };
    frames.get(index)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sparrow_scaled_command(
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frame: &SparrowFrame,
    position: glam::Vec2,
    scale: glam::Vec2,
    color: glam::Vec4,
    z: i32,
) -> DrawCommand {
    let draw_pos = position - frame_trim_offset(frame) * scale;
    let mut cmd = DrawCommand::sprite(texture_id, draw_pos, frame_draw_size(frame) * scale);
    cmd.camera = CameraId(1);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Linear;
    cmd.color = color;
    (cmd.uv_min, cmd.uv_max) = frame_uv(frame, texture_width, texture_height);
    cmd.uv_rotated = frame.rotated;
    cmd
}

pub(super) fn solid_command(
    pos: glam::Vec2,
    size: glam::Vec2,
    color: glam::Vec4,
    z: i32,
) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(WHITE_TEXTURE_ID, pos, size);
    cmd.camera = CameraId(1);
    cmd.layer = RenderLayer::Background;
    cmd.z = z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = color;
    cmd
}

fn frame_draw_size(frame: &SparrowFrame) -> glam::Vec2 {
    if frame.rotated {
        glam::vec2(frame.height as f32, frame.width as f32)
    } else {
        glam::vec2(frame.width as f32, frame.height as f32)
    }
}

fn frame_trim_offset(frame: &SparrowFrame) -> glam::Vec2 {
    glam::vec2(frame.frame_x as f32, frame.frame_y as f32)
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

/// Logical paths to assets the Freeplay screen depends on.
pub const REQUIRED_FREEPLAY_ASSETS: &[&str] = &[
    "images/freeplay/pinkBack.png",
    "images/freeplay/freeplayBackTriangle.png",
    "images/freeplay/freeplayBGweek1-bf.png",
    "images/freeplay/freeplayCapsule/capsule/freeplayCapsule.png",
    "images/freeplay/freeplayCapsule/capsule/freeplayCapsule.xml",
    "images/freeplay/freeplayCapsule/bignumbers.png",
    "images/freeplay/freeplayCapsule/bignumbers.xml",
    "images/freeplay/freeplayCapsule/smallnumbers.png",
    "images/freeplay/freeplayCapsule/smallnumbers.xml",
    "images/freeplay/freeplayCapsule/bpmtext.png",
    "images/freeplay/freeplayCapsule/difficultytext.png",
    "images/freeplay/freeplaySelector/freeplaySelector.png",
    "images/freeplay/freeplaySelector/freeplaySelector.xml",
    "images/freeplay/freeplayeasy.png",
    "images/freeplay/freeplaynormal.png",
    "images/freeplay/freeplayhard.png",
    "images/freeplay/freeplayerect.png",
    "images/freeplay/freeplaynightmare.png",
    "images/freeplay/freeplaynightmare.xml",
    "images/freeplay/highscore.png",
    "images/freeplay/highscore.xml",
    "images/freeplay/sparkle.png",
    "images/freeplay/sparkle.xml",
    "images/freeplay/miniArrow.png",
    "images/freeplay/freeplayStars/Animation.json",
    "images/freeplay/freeplayStars/spritemap1.json",
    "images/freeplay/freeplayStars/spritemap1.png",
    "images/freeplay/albumRoll/volume1.png",
    "images/freeplay/albumRoll/volume1-text.png",
    "images/freeplay/albumRoll/volume1-text.xml",
    "data/ui/freeplay/albums/volume1.json",
    "images/freeplay/albumRoll/volume2.png",
    "images/freeplay/albumRoll/volume2-text.png",
    "images/freeplay/albumRoll/volume2-text.xml",
    "data/ui/freeplay/albums/volume2.json",
    "images/freeplay/albumRoll/volume3.png",
    "images/freeplay/albumRoll/volume3-text.png",
    "images/freeplay/albumRoll/volume3-text.xml",
    "data/ui/freeplay/albums/volume3.json",
    "images/freeplay/albumRoll/volume4.png",
    "images/freeplay/albumRoll/volume4-text.png",
    "images/freeplay/albumRoll/volume4-text.xml",
    "data/ui/freeplay/albums/volume4.json",
    "images/freeplay/albumRoll/expansion1.png",
    "images/freeplay/albumRoll/expansion1-text.png",
    "images/freeplay/albumRoll/expansion1-text.xml",
    "data/ui/freeplay/albums/expansion1.json",
    "images/freeplay/albumRoll/expansion2.png",
    "images/freeplay/albumRoll/expansion2-text.png",
    "images/freeplay/albumRoll/expansion2-text.xml",
    "data/ui/freeplay/albums/expansion2.json",
    "images/freeplay/albumRoll/spaghetti.png",
    "images/freeplay/albumRoll/spaghetti-text.png",
    "images/freeplay/albumRoll/spaghetti-text.xml",
    "data/ui/freeplay/albums/spaghetti.json",
    "images/freeplay/seperator.png",
    "images/freeplay/clearBox.png",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capsule_position_uses_sin_offset() {
        let center = capsule_position(0.0);
        assert!((center.x - CAPSULE_BASE_X).abs() < 0.01);
        assert!((center.y - CAPSULE_BASE_Y).abs() < 0.01);
    }

    #[test]
    fn capsule_position_above_selected_is_negative() {
        let above = capsule_position(-1.0);
        let below = capsule_position(1.0);
        assert!(above.y < CAPSULE_BASE_Y);
        assert!(below.y > CAPSULE_BASE_Y);
    }

    #[test]
    fn frame_for_cursor_handles_empty() {
        assert!(frame_for_cursor(&[], Samples(0), 48_000, 24, true).is_none());
    }

    #[test]
    fn static_background_command_sorts_under_character_layer() {
        let texture = StaticTexture {
            texture_id: AssetId::new(1),
            width: 8,
            height: 8,
            filter: FilterMode::Linear,
        };

        let cmd = texture.background_command(
            glam::Vec2::ZERO,
            glam::Vec4::ONE,
            -90,
            glam::Vec2::splat(8.0),
        );

        assert_eq!(cmd.layer, RenderLayer::Background);
    }

    #[test]
    fn sparrow_scaled_command_applies_trim_offset() {
        let atlas = SparrowAtlas::parse(
            br#"
            <TextureAtlas imagePath="test.png">
              <SubTexture name="trimmed0000" x="0" y="0" width="80" height="20"
                frameX="-10" frameY="5" frameWidth="100" frameHeight="40"/>
            </TextureAtlas>
            "#,
        )
        .unwrap();
        let frame = atlas.frames.first().unwrap();

        let cmd = sparrow_scaled_command(
            AssetId::new(2),
            128,
            128,
            frame,
            glam::vec2(50.0, 60.0),
            glam::vec2(2.0, 3.0),
            glam::Vec4::ONE,
            10,
        );

        assert_eq!(cmd.world_pos, glam::vec2(70.0, 45.0));
        assert_eq!(cmd.size, glam::vec2(160.0, 60.0));
    }

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
        for logical in REQUIRED_FREEPLAY_ASSETS {
            let path = source_root.join(logical);
            if !path.exists() {
                missing.push(path.display().to_string());
            }
        }
        assert!(
            missing.is_empty(),
            "freeplay assets missing - required for the OG-fidelity port:\n{}",
            missing.join("\n"),
        );
    }
}
