//! Funkin' v0.8.5 note, receptor, and hold-trail rendering.
// LINT-ALLOW: long-file note, receptor, hold-trail rendering, and tests stay together.
use crate::animation_timing::flixel_frame_index;
use crate::lane_state::ReceptorState;
use anyhow::{Context, Result};
use rustic_asset::{
    load_png, load_sparrow, AssetPath, OverlayResolver, SparrowAtlas, SparrowFrame,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_game::{HoldTrailView, Lane, NoteView};
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const FNF_WIDTH: f32 = 1280.0;
const STRUMLINE_X_OFFSET: f32 = 48.0;
const STRUMLINE_Y_OFFSET: f32 = 24.0;
const STRUMLINE_SIZE: f32 = 104.0;
const NOTE_SPACING: f32 = STRUMLINE_SIZE + 8.0;
const NOTE_NUDGE: f32 = 2.0;
const NOTE_ASSET_SCALE: f32 = 0.7;
const HOLD_TRAIL_SEGMENTS: f32 = 8.0;
const HOLD_TRAIL_END_OFFSET: f32 = 0.5;
const HOLD_TRAIL_BOTTOM_CLIP: f32 = 0.9;
const RECEPTOR_ANIMATION_FPS: u16 = 24;
const CONFIRM_HOLD_TIME_SECS: f64 = 0.15;
const LANES: [Lane; 4] = [Lane::Left, Lane::Down, Lane::Up, Lane::Right];

#[derive(Debug, Clone)]
pub struct NoteSkin {
    scale: f32,
    filter: FilterMode,
    strumline_offset: glam::Vec2,
    tap_texture_id: AssetId,
    tap_texture_width: u32,
    tap_texture_height: u32,
    strumline_texture_id: AssetId,
    strumline_texture_width: u32,
    strumline_texture_height: u32,
    hold_texture_id: AssetId,
    hold_texture_width: u32,
    hold_texture_height: u32,
    hold_trail_texture_id: AssetId,
    hold_trail_texture_width: u32,
    hold_trail_texture_height: u32,
    static_frames: [SparrowFrame; 4],
    press_frames: [Vec<SparrowFrame>; 4],
    confirm_frames: [Vec<SparrowFrame>; 4],
    confirm_hold_frames: [Vec<SparrowFrame>; 4],
    tap_frames: [SparrowFrame; 4],
    hold_frames: [SparrowFrame; 4],
    hold_end_frames: [SparrowFrame; 4],
}

impl NoteSkin {
    pub fn command_for_view(&self, view: &NoteView) -> DrawCommand {
        let (frame, texture_id, texture_width, texture_height) = self.frame_for_view(view);
        let size = frame_draw_size(frame) * self.scale;
        let source_size = frame_source_size(frame) * self.scale;
        let trimmed = frame_trim_offset(frame) * self.scale;
        let x = if view.is_sustain {
            view.x + self.strumline_offset.x + STRUMLINE_SIZE * 0.5
                - source_size.x * 0.5
                - trimmed.x
        } else {
            note_sprite_x(view.x + self.strumline_offset.x, source_size.x) - trimmed.x
        };
        let y = view.y + self.strumline_offset.y - trimmed.y;

        let mut cmd = DrawCommand::sprite(texture_id, glam::vec2(x, y), size);
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Notes;
        cmd.z = if view.is_sustain { 0 } else { 1 };
        cmd.filter = self.filter;
        (cmd.uv_min, cmd.uv_max) = frame_uv(frame, texture_width, texture_height);
        cmd.uv_rotated = frame.rotated;
        if view.is_sustain {
            cmd.color.w = 0.6;
        }
        cmd
    }

    pub fn hold_trail_commands(&self, view: &HoldTrailView) -> Vec<DrawCommand> {
        // ref: bdedc0aa:source/funkin/play/notes/SustainTrail.hx:290-393
        let segment_width = self.hold_trail_texture_width as f32 / HOLD_TRAIL_SEGMENTS;
        let scaled_width = segment_width * self.scale;
        let scaled_texture_height = self.hold_trail_texture_height as f32 * self.scale;
        let bottom_height = scaled_texture_height * HOLD_TRAIL_END_OFFSET;
        let part_height = view.height - bottom_height;
        let tail_height = part_height.max(0.0);
        let extra_cap = scaled_texture_height * (HOLD_TRAIL_BOTTOM_CLIP - HOLD_TRAIL_END_OFFSET);
        let cap_height = (view.height + extra_cap)
            .min(scaled_texture_height * HOLD_TRAIL_BOTTOM_CLIP)
            .max(0.0);
        let cap_uv_top = if tail_height > 0.1 {
            0.0
        } else {
            ((bottom_height - view.height) / scaled_texture_height)
                .clamp(0.0, HOLD_TRAIL_BOTTOM_CLIP)
        };
        let x = view.x + self.strumline_offset.x + STRUMLINE_SIZE * 0.5 - scaled_width * 0.5;
        let lane_u = lane_index(view.lane) as f32 * 0.25;
        let u_width = 1.0 / HOLD_TRAIL_SEGMENTS;
        let mut commands = Vec::new();

        if tail_height > 0.1 {
            let mut cmd = self.hold_trail_rect(
                glam::vec2(x, view.y + self.strumline_offset.y),
                glam::vec2(scaled_width, tail_height),
                glam::vec2(lane_u, -part_height / scaled_texture_height),
                glam::vec2(lane_u + u_width, 0.0),
            );
            cmd.uv_wrap_y = true;
            cmd.z = 0;
            commands.push(cmd);
        }

        if cap_height > 0.1 {
            let mut cmd = self.hold_trail_rect(
                glam::vec2(x, view.y + self.strumline_offset.y + tail_height),
                glam::vec2(scaled_width, cap_height),
                glam::vec2(lane_u + u_width, cap_uv_top),
                glam::vec2(lane_u + u_width * 2.0, HOLD_TRAIL_BOTTOM_CLIP),
            );
            cmd.z = 0;
            commands.push(cmd);
        }

        commands
    }

    fn hold_trail_rect(
        &self,
        world_pos: glam::Vec2,
        size: glam::Vec2,
        uv_min: glam::Vec2,
        uv_max: glam::Vec2,
    ) -> DrawCommand {
        let mut cmd = DrawCommand::sprite(self.hold_trail_texture_id, world_pos, size);
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Notes;
        cmd.filter = self.filter;
        cmd.uv_min = uv_min;
        cmd.uv_max = uv_max;
        cmd
    }

    fn frame_for_view(&self, view: &NoteView) -> (&SparrowFrame, AssetId, u32, u32) {
        let index = lane_index(view.lane);
        if view.is_sustain {
            let frame = if view.is_sustain_end {
                &self.hold_end_frames[index]
            } else {
                &self.hold_frames[index]
            };
            (
                frame,
                self.hold_texture_id,
                self.hold_texture_width,
                self.hold_texture_height,
            )
        } else {
            (
                &self.tap_frames[index],
                self.tap_texture_id,
                self.tap_texture_width,
                self.tap_texture_height,
            )
        }
    }

    pub fn receptor_commands<F>(
        &self,
        cursor: Samples,
        sample_rate: u32,
        lane_state: F,
    ) -> Vec<DrawCommand>
    where
        F: Fn(u8, Lane) -> ReceptorState,
    {
        self.receptor_commands_with_layout(cursor, sample_rate, true, 0.0, lane_state)
    }

    pub fn receptor_commands_with_layout<F>(
        &self,
        cursor: Samples,
        sample_rate: u32,
        show_opponent: bool,
        player_x_offset: f32,
        lane_state: F,
    ) -> Vec<DrawCommand>
    where
        F: Fn(u8, Lane) -> ReceptorState,
    {
        let mut commands = Vec::with_capacity(8);
        for player in 0..=1 {
            if player == 0 && !show_opponent {
                continue;
            }
            for lane in LANES {
                let state = lane_state(player, lane);
                let mut command = self.receptor_command(player, lane, state, cursor, sample_rate);
                if player == 1 {
                    command.world_pos.x += player_x_offset;
                }
                commands.push(command);
            }
        }
        commands
    }

    fn receptor_command(
        &self,
        player: u8,
        lane: Lane,
        state: ReceptorState,
        cursor: Samples,
        sample_rate: u32,
    ) -> DrawCommand {
        let frame = self.receptor_frame(lane, state, cursor, sample_rate);
        let size = frame_draw_size(frame) * self.scale;
        let mut cmd = DrawCommand::sprite(
            self.strumline_texture_id,
            receptor_sprite_pos(player, lane, frame, self.scale, self.strumline_offset),
            size,
        );
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Notes;
        cmd.filter = self.filter;
        (cmd.uv_min, cmd.uv_max) = frame_uv(
            frame,
            self.strumline_texture_width,
            self.strumline_texture_height,
        );
        cmd.uv_rotated = frame.rotated;
        cmd
    }

    fn receptor_frame(
        &self,
        lane: Lane,
        state: ReceptorState,
        cursor: Samples,
        sample_rate: u32,
    ) -> &SparrowFrame {
        let index = lane_index(lane);
        match state {
            ReceptorState::Static => &self.static_frames[index],
            ReceptorState::Pressed { started_at } => animated_note_frame(
                &self.press_frames[index],
                cursor,
                sample_rate,
                started_at,
                RECEPTOR_ANIMATION_FPS,
                false,
            ),
            ReceptorState::Confirm { started_at, hold } => {
                if hold
                    && cursor.0.saturating_sub(started_at.0)
                        >= self.confirm_animation_duration(sample_rate).0
                {
                    animated_note_frame(
                        &self.confirm_hold_frames[index],
                        cursor,
                        sample_rate,
                        Samples(started_at.0 + self.confirm_animation_duration(sample_rate).0),
                        RECEPTOR_ANIMATION_FPS,
                        true,
                    )
                } else {
                    animated_note_frame(
                        &self.confirm_frames[index],
                        cursor,
                        sample_rate,
                        started_at,
                        RECEPTOR_ANIMATION_FPS,
                        false,
                    )
                }
            }
        }
    }

    pub fn confirm_duration(&self, sample_rate: u32) -> Samples {
        let animation = self.confirm_animation_duration(sample_rate);
        Samples(animation.0 + (f64::from(sample_rate) * CONFIRM_HOLD_TIME_SECS).round() as i64)
    }

    fn confirm_animation_duration(&self, sample_rate: u32) -> Samples {
        let frame_count = self.confirm_frames.iter().map(Vec::len).max().unwrap_or(1);
        animation_duration_samples(sample_rate, RECEPTOR_ANIMATION_FPS, frame_count)
    }
}

pub fn confirm_duration_or_default(note_skin: Option<&NoteSkin>, sample_rate: u32) -> Samples {
    note_skin
        .map(|note_skin| note_skin.confirm_duration(sample_rate))
        .unwrap_or(Samples(i64::from(sample_rate) / 6))
}

pub fn load_note_skin(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<NoteSkin> {
    load_note_skin_for_style(device, queue, resolver, textures, "funkin")
}

pub fn load_note_skin_for_style(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    note_style: &str,
) -> Result<NoteSkin> {
    match note_style {
        "pixel" => load_pixel_note_skin(device, queue, resolver, textures),
        _ => load_funkin_note_skin(device, queue, resolver, textures),
    }
}

fn load_funkin_note_skin(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<NoteSkin> {
    let tap = load_sparrow_texture(
        device,
        queue,
        resolver,
        textures,
        "images/notes.xml",
        FilterMode::Linear,
    )?;
    let strumline = load_sparrow_texture(
        device,
        queue,
        resolver,
        textures,
        "images/noteStrumline.xml",
        FilterMode::Linear,
    )?;
    let hold = load_sparrow_texture(
        device,
        queue,
        resolver,
        textures,
        "images/NOTE_assets.xml",
        FilterMode::Linear,
    )?;
    let hold_trail = load_png_texture(
        device,
        queue,
        resolver,
        textures,
        "images/NOTE_hold_assets.png",
        FilterMode::Linear,
    )?;

    Ok(NoteSkin {
        scale: NOTE_ASSET_SCALE,
        filter: FilterMode::Linear,
        strumline_offset: glam::Vec2::ZERO,
        tap_texture_id: tap.texture_id,
        tap_texture_width: tap.texture_width,
        tap_texture_height: tap.texture_height,
        strumline_texture_id: strumline.texture_id,
        strumline_texture_width: strumline.texture_width,
        strumline_texture_height: strumline.texture_height,
        hold_texture_id: hold.texture_id,
        hold_texture_width: hold.texture_width,
        hold_texture_height: hold.texture_height,
        hold_trail_texture_id: hold_trail.texture_id,
        hold_trail_texture_width: hold_trail.texture_width,
        hold_trail_texture_height: hold_trail.texture_height,
        static_frames: [
            cloned_first_frame(&strumline.atlas, "staticLeft0")?,
            cloned_first_frame(&strumline.atlas, "staticDown0")?,
            cloned_first_frame(&strumline.atlas, "staticUp0")?,
            cloned_first_frame(&strumline.atlas, "staticRight0")?,
        ],
        press_frames: [
            cloned_animation_frames(&strumline.atlas, "pressLeft0")?,
            cloned_animation_frames(&strumline.atlas, "pressDown0")?,
            cloned_animation_frames(&strumline.atlas, "pressUp0")?,
            cloned_animation_frames(&strumline.atlas, "pressRight0")?,
        ],
        confirm_frames: [
            cloned_animation_frames(&strumline.atlas, "confirmLeft0")?,
            cloned_animation_frames(&strumline.atlas, "confirmDown0")?,
            cloned_animation_frames(&strumline.atlas, "confirmUp0")?,
            cloned_animation_frames(&strumline.atlas, "confirmRight0")?,
        ],
        confirm_hold_frames: [
            cloned_animation_frames(&strumline.atlas, "confirmHoldLeft0")?,
            cloned_animation_frames(&strumline.atlas, "confirmHoldDown0")?,
            cloned_animation_frames(&strumline.atlas, "confirmHoldUp0")?,
            cloned_animation_frames(&strumline.atlas, "confirmHoldRight0")?,
        ],
        tap_frames: [
            cloned_first_frame(&tap.atlas, "noteLeft0")?,
            cloned_first_frame(&tap.atlas, "noteDown0")?,
            cloned_first_frame(&tap.atlas, "noteUp0")?,
            cloned_first_frame(&tap.atlas, "noteRight0")?,
        ],
        hold_frames: [
            cloned_first_frame(&hold.atlas, "purple hold piece")?,
            cloned_first_frame(&hold.atlas, "blue hold piece")?,
            cloned_first_frame(&hold.atlas, "green hold piece")?,
            cloned_first_frame(&hold.atlas, "red hold piece")?,
        ],
        hold_end_frames: [
            cloned_first_frame(&hold.atlas, "pruple end hold")?,
            cloned_first_frame(&hold.atlas, "blue hold end")?,
            cloned_first_frame(&hold.atlas, "green hold end")?,
            cloned_first_frame(&hold.atlas, "red hold end")?,
        ],
    })
}

fn load_pixel_note_skin(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<NoteSkin> {
    let arrows = load_sparrow_texture(
        device,
        queue,
        resolver,
        textures,
        "images/weeb/pixelUI/arrows-pixels.xml",
        FilterMode::Nearest,
    )?;
    let hold = load_png_texture(
        device,
        queue,
        resolver,
        textures,
        "images/weeb/pixelUI/arrowEndsNew.png",
        FilterMode::Nearest,
    )?;

    Ok(NoteSkin {
        scale: 6.0,
        filter: FilterMode::Nearest,
        strumline_offset: glam::vec2(28.0, 32.0),
        tap_texture_id: arrows.texture_id,
        tap_texture_width: arrows.texture_width,
        tap_texture_height: arrows.texture_height,
        strumline_texture_id: arrows.texture_id,
        strumline_texture_width: arrows.texture_width,
        strumline_texture_height: arrows.texture_height,
        hold_texture_id: hold.texture_id,
        hold_texture_width: hold.texture_width,
        hold_texture_height: hold.texture_height,
        hold_trail_texture_id: hold.texture_id,
        hold_trail_texture_width: hold.texture_width,
        hold_trail_texture_height: hold.texture_height,
        static_frames: [
            cloned_first_frame(&arrows.atlas, "staticLeft0")?,
            cloned_first_frame(&arrows.atlas, "staticDown0")?,
            cloned_first_frame(&arrows.atlas, "staticUp0")?,
            cloned_first_frame(&arrows.atlas, "staticRight0")?,
        ],
        press_frames: [
            cloned_animation_frames(&arrows.atlas, "pressedLeft0")?,
            cloned_animation_frames(&arrows.atlas, "pressedDown0")?,
            cloned_animation_frames(&arrows.atlas, "pressedUp0")?,
            cloned_animation_frames(&arrows.atlas, "pressedRight0")?,
        ],
        confirm_frames: [
            cloned_animation_frames(&arrows.atlas, "confirmLeft0")?,
            cloned_animation_frames(&arrows.atlas, "confirmDown0")?,
            cloned_animation_frames(&arrows.atlas, "confirmUp0")?,
            cloned_animation_frames(&arrows.atlas, "confirmRight0")?,
        ],
        confirm_hold_frames: [
            cloned_animation_frames(&arrows.atlas, "confirmLeft0")?,
            cloned_animation_frames(&arrows.atlas, "confirmDown0")?,
            cloned_animation_frames(&arrows.atlas, "confirmUp0")?,
            cloned_animation_frames(&arrows.atlas, "confirmRight0")?,
        ],
        tap_frames: [
            cloned_first_frame(&arrows.atlas, "noteLeft0")?,
            cloned_first_frame(&arrows.atlas, "noteDown0")?,
            cloned_first_frame(&arrows.atlas, "noteUp0")?,
            cloned_first_frame(&arrows.atlas, "noteRight0")?,
        ],
        hold_frames: pixel_hold_frames(hold.texture_width, hold.texture_height, false),
        hold_end_frames: pixel_hold_frames(hold.texture_width, hold.texture_height, true),
    })
}

struct LoadedSparrowTexture {
    atlas: SparrowAtlas,
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
}

struct LoadedTexture {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
}

fn pixel_hold_frames(texture_width: u32, texture_height: u32, end: bool) -> [SparrowFrame; 4] {
    let segment_width = texture_width / 8;
    std::array::from_fn(|lane| {
        let segment = lane * 2 + usize::from(end);
        SparrowFrame::untrimmed(
            format!("pixel hold {} {}", lane, if end { "end" } else { "piece" }),
            (segment as u32 * segment_width) as i32,
            0,
            segment_width,
            texture_height,
        )
    })
}

fn load_png_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    texture_path: &str,
    filter: FilterMode,
) -> Result<LoadedTexture> {
    let texture_path = AssetPath::new(texture_path)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(texture_path.as_str()));
    textures.insert(texture_id, texture);

    Ok(LoadedTexture {
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
    })
}

fn load_sparrow_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    atlas_path: &str,
    filter: FilterMode,
) -> Result<LoadedSparrowTexture> {
    let atlas_path = AssetPath::new(atlas_path)?;
    let atlas = load_sparrow(resolver, &atlas_path)
        .with_context(|| format!("load {}", atlas_path.as_str()))?;
    let texture_path = atlas_texture_path(&atlas_path, &atlas)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);
    let texture =
        Texture::from_png_image(device, queue, &image, filter, Some(texture_path.as_str()));
    textures.insert(texture_id, texture);

    Ok(LoadedSparrowTexture {
        atlas,
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
    })
}

fn note_sprite_x(slot_x: f32, sprite_width: f32) -> f32 {
    // ref: bdedc0aa:source/funkin/play/notes/Strumline.hx:1172-1174
    slot_x - (sprite_width - STRUMLINE_SIZE) * 0.5 - NOTE_NUDGE
}

fn receptor_sprite_pos(
    player: u8,
    lane: Lane,
    frame: &SparrowFrame,
    scale: f32,
    strumline_offset: glam::Vec2,
) -> glam::Vec2 {
    // ref: bdedc0aa:source/funkin/play/notes/StrumlineNote.hx:121-127
    // playAnimation() calls centerOffsets()/centerOrigin() only; fixOffsets() with DEFAULT_OFFSET=13
    // (line 178) is dead code in v0.8.5 and must not be applied.
    let center = glam::vec2(
        strumline_x(player)
            + strumline_offset.x
            + lane_index(lane) as f32 * NOTE_SPACING
            + STRUMLINE_SIZE * 0.5,
        STRUMLINE_Y_OFFSET + strumline_offset.y + STRUMLINE_SIZE * 0.5,
    );
    center - (frame_source_size(frame) * 0.5 + frame_trim_offset(frame)) * scale
}

fn strumline_x(player: u8) -> f32 {
    STRUMLINE_X_OFFSET + player as f32 * (FNF_WIDTH / 2.0)
}

fn frame_draw_size(frame: &SparrowFrame) -> glam::Vec2 {
    if frame.rotated {
        glam::vec2(frame.height as f32, frame.width as f32)
    } else {
        glam::vec2(frame.width as f32, frame.height as f32)
    }
}

fn frame_source_size(frame: &SparrowFrame) -> glam::Vec2 {
    glam::vec2(frame.frame_width as f32, frame.frame_height as f32)
}

fn frame_trim_offset(frame: &SparrowFrame) -> glam::Vec2 {
    glam::vec2(frame.frame_x as f32, frame.frame_y as f32)
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

fn atlas_texture_path(atlas_path: &AssetPath, atlas: &SparrowAtlas) -> Result<AssetPath> {
    if atlas.image_path.contains('/') {
        Ok(AssetPath::new(atlas.image_path.clone())?)
    } else {
        Ok(atlas_path.sibling(&atlas.image_path)?)
    }
}

fn cloned_first_frame(atlas: &SparrowAtlas, prefix: &str) -> Result<SparrowFrame> {
    atlas
        .first_animation_frame(prefix, &[])
        .cloned()
        .with_context(|| format!("resolve note frame {prefix}"))
}

fn cloned_animation_frames(atlas: &SparrowAtlas, prefix: &str) -> Result<Vec<SparrowFrame>> {
    let frames: Vec<_> = atlas
        .animation_frames(prefix, &[])
        .into_iter()
        .cloned()
        .collect();
    if frames.is_empty() {
        anyhow::bail!("resolve note frame {prefix}");
    }
    Ok(frames)
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

fn animated_note_frame(
    frames: &[SparrowFrame],
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
    fps: u16,
    looped: bool,
) -> &SparrowFrame {
    let index = animation_frame_index(cursor, sample_rate, started_at, fps, frames.len(), looped);
    &frames[index]
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
#[path = "note_assets_tests.rs"]
mod tests;
