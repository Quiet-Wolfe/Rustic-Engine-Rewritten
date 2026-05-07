//! Note splash asset loading and transient draw commands.
//!
//! ref: bdedc0aa:source/funkin/play/notes/NoteSplash.hx:42-61
//! ref: bdedc0aa:source/funkin/play/notes/Strumline.hx:49,1079-1100,1228-1260
//! ref: bdedc0aa:source/funkin/play/notes/notestyle/NoteStyle.hx:853-975

use anyhow::{Context, Result};
use rustic_asset::{
    load_png, load_sparrow, AssetPath, OverlayResolver, SparrowAtlas, SparrowFrame,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_game::Lane;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const PLAYER_STRUMLINE_X: f32 = 1280.0 / 2.0 + 48.0;
const STRUMLINE_Y_OFFSET: f32 = 24.0;
const STRUMLINE_SIZE: f32 = 104.0;
const NOTE_SPACING: f32 = STRUMLINE_SIZE + 8.0;
const INITIAL_OFFSET: f32 = -0.275 * STRUMLINE_SIZE;
const SPLASH_OFFSET: glam::Vec2 = glam::vec2(25.0, -5.0);
const SPLASH_SCALE: f32 = 1.0;
const SPLASH_ALPHA: f32 = 0.8;
const SPLASH_FPS: u16 = 24;
const SPLASH_FPS_VARIANCE: i16 = 2;
const NOTE_SPLASH_CAP: usize = 6;

#[derive(Debug, Clone)]
pub struct NoteSplashSkin {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    lanes: [LaneSplashFrames; 4],
}

#[derive(Debug, Clone)]
struct LaneSplashFrames {
    variants: [Vec<SparrowFrame>; 2],
}

#[derive(Debug, Default, Clone)]
pub struct NoteSplashes {
    active: Vec<ActiveNoteSplash>,
    sequence: u32,
}

#[derive(Debug, Clone, Copy)]
struct ActiveNoteSplash {
    lane: Lane,
    variant: usize,
    fps: u16,
    started_at: Samples,
}

impl NoteSplashes {
    pub fn push(&mut self, lane: Lane, cursor: Samples) {
        let variant = self.sequence as usize % 2;
        let fps_delta = self.sequence.wrapping_div(2) as i16 % (SPLASH_FPS_VARIANCE * 2 + 1)
            - SPLASH_FPS_VARIANCE;
        self.sequence = self.sequence.wrapping_add(1);

        if self.active.len() >= NOTE_SPLASH_CAP {
            self.active.remove(0);
        }
        self.active.push(ActiveNoteSplash {
            lane,
            variant,
            fps: (SPLASH_FPS as i16 + fps_delta).max(1) as u16,
            started_at: cursor,
        });
    }

    pub fn commands(
        &mut self,
        skin: &NoteSplashSkin,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        self.active
            .retain(|splash| splash_frame_index(splash, skin, cursor, sample_rate).is_some());

        let mut commands = Vec::with_capacity(self.active.len());
        for splash in &self.active {
            let Some(index) = splash_frame_index(splash, skin, cursor, sample_rate) else {
                continue;
            };
            let frame = &skin.frames(splash.lane, splash.variant)[index];
            let mut cmd = DrawCommand::sprite(
                skin.texture_id,
                splash_world_pos(splash.lane, frame),
                glam::vec2(
                    frame.width as f32 * SPLASH_SCALE,
                    frame.height as f32 * SPLASH_SCALE,
                ),
            );
            cmd.camera = CameraId(1);
            cmd.pivot = glam::Vec2::ZERO;
            cmd.layer = RenderLayer::Notes;
            cmd.z = 8;
            cmd.filter = FilterMode::Linear;
            cmd.color.w = SPLASH_ALPHA;
            (cmd.uv_min, cmd.uv_max) = frame_uv(frame, skin.texture_width, skin.texture_height);
            commands.push(cmd);
        }
        commands
    }
}

impl NoteSplashSkin {
    fn frames(&self, lane: Lane, variant: usize) -> &[SparrowFrame] {
        &self.lanes[lane_index(lane)].variants[variant.min(1)]
    }
}

pub fn load_note_splash_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<NoteSplashSkin> {
    let atlas_path = AssetPath::new("images/noteSplashes.xml")?;
    let atlas = load_sparrow(resolver, &atlas_path)
        .with_context(|| format!("load {}", atlas_path.as_str()))?;
    let texture_path = atlas_texture_path(&atlas_path, &atlas)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);

    textures.insert(
        texture_id,
        Texture::from_png_image(
            device,
            queue,
            &image,
            FilterMode::Linear,
            Some(texture_path.as_str()),
        ),
    );

    Ok(NoteSplashSkin {
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
        lanes: [
            LaneSplashFrames {
                variants: [
                    cloned_animation_frames(&atlas, "note impact 1 purple0")?,
                    cloned_animation_frames(&atlas, "note impact 2 purple0")?,
                ],
            },
            LaneSplashFrames {
                variants: [
                    cloned_animation_frames(&atlas, "note impact 1  blue0")?,
                    cloned_animation_frames(&atlas, "note impact 2 blue0")?,
                ],
            },
            LaneSplashFrames {
                variants: [
                    cloned_animation_frames(&atlas, "note impact 1 green0")?,
                    cloned_animation_frames(&atlas, "note impact 2 green0")?,
                ],
            },
            LaneSplashFrames {
                variants: [
                    cloned_animation_frames(&atlas, "note impact 1 red0")?,
                    cloned_animation_frames(&atlas, "note impact 2 red0")?,
                ],
            },
        ],
    })
}

fn splash_frame_index(
    splash: &ActiveNoteSplash,
    skin: &NoteSplashSkin,
    cursor: Samples,
    sample_rate: u32,
) -> Option<usize> {
    let frames = skin.frames(splash.lane, splash.variant);
    if frames.is_empty() || cursor < splash.started_at {
        return None;
    }
    let elapsed = cursor.0 - splash.started_at.0;
    let samples_per_frame = f64::from(sample_rate.max(1)) / f64::from(splash.fps.max(1));
    let index = (elapsed as f64 / samples_per_frame).floor() as usize;
    (index < frames.len()).then_some(index)
}

fn splash_world_pos(lane: Lane, frame: &SparrowFrame) -> glam::Vec2 {
    let base = glam::vec2(
        PLAYER_STRUMLINE_X + lane_index(lane) as f32 * NOTE_SPACING + INITIAL_OFFSET,
        STRUMLINE_Y_OFFSET - INITIAL_OFFSET,
    ) + SPLASH_OFFSET * SPLASH_SCALE;
    let offset =
        glam::vec2(frame.frame_width as f32, frame.frame_height as f32) * SPLASH_SCALE * 0.3;
    let trimmed = glam::vec2(frame.frame_x as f32, frame.frame_y as f32) * SPLASH_SCALE;
    base - offset - trimmed
}

fn atlas_texture_path(atlas_path: &AssetPath, atlas: &SparrowAtlas) -> Result<AssetPath> {
    if atlas.image_path.contains('/') {
        Ok(AssetPath::new(atlas.image_path.clone())?)
    } else {
        Ok(atlas_path.sibling(&atlas.image_path)?)
    }
}

fn cloned_animation_frames(atlas: &SparrowAtlas, prefix: &str) -> Result<Vec<SparrowFrame>> {
    let frames: Vec<_> = atlas
        .animation_frames(prefix, &[])
        .into_iter()
        .cloned()
        .collect();
    if frames.is_empty() {
        anyhow::bail!("resolve note splash frame {prefix}");
    }
    Ok(frames)
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
mod tests {
    use super::*;

    fn frame() -> SparrowFrame {
        SparrowAtlas::parse(
            br#"
            <TextureAtlas imagePath="noteSplashes.png">
              <SubTexture name="note impact 1 purple0000" x="100" y="50"
                width="189" height="270" frameX="-32" frameY="-12"
                frameWidth="260" frameHeight="298"/>
            </TextureAtlas>
            "#,
        )
        .unwrap()
        .frames[0]
            .clone()
    }

    fn skin() -> NoteSplashSkin {
        let f = frame();
        NoteSplashSkin {
            texture_id: AssetId::new(7),
            texture_width: 2048,
            texture_height: 1088,
            lanes: std::array::from_fn(|_| LaneSplashFrames {
                variants: [vec![f.clone(), f.clone()], vec![f.clone()]],
            }),
        }
    }

    #[test]
    fn splash_position_uses_strumline_offsets_trim_and_note_splash_offset() {
        let pos = splash_world_pos(Lane::Left, &frame());
        assert!((pos.x - 638.4).abs() < 1e-3);
        assert!((pos.y - -29.800003).abs() < 1e-3);
    }

    #[test]
    fn splash_commands_advance_by_variant_frame_rate() {
        let mut splashes = NoteSplashes::default();
        splashes.push(Lane::Left, Samples(0));
        let commands = splashes.commands(&skin(), Samples(0), 48_000);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].texture, AssetId::new(7));
        assert_eq!(commands[0].camera, CameraId(1));
        assert_eq!(commands[0].layer, RenderLayer::Notes);
        assert!((commands[0].color.w - SPLASH_ALPHA).abs() < 1e-6);

        let commands = splashes.commands(&skin(), Samples(48_000), 48_000);
        assert!(commands.is_empty());
    }

    #[test]
    fn splash_pool_caps_to_og_size() {
        let mut splashes = NoteSplashes::default();
        for i in 0..8 {
            splashes.push(Lane::Left, Samples(i));
        }
        assert_eq!(splashes.active.len(), NOTE_SPLASH_CAP);
        assert_eq!(splashes.active[0].started_at, Samples(2));
    }
}
