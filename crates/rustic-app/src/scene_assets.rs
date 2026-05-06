//! Startup scene asset wiring.
//!
//! This module is intentionally app-owned: it resolves vanilla assets,
//! uploads textures, and emits render commands, but gameplay and render
//! crates remain free of filesystem and wgpu wiring.
// LINT-ALLOW: long-file startup scene plus current NOTE_assets skin wiring
use crate::hud_assets::{load_hud_assets, HudSkin};
use crate::popup_assets::{load_popup_assets, PopupSkin};
use anyhow::{Context, Result};
use rustic_asset::{
    load_character, load_chart, load_png, load_sparrow, load_stage, AssetPath, CharacterAnimation,
    CharacterDefinition, OverlayResolver, SparrowAtlas, SparrowFrame, StageCharacterSlot,
    StageDefinition, StageObject,
};
use rustic_core::ids::{AssetId, CameraId, SongId};
use rustic_core::render::RenderLayer;
use rustic_game::{Lane, NoteView, PlayState};
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;
pub const SAMPLE_RATE: u32 = 48_000;
const NOTE_SWAG_WIDTH: f32 = 160.0 * 0.7;
const STRUM_LINE_Y: f32 = 50.0;
const FNF_WIDTH: f32 = 1280.0;
const NOTE_ASSET_SCALE: f32 = 0.7;
const LANES: [Lane; 4] = [Lane::Left, Lane::Down, Lane::Up, Lane::Right];
pub struct LoadedScene {
    pub camera_zoom: f32,
    pub commands: RenderCommandList,
    pub textures: HashMap<AssetId, Texture>,
    pub note_skin: Option<NoteSkin>,
    pub hud_skin: Option<HudSkin>,
    pub popup_skin: Option<PopupSkin>,
}
#[derive(Debug, Clone)]
pub struct NoteSkin {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    static_frames: [SparrowFrame; 4],
    press_frames: [SparrowFrame; 4],
    confirm_frames: [SparrowFrame; 4],
    tap_frames: [SparrowFrame; 4],
    hold_frames: [SparrowFrame; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceptorState {
    Static,
    Pressed,
    Confirm,
}

impl NoteSkin {
    pub fn command_for_view(&self, view: &NoteView) -> DrawCommand {
        let frame = self.frame_for_view(view);
        let size = glam::vec2(
            frame.width as f32 * NOTE_ASSET_SCALE,
            frame.height as f32 * NOTE_ASSET_SCALE,
        );
        let x = if view.is_sustain {
            view.x + (NOTE_SWAG_WIDTH - size.x) * 0.5
        } else {
            view.x
        };

        let mut cmd = DrawCommand::sprite(self.texture_id, glam::vec2(x, view.y), size);
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Notes;
        cmd.z = if view.is_sustain { 0 } else { 1 };
        cmd.filter = FilterMode::Linear;
        (cmd.uv_min, cmd.uv_max) = frame_uv(frame, self.texture_width, self.texture_height);
        if view.is_sustain {
            cmd.color.w = 0.6;
        }
        cmd
    }

    fn frame_for_view(&self, view: &NoteView) -> &SparrowFrame {
        let index = lane_index(view.lane);
        if view.is_sustain {
            &self.hold_frames[index]
        } else {
            &self.tap_frames[index]
        }
    }

    pub fn receptor_commands<F>(&self, lane_state: F) -> Vec<DrawCommand>
    where
        F: Fn(Lane) -> ReceptorState,
    {
        let mut commands = Vec::with_capacity(8);
        for player in 0..=1 {
            for lane in LANES {
                let state = if player == 1 {
                    lane_state(lane)
                } else {
                    ReceptorState::Static
                };
                commands.push(self.receptor_command(player, lane, state));
            }
        }
        commands
    }

    fn receptor_command(&self, player: u8, lane: Lane, state: ReceptorState) -> DrawCommand {
        let frame = self.receptor_frame(lane, state);
        let lane = lane_index(lane);
        let base_x = 50.0 + lane as f32 * NOTE_SWAG_WIDTH + player as f32 * (FNF_WIDTH / 2.0);
        let base_y = STRUM_LINE_Y;
        let mut cmd = DrawCommand::sprite(
            self.texture_id,
            glam::vec2(
                base_x - frame.frame_x as f32 * NOTE_ASSET_SCALE,
                base_y - frame.frame_y as f32 * NOTE_ASSET_SCALE,
            ),
            glam::vec2(
                frame.width as f32 * NOTE_ASSET_SCALE,
                frame.height as f32 * NOTE_ASSET_SCALE,
            ),
        );
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Notes;
        cmd.filter = FilterMode::Linear;
        (cmd.uv_min, cmd.uv_max) = frame_uv(frame, self.texture_width, self.texture_height);
        cmd
    }

    fn receptor_frame(&self, lane: Lane, state: ReceptorState) -> &SparrowFrame {
        let index = lane_index(lane);
        match state {
            ReceptorState::Static => &self.static_frames[index],
            ReceptorState::Pressed => &self.press_frames[index],
            ReceptorState::Confirm => &self.confirm_frames[index],
        }
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
        note_skin: None,
        hud_skin: None,
        popup_skin: None,
    };

    for object in &stage.objects {
        load_stage_object(device, queue, &resolver, object, &mut scene)?;
    }
    load_stage_characters(device, queue, &resolver, &stage, &mut scene)?;
    scene.note_skin = Some(load_note_skin(device, queue, &resolver, &mut scene)?);
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
    Ok(scene)
}

pub fn load_preview_play_state() -> Result<PlayState> {
    let resolver = OverlayResolver::new().with_baked_root("assets/baked");
    let chart_path = AssetPath::new("data/bopeebo/bopeebo.json")?;
    let chart =
        load_chart(&resolver, &chart_path).with_context(|| format!("load {}", chart_path))?;
    Ok(PlayState::from_chart(SongId::new(0), &chart, SAMPLE_RATE))
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
) -> Result<()> {
    load_character_slot(
        device,
        queue,
        resolver,
        "data/characters/gf.json",
        stage.girlfriend,
        false,
        0,
        scene,
    )?;
    load_character_slot(
        device,
        queue,
        resolver,
        "data/characters/dad.json",
        stage.opponent,
        false,
        1,
        scene,
    )?;
    load_character_slot(
        device,
        queue,
        resolver,
        "data/characters/bf.json",
        stage.boyfriend,
        true,
        2,
        scene,
    )
}

fn load_note_skin(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    scene: &mut LoadedScene,
) -> Result<NoteSkin> {
    let atlas_path = AssetPath::new("images/NOTE_assets.xml")?;
    let atlas = load_sparrow(resolver, &atlas_path)
        .with_context(|| format!("load {}", atlas_path.as_str()))?;
    let texture_path = atlas_texture_path(&atlas_path, &atlas)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);
    let texture = Texture::from_png_image(
        device,
        queue,
        &image,
        FilterMode::Linear,
        Some(texture_path.as_str()),
    );
    scene.textures.insert(texture_id, texture);

    let note_skin = NoteSkin {
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
        static_frames: [
            cloned_first_frame(&atlas, "arrowLEFT")?,
            cloned_first_frame(&atlas, "arrowDOWN")?,
            cloned_first_frame(&atlas, "arrowUP")?,
            cloned_first_frame(&atlas, "arrowRIGHT")?,
        ],
        press_frames: [
            cloned_first_frame(&atlas, "left press")?,
            cloned_first_frame(&atlas, "down press")?,
            cloned_first_frame(&atlas, "up press")?,
            cloned_first_frame(&atlas, "right press")?,
        ],
        confirm_frames: [
            cloned_first_frame(&atlas, "left confirm")?,
            cloned_first_frame(&atlas, "down confirm")?,
            cloned_first_frame(&atlas, "up confirm")?,
            cloned_first_frame(&atlas, "right confirm")?,
        ],
        tap_frames: [
            cloned_first_frame(&atlas, "purple0")?,
            cloned_first_frame(&atlas, "blue0")?,
            cloned_first_frame(&atlas, "green0")?,
            cloned_first_frame(&atlas, "red0")?,
        ],
        hold_frames: [
            cloned_first_frame(&atlas, "purple hold piece")?,
            cloned_first_frame(&atlas, "blue hold piece")?,
            cloned_first_frame(&atlas, "green hold piece")?,
            cloned_first_frame(&atlas, "red hold piece")?,
        ],
    };

    Ok(note_skin)
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
) -> Result<()> {
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
    let animation = initial_animation(&character)?;
    let frame = atlas
        .first_animation_frame(&animation.prefix, &animation.indices)
        .with_context(|| format!("resolve initial frame {}:{}", character.id, animation.name))?;

    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(texture_path.as_str()));
    scene.textures.insert(texture_id, texture);

    let mut cmd = DrawCommand::sprite(
        texture_id,
        character_frame_pos(&character, animation, frame, slot),
        glam::vec2(
            frame.width as f32 * character.scale,
            frame.height as f32 * character.scale,
        ),
    );
    cmd.pivot = glam::Vec2::ZERO;
    cmd.layer = RenderLayer::Characters;
    cmd.z = z;
    cmd.filter = filter;
    (cmd.uv_min, cmd.uv_max) = frame_uv(frame, image.width, image.height);
    if effective_flip_x(&character, is_player) {
        std::mem::swap(&mut cmd.uv_min.x, &mut cmd.uv_max.x);
    }
    scene.commands.push(cmd);
    Ok(())
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

fn cloned_first_frame(atlas: &SparrowAtlas, prefix: &str) -> Result<SparrowFrame> {
    atlas
        .first_animation_frame(prefix, &[])
        .cloned()
        .with_context(|| format!("resolve note frame {prefix}"))
}

fn lane_index(lane: Lane) -> usize {
    match lane {
        Lane::Left => 0,
        Lane::Down => 1,
        Lane::Up => 2,
        Lane::Right => 3,
        _ => 3,
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
