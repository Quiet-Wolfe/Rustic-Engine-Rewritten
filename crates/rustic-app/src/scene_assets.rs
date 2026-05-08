//! Startup scene asset wiring.
//!
//! This module is intentionally app-owned: it resolves vanilla assets,
//! uploads textures, and emits render commands, but gameplay and render
//! crates remain free of filesystem and wgpu wiring.
// LINT-ALLOW: long-file startup scene plus current NOTE_assets skin wiring
use crate::animate_character_assets::{load_animate_character_sprite, AnimateCharacterSprite};
use crate::asset_roots::baked_assets_root;
use crate::bitmap_text_assets::{load_bitmap_text_assets, BitmapTextSkin};
use crate::character_anim::CountAnimationTiming;
use crate::character_anim::COUNT_ANIM_SLOTS;
use crate::character_anim::{CharacterAnimTimings, CharacterPoseNames, CharacterPoseRequest};
use crate::countdown_assets::{load_countdown_assets, CountdownSkin};
use crate::hold_cover_assets::{load_hold_cover_assets, HoldCoverSkin};
use crate::hud_assets::{load_hud_assets, HudSkin};
use crate::note_assets::{load_note_skin, NoteSkin};
use crate::note_splash_assets::{load_note_splash_assets, NoteSplashSkin};
use crate::popup_assets::{load_popup_assets, PopupSkin};
use crate::preview_song::PreviewSelection;
use anyhow::{Context, Result};
use rustic_asset::{
    load_character, load_png, load_sparrow, load_stage, load_vslice_chart, AssetPath,
    CharacterAnimation, CharacterDefinition, CharacterRenderType, OverlayResolver, ParsedSong,
    SparrowAtlas, SparrowFrame, StageCharacterSlot, StageDefinition, StageObject,
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
    pub camera_focus: CameraFocusPoints,
    pub commands: RenderCommandList,
    pub textures: HashMap<AssetId, Texture>,
    pub characters: Option<CharacterSet>,
    pub bitmap_text_skin: Option<BitmapTextSkin>,
    pub note_skin: Option<NoteSkin>,
    pub note_splash_skin: Option<NoteSplashSkin>,
    pub hold_cover_skin: Option<HoldCoverSkin>,
    pub hud_skin: Option<HudSkin>,
    pub popup_skin: Option<PopupSkin>,
    pub countdown_skin: Option<CountdownSkin>,
}

#[derive(Debug, Clone, Copy)]
pub struct CameraFocusPoints {
    pub player: glam::Vec2,
    pub opponent: glam::Vec2,
    pub girlfriend: glam::Vec2,
}

impl Default for CameraFocusPoints {
    fn default() -> Self {
        let center = glam::vec2(1280.0 * 0.5, 720.0 * 0.5);
        Self {
            player: center,
            opponent: center,
            girlfriend: center,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CharacterSet {
    girlfriend: Option<CharacterSprite>,
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
        let mut commands = Vec::new();
        if let Some(girlfriend) = &self.girlfriend {
            commands.extend(girlfriend.commands(poses.girlfriend, cursor, sample_rate));
        }
        commands.extend(self.opponent.commands(poses.opponent, cursor, sample_rate));
        commands.extend(self.player.commands(poses.player, cursor, sample_rate));
        commands
    }

    pub fn player_commands(
        &self,
        request: CharacterPoseRequest,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        self.player.commands(request, cursor, sample_rate)
    }

    pub fn player_animation_duration(
        &self,
        animation_name: &str,
        sample_rate: u32,
    ) -> Option<Samples> {
        self.player.animation_duration(animation_name, sample_rate)
    }

    pub fn camera_focus_points(&self) -> CameraFocusPoints {
        CameraFocusPoints {
            player: self.player.camera_focus_point(),
            opponent: self.opponent.camera_focus_point(),
            girlfriend: self.girlfriend.as_ref().map_or_else(
                || self.player.camera_focus_point(),
                CharacterSprite::camera_focus_point,
            ),
        }
    }

    pub fn anim_timings(&self) -> CharacterAnimTimings {
        CharacterAnimTimings {
            player_sing_steps: self.player.definition().sing_time,
            opponent_sing_steps: self.opponent.definition().sing_time,
            girlfriend_combo_timings: self
                .girlfriend
                .as_ref()
                .map(|sprite| count_animation_timings(sprite, "combo"))
                .unwrap_or_default(),
            girlfriend_drop_timings: self
                .girlfriend
                .as_ref()
                .map(|sprite| count_animation_timings(sprite, "drop"))
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
enum CharacterSprite {
    Sparrow(SparrowCharacterSprite),
    Animate(AnimateCharacterSprite),
}

impl CharacterSprite {
    fn commands(
        &self,
        request: CharacterPoseRequest,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        match self {
            Self::Sparrow(sprite) => vec![sprite.command(request, cursor, sample_rate)],
            Self::Animate(sprite) => sprite.commands(request, cursor, sample_rate),
        }
    }

    fn animation_duration(&self, animation_name: &str, sample_rate: u32) -> Option<Samples> {
        match self {
            Self::Sparrow(sprite) => sprite.animation_duration(animation_name, sample_rate),
            Self::Animate(sprite) => sprite.animation_duration(animation_name, sample_rate),
        }
    }

    fn camera_focus_point(&self) -> glam::Vec2 {
        match self {
            Self::Sparrow(sprite) => sprite.camera_focus_point(),
            Self::Animate(sprite) => sprite.camera_focus_point(),
        }
    }

    fn definition(&self) -> &CharacterDefinition {
        match self {
            Self::Sparrow(sprite) => &sprite.character,
            Self::Animate(sprite) => sprite.definition(),
        }
    }
}

#[derive(Debug, Clone)]
struct SparrowCharacterSprite {
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

impl SparrowCharacterSprite {
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
        cmd.uv_rotated = frame.rotated;
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

    fn camera_focus_point(&self) -> glam::Vec2 {
        glam::vec2(
            self.slot.position.x + self.character.position.x,
            self.slot.position.y + self.character.position.y,
        ) + glam::vec2(self.slot.camera_offset.x, self.slot.camera_offset.y)
            + glam::vec2(
                self.character.camera_offset.x,
                self.character.camera_offset.y,
            )
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
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    load_scene_for_ids(device, queue, &resolver, "stage", Some("gf"), "dad", "bf")
}

pub(crate) fn load_preview_scene_for(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    selection: PreviewSelection,
) -> Result<LoadedScene> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let chart = load_preview_song_for(selection)?;
    load_scene_for_chart(device, queue, &resolver, &chart)
}

fn load_scene_for_chart(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    parsed: &ParsedSong,
) -> Result<LoadedScene> {
    let chart = &parsed.chart;
    load_scene_for_ids(
        device,
        queue,
        resolver,
        stage_asset_id(&chart.stage),
        character_id(&chart.girlfriend),
        &chart.player2,
        &chart.player1,
    )
}

fn load_scene_for_ids(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    stage_id: &str,
    girlfriend_id: Option<&str>,
    opponent_id: &str,
    player_id: &str,
) -> Result<LoadedScene> {
    let stage_path = AssetPath::new(format!("data/stages/{stage_id}.json"))?;
    let stage = load_stage(resolver, &stage_path).context("load default stage definition")?;

    let mut scene = LoadedScene {
        camera_zoom: stage.camera_zoom,
        camera_focus: CameraFocusPoints::default(),
        commands: RenderCommandList::new(),
        textures: HashMap::new(),
        characters: None,
        bitmap_text_skin: None,
        note_skin: None,
        note_splash_skin: None,
        hold_cover_skin: None,
        hud_skin: None,
        popup_skin: None,
        countdown_skin: None,
    };

    for object in &stage.objects {
        load_stage_object(device, queue, resolver, object, &mut scene)?;
    }
    let characters = load_stage_characters(
        device,
        queue,
        resolver,
        &stage,
        girlfriend_id,
        opponent_id,
        player_id,
        &mut scene,
    )?;
    scene.camera_focus = characters.camera_focus_points();
    scene.characters = Some(characters);
    scene.bitmap_text_skin = Some(load_bitmap_text_assets(
        device,
        queue,
        &resolver,
        &mut scene.textures,
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
    scene.hold_cover_skin = Some(load_hold_cover_assets(
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
    load_preview_play_state_for(PreviewSelection::from_env(), sample_rate)
}

pub(crate) fn load_preview_song_for(selection: PreviewSelection) -> Result<ParsedSong> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let song = selection.song;
    let chart_path = AssetPath::new(song.chart_path())?;
    let metadata_path = AssetPath::new(song.metadata_path())?;
    let difficulty = selection.difficulty.as_str();
    load_vslice_chart(&resolver, &chart_path, &metadata_path, difficulty)
        .with_context(|| format!("load {} + {} [{}]", chart_path, metadata_path, difficulty))
}

pub(crate) fn load_preview_play_state_for(
    selection: PreviewSelection,
    sample_rate: u32,
) -> Result<PlayState> {
    let song = selection.song;
    let chart = load_preview_song_for(selection)?;
    Ok(PlayState::from_chart(
        SongId::new(song.id),
        &chart,
        sample_rate,
    ))
}

fn stage_asset_id(stage: &str) -> &str {
    match stage {
        "mainStage" | "mainStageErect" => "stage",
        other => other,
    }
}

fn character_id(id: &str) -> Option<&str> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
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
    cmd.scroll_factor = glam::vec2(object.scroll_factor.x, object.scroll_factor.y);
    scene.commands.push(cmd);
    Ok(())
}

fn load_stage_characters(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    stage: &StageDefinition,
    girlfriend_id: Option<&str>,
    opponent_id: &str,
    player_id: &str,
    scene: &mut LoadedScene,
) -> Result<CharacterSet> {
    let girlfriend = girlfriend_id
        .map(|id| {
            load_character_slot(
                device,
                queue,
                resolver,
                id,
                stage.girlfriend,
                false,
                0,
                scene,
            )
        })
        .transpose()?;
    let opponent = load_character_slot(
        device,
        queue,
        resolver,
        opponent_id,
        stage.opponent,
        false,
        1,
        scene,
    )?;
    let player = load_character_slot(
        device,
        queue,
        resolver,
        player_id,
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
    character_id: &str,
    slot: StageCharacterSlot,
    is_player: bool,
    z: i32,
    scene: &mut LoadedScene,
) -> Result<CharacterSprite> {
    let character_path = AssetPath::new(format!("data/characters/{character_id}.json"))?;
    let character = load_character(resolver, &character_path)
        .with_context(|| format!("load {}", character_path.as_str()))?;
    match character.render_type {
        CharacterRenderType::Sparrow | CharacterRenderType::MultiSparrow => {
            load_sparrow_character_slot(
                device, queue, resolver, character, slot, is_player, z, scene,
            )
        }
        CharacterRenderType::AnimateAtlas | CharacterRenderType::MultiAnimateAtlas => {
            Ok(CharacterSprite::Animate(load_animate_character_sprite(
                device,
                queue,
                resolver,
                character,
                slot,
                is_player,
                z,
                &mut scene.textures,
            )?))
        }
        _ => anyhow::bail!(
            "character {} uses unsupported renderType {:?}",
            character.id,
            character.render_type
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn load_sparrow_character_slot(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    character: CharacterDefinition,
    slot: StageCharacterSlot,
    is_player: bool,
    z: i32,
    scene: &mut LoadedScene,
) -> Result<CharacterSprite> {
    let atlas_path = character.atlas.as_ref().with_context(|| {
        format!(
            "character {} uses {:?}; Sparrow renderer needs atlas",
            character.id, character.render_type
        )
    })?;
    let atlas = load_sparrow(resolver, atlas_path)
        .with_context(|| format!("load {}", atlas_path.as_str()))?;
    let texture_path = atlas_texture_path(atlas_path, &atlas)?;
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

    Ok(CharacterSprite::Sparrow(SparrowCharacterSprite {
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
    }))
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

fn count_animation_timings(
    sprite: &CharacterSprite,
    prefix: &str,
) -> [Option<CountAnimationTiming>; COUNT_ANIM_SLOTS] {
    let mut timings = sprite
        .definition()
        .animations
        .iter()
        .filter_map(|animation| {
            let count = animation.name.strip_prefix(prefix)?.parse::<u32>().ok()?;
            let duration = sprite.animation_duration(&animation.name, 48_000)?;
            Some(CountAnimationTiming {
                count,
                duration_seconds: duration.0 as f64 / 48_000.0,
            })
        })
        .collect::<Vec<_>>();
    timings.sort_by_key(|timing| timing.count);
    let mut out = [None; COUNT_ANIM_SLOTS];
    for (slot, timing) in out.iter_mut().zip(timings) {
        *slot = Some(timing);
    }
    out
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
#[path = "scene_assets_tests.rs"]
mod scene_assets_tests;
