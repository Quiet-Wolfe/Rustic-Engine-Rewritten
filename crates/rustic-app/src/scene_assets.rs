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
use crate::countdown_assets::{load_countdown_assets_for_style, CountdownSkin};
use crate::hold_cover_assets::{load_hold_cover_assets_for_style, HoldCoverSkin};
use crate::hud_assets::{load_hud_assets_for_icons, HudSkin};
use crate::note_assets::{load_note_skin_for_style, NoteSkin};
use crate::note_splash_assets::{load_note_splash_assets_for_style, NoteSplashSkin};
use crate::popup_assets::{load_popup_assets_for_style, PopupSkin};
use crate::preview_song::PreviewSelection;
use crate::scripted_stage_objects::scripted_stage_objects;
use crate::sparrow_character_assets::{
    load_packer_character_sprite, load_sparrow_character_sprite, SparrowCharacterSprite,
};
use crate::sserafim_stage::{SserafimMember, SserafimStageState};
use crate::stage_object_assets::{load_stage_object, StagePropSet};
use anyhow::{Context, Result};
use rustic_asset::{
    load_character, load_stage, load_vslice_chart, AssetPath, AssetResolver, AssetVec2,
    CharacterDefinition, CharacterRenderType, OverlayResolver, ParsedSong, StageCharacterSlot,
    StageDefinition,
};
use rustic_core::ids::{AssetId, SongId};
use rustic_core::time::Samples;
use rustic_game::PlayState;
use rustic_render::{DrawCommand, RenderCommandList, Texture};
use std::collections::HashMap;
pub const SAMPLE_RATE: u32 = 48_000;
pub struct LoadedScene {
    pub note_style: String,
    pub bpm: f64,
    pub camera_zoom: f32,
    pub camera_focus: CameraFocusPoints,
    pub commands: RenderCommandList,
    pub(crate) stage_props: StagePropSet,
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
    pub girlfriend: Option<glam::Vec2>,
}

impl Default for CameraFocusPoints {
    fn default() -> Self {
        let center = glam::vec2(1280.0 * 0.5, 720.0 * 0.5);
        Self {
            player: center,
            opponent: center,
            girlfriend: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CharacterSet {
    girlfriend: Option<CharacterSprite>,
    opponent: CharacterSprite,
    player: CharacterSprite,
    sserafim_extras: Vec<SserafimExtraCharacter>,
    opponent_icon_id: String,
    player_icon_id: String,
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

    pub(crate) fn commands_with_sserafim(
        &self,
        state: &SserafimStageState,
        poses: CharacterPoseNames,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        if !state.active() {
            return self.commands(poses, cursor, sample_rate);
        }
        let mut commands = Vec::new();
        for extra in &self.sserafim_extras {
            if let Some(request) = state.pose_for_member(extra.member, poses, cursor) {
                commands.extend(extra.sprite.commands_with_lip_sync(
                    request,
                    cursor,
                    sample_rate,
                    state.member_sings(extra.member),
                ));
            }
        }
        if let Some(girlfriend) = &self.girlfriend {
            if let Some(request) = state.pose_for_member(SserafimMember::Girlfriend, poses, cursor)
            {
                commands.extend(girlfriend.commands(request, cursor, sample_rate));
            }
        }
        if let Some(request) = state.pose_for_member(SserafimMember::Kazuha, poses, cursor) {
            commands.extend(self.opponent.commands_with_lip_sync(
                request,
                cursor,
                sample_rate,
                state.member_sings(SserafimMember::Kazuha),
            ));
        }
        if let Some(request) = state.pose_for_member(SserafimMember::Sakura, poses, cursor) {
            commands.extend(self.player.commands_with_lip_sync(
                request,
                cursor,
                sample_rate,
                state.member_sings(SserafimMember::Sakura),
            ));
        }
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

    pub fn player_game_over_camera(&self, stage_zoom: f32) -> (glam::Vec2, f32) {
        self.player.game_over_camera(stage_zoom)
    }

    pub fn camera_focus_points(&self) -> CameraFocusPoints {
        CameraFocusPoints {
            player: self.player.camera_focus_point(),
            opponent: self.opponent.camera_focus_point(),
            girlfriend: self
                .girlfriend
                .as_ref()
                .map(CharacterSprite::camera_focus_point),
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

    pub fn hud_icon_ids(&self) -> (&str, &str) {
        (&self.player_icon_id, &self.opponent_icon_id)
    }

    pub fn player_icon_id(&self) -> &str {
        &self.player_icon_id
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
enum CharacterSprite {
    Sparrow(SparrowCharacterSprite),
    Animate(AnimateCharacterSprite),
}

#[derive(Debug, Clone)]
struct SserafimExtraCharacter {
    member: SserafimMember,
    sprite: CharacterSprite,
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

    fn commands_with_lip_sync(
        &self,
        request: CharacterPoseRequest,
        cursor: Samples,
        sample_rate: u32,
        lip_sync_active: bool,
    ) -> Vec<DrawCommand> {
        match self {
            Self::Sparrow(sprite) => vec![sprite.command(request, cursor, sample_rate)],
            Self::Animate(sprite) => {
                sprite.commands_with_lip_sync(request, cursor, sample_rate, lip_sync_active)
            }
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
            Self::Sparrow(sprite) => sprite.definition(),
            Self::Animate(sprite) => sprite.definition(),
        }
    }

    fn game_over_camera(&self, stage_zoom: f32) -> (glam::Vec2, f32) {
        let (focus, character, slot) = match self {
            Self::Sparrow(sprite) => (
                sprite.camera_focus_point(),
                sprite.definition(),
                sprite.slot(),
            ),
            Self::Animate(sprite) => (
                sprite.camera_focus_point(),
                sprite.definition(),
                sprite.slot(),
            ),
        };
        let char_offset = glam::vec2(character.camera_offset.x, character.camera_offset.y);
        let stage_offset = glam::vec2(slot.camera_offset.x, slot.camera_offset.y);
        let death_offset = glam::vec2(
            character.death.camera_offset.x,
            character.death.camera_offset.y,
        );
        (
            focus - char_offset - stage_offset + death_offset,
            stage_zoom * character.death.camera_zoom,
        )
    }
}

pub fn load_preview_scene_for(
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
        &chart.note_style,
        chart.bpm,
    )
}

#[allow(clippy::too_many_arguments)]
fn load_scene_for_ids(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    stage_id: &str,
    girlfriend_id: Option<&str>,
    opponent_id: &str,
    player_id: &str,
    note_style: &str,
    bpm: f64,
) -> Result<LoadedScene> {
    let stage_path = AssetPath::new(format!("data/stages/{stage_id}.json"))?;
    let stage = load_stage(resolver, &stage_path).context("load default stage definition")?;

    let mut scene = LoadedScene {
        note_style: note_style.to_owned(),
        bpm,
        camera_zoom: stage.camera_zoom,
        camera_focus: CameraFocusPoints::default(),
        commands: RenderCommandList::new(),
        stage_props: StagePropSet::default(),
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

    for object in scripted_stage_objects(stage_id)? {
        if let Some(prop) = load_stage_object(
            device,
            queue,
            resolver,
            &object,
            &mut scene.textures,
            &mut scene.commands,
        )? {
            scene.stage_props.push(prop);
        }
    }

    for mut object in stage.objects.clone() {
        if let Some(dir) = &stage.directory {
            if let Some(stripped) = object.image.as_str().strip_prefix("images/") {
                if let Ok(candidate) =
                    rustic_asset::AssetPath::new(format!("images/{}/{}", dir, stripped))
                {
                    if resolver.resolve(&candidate).is_ok() {
                        object.image = candidate;
                    }
                }
            }
        }
        if let Some(prop) = load_stage_object(
            device,
            queue,
            resolver,
            &object,
            &mut scene.textures,
            &mut scene.commands,
        )? {
            scene.stage_props.push(prop);
        }
    }
    let characters = load_stage_characters(
        device,
        queue,
        resolver,
        stage_id,
        &stage,
        girlfriend_id,
        opponent_id,
        player_id,
        &mut scene,
    )?;
    let (player_icon_id, opponent_icon_id) = {
        let (player, opponent) = characters.hud_icon_ids();
        (player.to_string(), opponent.to_string())
    };
    scene.camera_focus = characters.camera_focus_points();
    scene.characters = Some(characters);
    scene.bitmap_text_skin = Some(load_bitmap_text_assets(
        device,
        queue,
        resolver,
        &mut scene.textures,
    )?);
    scene.note_skin = Some(load_note_skin_for_style(
        device,
        queue,
        resolver,
        &mut scene.textures,
        note_style,
    )?);
    scene.note_splash_skin = Some(load_note_splash_assets_for_style(
        device,
        queue,
        resolver,
        &mut scene.textures,
        note_style,
    )?);
    scene.hold_cover_skin = Some(load_hold_cover_assets_for_style(
        device,
        queue,
        resolver,
        &mut scene.textures,
        note_style,
    )?);
    scene.hud_skin = Some(load_hud_assets_for_icons(
        device,
        queue,
        resolver,
        &mut scene.textures,
        &player_icon_id,
        &opponent_icon_id,
    )?);
    scene.popup_skin = Some(load_popup_assets_for_style(
        device,
        queue,
        resolver,
        &mut scene.textures,
        note_style,
    )?);
    scene.countdown_skin = Some(load_countdown_assets_for_style(
        device,
        queue,
        resolver,
        &mut scene.textures,
        note_style,
    )?);
    Ok(scene)
}

pub fn load_preview_play_state(sample_rate: u32) -> Result<PlayState> {
    load_preview_play_state_for(PreviewSelection::from_env(), sample_rate)
}

pub(crate) fn load_preview_song_for(selection: PreviewSelection) -> Result<ParsedSong> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let difficulty = selection.difficulty.as_str();
    let chart_path = AssetPath::new(selection.chart_path())?;
    let metadata_path = AssetPath::new(selection.metadata_path())?;
    load_vslice_chart(&resolver, &chart_path, &metadata_path, difficulty)
        .with_context(|| format!("load {} + {} [{}]", chart_path, metadata_path, difficulty))
}

pub fn load_preview_play_state_for(
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
    stage
}

fn character_id(id: &str) -> Option<&str> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[allow(clippy::too_many_arguments)]
fn load_stage_characters(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    stage_id: &str,
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
                character_z(stage.girlfriend, 0),
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
        character_z(stage.opponent, 1),
        scene,
    )?;
    let player = load_character_slot(
        device,
        queue,
        resolver,
        player_id,
        stage.boyfriend,
        true,
        character_z(stage.boyfriend, 2),
        scene,
    )?;
    let opponent_icon_id = character_health_icon_id(&opponent, opponent_id);
    let player_icon_id = character_health_icon_id(&player, player_id);
    let sserafim_extras = if stage_id == "sserafim" {
        load_sserafim_extras(device, queue, resolver, scene)?
    } else {
        Vec::new()
    };
    Ok(CharacterSet {
        girlfriend,
        opponent,
        player,
        sserafim_extras,
        opponent_icon_id,
        player_icon_id,
    })
}

fn load_sserafim_extras(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    scene: &mut LoadedScene,
) -> Result<Vec<SserafimExtraCharacter>> {
    let specs = [
        (
            SserafimMember::Yunjin,
            "sserafim-yunjin",
            glam::vec2(-621.0, 154.0),
        ),
        (
            SserafimMember::Chaewon,
            "sserafim-chaewon",
            glam::vec2(687.0, 98.0),
        ),
        (
            SserafimMember::Eunchae,
            "sserafim-eunchae",
            glam::vec2(770.0, 675.0),
        ),
    ];
    specs
        .into_iter()
        .map(|(member, id, position)| {
            let slot = sserafim_extra_slot(position);
            Ok(SserafimExtraCharacter {
                member,
                sprite: load_character_slot(
                    device,
                    queue,
                    resolver,
                    id,
                    slot,
                    false,
                    character_z(slot, 1000),
                    scene,
                )?,
            })
        })
        .collect()
}

fn sserafim_extra_slot(position: glam::Vec2) -> StageCharacterSlot {
    let mut slot = StageCharacterSlot::default();
    slot.position = AssetVec2::new(position.x, position.y);
    slot.camera_offset = AssetVec2::ZERO;
    slot.z = 1000;
    slot
}

fn character_health_icon_id(sprite: &CharacterSprite, fallback_id: &str) -> String {
    sprite
        .definition()
        .health_icon
        .id
        .as_deref()
        .unwrap_or(fallback_id)
        .to_string()
}

fn character_z(slot: StageCharacterSlot, fallback: i32) -> i32 {
    if slot.z == 0 {
        fallback
    } else {
        slot.z
    }
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
            Ok(CharacterSprite::Sparrow(load_sparrow_character_sprite(
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
        CharacterRenderType::Packer => Ok(CharacterSprite::Sparrow(load_packer_character_sprite(
            device,
            queue,
            resolver,
            character,
            slot,
            is_player,
            z,
            &mut scene.textures,
        )?)),
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

#[cfg(test)]
#[path = "scene_assets_tests.rs"]
mod scene_assets_tests;
