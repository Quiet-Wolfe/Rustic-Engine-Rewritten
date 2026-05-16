//! Stage prop texture loading and render command wiring.
// LINT-ALLOW: long-file stage prop loading, animation dispatch, and tests share fixtures.

use crate::preview_song::PreviewSong;
use crate::sserafim_stage::{
    sserafim_intro_elapsed, sserafim_intro_event_cursor, sserafim_intro_start_cursor,
};
use crate::stage_object_asset_helpers::{
    asset_id_for_path, filter_for_antialiasing, halloween_lightning_start, stage_beat,
    stage_beat_start, stage_frame_index,
};
use crate::stage_scripted_motion::{
    limo_shooting_star_state, philly_blazin_lightning_state, philly_car_pose,
    philly_traffic_light_state, tank_rolling_pose, PhillyCarLane,
};
use crate::stage_static_prop::{scripted_static_stage_object, StaticStagePropSprite};
use anyhow::{Context, Result};
use rustic_asset::{
    load_animate_animation, load_animate_spritemap, load_bytes, load_png, load_sparrow,
    AnimateAnimation, AnimateAtlas, AnimateDrawPart, AssetPath, OverlayResolver, SparrowAtlas,
    SparrowFrame, StageObject, StageObjectRenderType,
};
use rustic_core::ids::AssetId;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub(crate) struct StagePropSet {
    props: Vec<StagePropSprite>,
}

impl StagePropSet {
    pub(crate) fn push(&mut self, prop: StagePropSprite) {
        self.props.push(prop);
    }

    pub(crate) fn commands(
        &self,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        song: Option<PreviewSong>,
    ) -> Vec<DrawCommand> {
        self.props
            .iter()
            .flat_map(|prop| prop.commands(cursor, sample_rate, bpm, song))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum StagePropSprite {
    Static(StaticStagePropSprite),
    Sparrow(SparrowStagePropSprite),
    Animate(AnimateStagePropSprite),
}

impl StagePropSprite {
    fn commands(
        &self,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        song: Option<PreviewSong>,
    ) -> Vec<DrawCommand> {
        match self {
            Self::Static(prop) => prop.commands(cursor, sample_rate, bpm),
            Self::Sparrow(prop) => prop.commands(cursor, sample_rate, bpm, song),
            Self::Animate(prop) => prop.commands(cursor, sample_rate, bpm, song),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SparrowStagePropSprite {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    object: StageObject,
    animations: Vec<LoadedSparrowStageAnimation>,
    starting_animation: usize,
    dance_left: Option<usize>,
    dance_right: Option<usize>,
    filter: FilterMode,
}

#[derive(Debug, Clone)]
struct LoadedSparrowStageAnimation {
    name: String,
    frames: Vec<SparrowFrame>,
    frame_rate: u16,
    looped: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct AnimateStagePropSprite {
    texture_id: AssetId,
    object: StageObject,
    animation: AnimateAnimation,
    atlas: AnimateAtlas,
    label: String,
    source: AnimateStagePoseSource,
    frame_count: usize,
    frame_rate: u16,
    looped: bool,
    filter: FilterMode,
}

#[derive(Debug, Clone, Copy)]
enum AnimateStagePoseSource {
    Root,
    FrameLabel,
    Symbol,
}

pub(crate) fn load_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    object: &StageObject,
    textures: &mut HashMap<AssetId, Texture>,
    commands: &mut RenderCommandList,
) -> Result<Option<StagePropSprite>> {
    if object.solid_color.is_some() {
        Ok(load_solid_stage_object(
            device, queue, object, textures, commands,
        ))
    } else if object.render_type == StageObjectRenderType::AnimateAtlas {
        load_animate_stage_object(device, queue, resolver, object, textures)
    } else if object.render_type == StageObjectRenderType::Packer {
        load_packer_stage_object(device, queue, resolver, object, textures, commands)
    } else if let Some(animation) = &object.animation {
        load_sparrow_stage_object(device, queue, resolver, object, animation, textures)
    } else {
        load_png_stage_object(device, queue, resolver, object, textures, commands)
    }
}

impl SparrowStagePropSprite {
    fn commands(
        &self,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        song: Option<PreviewSong>,
    ) -> Vec<DrawCommand> {
        if self.object.image.as_str() == "images/phillyBlazin/lightning.png"
            && philly_blazin_lightning_state(cursor, sample_rate).is_none()
            || self.object.id == "shootingStar"
                && limo_shooting_star_state(cursor, sample_rate, bpm).is_none()
        {
            return Vec::new();
        }
        let mut cmd = self.command_for_song(cursor, sample_rate, bpm, song);
        self.apply_scripted_motion(&mut cmd, cursor, sample_rate, bpm);
        vec![cmd]
    }

    #[cfg(test)]
    fn command(&self, cursor: Samples, sample_rate: u32, bpm: f64) -> DrawCommand {
        self.command_for_song(cursor, sample_rate, bpm, None)
    }

    fn command_for_song(
        &self,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        song: Option<PreviewSong>,
    ) -> DrawCommand {
        let (animation, started_at) = self.active_animation(cursor, sample_rate, bpm, song);
        let frame = &animation.frames[stage_frame_index(
            cursor,
            sample_rate,
            started_at,
            animation.frame_rate,
            animation.frames.len(),
            animation.looped,
        )];
        let mut cmd = base_stage_command(
            &self.object,
            self.texture_id,
            self.filter,
            stage_frame_pos(&self.object, frame),
            frame_draw_size(frame) * glam::vec2(self.object.scale.x, self.object.scale.y),
        );
        (cmd.uv_min, cmd.uv_max) = frame_uv(frame, self.texture_width, self.texture_height);
        cmd.uv_rotated = frame.rotated;
        cmd
    }

    fn apply_scripted_motion(
        &self,
        cmd: &mut DrawCommand,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        match self.object.id.as_str() {
            "phillyCars" => {
                self.apply_philly_car_motion(cmd, cursor, sample_rate, bpm, PhillyCarLane::Forward)
            }
            "phillyCars2" => {
                self.apply_philly_car_motion(cmd, cursor, sample_rate, bpm, PhillyCarLane::Back)
            }
            "shootingStar" => {
                if let Some(state) = limo_shooting_star_state(cursor, sample_rate, bpm) {
                    let base_pos = glam::vec2(self.object.position.x, self.object.position.y);
                    cmd.world_pos += state.position - base_pos;
                }
            }
            "tankRolling" => {
                let pose = tank_rolling_pose(cursor, sample_rate);
                let base_pos = glam::vec2(self.object.position.x, self.object.position.y);
                cmd.world_pos += pose.position - base_pos;
                cmd.rotation = pose.rotation;
            }
            _ if self.object.image.as_str() == "images/phillyBlazin/lightning.png" => {
                if let Some(state) = philly_blazin_lightning_state(cursor, sample_rate) {
                    let base_pos = glam::vec2(self.object.position.x, self.object.position.y);
                    cmd.world_pos += glam::vec2(state.x, self.object.position.y) - base_pos;
                    cmd.color.w *= state.alpha;
                }
            }
            _ => {}
        }
    }

    fn apply_philly_car_motion(
        &self,
        cmd: &mut DrawCommand,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        lane: PhillyCarLane,
    ) {
        if let Some(pose) = philly_car_pose(cursor, sample_rate, bpm, lane) {
            let base_pos = glam::vec2(self.object.position.x, self.object.position.y);
            cmd.world_pos += pose.position - base_pos;
            cmd.rotation = pose.rotation;
        }
    }

    fn active_animation(
        &self,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        song: Option<PreviewSong>,
    ) -> (&LoadedSparrowStageAnimation, Samples) {
        if self.object.id == "halloweenBG" {
            if let (Some(lightning), Some(started_at)) = (
                animation_index(&self.animations, "lightning"),
                halloween_lightning_start(cursor, sample_rate, bpm),
            ) {
                return (&self.animations[lightning], started_at);
            }
        }
        if self.object.id == "phillyTraffic" {
            if let Some((name, started_at)) = philly_traffic_light_state(cursor, sample_rate, bpm) {
                if let Some(animation) = animation_index(&self.animations, name) {
                    return (&self.animations[animation], started_at);
                }
            }
        }
        let lane = match self.object.id.as_str() {
            "phillyCars" => Some(PhillyCarLane::Forward),
            "phillyCars2" => Some(PhillyCarLane::Back),
            _ => None,
        };
        if let Some((pose, animation)) = lane
            .and_then(|lane| philly_car_pose(cursor, sample_rate, bpm, lane))
            .and_then(|pose| {
                animation_index(&self.animations, pose.animation).map(|index| (pose, index))
            })
        {
            return (&self.animations[animation], pose.started_at);
        }
        if self.object.id == "shootingStar" {
            if let Some(state) = limo_shooting_star_state(cursor, sample_rate, bpm) {
                return (&self.animations[self.starting_animation], state.started_at);
            }
        }
        if self.object.image.as_str() == "images/phillyBlazin/lightning.png" {
            if let Some(state) = philly_blazin_lightning_state(cursor, sample_rate) {
                return (&self.animations[self.starting_animation], state.started_at);
            }
        }
        if self.object.dance_every > 0.0 {
            if let (Some(left), Some(right)) = (self.dance_left, self.dance_right) {
                let scared = self.object.id == "freaks" && song == Some(PreviewSong::ROSES);
                let left = if scared {
                    animation_index(&self.animations, "danceLeft-scared").unwrap_or(left)
                } else {
                    left
                };
                let right = if scared {
                    animation_index(&self.animations, "danceRight-scared").unwrap_or(right)
                } else {
                    right
                };
                let beat = stage_beat(cursor, sample_rate, bpm);
                let interval = self.object.dance_every.max(1.0).round() as i64;
                let dance_index = beat.div_euclid(interval.max(1));
                let animation = if dance_index % 2 == 0 {
                    &self.animations[left]
                } else {
                    &self.animations[right]
                };
                return (animation, stage_beat_start(cursor, sample_rate, bpm));
            }
        }
        (&self.animations[self.starting_animation], Samples(0))
    }
}

impl AnimateStagePropSprite {
    fn commands(
        &self,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        song: Option<PreviewSong>,
    ) -> Vec<DrawCommand> {
        let Some(started_at) =
            animate_stage_started_at(&self.object.id, cursor, sample_rate, bpm, song)
        else {
            return Vec::new();
        };
        let frame_index = stage_frame_index(
            cursor,
            sample_rate,
            started_at,
            self.frame_rate,
            self.frame_count,
            self.looped,
        );
        let frame_offset = self
            .object
            .animation
            .as_ref()
            .and_then(|animation| animation.indices.get(frame_index))
            .map(|index| u32::from(*index))
            .unwrap_or(frame_index as u32);
        let parts = match self.source {
            AnimateStagePoseSource::Root => self.animation.flatten_root_frame(frame_offset),
            AnimateStagePoseSource::FrameLabel => self
                .animation
                .flatten_label_frame(&self.label, frame_offset),
            AnimateStagePoseSource::Symbol => self
                .animation
                .flatten_symbol_frame(&self.label, frame_offset),
        };
        parts
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|part| self.command_for_part(part))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn command_for_part(&self, part: &AnimateDrawPart) -> Option<DrawCommand> {
        let frame = self.atlas.frame(&part.frame_name)?;
        let mut cmd = base_stage_command(
            &self.object,
            self.texture_id,
            self.filter,
            glam::vec2(self.object.position.x, self.object.position.y),
            frame.size,
        );
        cmd.affine = scaled_affine(part.matrix, self.object.scale.x);
        cmd.uv_min = frame.uv_min;
        cmd.uv_max = frame.uv_max;
        cmd.uv_rotated = frame.rotated;
        cmd.color = glam::Vec4::from_array(part.color);
        cmd.color.w *= self.object.alpha;
        cmd.color_offset = glam::Vec4::from_array(part.color_offset);
        Some(cmd)
    }
}

fn load_solid_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    object: &StageObject,
    textures: &mut HashMap<AssetId, Texture>,
    commands: &mut RenderCommandList,
) -> Option<StagePropSprite> {
    let texture_id = asset_id_for_path(&object.image);
    let filter = filter_for_antialiasing(object.antialiasing);
    let color = object.solid_color.unwrap_or([0, 0, 0, 255]);
    let texture = Texture::from_rgba8(
        device,
        queue,
        &color,
        1,
        1,
        filter,
        Some(object.image.as_str()),
    );
    textures.insert(texture_id, texture);
    let sprite = StaticStagePropSprite::new(
        texture_id,
        object.clone(),
        glam::vec2(object.scale.x, object.scale.y),
        filter,
    );
    if scripted_static_stage_object(object) {
        Some(StagePropSprite::Static(sprite))
    } else {
        commands.push(sprite.command());
        None
    }
}

fn load_png_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    object: &StageObject,
    textures: &mut HashMap<AssetId, Texture>,
    commands: &mut RenderCommandList,
) -> Result<Option<StagePropSprite>> {
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
    textures.insert(texture_id, texture);
    let sprite = StaticStagePropSprite::new(texture_id, object.clone(), size, filter);
    if scripted_static_stage_object(object) {
        Ok(Some(StagePropSprite::Static(sprite)))
    } else {
        commands.push(sprite.command());
        Ok(None)
    }
}

fn load_sparrow_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    object: &StageObject,
    animation: &rustic_asset::StageObjectAnimation,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<Option<StagePropSprite>> {
    let atlas_path = stage_object_atlas_path(&object.image)?;
    let atlas = load_sparrow(resolver, &atlas_path)
        .with_context(|| format!("load {}", atlas_path.as_str()))?;
    let animations = load_sparrow_stage_animations(object, animation, &atlas)?;
    let texture_path = atlas_texture_path(&atlas_path, &atlas)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);
    let filter = filter_for_antialiasing(object.antialiasing);
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(texture_path.as_str()));
    textures.insert(texture_id, texture);
    Ok(Some(StagePropSprite::Sparrow(SparrowStagePropSprite {
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
        object: object.clone(),
        starting_animation: starting_animation_index(&animations, &animation.name),
        dance_left: animation_index(&animations, "danceLeft"),
        dance_right: animation_index(&animations, "danceRight"),
        animations,
        filter,
    })))
}

fn load_packer_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    object: &StageObject,
    textures: &mut HashMap<AssetId, Texture>,
    commands: &mut RenderCommandList,
) -> Result<Option<StagePropSprite>> {
    let Some(animation) = &object.animation else {
        load_png_stage_object(device, queue, resolver, object, textures, commands)?;
        return Ok(None);
    };
    let image = load_png(resolver, &object.image)
        .with_context(|| format!("load {}", object.image.as_str()))?;
    let texture_id = asset_id_for_path(&object.image);
    let filter = filter_for_antialiasing(object.antialiasing);
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(object.image.as_str()));
    textures.insert(texture_id, texture);

    let packer_path = stage_object_packer_path(&object.image)?;
    let bytes = load_bytes(resolver, &packer_path)
        .with_context(|| format!("load {}", packer_path.as_str()))?;
    let frames = packer_animation_frames(&bytes, animation)
        .with_context(|| format!("parse {}", packer_path.as_str()))?;
    if frames.is_empty() {
        anyhow::bail!(
            "resolve packer stage prop frame {}:{}",
            object.id,
            animation.name
        );
    }
    Ok(Some(StagePropSprite::Sparrow(SparrowStagePropSprite {
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
        object: object.clone(),
        animations: vec![LoadedSparrowStageAnimation {
            name: animation.name.clone(),
            frames,
            frame_rate: animation.frame_rate,
            looped: animation.looped,
        }],
        starting_animation: 0,
        dance_left: None,
        dance_right: None,
        filter,
    })))
}

fn load_animate_stage_object(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    object: &StageObject,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<Option<StagePropSprite>> {
    let animation_def = object
        .animation
        .as_ref()
        .with_context(|| format!("stage animate prop {} has no starting animation", object.id))?;
    let animation_path = animate_asset_file(&object.image, "Animation.json")?;
    let spritemap_path = animate_asset_file(&object.image, "spritemap1.json")?;
    let texture_path = animate_asset_file(&object.image, "spritemap1.png")?;
    let animation = load_animate_animation(resolver, &animation_path)
        .with_context(|| format!("load {}", animation_path.as_str()))?;
    let source = animate_stage_pose_source(&animation, animation_def);
    let frame_count = animate_stage_frame_count(&animation, animation_def, source)?;
    let atlas = load_animate_spritemap(resolver, &spritemap_path)
        .with_context(|| format!("load {}", spritemap_path.as_str()))?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);
    let filter = filter_for_antialiasing(object.antialiasing);
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(texture_path.as_str()));
    textures.insert(texture_id, texture);

    let sprite = AnimateStagePropSprite {
        texture_id,
        object: object.clone(),
        animation,
        atlas,
        label: animation_def.prefix.clone(),
        source,
        frame_count,
        frame_rate: animation_def.frame_rate,
        looped: animation_def.looped,
        filter,
    };
    if sprite.commands(Samples(0), 48_000, 100.0, None).is_empty()
        && !is_sserafim_cutscene_animate(&sprite.object.id)
    {
        anyhow::bail!(
            "resolve animate stage prop frame {}:{}",
            object.id,
            animation_def.prefix
        );
    }
    Ok(Some(StagePropSprite::Animate(sprite)))
}

fn animate_stage_started_at(
    id: &str,
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
    song: Option<PreviewSong>,
) -> Option<Samples> {
    if !is_sserafim_cutscene_animate(id) {
        return Some(Samples(0));
    }
    if song != Some(PreviewSong::SPAGHETTI) {
        return None;
    }
    sserafim_cutscene_animate_started_at(id, cursor, sample_rate, bpm)
}

fn is_sserafim_cutscene_animate(id: &str) -> bool {
    matches!(
        id,
        "sserafimCutsceneMain" | "sserafimBfGetUp" | "sserafimGfGetUp"
    )
}

fn sserafim_cutscene_animate_started_at(
    id: &str,
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
) -> Option<Samples> {
    match id {
        "sserafimCutsceneMain" => {
            let elapsed = sserafim_intro_elapsed(cursor, sample_rate, bpm)?;
            (elapsed < frame_samples_at_rate(563.0, sample_rate))
                .then_some(sserafim_intro_start_cursor(sample_rate, bpm))
        }
        "sserafimBfGetUp" | "sserafimGfGetUp" => {
            let started_at = sserafim_intro_event_cursor(710.0, sample_rate, bpm);
            (cursor >= started_at).then_some(started_at)
        }
        _ => None,
    }
}

fn frame_samples_at_rate(frame: f32, sample_rate: u32) -> i64 {
    ((frame / 24.0).max(0.0) * sample_rate.max(1) as f32).round() as i64
}

fn base_stage_command(
    object: &StageObject,
    texture_id: AssetId,
    filter: FilterMode,
    world_pos: glam::Vec2,
    size: glam::Vec2,
) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(texture_id, world_pos, size);
    cmd.pivot = glam::Vec2::ZERO;
    cmd.layer = object.layer;
    cmd.z = object.z;
    cmd.filter = filter;
    cmd.scroll_factor = glam::vec2(object.scroll_factor.x, object.scroll_factor.y);
    cmd.color.w = object.alpha;
    cmd
}
fn stage_object_atlas_path(image: &AssetPath) -> Result<AssetPath> {
    let base = image
        .as_str()
        .strip_suffix(".png")
        .with_context(|| format!("stage prop image is not a png: {}", image.as_str()))?;
    Ok(AssetPath::new(format!("{base}.xml"))?)
}
fn stage_object_packer_path(image: &AssetPath) -> Result<AssetPath> {
    let base = image
        .as_str()
        .strip_suffix(".png")
        .with_context(|| format!("stage prop image is not a png: {}", image.as_str()))?;
    Ok(AssetPath::new(format!("{base}.txt"))?)
}
fn atlas_texture_path(atlas_path: &AssetPath, atlas: &SparrowAtlas) -> Result<AssetPath> {
    if atlas.image_path.contains('/') {
        Ok(AssetPath::new(atlas.image_path.clone())?)
    } else {
        Ok(atlas_path.sibling(&atlas.image_path)?)
    }
}
fn animate_asset_file(asset_path: &AssetPath, file_name: &str) -> Result<AssetPath> {
    Ok(AssetPath::new(format!(
        "{}/{file_name}",
        asset_path.as_str()
    ))?)
}

fn packer_animation_frames(
    bytes: &[u8],
    animation: &rustic_asset::StageObjectAnimation,
) -> Result<Vec<SparrowFrame>> {
    let text = std::str::from_utf8(bytes).context("packer atlas is not utf-8")?;
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

    let selected = frames
        .into_iter()
        .filter(|frame| animation.prefix.is_empty() || frame.name.starts_with(&animation.prefix))
        .collect::<Vec<_>>();
    if animation.indices.is_empty() {
        return Ok(selected);
    }
    Ok(animation
        .indices
        .iter()
        .filter_map(|index| selected.get(usize::from(*index)).cloned())
        .collect())
}
fn animate_stage_frame_count(
    animation: &AnimateAnimation,
    animation_def: &rustic_asset::StageObjectAnimation,
    source: AnimateStagePoseSource,
) -> Result<usize> {
    if !animation_def.indices.is_empty() {
        return Ok(animation_def.indices.len());
    }
    match source {
        AnimateStagePoseSource::Root => Ok(animation.duration().max(1) as usize),
        AnimateStagePoseSource::FrameLabel => {
            let label = animation
                .label(&animation_def.prefix)
                .with_context(|| format!("resolve animate label {}", animation_def.prefix))?;
            Ok(label.duration.max(1) as usize)
        }
        AnimateStagePoseSource::Symbol => {
            let symbol = animation
                .symbol(&animation_def.prefix)
                .with_context(|| format!("resolve animate symbol {}", animation_def.prefix))?;
            Ok(symbol.duration().max(1) as usize)
        }
    }
}
fn animate_stage_pose_source(
    animate: &AnimateAnimation,
    animation_def: &rustic_asset::StageObjectAnimation,
) -> AnimateStagePoseSource {
    match animation_def.anim_type.as_deref() {
        Some("symbol") if animate.symbol(&animation_def.prefix).is_some() => {
            AnimateStagePoseSource::Symbol
        }
        Some("symbol") => AnimateStagePoseSource::Root,
        _ => AnimateStagePoseSource::FrameLabel,
    }
}
fn scaled_affine(matrix: [f32; 6], scale: f32) -> [f32; 6] {
    [
        matrix[0] * scale,
        matrix[1] * scale,
        matrix[2] * scale,
        matrix[3] * scale,
        matrix[4],
        matrix[5],
    ]
}
fn stage_frame_pos(object: &StageObject, frame: &SparrowFrame) -> glam::Vec2 {
    glam::vec2(object.position.x, object.position.y)
        - glam::vec2(frame.frame_x as f32, frame.frame_y as f32)
            * glam::vec2(object.scale.x, object.scale.y)
}

fn load_sparrow_stage_animations(
    object: &StageObject,
    starting_animation: &rustic_asset::StageObjectAnimation,
    atlas: &SparrowAtlas,
) -> Result<Vec<LoadedSparrowStageAnimation>> {
    let animation_defs = if object.animations.is_empty() {
        vec![starting_animation.clone()]
    } else {
        object.animations.clone()
    };
    animation_defs
        .into_iter()
        .map(|animation| {
            let frames: Vec<_> = atlas
                .animation_frames(&animation.prefix, &animation.indices)
                .into_iter()
                .cloned()
                .collect();
            if frames.is_empty() {
                anyhow::bail!(
                    "resolve stage prop frame {}:{}",
                    object.id,
                    animation.prefix
                );
            }
            Ok(LoadedSparrowStageAnimation {
                name: animation.name,
                frames,
                frame_rate: animation.frame_rate,
                looped: animation.looped,
            })
        })
        .collect()
}
fn starting_animation_index(
    animations: &[LoadedSparrowStageAnimation],
    starting_name: &str,
) -> usize {
    animation_index(animations, starting_name).unwrap_or(0)
}
fn animation_index(animations: &[LoadedSparrowStageAnimation], name: &str) -> Option<usize> {
    animations
        .iter()
        .position(|animation| animation.name == name)
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

#[cfg(test)]
#[path = "stage_object_assets_tests.rs"]
mod tests;
