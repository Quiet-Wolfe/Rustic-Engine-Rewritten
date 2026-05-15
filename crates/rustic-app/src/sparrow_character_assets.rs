//! Sparrow and Packer character asset loading for gameplay scenes.
// LINT-ALLOW: long-file character atlas loading and frame expansion stay together.

use crate::animation_timing::flixel_frame_index;
use crate::character_anim::CharacterPoseRequest;
use anyhow::{Context, Result};
use rustic_asset::{
    load_bytes, load_png, load_sparrow, AssetPath, CharacterAnimation, CharacterDefinition,
    OverlayResolver, SparrowAtlas, SparrowFrame, StageCharacterSlot,
};
use rustic_core::ids::AssetId;
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct SparrowCharacterSprite {
    character: CharacterDefinition,
    slot: StageCharacterSlot,
    is_player: bool,
    z: i32,
    filter: FilterMode,
    assets: Vec<LoadedSparrowAtlas>,
    poses: Vec<CharacterPose>,
    initial_pose: usize,
}

impl SparrowCharacterSprite {
    pub(crate) fn command(
        &self,
        request: CharacterPoseRequest,
        cursor: Samples,
        sample_rate: u32,
    ) -> DrawCommand {
        let pose = self.pose(request.name);
        let asset = &self.assets[pose.asset_index];
        let frame = pose.frame(cursor, sample_rate, request.started_at);
        let mut cmd = DrawCommand::sprite(
            asset.texture_id,
            character_frame_pos(&self.character, &pose.animation, frame, self.slot),
            glam::vec2(
                frame.width as f32 * self.character.scale,
                frame.height as f32 * self.character.scale,
            ),
        );
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Characters;
        cmd.z = self.z;
        cmd.filter = self.filter;
        (cmd.uv_min, cmd.uv_max) = frame_uv(frame, asset.width, asset.height);
        cmd.uv_rotated = frame.rotated;
        if effective_flip_x(&self.character, self.is_player) {
            std::mem::swap(&mut cmd.uv_min.x, &mut cmd.uv_max.x);
        }
        cmd
    }

    pub(crate) fn animation_duration(
        &self,
        animation_name: &str,
        sample_rate: u32,
    ) -> Option<Samples> {
        let pose = self
            .poses
            .iter()
            .find(|pose| pose.animation.name == animation_name)?;
        Some(animation_duration_samples(
            sample_rate,
            pose.animation.fps,
            pose.frames.len(),
        ))
    }

    pub(crate) fn has_pose(&self, animation_name: &str) -> bool {
        self.poses
            .iter()
            .any(|pose| pose.animation.name == animation_name)
    }

    pub(crate) fn camera_focus_point(&self) -> glam::Vec2 {
        let frame = &self.poses[self.initial_pose].frames[0];
        glam::vec2(
            self.slot.position.x + self.character.position.x,
            self.slot.position.y + self.character.position.y
                - frame.frame_height as f32 * self.character.scale * 0.5,
        ) + glam::vec2(self.slot.camera_offset.x, self.slot.camera_offset.y)
            + glam::vec2(
                self.character.camera_offset.x,
                self.character.camera_offset.y,
            )
    }

    pub(crate) fn definition(&self) -> &CharacterDefinition {
        &self.character
    }

    pub(crate) fn slot(&self) -> StageCharacterSlot {
        self.slot
    }

    fn pose(&self, animation_name: &str) -> &CharacterPose {
        self.poses
            .iter()
            .find(|pose| pose.animation.name == animation_name)
            .unwrap_or(&self.poses[self.initial_pose])
    }
}

#[derive(Debug, Clone)]
struct LoadedSparrowAtlas {
    texture_id: AssetId,
    width: u32,
    height: u32,
    atlas: SparrowAtlas,
}

#[derive(Debug, Clone)]
struct CharacterPose {
    animation: CharacterAnimation,
    asset_index: usize,
    frames: Vec<SparrowFrame>,
}

impl CharacterPose {
    fn frame(&self, cursor: Samples, sample_rate: u32, started_at: Samples) -> &SparrowFrame {
        let index = flixel_frame_index(
            cursor,
            sample_rate,
            started_at,
            self.animation.fps,
            self.frames.len(),
            self.animation.looped,
        );
        &self.frames[index]
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn load_sparrow_character_sprite(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    character: CharacterDefinition,
    slot: StageCharacterSlot,
    is_player: bool,
    z: i32,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<SparrowCharacterSprite> {
    let filter = filter_for_antialiasing(character.antialiasing);
    let fallback_atlas = character_default_sparrow_atlas(&character)?;
    let mut assets = Vec::new();
    let mut asset_indices = HashMap::new();
    for animation in &character.animations {
        let atlas_path = sparrow_animation_atlas_path(animation, &fallback_atlas)?;
        let key = atlas_path.as_str().to_owned();
        if asset_indices.contains_key(&key) {
            continue;
        }
        let loaded =
            load_sparrow_character_atlas(device, queue, resolver, &atlas_path, filter, textures)?;
        asset_indices.insert(key, assets.len());
        assets.push(loaded);
    }
    let poses = character_poses(&character, &assets, &asset_indices, &fallback_atlas)?;
    let initial_animation = initial_animation(&character)?;
    let initial_pose = poses
        .iter()
        .position(|pose| pose.animation.name == initial_animation.name)
        .with_context(|| format!("resolve initial pose {}", character.id))?;

    Ok(SparrowCharacterSprite {
        character,
        slot,
        is_player,
        z,
        filter,
        assets,
        poses,
        initial_pose,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn load_packer_character_sprite(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    character: CharacterDefinition,
    slot: StageCharacterSlot,
    is_player: bool,
    z: i32,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<SparrowCharacterSprite> {
    let image_path = character
        .asset_path
        .as_ref()
        .map(packer_image_path)
        .transpose()?
        .with_context(|| format!("character {} uses packer without assetPath", character.id))?;
    let packer_path = image_path
        .as_str()
        .strip_suffix(".png")
        .map(|base| AssetPath::new(format!("{base}.txt")))
        .transpose()?
        .context("resolve packer atlas path")?;
    let frames = load_packer_frames(resolver, &packer_path)?;
    let atlas = SparrowAtlas::from_frames(image_path.as_str().to_string(), frames);
    let image =
        load_png(resolver, &image_path).with_context(|| format!("load {}", image_path.as_str()))?;
    let texture_id = asset_id_for_path(&image_path);
    let filter = filter_for_antialiasing(character.antialiasing);
    let texture = Texture::from_png_image(device, queue, &image, filter, Some(image_path.as_str()));
    textures.insert(texture_id, texture);
    let loaded = LoadedSparrowAtlas {
        texture_id,
        width: image.width,
        height: image.height,
        atlas,
    };
    let poses = packer_character_poses(&character, &loaded.atlas)?;
    let initial_animation = initial_animation(&character)?;
    let initial_pose = poses
        .iter()
        .position(|pose| pose.animation.name == initial_animation.name)
        .with_context(|| format!("resolve initial pose {}", character.id))?;
    Ok(SparrowCharacterSprite {
        character,
        slot,
        is_player,
        z,
        filter,
        assets: vec![loaded],
        poses,
        initial_pose,
    })
}

fn character_poses(
    character: &CharacterDefinition,
    assets: &[LoadedSparrowAtlas],
    asset_indices: &HashMap<String, usize>,
    fallback_atlas: &AssetPath,
) -> Result<Vec<CharacterPose>> {
    character
        .animations
        .iter()
        .map(|animation| {
            let atlas_path = sparrow_animation_atlas_path(animation, fallback_atlas)?;
            let asset_index = *asset_indices
                .get(atlas_path.as_str())
                .with_context(|| format!("resolve sparrow asset {}", atlas_path.as_str()))?;
            let frames: Vec<_> = assets[asset_index]
                .atlas
                .animation_frames(&animation.prefix, &animation.indices)
                .into_iter()
                .cloned()
                .collect();
            if frames.is_empty() {
                anyhow::bail!("resolve frame {}:{}", character.id, animation.name);
            }
            Ok(CharacterPose {
                animation: animation.clone(),
                asset_index,
                frames,
            })
        })
        .collect()
}

fn packer_character_poses(
    character: &CharacterDefinition,
    atlas: &SparrowAtlas,
) -> Result<Vec<CharacterPose>> {
    character
        .animations
        .iter()
        .map(|animation| {
            let frames: Vec<_> = atlas
                .animation_frames(&animation.prefix, &animation.indices)
                .into_iter()
                .cloned()
                .collect();
            if frames.is_empty() {
                anyhow::bail!("resolve frame {}:{}", character.id, animation.name);
            }
            Ok(CharacterPose {
                animation: animation.clone(),
                asset_index: 0,
                frames,
            })
        })
        .collect()
}

fn animation_duration_samples(sample_rate: u32, fps: u16, frame_count: usize) -> Samples {
    let samples_per_frame = f64::from(sample_rate.max(1)) / f64::from(fps.max(1));
    Samples((samples_per_frame * frame_count.max(1) as f64).ceil() as i64)
}

fn initial_animation(character: &CharacterDefinition) -> Result<&CharacterAnimation> {
    match character.initial_animation.as_deref() {
        Some(name) => character
            .animations
            .iter()
            .find(|animation| animation.name == name)
            .with_context(|| format!("character {} missing {name}", character.id)),
        None => character
            .animations
            .first()
            .with_context(|| format!("character {} has no animations", character.id)),
    }
}

fn atlas_texture_path(atlas_path: &AssetPath, atlas: &SparrowAtlas) -> Result<AssetPath> {
    if atlas.image_path.contains('/') {
        Ok(AssetPath::new(atlas.image_path.clone())?)
    } else {
        Ok(atlas_path.sibling(&atlas.image_path)?)
    }
}

fn character_default_sparrow_atlas(character: &CharacterDefinition) -> Result<AssetPath> {
    character
        .atlas
        .clone()
        .or_else(|| {
            character
                .asset_path
                .as_ref()
                .and_then(|path| sparrow_asset_path(path).ok())
        })
        .with_context(|| {
            format!(
                "character {} uses {:?}; Sparrow renderer needs atlas or assetPath",
                character.id, character.render_type
            )
        })
}

fn sparrow_animation_atlas_path(
    animation: &CharacterAnimation,
    fallback_atlas: &AssetPath,
) -> Result<AssetPath> {
    animation
        .asset_path
        .as_ref()
        .map(sparrow_asset_path)
        .transpose()?
        .or_else(|| Some(fallback_atlas.clone()))
        .context("resolve sparrow animation atlas")
}

fn sparrow_asset_path(asset_path: &AssetPath) -> Result<AssetPath> {
    let raw = asset_path.as_str();
    let stripped = raw.split_once(':').map(|(_, path)| path).unwrap_or(raw);
    Ok(AssetPath::new(format!("images/{stripped}.xml"))?)
}

fn packer_image_path(asset_path: &AssetPath) -> Result<AssetPath> {
    let raw = asset_path.as_str();
    let stripped = raw.split_once(':').map(|(_, path)| path).unwrap_or(raw);
    Ok(AssetPath::new(format!("images/{stripped}.png"))?)
}

fn load_packer_frames(
    resolver: &OverlayResolver,
    packer_path: &AssetPath,
) -> Result<Vec<SparrowFrame>> {
    let bytes = load_bytes(resolver, packer_path)
        .with_context(|| format!("load {}", packer_path.as_str()))?;
    let text = std::str::from_utf8(&bytes).context("packer atlas is not utf-8")?;
    let mut frames = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (name, coords) = line
            .split_once('=')
            .with_context(|| format!("packer line {} missing '='", line_index + 1))?;
        let values = coords
            .split_whitespace()
            .map(str::parse::<i32>)
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("packer line {} has invalid number", line_index + 1))?;
        if values.len() != 4 {
            anyhow::bail!("packer line {} needs 4 numbers", line_index + 1);
        }
        let width = u32::try_from(values[2]).context("packer width is negative")?;
        let height = u32::try_from(values[3]).context("packer height is negative")?;
        frames.push(SparrowFrame::untrimmed(
            name.trim().to_string(),
            values[0],
            values[1],
            width,
            height,
        ));
    }
    Ok(frames)
}

fn load_sparrow_character_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    atlas_path: &AssetPath,
    filter: FilterMode,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<LoadedSparrowAtlas> {
    let atlas = load_sparrow(resolver, atlas_path)
        .with_context(|| format!("load {}", atlas_path.as_str()))?;
    let texture_path = atlas_texture_path(atlas_path, &atlas)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(texture_path.as_str()));
    textures.insert(texture_id, texture);
    Ok(LoadedSparrowAtlas {
        texture_id,
        width: image.width,
        height: image.height,
        atlas,
    })
}

fn character_frame_pos(
    character: &CharacterDefinition,
    animation: &CharacterAnimation,
    frame: &SparrowFrame,
    slot: StageCharacterSlot,
) -> glam::Vec2 {
    let origin = sparrow_character_origin(frame, character.scale);
    glam::vec2(
        slot.position.x + character.position.x
            - origin.x
            - animation.offset.x
            - frame.frame_x as f32 * character.scale,
        slot.position.y + character.position.y
            - origin.y
            - animation.offset.y
            - frame.frame_y as f32 * character.scale,
    )
}

fn sparrow_character_origin(frame: &SparrowFrame, scale: f32) -> glam::Vec2 {
    glam::vec2(
        frame.frame_width as f32 * scale * 0.5,
        frame.frame_height as f32 * scale,
    )
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

fn effective_flip_x(character: &CharacterDefinition, is_player: bool) -> bool {
    if is_player {
        !character.flip_x
    } else {
        character.flip_x
    }
}

fn filter_for_antialiasing(antialiasing: bool) -> FilterMode {
    if antialiasing {
        FilterMode::Linear
    } else {
        FilterMode::Nearest
    }
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
#[path = "sparrow_character_assets_tests.rs"]
mod tests;
