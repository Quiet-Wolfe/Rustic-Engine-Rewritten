//! Funkin' v0.8.5 note, receptor, and legacy sustain-piece rendering.
// LINT-ALLOW: long-file note atlas wiring plus placement unit tests

use crate::lane_state::ReceptorState;
use anyhow::{Context, Result};
use rustic_asset::{
    load_png, load_sparrow, AssetPath, OverlayResolver, SparrowAtlas, SparrowFrame,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_game::{Lane, NoteView};
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const FNF_WIDTH: f32 = 1280.0;
const STRUMLINE_X_OFFSET: f32 = 48.0;
const STRUMLINE_Y_OFFSET: f32 = 24.0;
const STRUMLINE_SIZE: f32 = 104.0;
const NOTE_SPACING: f32 = STRUMLINE_SIZE + 8.0;
const NOTE_NUDGE: f32 = 2.0;
const NOTE_ASSET_SCALE: f32 = 0.7;
const RECEPTOR_ANIMATION_FPS: u16 = 24;
const CONFIRM_HOLD_TIME_SECS: f64 = 0.15;
const LANES: [Lane; 4] = [Lane::Left, Lane::Down, Lane::Up, Lane::Right];

#[derive(Debug, Clone)]
pub struct NoteSkin {
    tap_texture_id: AssetId,
    tap_texture_width: u32,
    tap_texture_height: u32,
    strumline_texture_id: AssetId,
    strumline_texture_width: u32,
    strumline_texture_height: u32,
    hold_texture_id: AssetId,
    hold_texture_width: u32,
    hold_texture_height: u32,
    static_frames: [SparrowFrame; 4],
    press_frames: [Vec<SparrowFrame>; 4],
    confirm_frames: [Vec<SparrowFrame>; 4],
    tap_frames: [SparrowFrame; 4],
    hold_frames: [SparrowFrame; 4],
    hold_end_frames: [SparrowFrame; 4],
}

impl NoteSkin {
    pub fn command_for_view(&self, view: &NoteView) -> DrawCommand {
        let (frame, texture_id, texture_width, texture_height) = self.frame_for_view(view);
        let size = glam::vec2(
            frame.width as f32 * NOTE_ASSET_SCALE,
            frame.height as f32 * NOTE_ASSET_SCALE,
        );
        let x = if view.is_sustain {
            view.x + (NOTE_SPACING - size.x) * 0.5
        } else {
            note_sprite_x(view.x, size.x)
        };

        let mut cmd = DrawCommand::sprite(texture_id, glam::vec2(x, view.y), size);
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Notes;
        cmd.z = if view.is_sustain { 0 } else { 1 };
        cmd.filter = FilterMode::Linear;
        (cmd.uv_min, cmd.uv_max) = frame_uv(frame, texture_width, texture_height);
        if view.is_sustain {
            cmd.color.w = 0.6;
        }
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
                commands.push(self.receptor_command(player, lane, state, cursor, sample_rate));
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
        let size = glam::vec2(
            frame.width as f32 * NOTE_ASSET_SCALE,
            frame.height as f32 * NOTE_ASSET_SCALE,
        );
        let mut cmd = DrawCommand::sprite(
            self.strumline_texture_id,
            receptor_sprite_pos(player, lane, size),
            size,
        );
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Notes;
        cmd.filter = FilterMode::Linear;
        (cmd.uv_min, cmd.uv_max) = frame_uv(
            frame,
            self.strumline_texture_width,
            self.strumline_texture_height,
        );
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
            ReceptorState::Pressed { started_at } => {
                animated_note_frame(&self.press_frames[index], cursor, sample_rate, started_at)
            }
            ReceptorState::Confirm { started_at } => {
                animated_note_frame(&self.confirm_frames[index], cursor, sample_rate, started_at)
            }
        }
    }

    pub fn confirm_duration(&self, sample_rate: u32) -> Samples {
        let frame_count = self.confirm_frames.iter().map(Vec::len).max().unwrap_or(1);
        let animation =
            animation_duration_samples(sample_rate, RECEPTOR_ANIMATION_FPS, frame_count);
        Samples(animation.0 + (f64::from(sample_rate) * CONFIRM_HOLD_TIME_SECS).round() as i64)
    }
}

pub fn load_note_skin(
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

    Ok(NoteSkin {
        tap_texture_id: tap.texture_id,
        tap_texture_width: tap.texture_width,
        tap_texture_height: tap.texture_height,
        strumline_texture_id: strumline.texture_id,
        strumline_texture_width: strumline.texture_width,
        strumline_texture_height: strumline.texture_height,
        hold_texture_id: hold.texture_id,
        hold_texture_width: hold.texture_width,
        hold_texture_height: hold.texture_height,
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

struct LoadedSparrowTexture {
    atlas: SparrowAtlas,
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
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

fn receptor_sprite_pos(player: u8, lane: Lane, size: glam::Vec2) -> glam::Vec2 {
    let center = glam::vec2(
        strumline_x(player) + lane_index(lane) as f32 * NOTE_SPACING + STRUMLINE_SIZE * 0.5,
        STRUMLINE_Y_OFFSET + STRUMLINE_SIZE * 0.5,
    );
    center - size * 0.5
}

fn strumline_x(player: u8) -> f32 {
    STRUMLINE_X_OFFSET + player as f32 * (FNF_WIDTH / 2.0)
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

fn animated_note_frame(
    frames: &[SparrowFrame],
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
) -> &SparrowFrame {
    let index = animation_frame_index(
        cursor,
        sample_rate,
        started_at,
        RECEPTOR_ANIMATION_FPS,
        frames.len(),
        false,
    );
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
mod tests {
    use super::*;

    fn test_note_skin() -> NoteSkin {
        let strumline = SparrowAtlas::parse(
            br#"
            <TextureAtlas imagePath="noteStrumline.png">
                <SubTexture name="staticLeft0001" x="0" y="0" width="154" height="157"/>
                <SubTexture name="pressLeft0001" x="154" y="0" width="139" height="141"
                    frameX="-4" frameY="-3" frameWidth="146" frameHeight="148"/>
                <SubTexture name="pressLeft0002" x="293" y="0" width="146" height="148"/>
                <SubTexture name="confirmLeft0001" x="439" y="0" width="226" height="228"/>
                <SubTexture name="confirmLeft0002" x="665" y="0" width="216" height="218"/>
                <SubTexture name="confirmLeft0003" x="881" y="0" width="217" height="217"/>
                <SubTexture name="confirmLeft0004" x="881" y="0" width="217" height="217"/>
            </TextureAtlas>
            "#,
        )
        .unwrap();
        let tap = SparrowAtlas::parse(
            br#"
            <TextureAtlas imagePath="notes.png">
                <SubTexture name="noteLeft0001" x="0" y="0" width="154" height="157"/>
            </TextureAtlas>
            "#,
        )
        .unwrap();
        let hold = SparrowAtlas::parse(
            br#"
            <TextureAtlas imagePath="NOTE_assets.png">
                <SubTexture name="purple hold piece0000" x="0" y="0" width="50" height="44"/>
                <SubTexture name="pruple end hold0000" x="50" y="0" width="50" height="64"/>
            </TextureAtlas>
            "#,
        )
        .unwrap();
        let static_frame = strumline
            .first_animation_frame("staticLeft0", &[])
            .unwrap()
            .clone();
        let press_frames: Vec<_> = strumline
            .animation_frames("pressLeft0", &[])
            .into_iter()
            .cloned()
            .collect();
        let confirm_frames: Vec<_> = strumline
            .animation_frames("confirmLeft0", &[])
            .into_iter()
            .cloned()
            .collect();
        let tap_frame = tap.first_animation_frame("noteLeft0", &[]).unwrap().clone();
        let hold_frame = hold
            .first_animation_frame("purple hold piece", &[])
            .unwrap()
            .clone();
        let hold_end_frame = hold
            .first_animation_frame("pruple end hold", &[])
            .unwrap()
            .clone();

        NoteSkin {
            tap_texture_id: AssetId::new(1),
            tap_texture_width: 311,
            tap_texture_height: 311,
            strumline_texture_id: AssetId::new(2),
            strumline_texture_width: 2019,
            strumline_texture_height: 810,
            hold_texture_id: AssetId::new(3),
            hold_texture_width: 2048,
            hold_texture_height: 1024,
            static_frames: std::array::from_fn(|_| static_frame.clone()),
            press_frames: std::array::from_fn(|_| press_frames.clone()),
            confirm_frames: std::array::from_fn(|_| confirm_frames.clone()),
            tap_frames: std::array::from_fn(|_| tap_frame.clone()),
            hold_frames: std::array::from_fn(|_| hold_frame.clone()),
            hold_end_frames: std::array::from_fn(|_| hold_end_frame.clone()),
        }
    }

    #[test]
    fn note_sprite_x_centers_tap_assets_on_v085_slot() {
        let width = 154.0 * NOTE_ASSET_SCALE;

        assert!((note_sprite_x(688.0, width) - 684.1).abs() < 1e-4);
    }

    #[test]
    fn receptor_frames_share_the_same_lane_center() {
        let skin = test_note_skin();
        let static_cmd =
            skin.receptor_command(1, Lane::Left, ReceptorState::Static, Samples(0), 48_000);
        let press_cmd = skin.receptor_command(
            1,
            Lane::Left,
            ReceptorState::Pressed {
                started_at: Samples(0),
            },
            Samples(0),
            48_000,
        );

        let static_center = static_cmd.world_pos + static_cmd.size * 0.5;
        let press_center = press_cmd.world_pos + press_cmd.size * 0.5;

        assert_eq!(static_cmd.texture, AssetId::new(2));
        assert_eq!(press_cmd.texture, AssetId::new(2));
        assert!((static_center.x - 740.0).abs() < 1e-5);
        assert!((static_center.y - 76.0).abs() < 1e-5);
        assert!((press_center.x - static_center.x).abs() < 1e-5);
        assert!((press_center.y - static_center.y).abs() < 1e-5);
    }

    #[test]
    fn confirm_duration_includes_og_hold_timer() {
        assert_eq!(test_note_skin().confirm_duration(48_000), Samples(15_200));
    }

    #[test]
    fn animated_note_frame_uses_started_cursor_and_clamps() {
        let atlas = SparrowAtlas::parse(
            br#"
            <TextureAtlas imagePath="noteStrumline.png">
                <SubTexture name="confirm0000" x="0" y="0" width="1" height="1"/>
                <SubTexture name="confirm0001" x="1" y="0" width="1" height="1"/>
                <SubTexture name="confirm0002" x="2" y="0" width="1" height="1"/>
            </TextureAtlas>
            "#,
        )
        .unwrap();
        let frames: Vec<_> = atlas
            .animation_frames("confirm", &[])
            .into_iter()
            .cloned()
            .collect();

        assert_eq!(
            animated_note_frame(&frames, Samples(12_000), 48_000, Samples(10_000)).name,
            "confirm0001"
        );
        assert_eq!(
            animated_note_frame(&frames, Samples(96_000), 48_000, Samples(10_000)).name,
            "confirm0002"
        );
    }
}
