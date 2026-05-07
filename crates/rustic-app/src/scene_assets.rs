//! Startup scene asset wiring.
//!
//! This module is intentionally app-owned: it resolves vanilla assets,
//! uploads textures, and emits render commands, but gameplay and render
//! crates remain free of filesystem and wgpu wiring.
// LINT-ALLOW: long-file startup scene plus current NOTE_assets skin wiring
use crate::character_anim::{CharacterPoseNames, CharacterPoseRequest};
use crate::countdown_assets::{load_countdown_assets, CountdownSkin};
use crate::hud_assets::{load_hud_assets, HudSkin};
use crate::note_assets::{load_note_skin, NoteSkin};
use crate::note_splash_assets::{load_note_splash_assets, NoteSplashSkin};
use crate::popup_assets::{load_popup_assets, PopupSkin};
use anyhow::{Context, Result};
use rustic_asset::{
    load_character, load_chart, load_png, load_sparrow, load_stage, AssetPath, CharacterAnimation,
    CharacterDefinition, OverlayResolver, SparrowAtlas, SparrowFrame, StageCharacterSlot,
    StageDefinition, StageObject,
};
use rustic_core::ids::{AssetId, SongId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_game::PlayState;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;
pub const SAMPLE_RATE: u32 = 48_000;
pub struct LoadedScene {
    pub camera_zoom: f32,
    pub commands: RenderCommandList,
    pub textures: HashMap<AssetId, Texture>,
    pub characters: Option<CharacterSet>,
    pub note_skin: Option<NoteSkin>,
    pub note_splash_skin: Option<NoteSplashSkin>,
    pub hud_skin: Option<HudSkin>,
    pub popup_skin: Option<PopupSkin>,
    pub countdown_skin: Option<CountdownSkin>,
}

#[derive(Debug, Clone)]
pub struct CharacterSet {
    girlfriend: CharacterSprite,
    opponent: CharacterSprite,
    player: CharacterSprite,
}

impl CharacterSet {
    pub fn commands(
        &self,
        poses: CharacterPoseNames,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        vec![
            self.girlfriend
                .command(poses.girlfriend, cursor, sample_rate),
            self.opponent.command(poses.opponent, cursor, sample_rate),
            self.player.command(poses.player, cursor, sample_rate),
        ]
    }

    pub fn player_command(
        &self,
        request: CharacterPoseRequest,
        cursor: Samples,
        sample_rate: u32,
    ) -> DrawCommand {
        self.player.command(request, cursor, sample_rate)
    }

    pub fn player_animation_duration(
        &self,
        animation_name: &str,
        sample_rate: u32,
    ) -> Option<Samples> {
        self.player.animation_duration(animation_name, sample_rate)
    }
}

#[derive(Debug, Clone)]
struct CharacterSprite {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    character: CharacterDefinition,
    slot: StageCharacterSlot,
    is_player: bool,
    z: i32,
    filter: FilterMode,
    poses: Vec<CharacterPose>,
    initial_pose: usize,
}

impl CharacterSprite {
    fn command(
        &self,
        request: CharacterPoseRequest,
        cursor: Samples,
        sample_rate: u32,
    ) -> DrawCommand {
        let pose = self.pose(request.name);
        let frame = pose.frame(cursor, sample_rate, request.started_at);
        let mut cmd = DrawCommand::sprite(
            self.texture_id,
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
        (cmd.uv_min, cmd.uv_max) = frame_uv(frame, self.texture_width, self.texture_height);
        if effective_flip_x(&self.character, self.is_player) {
            std::mem::swap(&mut cmd.uv_min.x, &mut cmd.uv_max.x);
        }
        cmd
    }

    fn pose(&self, animation_name: &str) -> &CharacterPose {
        self.poses
            .iter()
            .find(|pose| pose.animation.name == animation_name)
            .unwrap_or(&self.poses[self.initial_pose])
    }

    fn animation_duration(&self, animation_name: &str, sample_rate: u32) -> Option<Samples> {
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
}

#[derive(Debug, Clone)]
struct CharacterPose {
    animation: CharacterAnimation,
    frames: Vec<SparrowFrame>,
}

impl CharacterPose {
    fn frame(&self, cursor: Samples, sample_rate: u32, started_at: Samples) -> &SparrowFrame {
        let index = animation_frame_index(
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
pub fn load_default_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<LoadedScene> {
    let resolver = OverlayResolver::new().with_baked_root("assets/baked");
    let stage_path = AssetPath::new("data/stages/stage.json")?;
    let stage = load_stage(&resolver, &stage_path).context("load default stage definition")?;

    let mut scene = LoadedScene {
        camera_zoom: stage.camera_zoom,
        commands: RenderCommandList::new(),
        textures: HashMap::new(),
        characters: None,
        note_skin: None,
        note_splash_skin: None,
        hud_skin: None,
        popup_skin: None,
        countdown_skin: None,
    };

    for object in &stage.objects {
        load_stage_object(device, queue, &resolver, object, &mut scene)?;
    }
    scene.characters = Some(load_stage_characters(
        device, queue, &resolver, &stage, &mut scene,
    )?);
    scene.note_skin = Some(load_note_skin(
        device,
        queue,
        &resolver,
        &mut scene.textures,
    )?);
    scene.note_splash_skin = Some(load_note_splash_assets(
        device,
        queue,
        &resolver,
        &mut scene.textures,
    )?);
    scene.hud_skin = Some(load_hud_assets(
        device,
        queue,
        &resolver,
        &mut scene.textures,
    )?);
    scene.popup_skin = Some(load_popup_assets(
        device,
        queue,
        &resolver,
        &mut scene.textures,
    )?);
    scene.countdown_skin = Some(load_countdown_assets(
        device,
        queue,
        &resolver,
        &mut scene.textures,
    )?);
    Ok(scene)
}

pub fn load_preview_play_state(sample_rate: u32) -> Result<PlayState> {
    let resolver = OverlayResolver::new().with_baked_root("assets/baked");
    let chart_path = AssetPath::new("data/bopeebo/bopeebo.json")?;
    let chart =
        load_chart(&resolver, &chart_path).with_context(|| format!("load {}", chart_path))?;
    Ok(PlayState::from_chart(SongId::new(0), &chart, sample_rate))
}

fn load_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    object: &StageObject,
    scene: &mut LoadedScene,
) -> Result<()> {
    let image = load_png(resolver, &object.image)
        .with_context(|| format!("load {}", object.image.as_str()))?;
    let texture_id = asset_id_for_path(&object.image);
    let filter = filter_for_antialiasing(object.antialiasing);
    let size = glam::vec2(
        image.width as f32 * object.scale.x,
        image.height as f32 * object.scale.y,
    );
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(object.image.as_str()));
    scene.textures.insert(texture_id, texture);

    let mut cmd = DrawCommand::sprite(
        texture_id,
        glam::vec2(object.position.x, object.position.y),
        size,
    );
    cmd.pivot = glam::Vec2::ZERO;
    cmd.layer = object.layer;
    cmd.z = object.z;
    cmd.filter = filter;
    scene.commands.push(cmd);
    Ok(())
}

fn load_stage_characters(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    stage: &StageDefinition,
    scene: &mut LoadedScene,
) -> Result<CharacterSet> {
    let girlfriend = load_character_slot(
        device,
        queue,
        resolver,
        "data/characters/gf.json",
        stage.girlfriend,
        false,
        0,
        scene,
    )?;
    let opponent = load_character_slot(
        device,
        queue,
        resolver,
        "data/characters/dad.json",
        stage.opponent,
        false,
        1,
        scene,
    )?;
    let player = load_character_slot(
        device,
        queue,
        resolver,
        "data/characters/bf.json",
        stage.boyfriend,
        true,
        2,
        scene,
    )?;
    Ok(CharacterSet {
        girlfriend,
        opponent,
        player,
    })
}

#[allow(clippy::too_many_arguments)]
fn load_character_slot(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    character_path: &str,
    slot: StageCharacterSlot,
    is_player: bool,
    z: i32,
    scene: &mut LoadedScene,
) -> Result<CharacterSprite> {
    let character_path = AssetPath::new(character_path)?;
    let character = load_character(resolver, &character_path)
        .with_context(|| format!("load {}", character_path.as_str()))?;
    let atlas = load_sparrow(resolver, &character.atlas)
        .with_context(|| format!("load {}", character.atlas.as_str()))?;
    let texture_path = atlas_texture_path(&character.atlas, &atlas)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);
    let filter = filter_for_antialiasing(character.antialiasing);
    let poses = character_poses(&character, &atlas)?;
    let initial_animation = initial_animation(&character)?;
    let initial_pose = poses
        .iter()
        .position(|pose| pose.animation.name == initial_animation.name)
        .with_context(|| format!("resolve initial pose {}", character.id))?;

    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(texture_path.as_str()));
    scene.textures.insert(texture_id, texture);

    Ok(CharacterSprite {
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
        character,
        slot,
        is_player,
        z,
        filter,
        poses,
        initial_pose,
    })
}

fn character_poses(
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
                frames,
            })
        })
        .collect()
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

fn atlas_texture_path(atlas_path: &AssetPath, atlas: &SparrowAtlas) -> Result<AssetPath> {
    if atlas.image_path.contains('/') {
        Ok(AssetPath::new(atlas.image_path.clone())?)
    } else {
        Ok(atlas_path.sibling(&atlas.image_path)?)
    }
}

fn character_frame_pos(
    character: &CharacterDefinition,
    animation: &CharacterAnimation,
    frame: &SparrowFrame,
    slot: StageCharacterSlot,
) -> glam::Vec2 {
    glam::vec2(
        slot.position.x + character.position.x - animation.offset.x - frame.frame_x as f32,
        slot.position.y + character.position.y - animation.offset.y - frame.frame_y as f32,
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
mod tests {
    use super::*;

    #[test]
    fn non_looping_animation_frames_clamp_to_last_frame() {
        // ref: 50fccded:source/Character.hx:271-284
        assert_eq!(
            animation_frame_index(Samples(0), 48_000, Samples(0), 24, 3, false),
            0
        );
        assert_eq!(
            animation_frame_index(Samples(2_000), 48_000, Samples(0), 24, 3, false),
            1
        );
        assert_eq!(
            animation_frame_index(Samples(96_000), 48_000, Samples(0), 24, 3, false),
            2
        );
    }

    #[test]
    fn looping_animation_frames_wrap() {
        // ref: 50fccded:source/Character.hx:128-133
        assert_eq!(
            animation_frame_index(Samples(6_000), 48_000, Samples(0), 24, 3, true),
            0
        );
        assert_eq!(
            animation_frame_index(Samples(8_000), 48_000, Samples(0), 24, 3, true),
            1
        );
    }

    #[test]
    fn animation_frame_index_uses_pose_start_cursor() {
        assert_eq!(
            animation_frame_index(Samples(12_000), 48_000, Samples(10_000), 24, 3, false),
            1
        );
    }

    #[test]
    fn animation_duration_uses_frame_count_and_fps() {
        assert_eq!(animation_duration_samples(48_000, 24, 12), Samples(24_000));
        assert_eq!(animation_duration_samples(48_000, 0, 0), Samples(48_000));
    }
}
