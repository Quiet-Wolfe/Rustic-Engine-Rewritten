//! Startup scene asset wiring.
//!
//! This module is intentionally app-owned: it resolves vanilla assets,
//! uploads textures, and emits render commands, but gameplay and render
//! crates remain free of filesystem and wgpu wiring.

use anyhow::{Context, Result};
use rustic_asset::{
    load_character, load_png, load_sparrow, load_stage, AssetPath, CharacterAnimation,
    CharacterDefinition, OverlayResolver, SparrowAtlas, SparrowFrame, StageCharacterSlot,
    StageDefinition, StageObject,
};
use rustic_core::ids::AssetId;
use rustic_core::render::RenderLayer;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;

pub struct LoadedScene {
    pub camera_zoom: f32,
    pub commands: RenderCommandList,
    pub textures: HashMap<AssetId, Texture>,
}

pub fn load_default_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<LoadedScene> {
    let resolver = OverlayResolver::new().with_baked_root("assets/baked");
    let stage_path = AssetPath::new("data/stages/stage.json")?;
    let stage = load_stage(&resolver, &stage_path).context("load default stage definition")?;

    let mut scene = LoadedScene {
        camera_zoom: stage.camera_zoom,
        commands: RenderCommandList::new(),
        textures: HashMap::new(),
    };

    for object in &stage.objects {
        load_stage_object(device, queue, &resolver, object, &mut scene)?;
    }
    load_stage_characters(device, queue, &resolver, &stage, &mut scene)?;
    Ok(scene)
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
