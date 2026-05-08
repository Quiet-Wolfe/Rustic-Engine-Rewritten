//! Adobe Animate character asset loading and draw command expansion.
// LINT-ALLOW: long-file Animate atlas loading plus runtime draw expansion.

use crate::character_anim::CharacterPoseRequest;
use anyhow::{Context, Result};
use rustic_asset::{
    load_animate_animation, load_animate_spritemap, load_png, AnimateAnimation, AnimateAtlas,
    AnimateDrawPart, AssetPath, CharacterAnimation, CharacterDefinition, OverlayResolver,
    StageCharacterSlot,
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
    is_player: bool,
    z: i32,
    filter: FilterMode,
    assets: Vec<LoadedAnimateAtlas>,
    poses: Vec<AnimateCharacterPose>,
    initial_pose: usize,
}

impl AnimateCharacterSprite {
    pub(crate) fn commands(
        &self,
        request: CharacterPoseRequest,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        let (pose, started_at) = self.resolve_pose_for_request(request, cursor, sample_rate);
        let asset = &self.assets[pose.asset_index];
        let parts = pose
            .parts(asset, cursor, sample_rate, started_at)
            .unwrap_or_default();
        parts
            .iter()
            .filter_map(|part| self.command_for_part(pose, asset, part))
            .collect()
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
            pose.frame_count,
        ))
    }

    pub(crate) fn camera_focus_point(&self) -> glam::Vec2 {
        glam::vec2(
            self.slot.position.x + self.character.position.x,
            self.slot.position.y + self.character.position.y,
        ) + glam::vec2(self.slot.camera_offset.x, self.slot.camera_offset.y)
            + glam::vec2(
                self.character.camera_offset.x,
                self.character.camera_offset.y,
            )
    }

    pub(crate) fn definition(&self) -> &CharacterDefinition {
        &self.character
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
                self.slot.position.x + self.character.position.x - pose.animation.offset.x,
                self.slot.position.y + self.character.position.y - pose.animation.offset.y,
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
    let mut assets = Vec::new();
    let mut asset_indices = HashMap::new();
    for animation in &character.animations {
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
    let initial_animation = initial_animation(&character)?;
    let initial_pose = poses
        .iter()
        .position(|pose| pose.animation.name == initial_animation.name)
        .with_context(|| format!("resolve initial pose {}", character.id))?;

    Ok(AnimateCharacterSprite {
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

#[derive(Debug, Clone)]
struct LoadedAnimateAtlas {
    texture_id: AssetId,
    flat_labels: Option<FlatLabelAtlas>,
    animation: AnimateAnimation,
    atlas: AnimateAtlas,
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
    if frame_count <= 1 {
        return 0;
    }
    let elapsed = cursor.0.saturating_sub(started_at.0).max(0) as f64;
    let samples_per_frame = f64::from(sample_rate.max(1)) / f64::from(fps.max(1));
    let frame = (elapsed / samples_per_frame).floor() as usize;
    if looped {
        frame % frame_count
    } else {
        frame.min(frame_count - 1)
    }
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::asset_roots::baked_assets_root;
    use rustic_asset::load_character;

    fn character_with_hold() -> CharacterDefinition {
        CharacterDefinition::parse(
            br#"{
                "id": "dad",
                "assetPath": "shared:characters/dad",
                "animations": [
                    { "name": "singUP", "prefix": "Up", "frameRate": 24 },
                    { "name": "singUP-hold", "prefix": "Up", "frameRate": 24,
                      "looped": true, "frameIndices": [3, 4, 5] }
                ]
            }"#,
        )
        .unwrap()
    }

    fn pose(animation: CharacterAnimation, frame_count: usize) -> AnimateCharacterPose {
        AnimateCharacterPose {
            animation,
            asset_index: 0,
            source: AnimatePoseSource::FrameLabel,
            frame_count,
        }
    }

    fn baked_resolver() -> OverlayResolver {
        OverlayResolver::new().with_baked_root(baked_assets_root())
    }

    fn test_animate_atlas(
        resolver: &OverlayResolver,
        asset_path: &AssetPath,
    ) -> LoadedAnimateAtlas {
        let animation_path = animate_asset_file(asset_path, "Animation.json").unwrap();
        let spritemap_path = animate_asset_file(asset_path, "spritemap1.json").unwrap();
        let animation = load_animate_animation(resolver, &animation_path).unwrap();
        let atlas = load_animate_spritemap(resolver, &spritemap_path).unwrap();
        LoadedAnimateAtlas {
            texture_id: AssetId::new(1),
            flat_labels: FlatLabelAtlas::new(&animation, &atlas),
            animation,
            atlas,
        }
    }

    #[test]
    fn finished_animate_pose_switches_to_matching_hold_animation() {
        let character = character_with_hold();
        let sprite = AnimateCharacterSprite {
            poses: vec![
                pose(character.animations[0].clone(), 2),
                pose(character.animations[1].clone(), 3),
            ],
            character,
            slot: StageCharacterSlot::default(),
            is_player: false,
            z: 0,
            filter: FilterMode::Nearest,
            assets: Vec::new(),
            initial_pose: 0,
        };
        let request = CharacterPoseRequest {
            name: "singUP",
            started_at: Samples(100),
        };

        let (before, before_started) =
            sprite.resolve_pose_for_request(request, Samples(4_099), 48_000);
        assert_eq!(before.animation.name, "singUP");
        assert_eq!(before_started, Samples(100));

        let (after, after_started) =
            sprite.resolve_pose_for_request(request, Samples(4_100), 48_000);
        assert_eq!(after.animation.name, "singUP-hold");
        assert_eq!(after_started, Samples(4_100));
    }

    #[test]
    fn flat_label_indices_are_relative_to_the_label_prefix() {
        let animation = AnimateAnimation::parse(
            br#"{
                "AN": {
                    "N": "Test",
                    "SN": "Root",
                    "TL": {
                        "L": [
                            { "LN": "labels", "FR": [
                                { "N": "Idle", "I": 0, "DU": 3, "E": [] },
                                { "N": "Up", "I": 3, "DU": 3, "E": [] }
                            ] },
                            { "LN": "art", "FR": [
                                { "I": 0, "DU": 6, "E": [] }
                            ] }
                        ]
                    }
                }
            }"#,
        )
        .unwrap();
        let atlas = AnimateAtlas::parse_spritemap(
            br#"{
                "ATLAS": {
                    "SPRITES": [
                        { "SPRITE": { "name": "idle0", "x": 0, "y": 0, "w": 10, "h": 10 } },
                        { "SPRITE": { "name": "idle1", "x": 10, "y": 0, "w": 10, "h": 10 } },
                        { "SPRITE": { "name": "idle2", "x": 20, "y": 0, "w": 10, "h": 10 } },
                        { "SPRITE": { "name": "up0", "x": 30, "y": 0, "w": 10, "h": 10 } },
                        { "SPRITE": { "name": "up1", "x": 40, "y": 0, "w": 10, "h": 10 } },
                        { "SPRITE": { "name": "up2", "x": 50, "y": 0, "w": 10, "h": 10 } }
                    ],
                    "meta": { "size": { "w": 60, "h": 10 } }
                }
            }"#,
        )
        .unwrap();
        let flat = FlatLabelAtlas::new(&animation, &atlas).unwrap();
        let character = CharacterDefinition::parse(
            br#"{
                "id": "test",
                "assetPath": "shared:characters/test",
                "animations": [
                    { "name": "singUP-hold", "prefix": "Up", "frameIndices": [1] }
                ]
            }"#,
        )
        .unwrap();

        let parts = flat.parts(&animation, &character.animations[0], 1).unwrap();
        assert_eq!(parts[0].frame_name, "up1");
    }

    #[test]
    fn gf_dance_left_uses_animate_symbols_instead_of_flat_spritemap_slices() {
        let resolver = baked_resolver();
        let character = load_character(
            &resolver,
            &AssetPath::new("data/characters/gf.json").unwrap(),
        )
        .unwrap();
        let asset_path = animation_asset_path(&character, &character.animations[0]).unwrap();
        let loaded = test_animate_atlas(&resolver, &asset_path);
        assert!(loaded.flat_labels.is_none());
        let mut asset_indices = HashMap::new();
        asset_indices.insert(asset_path.as_str().to_owned(), 0);
        let poses =
            animate_character_poses(&character, std::slice::from_ref(&loaded), &asset_indices)
                .unwrap();
        let dance_left = poses
            .iter()
            .find(|pose| pose.animation.name == "danceLeft")
            .unwrap();
        let parts = dance_left
            .parts(&loaded, Samples(0), SAMPLE_RATE, Samples(0))
            .unwrap();
        let frame_names = parts
            .iter()
            .map(|part| part.frame_name.as_str())
            .collect::<Vec<_>>();

        assert!(frame_names.len() > 4);
        assert!(frame_names.contains(&"16"));
        assert!(!frame_names
            .iter()
            .any(|name| ["36", "37", "38"].contains(name)));
    }
}
