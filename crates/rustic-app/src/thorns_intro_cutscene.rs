//! Thorns story-mode intro cutscene.
//!
//! ref: bdedc0aa:assets/preload/scripts/songs/thorns.hxc:25-126

use crate::asset_roots::baked_assets_root;
use crate::pause_menu::PAUSE_OVERLAY_TEXTURE_ID;
use crate::preview_song::{PreviewSelection, PreviewSong};
use crate::stage_object_asset_helpers::asset_id_for_path;
use crate::stage_sfx::play_thorns_senpai_death_sound_or_warn;
use anyhow::{Context, Result};
use rustic_asset::{load_png, load_sparrow, AssetPath, OverlayResolver, SparrowFrame};
use rustic_audio::SharedMixer;
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;
use std::time::Instant;

const SENPAI_CRAZY_XML_PATH: &str = "images/weeb/senpaiCrazy.xml";
const SENPAI_CRAZY_PREFIX: &str = "Senpai Pre Explosion instance 1";
const SENPAI_CRAZY_FPS: u16 = 24;
const PIXEL_ART_SCALE: f32 = 6.0;

const FADE_STEP_SECONDS: f64 = 0.3;
const FADE_STEP_ALPHA: f32 = 0.15;
const ANIMATION_START_SECONDS: f64 = 2.1;
const WHITE_FADE_START_SECONDS: f64 = ANIMATION_START_SECONDS + 3.2;
const WHITE_FADE_DURATION_SECONDS: f64 = 1.4;
const BLACK_FADE_START_SECONDS: f64 = WHITE_FADE_START_SECONDS + WHITE_FADE_DURATION_SECONDS;
const BLACK_FADE_DURATION_SECONDS: f64 = 0.8;
const SENPAI_DIES_SECONDS: f64 = 7.333_333;
const DIALOGUE_DELAY_SECONDS: f64 = 1.6;
const CUTSCENE_END_SECONDS: f64 =
    ANIMATION_START_SECONDS + SENPAI_DIES_SECONDS + DIALOGUE_DELAY_SECONDS;

#[derive(Debug, Clone)]
pub(crate) struct ThornsIntroCutsceneState {
    started_at: Instant,
    assets: ThornsIntroCutsceneAssets,
    death_sound_played: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ThornsIntroCutsceneAssets {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frames: Vec<SparrowFrame>,
}

impl ThornsIntroCutsceneState {
    pub(crate) fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &mut HashMap<AssetId, Texture>,
    ) -> Result<Self> {
        Ok(Self {
            started_at: Instant::now(),
            assets: ThornsIntroCutsceneAssets::load(device, queue, textures)?,
            death_sound_played: false,
        })
    }

    pub(crate) fn tick_audio_or_warn(&mut self, mixer: &SharedMixer, sample_rate: u32) {
        if self.death_sound_played
            || self.elapsed_cursor(sample_rate).0
                < seconds_to_samples(ANIMATION_START_SECONDS, sample_rate)
        {
            return;
        }
        self.death_sound_played = true;
        play_thorns_senpai_death_sound_or_warn(mixer);
    }

    pub(crate) fn finished(&self, sample_rate: u32) -> bool {
        self.elapsed_cursor(sample_rate).0 >= seconds_to_samples(CUTSCENE_END_SECONDS, sample_rate)
    }

    pub(crate) fn elapsed_cursor(&self, sample_rate: u32) -> Samples {
        let elapsed = self.started_at.elapsed().as_secs_f64();
        Samples(seconds_to_samples(elapsed, sample_rate))
    }

    pub(crate) fn append_commands(&self, sprites: &mut RenderCommandList, sample_rate: u32) {
        let elapsed = self.elapsed_cursor(sample_rate);
        let elapsed_seconds = samples_to_seconds(elapsed, sample_rate);
        if elapsed_seconds < BLACK_FADE_START_SECONDS {
            sprites.push(cutscene_overlay_command(
                glam::vec4(1.0, 27.0 / 255.0, 49.0 / 255.0, 1.0),
                10_000,
            ));
        }
        if let Some(cmd) = self.assets.senpai_command(elapsed, sample_rate) {
            sprites.push(cmd);
        }
        let white_alpha = fade_alpha(
            elapsed_seconds,
            WHITE_FADE_START_SECONDS,
            WHITE_FADE_DURATION_SECONDS,
        );
        if white_alpha > 0.0 && elapsed_seconds < BLACK_FADE_START_SECONDS {
            sprites.push(cutscene_overlay_command(
                glam::vec4(1.0, 1.0, 1.0, white_alpha),
                10_020,
            ));
        }
        let black_alpha = fade_alpha(
            elapsed_seconds,
            BLACK_FADE_START_SECONDS,
            BLACK_FADE_DURATION_SECONDS,
        );
        if black_alpha > 0.0 {
            sprites.push(cutscene_overlay_command(
                glam::vec4(0.0, 0.0, 0.0, black_alpha),
                10_030,
            ));
        }
    }
}

impl ThornsIntroCutsceneAssets {
    fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &mut HashMap<AssetId, Texture>,
    ) -> Result<Self> {
        let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
        let xml_path = AssetPath::new(SENPAI_CRAZY_XML_PATH)?;
        let atlas = load_sparrow(&resolver, &xml_path).context("load Thorns Senpai atlas")?;
        let texture_path = xml_path.sibling(&atlas.image_path)?;
        let image = load_png(&resolver, &texture_path).context("load Thorns Senpai texture")?;
        let texture_id = asset_id_for_path(&texture_path);
        textures.insert(
            texture_id,
            Texture::from_png_image(
                device,
                queue,
                &image,
                FilterMode::Nearest,
                Some(texture_path.as_str()),
            ),
        );
        let frames = atlas
            .animation_frames(SENPAI_CRAZY_PREFIX, &[])
            .into_iter()
            .cloned()
            .collect();
        Ok(Self {
            texture_id,
            texture_width: image.width,
            texture_height: image.height,
            frames,
        })
    }

    fn senpai_command(&self, elapsed: Samples, sample_rate: u32) -> Option<DrawCommand> {
        if samples_to_seconds(elapsed, sample_rate) >= BLACK_FADE_START_SECONDS {
            return None;
        }
        let first = self.frames.first()?;
        let alpha = senpai_alpha(elapsed, sample_rate);
        if alpha <= 0.0 {
            return None;
        }
        let frame = if elapsed.0 < seconds_to_samples(ANIMATION_START_SECONDS, sample_rate) {
            first
        } else {
            let animation_elapsed = Samples(
                elapsed
                    .0
                    .saturating_sub(seconds_to_samples(ANIMATION_START_SECONDS, sample_rate)),
            );
            frame_for_cursor(
                &self.frames,
                animation_elapsed,
                sample_rate,
                SENPAI_CRAZY_FPS,
                false,
            )
            .unwrap_or(first)
        };
        let scale = glam::Vec2::splat(PIXEL_ART_SCALE);
        let untrimmed_size =
            glam::vec2(frame.frame_width as f32, frame.frame_height as f32) * scale;
        let mut position = (glam::vec2(1280.0, 720.0) - untrimmed_size) * 0.5;
        position.x += untrimmed_size.x / 5.0;
        Some(sparrow_command(
            self.texture_id,
            self.texture_width,
            self.texture_height,
            frame,
            position,
            scale,
            glam::vec4(1.0, 1.0, 1.0, alpha),
            10_010,
        ))
    }
}

pub(crate) fn should_play_thorns_intro_cutscene(
    selection: PreviewSelection,
    story_mode: bool,
) -> bool {
    story_mode && selection.song == PreviewSong::THORNS
}

fn senpai_alpha(elapsed: Samples, sample_rate: u32) -> f32 {
    let step_samples = seconds_to_samples(FADE_STEP_SECONDS, sample_rate).max(1);
    let steps = elapsed.0.max(0).div_euclid(step_samples);
    (steps as f32 * FADE_STEP_ALPHA).clamp(0.0, 1.0)
}

fn frame_for_cursor(
    frames: &[SparrowFrame],
    cursor: Samples,
    sample_rate: u32,
    fps: u16,
    looped: bool,
) -> Option<&SparrowFrame> {
    if frames.is_empty() {
        return None;
    }
    let elapsed = cursor.0.max(0) as u128;
    let index = (elapsed * u128::from(fps) / u128::from(sample_rate.max(1))) as usize;
    let frame_count = frames.len();
    let index = if looped {
        index % frame_count
    } else {
        index.min(frame_count - 1)
    };
    frames.get(index)
}

fn sparrow_command(
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frame: &SparrowFrame,
    position: glam::Vec2,
    scale: glam::Vec2,
    color: glam::Vec4,
    z: i32,
) -> DrawCommand {
    let draw_pos = position - frame_trim_offset(frame) * scale;
    let mut cmd = DrawCommand::sprite(texture_id, draw_pos, frame_draw_size(frame) * scale);
    cmd.camera = CameraId(2);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = color;
    (cmd.uv_min, cmd.uv_max) = frame_uv(frame, texture_width, texture_height);
    cmd.uv_rotated = frame.rotated;
    cmd
}

fn cutscene_overlay_command(color: glam::Vec4, z: i32) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(
        PAUSE_OVERLAY_TEXTURE_ID,
        glam::vec2(-20.0, -20.0),
        glam::vec2(1920.0, 1080.0),
    );
    cmd.camera = CameraId(2);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = color;
    cmd
}

fn frame_draw_size(frame: &SparrowFrame) -> glam::Vec2 {
    if frame.rotated {
        glam::vec2(frame.height as f32, frame.width as f32)
    } else {
        glam::vec2(frame.width as f32, frame.height as f32)
    }
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

fn fade_alpha(elapsed: f64, start: f64, duration: f64) -> f32 {
    if elapsed <= start {
        return 0.0;
    }
    ((elapsed - start) / duration.max(f64::EPSILON)).clamp(0.0, 1.0) as f32
}

fn seconds_to_samples(seconds: f64, sample_rate: u32) -> i64 {
    (seconds.max(0.0) * f64::from(sample_rate.max(1))).round() as i64
}

fn samples_to_seconds(samples: Samples, sample_rate: u32) -> f64 {
    samples.0.max(0) as f64 / f64::from(sample_rate.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_song::PreviewDifficulty;
    use std::path::Path;

    #[test]
    fn only_story_thorns_uses_intro_cutscene() {
        let thorns = PreviewSelection::new(PreviewSong::THORNS, PreviewDifficulty::Hard);
        let roses = PreviewSelection::new(PreviewSong::ROSES, PreviewDifficulty::Hard);

        assert!(should_play_thorns_intro_cutscene(thorns, true));
        assert!(!should_play_thorns_intro_cutscene(thorns, false));
        assert!(!should_play_thorns_intro_cutscene(roses, true));
    }

    #[test]
    fn senpai_fade_is_stepwise_like_script() {
        assert_eq!(senpai_alpha(Samples(0), 48_000), 0.0);
        assert_eq!(senpai_alpha(Samples(14_400), 48_000), 0.15);
        assert!((senpai_alpha(Samples(86_400), 48_000) - 0.9).abs() < f32::EPSILON);
        assert_eq!(senpai_alpha(Samples(100_800), 48_000), 1.0);
    }

    #[test]
    fn black_fade_starts_after_white_flash() {
        assert_eq!(
            fade_alpha(6.6, BLACK_FADE_START_SECONDS, BLACK_FADE_DURATION_SECONDS),
            0.0
        );
        let alpha = fade_alpha(7.1, BLACK_FADE_START_SECONDS, BLACK_FADE_DURATION_SECONDS);
        assert!(alpha > 0.4 && alpha < 0.6);
        assert_eq!(
            fade_alpha(7.6, BLACK_FADE_START_SECONDS, BLACK_FADE_DURATION_SECONDS),
            1.0
        );
    }

    #[test]
    fn senpai_crazy_source_frames_match_upstream_prefix() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        let resolver = OverlayResolver::new().with_baked_root(workspace.join("assets/source"));
        let path = AssetPath::new(SENPAI_CRAZY_XML_PATH).unwrap();
        let atlas = load_sparrow(&resolver, &path).unwrap();

        assert_eq!(atlas.animation_frames(SENPAI_CRAZY_PREFIX, &[]).len(), 125);
    }
}
