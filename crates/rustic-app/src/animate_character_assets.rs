//! Adobe Animate character asset loading and draw command expansion.
// LINT-ALLOW: long-file Animate atlas loading plus runtime draw expansion.

use crate::animation_timing::flixel_frame_index;
use crate::character_anim::CharacterPoseRequest;
use crate::sparrow_character_assets::{load_sparrow_character_sprite, SparrowCharacterSprite};
use anyhow::{Context, Result};
use rustic_asset::{
    load_animate_animation, load_animate_spritemap, load_png, AnimateAnimation, AnimateAtlas,
    AnimateDrawPart, AssetPath, CharacterAnimation, CharacterDefinition, CharacterRenderType,
    OverlayResolver, StageCharacterSlot,
};
use rustic_core::ids::AssetId;
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Clone)]
pub(crate) struct AnimateCharacterSprite {
    character: CharacterDefinition,
    slot: StageCharacterSlot,
    origin: glam::Vec2,
    visual_height: f32,
    is_player: bool,
    z: i32,
    filter: FilterMode,
    assets: Vec<LoadedAnimateAtlas>,
    poses: Vec<AnimateCharacterPose>,
    mixed_sparrow: Option<SparrowCharacterSprite>,
    lip_sync: Option<SserafimLipSyncOverlay>,
    initial_pose: usize,
}

impl AnimateCharacterSprite {
    pub(crate) fn commands(
        &self,
        request: CharacterPoseRequest,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        if let Some(sprite) = &self.mixed_sparrow {
            if sprite.has_pose(request.name) {
                return vec![sprite.command(request, cursor, sample_rate)];
            }
        }
        let (pose, started_at) = self.resolve_pose_for_request(request, cursor, sample_rate);
        let asset = &self.assets[pose.asset_index];
        let parts = pose
            .parts(asset, cursor, sample_rate, started_at)
            .unwrap_or_default();
        let mut commands: Vec<_> = parts
            .iter()
            .filter_map(|part| self.command_for_part(pose, asset, part))
            .collect();
        if let Some(lip_sync) = &self.lip_sync {
            commands.extend(lip_sync.commands(
                self,
                pose,
                &parts,
                request.name,
                cursor,
                sample_rate,
            ));
        }
        commands
    }

    pub(crate) fn animation_duration(
        &self,
        animation_name: &str,
        sample_rate: u32,
    ) -> Option<Samples> {
        if let Some(pose) = self
            .poses
            .iter()
            .find(|pose| pose.animation.name == animation_name)
        {
            return Some(animation_duration_samples(
                sample_rate,
                pose.animation.fps,
                pose.frame_count,
            ));
        }
        self.mixed_sparrow
            .as_ref()
            .and_then(|sprite| sprite.animation_duration(animation_name, sample_rate))
    }

    pub(crate) fn camera_focus_point(&self) -> glam::Vec2 {
        glam::vec2(
            self.slot.position.x + self.character.position.x,
            self.slot.position.y + self.character.position.y - self.visual_height * 0.5,
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

    fn pose(&self, animation_name: &str) -> &AnimateCharacterPose {
        self.poses
            .iter()
            .find(|pose| pose.animation.name == animation_name)
            .unwrap_or(&self.poses[self.initial_pose])
    }

    fn resolve_pose_for_request(
        &self,
        request: CharacterPoseRequest,
        cursor: Samples,
        sample_rate: u32,
    ) -> (&AnimateCharacterPose, Samples) {
        let pose = self.pose(request.name);
        let duration =
            animation_duration_samples(sample_rate, pose.animation.fps, pose.frame_count);
        if !pose.animation.looped && cursor.0.saturating_sub(request.started_at.0) >= duration.0 {
            if let Some(hold_pose) = self.hold_pose_for(pose) {
                return (
                    hold_pose,
                    Samples(request.started_at.0.saturating_add(duration.0)),
                );
            }
        }
        (pose, request.started_at)
    }

    fn hold_pose_for(&self, pose: &AnimateCharacterPose) -> Option<&AnimateCharacterPose> {
        self.poses.iter().find(|candidate| {
            candidate.animation.name.strip_prefix(&pose.animation.name) == Some("-hold")
        })
    }

    fn command_for_part(
        &self,
        pose: &AnimateCharacterPose,
        asset: &LoadedAnimateAtlas,
        part: &AnimateDrawPart,
    ) -> Option<DrawCommand> {
        let frame = asset.atlas.frame(&part.frame_name)?;
        let mut cmd = DrawCommand::sprite(
            asset.texture_id,
            glam::vec2(
                self.slot.position.x + self.character.position.x
                    - self.origin.x
                    - pose.animation.offset.x,
                self.slot.position.y + self.character.position.y
                    - self.origin.y
                    - pose.animation.offset.y,
            ),
            glam::vec2(frame.size.x, frame.size.y),
        );
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Characters;
        cmd.z = self.z;
        cmd.filter = self.filter;
        cmd.affine = scaled_affine(part.matrix, self.character.scale);
        cmd.uv_min = frame.uv_min;
        cmd.uv_max = frame.uv_max;
        cmd.uv_rotated = frame.rotated;
        cmd.color = glam::Vec4::from_array(part.color);
        cmd.color_offset = glam::Vec4::from_array(part.color_offset);
        if effective_flip_x(&self.character, self.is_player) {
            std::mem::swap(&mut cmd.uv_min.x, &mut cmd.uv_max.x);
        }
        Some(cmd)
    }

    fn command_for_overlay_part(
        &self,
        pose: &AnimateCharacterPose,
        overlay: &LoadedAnimateAtlas,
        part: &AnimateDrawPart,
        matrix: [f32; 6],
        alpha: f32,
    ) -> Option<DrawCommand> {
        let frame = overlay.atlas.frame(&part.frame_name)?;
        let mut cmd = DrawCommand::sprite(
            overlay.texture_id,
            glam::vec2(
                self.slot.position.x + self.character.position.x
                    - self.origin.x
                    - pose.animation.offset.x,
                self.slot.position.y + self.character.position.y
                    - self.origin.y
                    - pose.animation.offset.y,
            ),
            glam::vec2(frame.size.x, frame.size.y),
        );
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Characters;
        cmd.z = self.z + 1;
        cmd.filter = self.filter;
        cmd.affine = scaled_affine(matrix, self.character.scale);
        cmd.uv_min = frame.uv_min;
        cmd.uv_max = frame.uv_max;
        cmd.uv_rotated = frame.rotated;
        cmd.color = glam::Vec4::from_array(part.color);
        cmd.color.w *= alpha;
        cmd.color_offset = glam::Vec4::from_array(part.color_offset);
        if effective_flip_x(&self.character, self.is_player) {
            std::mem::swap(&mut cmd.uv_min.x, &mut cmd.uv_max.x);
        }
        Some(cmd)
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn load_animate_character_sprite(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    character: CharacterDefinition,
    slot: StageCharacterSlot,
    is_player: bool,
    z: i32,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<AnimateCharacterSprite> {
    let filter = filter_for_antialiasing(character.antialiasing);
    let mixed_sparrow = load_mixed_sparrow_character_sprite(
        device, queue, resolver, &character, slot, is_player, z, textures,
    )?;
    let mut assets = Vec::new();
    let mut asset_indices = HashMap::new();
    for animation in character
        .animations
        .iter()
        .filter(|animation| uses_animate_render_type(&character, animation))
    {
        let asset_path = animation_asset_path(&character, animation)?;
        let key = asset_path.as_str().to_owned();
        if asset_indices.contains_key(&key) {
            continue;
        }
        let loaded = load_animate_atlas(device, queue, resolver, &asset_path, filter, textures)?;
        asset_indices.insert(key, assets.len());
        assets.push(loaded);
    }

    let poses = animate_character_poses(&character, &assets, &asset_indices)?;
    let lip_sync = load_sserafim_lip_sync(device, queue, resolver, &character, filter, textures)?;
    let initial_animation = initial_animation(&character)?;
    let initial_pose = poses
        .iter()
        .position(|pose| pose.animation.name == initial_animation.name)
        .with_context(|| format!("resolve initial pose {}", character.id))?;
    let (origin, visual_height) =
        animate_character_origin(&poses[initial_pose], &assets, character.scale)?;

    Ok(AnimateCharacterSprite {
        character,
        slot,
        origin,
        visual_height,
        is_player,
        z,
        filter,
        assets,
        poses,
        mixed_sparrow,
        lip_sync,
        initial_pose,
    })
}

#[allow(clippy::too_many_arguments)]
fn load_mixed_sparrow_character_sprite(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    character: &CharacterDefinition,
    slot: StageCharacterSlot,
    is_player: bool,
    z: i32,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<Option<SparrowCharacterSprite>> {
    let animations = character
        .animations
        .iter()
        .filter(|animation| uses_sparrow_render_type(animation))
        .cloned()
        .collect::<Vec<_>>();
    let Some(first) = animations.first() else {
        return Ok(None);
    };
    let mut mixed = character.clone();
    mixed.render_type = CharacterRenderType::Sparrow;
    mixed.asset_path = first.asset_path.clone();
    mixed.initial_animation = Some(first.name.clone());
    mixed.animations = animations;
    Ok(Some(load_sparrow_character_sprite(
        device, queue, resolver, mixed, slot, is_player, z, textures,
    )?))
}

fn load_animate_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    asset_path: &AssetPath,
    filter: FilterMode,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<LoadedAnimateAtlas> {
    let animation_path = animate_asset_file(asset_path, "Animation.json")?;
    let spritemap_path = animate_asset_file(asset_path, "spritemap1.json")?;
    let texture_path = animate_asset_file(asset_path, "spritemap1.png")?;
    let animation = load_animate_animation(resolver, &animation_path)
        .with_context(|| format!("load {}", animation_path.as_str()))?;
    let atlas = load_animate_spritemap(resolver, &spritemap_path)
        .with_context(|| format!("load {}", spritemap_path.as_str()))?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(texture_path.as_str()));
    textures.insert(texture_id, texture);

    Ok(LoadedAnimateAtlas {
        texture_id,
        flat_labels: FlatLabelAtlas::new(&animation, &atlas),
        animation,
        atlas,
    })
}

fn load_sserafim_lip_sync(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    character: &CharacterDefinition,
    filter: FilterMode,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<Option<SserafimLipSyncOverlay>> {
    let Some(spec) = sserafim_lip_sync_spec(character) else {
        return Ok(None);
    };
    let asset_path = AssetPath::new(spec.asset_path)?;
    let asset = load_animate_atlas(device, queue, resolver, &asset_path, filter, textures)?;
    Ok(Some(SserafimLipSyncOverlay {
        asset,
        mouth_keyword: spec.mouth_keyword,
        offset: spec.offset,
        angle_degrees: spec.angle_degrees,
        alpha: spec.alpha,
    }))
}

#[derive(Debug, Clone, Copy)]
struct SserafimLipSyncSpec {
    asset_path: &'static str,
    mouth_keyword: &'static str,
    offset: glam::Vec2,
    angle_degrees: f32,
    alpha: f32,
}

fn sserafim_lip_sync_spec(character: &CharacterDefinition) -> Option<SserafimLipSyncSpec> {
    let path = character.asset_path.as_ref()?.as_str();
    let spec = if path.ends_with("sserafim/yunjin") {
        (
            "sserafim/sserafim-lipsync-yunjin",
            "mouth yunjin",
            glam::vec2(8.0, 6.0),
            23.0,
            1.0,
        )
    } else if path.ends_with("sserafim/sakura") {
        (
            "sserafim/sserafim-lipsync",
            "mouth edit",
            glam::vec2(7.0, 2.0),
            -14.0,
            1.0,
        )
    } else if path.ends_with("sserafim/chaewon") {
        (
            "sserafim/sserafim-lipsync",
            "mouth default",
            glam::vec2(41.0, 3.0),
            -166.0,
            0.5,
        )
    } else if path.ends_with("sserafim/eunchae") {
        (
            "sserafim/sserafim-lipsync",
            "mouth default",
            glam::vec2(43.0, 6.0),
            -168.0,
            1.0,
        )
    } else if path.ends_with("sserafim/kazuha") {
        (
            "sserafim/sserafim-lipsync",
            "mouth default",
            glam::vec2(5.0, 4.0),
            -13.0,
            1.0,
        )
    } else {
        return None;
    };
    Some(SserafimLipSyncSpec {
        asset_path: spec.0,
        mouth_keyword: spec.1,
        offset: spec.2,
        angle_degrees: spec.3,
        alpha: spec.4,
    })
}

#[derive(Debug, Clone)]
struct LoadedAnimateAtlas {
    texture_id: AssetId,
    flat_labels: Option<FlatLabelAtlas>,
    animation: AnimateAnimation,
    atlas: AnimateAtlas,
}

#[derive(Debug, Clone)]
struct SserafimLipSyncOverlay {
    asset: LoadedAnimateAtlas,
    mouth_keyword: &'static str,
    offset: glam::Vec2,
    angle_degrees: f32,
    alpha: f32,
}

impl SserafimLipSyncOverlay {
    fn commands(
        &self,
        owner: &AnimateCharacterSprite,
        pose: &AnimateCharacterPose,
        parts: &[AnimateDrawPart],
        request_name: &str,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        if !request_name.starts_with("sing") {
            return Vec::new();
        }
        let Some(mouth) = parts.iter().find(|part| {
            part.symbol_stack
                .iter()
                .any(|symbol| symbol == self.mouth_keyword)
        }) else {
            return Vec::new();
        };
        let frame_index = lip_sync_frame_index(cursor, sample_rate);
        let lip_parts = self
            .asset
            .animation
            .flatten_symbol_frame(&self.asset.animation.symbol_name, frame_index)
            .unwrap_or_default();
        lip_parts
            .iter()
            .filter_map(|part| {
                let matrix = compose_affine(
                    compose_affine(
                        mouth.matrix,
                        lip_sync_offset(self.offset, self.angle_degrees),
                    ),
                    part.matrix,
                );
                owner.command_for_overlay_part(pose, &self.asset, part, matrix, self.alpha)
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
struct FlatLabelAtlas {
    labels: HashMap<String, Vec<String>>,
}

impl FlatLabelAtlas {
    fn new(animation: &AnimateAnimation, atlas: &AnimateAtlas) -> Option<Self> {
        if !animation.symbols.is_empty() || animation.labels.is_empty() {
            return None;
        }
        let all_frames = atlas
            .sprites
            .iter()
            .flat_map(|sprite| sprite.frames.iter())
            .map(|frame| frame.name.clone())
            .collect::<Vec<_>>();
        if all_frames.is_empty() {
            return None;
        }

        let mut labels = HashMap::new();
        let mut cursor = 0usize;
        let mut remaining_duration = animation
            .labels
            .iter()
            .map(|label| label.duration.max(1) as usize)
            .sum::<usize>();
        for (index, label) in animation.labels.iter().enumerate() {
            let remaining_labels = animation.labels.len() - index;
            let remaining_frames = all_frames.len().saturating_sub(cursor);
            if remaining_frames == 0 {
                break;
            }
            let count = if remaining_labels == 1 {
                remaining_frames
            } else {
                let weighted = ((label.duration.max(1) as f64 / remaining_duration as f64)
                    * remaining_frames as f64)
                    .round() as usize;
                let max_count = remaining_frames
                    .saturating_sub(remaining_labels.saturating_sub(1))
                    .max(1);
                weighted.clamp(1, max_count)
            };
            labels.insert(
                label.name.clone(),
                all_frames[cursor..cursor + count].to_vec(),
            );
            cursor += count;
            remaining_duration = remaining_duration.saturating_sub(label.duration.max(1) as usize);
        }
        Some(Self { labels })
    }

    fn parts(
        &self,
        animation: &AnimateAnimation,
        pose: &CharacterAnimation,
        frame_offset: u32,
    ) -> Result<Vec<AnimateDrawPart>> {
        let frames = self
            .labels
            .get(&pose.prefix)
            .with_context(|| format!("resolve flat animate label {}", pose.prefix))?;
        let frame_name = frames
            .get(frame_offset as usize)
            .or_else(|| frames.last())
            .with_context(|| format!("resolve flat animate frame {}", pose.name))?;
        Ok(vec![AnimateDrawPart::atlas_frame(
            frame_name.clone(),
            flat_label_matrix(animation, &pose.prefix, frame_offset)?,
        )])
    }
}

#[derive(Debug, Clone)]
struct AnimateCharacterPose {
    animation: CharacterAnimation,
    asset_index: usize,
    source: AnimatePoseSource,
    frame_count: usize,
}

impl AnimateCharacterPose {
    fn parts(
        &self,
        asset: &LoadedAnimateAtlas,
        cursor: Samples,
        sample_rate: u32,
        started_at: Samples,
    ) -> Result<Vec<AnimateDrawPart>> {
        let sequence_index = animation_frame_index(
            cursor,
            sample_rate,
            started_at,
            self.animation.fps,
            self.frame_count,
            self.animation.looped,
        );
        let frame_offset = animation_frame_offset(&self.animation, sequence_index);
        match self.source {
            AnimatePoseSource::FrameLabel => {
                if let Some(flat_labels) = &asset.flat_labels {
                    flat_labels.parts(&asset.animation, &self.animation, frame_offset)
                } else {
                    Ok(asset
                        .animation
                        .flatten_label_frame(&self.animation.prefix, frame_offset)?)
                }
            }
            AnimatePoseSource::Symbol => Ok(asset
                .animation
                .flatten_symbol_frame(&self.animation.prefix, frame_offset)?),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AnimatePoseSource {
    FrameLabel,
    Symbol,
}

fn animate_character_poses(
    character: &CharacterDefinition,
    assets: &[LoadedAnimateAtlas],
    asset_indices: &HashMap<String, usize>,
) -> Result<Vec<AnimateCharacterPose>> {
    character
        .animations
        .iter()
        .filter(|animation| uses_animate_render_type(character, animation))
        .map(|animation| {
            let asset_path = animation_asset_path(character, animation)?;
            let asset_index = *asset_indices
                .get(asset_path.as_str())
                .with_context(|| format!("resolve animate asset {}", asset_path.as_str()))?;
            let source = animate_pose_source(animation);
            let frame_count = animate_pose_frame_count(animation, source, &assets[asset_index])?;
            let pose = AnimateCharacterPose {
                animation: animation.clone(),
                asset_index,
                source,
                frame_count,
            };
            if pose
                .parts(&assets[asset_index], Samples(0), SAMPLE_RATE, Samples(0))?
                .is_empty()
            {
                anyhow::bail!("resolve animate frame {}:{}", character.id, animation.name);
            }
            Ok(pose)
        })
        .collect()
}

fn animate_pose_frame_count(
    animation: &CharacterAnimation,
    source: AnimatePoseSource,
    asset: &LoadedAnimateAtlas,
) -> Result<usize> {
    if !animation.indices.is_empty() {
        return Ok(animation.indices.len());
    }
    let count = match source {
        AnimatePoseSource::FrameLabel => {
            if let Some(flat_labels) = &asset.flat_labels {
                flat_labels
                    .labels
                    .get(&animation.prefix)
                    .map(Vec::len)
                    .with_context(|| format!("resolve frame label {}", animation.prefix))?
            } else {
                asset
                    .animation
                    .label(&animation.prefix)
                    .map(|label| label.duration as usize)
                    .with_context(|| format!("resolve frame label {}", animation.prefix))?
            }
        }
        AnimatePoseSource::Symbol => asset
            .animation
            .symbol(&animation.prefix)
            .map(|symbol| symbol.duration() as usize)
            .with_context(|| format!("resolve symbol {}", animation.prefix))?,
    };
    if count == 0 {
        anyhow::bail!("animation {} has no frames", animation.name);
    }
    Ok(count)
}

fn animate_character_origin(
    pose: &AnimateCharacterPose,
    assets: &[LoadedAnimateAtlas],
    scale: f32,
) -> Result<(glam::Vec2, f32)> {
    let asset = &assets[pose.asset_index];
    let parts = pose.parts(asset, Samples(0), SAMPLE_RATE, Samples(0))?;
    let (min, max) =
        animate_parts_bounds(asset, &parts).context("resolve animate origin bounds")?;
    let size = max - min;
    Ok((
        (min + glam::vec2(size.x * 0.5, size.y)) * scale,
        size.y * scale,
    ))
}

fn animate_parts_bounds(
    asset: &LoadedAnimateAtlas,
    parts: &[AnimateDrawPart],
) -> Option<(glam::Vec2, glam::Vec2)> {
    let mut min = glam::Vec2::splat(f32::INFINITY);
    let mut max = glam::Vec2::splat(f32::NEG_INFINITY);
    for part in parts {
        let frame = asset.atlas.frame(&part.frame_name)?;
        for point in [
            glam::Vec2::ZERO,
            glam::vec2(frame.size.x, 0.0),
            glam::vec2(0.0, frame.size.y),
            frame.size,
        ] {
            let transformed = transform_affine_point(part.matrix, point);
            min = min.min(transformed);
            max = max.max(transformed);
        }
    }
    min.x.is_finite().then_some((min, max))
}

fn transform_affine_point(matrix: [f32; 6], point: glam::Vec2) -> glam::Vec2 {
    glam::vec2(
        point.x * matrix[0] + point.y * matrix[2] + matrix[4],
        point.x * matrix[1] + point.y * matrix[3] + matrix[5],
    )
}

fn flat_label_matrix(
    animation: &AnimateAnimation,
    label_name: &str,
    frame_offset: u32,
) -> Result<[f32; 6]> {
    let label = animation
        .label(label_name)
        .with_context(|| format!("resolve frame label {label_name}"))?;
    let frame_index = label
        .index
        .saturating_add(frame_offset.min(label.duration.saturating_sub(1)));
    for layer in animation.layers.iter().rev() {
        let Some(frame) = layer.frames.iter().find(|frame| {
            frame_index >= frame.index && frame_index < frame.index.saturating_add(frame.duration)
        }) else {
            continue;
        };
        if let Some(element) = frame.elements.first() {
            return Ok(element.matrix);
        }
    }
    Ok([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
}

fn animate_pose_source(animation: &CharacterAnimation) -> AnimatePoseSource {
    match animation.anim_type.as_deref() {
        Some("symbol") => AnimatePoseSource::Symbol,
        _ => AnimatePoseSource::FrameLabel,
    }
}

fn uses_animate_render_type(
    character: &CharacterDefinition,
    animation: &CharacterAnimation,
) -> bool {
    matches!(
        animation.render_type.unwrap_or(character.render_type),
        CharacterRenderType::AnimateAtlas | CharacterRenderType::MultiAnimateAtlas
    )
}

fn uses_sparrow_render_type(animation: &CharacterAnimation) -> bool {
    matches!(
        animation.render_type,
        Some(CharacterRenderType::Sparrow | CharacterRenderType::MultiSparrow)
    )
}

fn animation_asset_path(
    character: &CharacterDefinition,
    animation: &CharacterAnimation,
) -> Result<AssetPath> {
    animation
        .asset_path
        .clone()
        .or_else(|| character.asset_path.clone())
        .with_context(|| format!("character {} has no assetPath", character.id))
}

fn animation_frame_index(
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
    fps: u16,
    frame_count: usize,
    looped: bool,
) -> usize {
    flixel_frame_index(cursor, sample_rate, started_at, fps, frame_count, looped)
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

fn animation_frame_offset(animation: &CharacterAnimation, sequence_index: usize) -> u32 {
    animation
        .indices
        .get(sequence_index)
        .map(|index| u32::from(*index))
        .unwrap_or(sequence_index as u32)
}

fn animate_asset_file(asset_path: &AssetPath, file_name: &str) -> Result<AssetPath> {
    let raw = asset_path.as_str();
    let stripped = raw.split_once(':').map(|(_, path)| path).unwrap_or(raw);
    Ok(AssetPath::new(format!("images/{stripped}/{file_name}"))?)
}

fn lip_sync_frame_index(cursor: Samples, sample_rate: u32) -> u32 {
    let seconds = cursor.0.max(0) as f32 / sample_rate.max(1) as f32;
    (seconds * 24.0).floor().max(1.0) as u32 - 1
}

fn lip_sync_offset(offset: glam::Vec2, angle_degrees: f32) -> [f32; 6] {
    let radians = angle_degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    [cos, sin, -sin, cos, offset.x, offset.y]
}

fn compose_affine(parent: [f32; 6], child: [f32; 6]) -> [f32; 6] {
    [
        parent[0] * child[0] + parent[2] * child[1],
        parent[1] * child[0] + parent[3] * child[1],
        parent[0] * child[2] + parent[2] * child[3],
        parent[1] * child[2] + parent[3] * child[3],
        parent[0] * child[4] + parent[2] * child[5] + parent[4],
        parent[1] * child[4] + parent[3] * child[5] + parent[5],
    ]
}

fn scaled_affine(matrix: [f32; 6], scale: f32) -> [f32; 6] {
    [
        matrix[0] * scale,
        matrix[1] * scale,
        matrix[2] * scale,
        matrix[3] * scale,
        matrix[4] * scale,
        matrix[5] * scale,
    ]
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
#[path = "animate_character_assets_tests.rs"]
mod tests;
