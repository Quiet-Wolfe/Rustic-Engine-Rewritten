//! Freeplay-menu asset wiring from Funkin' v0.8.5.
//!
//! Phase 1 covers the backing card, capsule list with selected/unselected
//! animations, difficulty selector arrows + label, and the FREEPLAY/OST text.
//! The DJ atlas, album roll, score, letter sort, rank animations, and sticker
//! transition come in later phases.
//!
//! ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:331-694
//! ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:91-752
//! ref: bdedc0aa:source/funkin/ui/freeplay/backcards/BackingCard.hx:48-242
// LINT-ALLOW: long-file freeplay asset loading and layout stay co-located for fidelity.

use crate::asset_roots::baked_assets_root;
use crate::preview_song::{PreviewDifficulty, PreviewSelection, PreviewSong};
use anyhow::{Context, Result};
use rustic_asset::{load_png, load_sparrow, AssetPath, OverlayResolver, SparrowAtlas, SparrowFrame};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{
    DrawCommand, FilterMode, RenderCommandList, TextCommand, TextCommandList, Texture,
};
use std::collections::HashMap;

// ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:607
const CAPSULE_REAL_SCALED: f32 = 0.8;
// ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:745-753
const CAPSULE_BASE_X: f32 = 270.0;
const CAPSULE_BASE_Y: f32 = 120.0;
const CAPSULE_SIN_AMPLITUDE: f32 = 60.0;
const CAPSULE_SPACING_PAD: f32 = 10.0;
const CAPSULE_FRAME_HEIGHT: f32 = 132.0;
const CAPSULE_FRAME_WIDTH: f32 = 612.0;
const CAPSULE_ANIM_FPS: u16 = 24;
// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:354-356
const SELECTOR_LEFT_X: f32 = 20.0;
const SELECTOR_RIGHT_X: f32 = 325.0;
const SELECTOR_Y: f32 = 70.0;
const SELECTOR_ANIM_FPS: u16 = 24;
// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:339
const DIFFICULTY_GROUP_X: f32 = 90.0;
const DIFFICULTY_GROUP_Y: f32 = 80.0;
const ORANGE_BAR_X: f32 = 84.0;
const ORANGE_BAR_Y: f32 = 440.0;
const ORANGE_BAR_HEIGHT: f32 = 75.0;
const FREEPLAY_TITLE_X: f32 = 8.0;
const FREEPLAY_TITLE_Y: f32 = 8.0;
const PINKBACK_TARGET_HEIGHT: f32 = 720.0;

// ref: bdedc0aa:source/funkin/ui/freeplay/backcards/BackingCard.hx:62,57-58,129
const PINKBACK_COLOR: glam::Vec4 = glam::Vec4::new(
    0xFF as f32 / 255.0,
    0xD8 as f32 / 255.0,
    0x63 as f32 / 255.0,
    1.0,
);
const ORANGE_BAR_COLOR: glam::Vec4 = glam::Vec4::new(
    0xFE as f32 / 255.0,
    0xDA as f32 / 255.0,
    0x00 as f32 / 255.0,
    1.0,
);
// ref: bdedc0aa:assets/preload/data/ui/freeplay/styles/bf.json:7 (capsuleTextColors)
const CAPSULE_TEXT_COLOR: glam::Vec4 =
    glam::Vec4::new(0x00 as f32 / 255.0, 0xCC as f32 / 255.0, 1.0, 1.0);

const WHITE_TEXTURE_ID: AssetId = AssetId::new(0x4672_6565_706c_6179);

#[derive(Debug)]
pub struct FreeplayAssets {
    songs: Vec<FreeplayCapsule>,
    pink_back: StaticTexture,
    bg_image: StaticTexture,
    capsule_atlas: SparrowAtlasHandle,
    capsule_selected_frames: Vec<SparrowFrame>,
    capsule_unselected_frames: Vec<SparrowFrame>,
    selector_atlas: SparrowAtlasHandle,
    selector_frames: Vec<SparrowFrame>,
    difficulty_easy: StaticTexture,
    difficulty_normal: StaticTexture,
    difficulty_hard: StaticTexture,
    difficulty_erect: StaticTexture,
    difficulty_nightmare: SparrowAtlasHandle,
    difficulty_nightmare_frames: Vec<SparrowFrame>,
    pub textures: HashMap<AssetId, Texture>,
}

impl FreeplayAssets {
    pub fn commands(
        &self,
        selection: PreviewSelection,
        cursor: Samples,
        sample_rate: u32,
    ) -> RenderCommandList {
        let mut commands = RenderCommandList::new();
        commands.push(solid_command(
            glam::vec2(0.0, 0.0),
            glam::vec2(1280.0, 720.0),
            glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
            -100,
        ));

        let pink_back_size = self.pink_back_draw_size();
        commands.push(self.pink_back.command(
            glam::vec2(0.0, 0.0),
            PINKBACK_COLOR,
            -90,
            pink_back_size,
        ));
        commands.push(solid_command(
            glam::vec2(ORANGE_BAR_X, ORANGE_BAR_Y),
            glam::vec2(pink_back_size.x, ORANGE_BAR_HEIGHT),
            ORANGE_BAR_COLOR,
            -85,
        ));

        let bg_scale = bg_image_scale(&self.bg_image);
        let bg_pos = glam::vec2(pink_back_size.x * 0.74, 0.0);
        commands.push(self.bg_image.command(
            bg_pos,
            glam::Vec4::ONE,
            -80,
            glam::vec2(
                self.bg_image.width as f32 * bg_scale,
                self.bg_image.height as f32 * bg_scale,
            ),
        ));

        let selected_index = self.index_of(selection.song).unwrap_or(0);
        self.push_capsules(&mut commands, selected_index, cursor, sample_rate);
        self.push_difficulty(&mut commands, selection.difficulty, cursor, sample_rate);
        commands
    }

    pub fn text_commands(&self, selection: PreviewSelection) -> TextCommandList {
        let mut commands = TextCommandList::new();
        let selected_index = self.index_of(selection.song).unwrap_or(0);

        let mut title = TextCommand::new(
            "FREEPLAY",
            glam::vec2(FREEPLAY_TITLE_X, FREEPLAY_TITLE_Y),
            48.0,
        );
        title.color = glam::Vec4::new(1.0, 0.84, 0.26, 1.0);
        title.z = 300;
        commands.push(title);

        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:349 (ostName)
        let mut ost = TextCommand::new("MAIN OST V1", glam::vec2(950.0, 14.0), 36.0);
        ost.color = glam::Vec4::new(1.0, 1.0, 1.0, 0.9);
        ost.z = 300;
        commands.push(ost);

        for (index, capsule) in self.songs.iter().enumerate() {
            let offset = index as f32 - selected_index as f32;
            let pos = capsule_position(offset);
            let is_selected = index == selected_index;
            let mut text = TextCommand::new(
                capsule.display_name.clone(),
                pos + capsule_text_offset(),
                36.0 * CAPSULE_REAL_SCALED,
            );
            let mut color = CAPSULE_TEXT_COLOR;
            color.w = if is_selected { 1.0 } else { 0.6 };
            text.color = color;
            text.z = 320 + index as i32;
            commands.push(text);
        }

        commands
    }

    pub fn item_count(&self) -> usize {
        self.songs.len()
    }

    pub fn song_at(&self, index: usize) -> Option<PreviewSong> {
        self.songs.get(index).map(|capsule| capsule.song)
    }

    pub fn index_of(&self, song: PreviewSong) -> Option<usize> {
        self.songs
            .iter()
            .position(|capsule| capsule.song.id == song.id)
    }

    fn pink_back_draw_size(&self) -> glam::Vec2 {
        let aspect = self.pink_back.width.max(1) as f32 / self.pink_back.height.max(1) as f32;
        let height = PINKBACK_TARGET_HEIGHT;
        let width = height * aspect;
        glam::vec2(width, height)
    }

    fn push_capsules(
        &self,
        commands: &mut RenderCommandList,
        selected_index: usize,
        cursor: Samples,
        sample_rate: u32,
    ) {
        for index in 0..self.songs.len() {
            let offset = index as f32 - selected_index as f32;
            let pos = capsule_position(offset);
            let is_selected = index == selected_index;
            let frames = if is_selected {
                &self.capsule_selected_frames
            } else {
                &self.capsule_unselected_frames
            };
            let Some(frame) =
                frame_for_cursor(frames, cursor, sample_rate, CAPSULE_ANIM_FPS, true)
            else {
                continue;
            };
            commands.push(sparrow_scaled_command(
                self.capsule_atlas.texture_id,
                self.capsule_atlas.width,
                self.capsule_atlas.height,
                frame,
                pos,
                glam::Vec2::splat(CAPSULE_REAL_SCALED),
                glam::Vec4::ONE,
                200 + index as i32,
            ));
        }
    }

    fn push_difficulty(
        &self,
        commands: &mut RenderCommandList,
        difficulty: PreviewDifficulty,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if let Some(frame) = frame_for_cursor(
            &self.selector_frames,
            cursor,
            sample_rate,
            SELECTOR_ANIM_FPS,
            true,
        ) {
            commands.push(sparrow_scaled_command(
                self.selector_atlas.texture_id,
                self.selector_atlas.width,
                self.selector_atlas.height,
                frame,
                glam::vec2(SELECTOR_LEFT_X, SELECTOR_Y),
                glam::Vec2::ONE,
                glam::Vec4::ONE,
                280,
            ));
            commands.push(sparrow_scaled_command(
                self.selector_atlas.texture_id,
                self.selector_atlas.width,
                self.selector_atlas.height,
                frame,
                glam::vec2(SELECTOR_RIGHT_X + frame.frame_width as f32, SELECTOR_Y),
                glam::vec2(-1.0, 1.0),
                glam::Vec4::ONE,
                280,
            ));
        }
        match difficulty {
            PreviewDifficulty::Easy => self.push_static_difficulty(commands, &self.difficulty_easy),
            PreviewDifficulty::Normal => {
                self.push_static_difficulty(commands, &self.difficulty_normal)
            }
            PreviewDifficulty::Hard => self.push_static_difficulty(commands, &self.difficulty_hard),
            PreviewDifficulty::Erect => {
                self.push_static_difficulty(commands, &self.difficulty_erect)
            }
            PreviewDifficulty::Nightmare => {
                if let Some(frame) = frame_for_cursor(
                    &self.difficulty_nightmare_frames,
                    cursor,
                    sample_rate,
                    CAPSULE_ANIM_FPS,
                    true,
                ) {
                    commands.push(sparrow_scaled_command(
                        self.difficulty_nightmare.texture_id,
                        self.difficulty_nightmare.width,
                        self.difficulty_nightmare.height,
                        frame,
                        glam::vec2(DIFFICULTY_GROUP_X, DIFFICULTY_GROUP_Y),
                        glam::Vec2::ONE,
                        glam::Vec4::ONE,
                        290,
                    ));
                }
            }
        }
    }

    fn push_static_difficulty(&self, commands: &mut RenderCommandList, texture: &StaticTexture) {
        commands.push(texture.command(
            glam::vec2(DIFFICULTY_GROUP_X, DIFFICULTY_GROUP_Y),
            glam::Vec4::ONE,
            290,
            glam::vec2(texture.width as f32, texture.height as f32),
        ));
    }
}

#[derive(Debug)]
struct FreeplayCapsule {
    song: PreviewSong,
    display_name: String,
}

#[derive(Debug)]
struct StaticTexture {
    texture_id: AssetId,
    width: u32,
    height: u32,
    filter: FilterMode,
}

impl StaticTexture {
    fn command(
        &self,
        pos: glam::Vec2,
        color: glam::Vec4,
        z: i32,
        draw_size: glam::Vec2,
    ) -> DrawCommand {
        let mut cmd = DrawCommand::sprite(self.texture_id, pos, draw_size);
        cmd.camera = CameraId(1);
        cmd.layer = RenderLayer::Overlay;
        cmd.z = z;
        cmd.pivot = glam::Vec2::ZERO;
        cmd.filter = self.filter;
        cmd.color = color;
        cmd
    }
}

#[derive(Debug)]
struct SparrowAtlasHandle {
    texture_id: AssetId,
    width: u32,
    height: u32,
}

fn capsule_position(offset: f32) -> glam::Vec2 {
    let capsule_height_scaled = CAPSULE_FRAME_HEIGHT * CAPSULE_REAL_SCALED;
    let y = offset * (capsule_height_scaled + CAPSULE_SPACING_PAD) + CAPSULE_BASE_Y;
    let x = CAPSULE_BASE_X + CAPSULE_SIN_AMPLITUDE * offset.sin();
    glam::vec2(x, y)
}

fn capsule_text_offset() -> glam::Vec2 {
    // ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:200
    glam::vec2(
        CAPSULE_FRAME_WIDTH * 0.26 * CAPSULE_REAL_SCALED,
        40.0 * CAPSULE_REAL_SCALED,
    )
}

fn bg_image_scale(bg: &StaticTexture) -> f32 {
    PINKBACK_TARGET_HEIGHT / bg.height.max(1) as f32
}

pub fn load_freeplay_assets(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<FreeplayAssets> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let mut textures = HashMap::new();
    textures.insert(
        WHITE_TEXTURE_ID,
        Texture::from_rgba8(
            device,
            queue,
            &[255, 255, 255, 255],
            1,
            1,
            FilterMode::Nearest,
            Some("rustic.freeplay.white"),
        ),
    );
    let pink_back = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/pinkBack.png",
        FilterMode::Linear,
    )?;
    let bg_image = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayBGweek1-bf.png",
        FilterMode::Linear,
    )?;
    let (capsule_atlas, capsule_atlas_data) = load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayCapsule/capsule/freeplayCapsule.xml",
    )?;
    let capsule_selected_frames = clone_frames(&capsule_atlas_data, "mp3 capsule w backing0");
    let capsule_unselected_frames =
        clone_frames(&capsule_atlas_data, "mp3 capsule w backing NOT SELECTED");
    let (selector_atlas, selector_atlas_data) = load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplaySelector/freeplaySelector.xml",
    )?;
    let selector_frames = clone_frames(&selector_atlas_data, "arrow pointer loop");
    let difficulty_easy = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayeasy.png",
        FilterMode::Linear,
    )?;
    let difficulty_normal = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplaynormal.png",
        FilterMode::Linear,
    )?;
    let difficulty_hard = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayhard.png",
        FilterMode::Linear,
    )?;
    let difficulty_erect = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayerect.png",
        FilterMode::Linear,
    )
    .unwrap_or(StaticTexture {
        texture_id: difficulty_hard.texture_id,
        width: difficulty_hard.width,
        height: difficulty_hard.height,
        filter: difficulty_hard.filter,
    });
    let (difficulty_nightmare, nightmare_atlas) = load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplaynightmare.xml",
    )?;
    let difficulty_nightmare_frames = clone_frames(&nightmare_atlas, "idle");

    let songs = PreviewSong::CYCLABLE_WEEK1
        .iter()
        .map(|song| FreeplayCapsule {
            song: *song,
            display_name: song.display_name().to_ascii_uppercase(),
        })
        .collect();

    Ok(FreeplayAssets {
        songs,
        pink_back,
        bg_image,
        capsule_atlas,
        capsule_selected_frames,
        capsule_unselected_frames,
        selector_atlas,
        selector_frames,
        difficulty_easy,
        difficulty_normal,
        difficulty_hard,
        difficulty_erect,
        difficulty_nightmare,
        difficulty_nightmare_frames,
        textures,
    })
}

fn load_static_texture(
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

fn load_sparrow_atlas(
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

fn clone_frames(atlas: &SparrowAtlas, prefix: &str) -> Vec<SparrowFrame> {
    atlas
        .animation_frames(prefix, &[])
        .into_iter()
        .cloned()
        .collect()
}

fn frame_for_cursor<'a>(
    frames: &'a [SparrowFrame],
    cursor: Samples,
    sample_rate: u32,
    fps: u16,
    looped: bool,
) -> Option<&'a SparrowFrame> {
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

fn sparrow_scaled_command(
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frame: &SparrowFrame,
    position: glam::Vec2,
    scale: glam::Vec2,
    color: glam::Vec4,
    z: i32,
) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(texture_id, position, frame_draw_size(frame) * scale);
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

fn solid_command(pos: glam::Vec2, size: glam::Vec2, color: glam::Vec4, z: i32) -> DrawCommand {
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

/// Logical paths to assets the Freeplay screen depends on. Loaders above
/// reference these by string; this list is the authoritative inventory so
/// the `required_assets_present` test catches accidental deletes.
///
/// Do not remove entries here without removing the matching call site —
/// the Freeplay screen will not render correctly without all of them.
pub const REQUIRED_FREEPLAY_ASSETS: &[&str] = &[
    "images/freeplay/pinkBack.png",
    "images/freeplay/freeplayBGweek1-bf.png",
    "images/freeplay/freeplayCapsule/capsule/freeplayCapsule.png",
    "images/freeplay/freeplayCapsule/capsule/freeplayCapsule.xml",
    "images/freeplay/freeplaySelector/freeplaySelector.png",
    "images/freeplay/freeplaySelector/freeplaySelector.xml",
    "images/freeplay/freeplayeasy.png",
    "images/freeplay/freeplaynormal.png",
    "images/freeplay/freeplayhard.png",
    "images/freeplay/freeplayerect.png",
    "images/freeplay/freeplaynightmare.png",
    "images/freeplay/freeplaynightmare.xml",
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

    /// Locks the freeplay source asset inventory: if any of these files
    /// disappear from `assets/source/`, this test fails loudly. The
    /// Freeplay screen requires every entry — do not delete them.
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
            "freeplay assets missing — DO NOT DELETE these files, they are required for the OG-fidelity port:\n{}",
            missing.join("\n"),
        );
    }
}
