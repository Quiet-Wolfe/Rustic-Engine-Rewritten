//! Darnell story-mode in-game can cutscene.
//!
//! ref: bdedc0aa:assets/preload/scripts/songs/darnell.hxc:79-237

use crate::asset_roots::app_asset_resolver;
use crate::camera_fx::CameraFx;
use crate::character_anim::CharacterAnimState;
use crate::pause_menu::PAUSE_OVERLAY_TEXTURE_ID;
use crate::preview_song::{PreviewSelection, PreviewSong};
use crate::scene_assets::CameraFocusPoints;
use crate::stage_object_asset_helpers::asset_id_for_path;
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, load_png, load_sparrow, AssetPath, SparrowFrame};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{CameraRegistry, DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

const CUTSCENE_DURATION_SECONDS: f64 = 10.0;
const CUTSCENE_FPS: u16 = 24;
const CAN_ATLAS_PATH: &str = "images/wked1_cutscene_1_can.xml";
const CUTSCENE_CAN_POSITION: glam::Vec2 = glam::vec2(950.0, 725.0);
const CUTSCENE_CAN_Z: i32 = 689;

#[derive(Debug, Clone)]
pub(crate) struct DarnellIntroCutsceneState {
    started_at: Samples,
    countdown_cursor: Samples,
    assets: Option<DarnellIntroCutsceneAssets>,
    played_audio: [bool; CUTSCENE_AUDIO_CUES.len()],
}

#[derive(Debug, Clone)]
struct DarnellIntroCutsceneAssets {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    kicked_up: Vec<SparrowFrame>,
    kicked_forward: Vec<SparrowFrame>,
}

impl DarnellIntroCutsceneState {
    pub(crate) fn new(countdown_cursor: Samples, sample_rate: u32) -> Self {
        Self {
            started_at: Samples(
                countdown_cursor
                    .0
                    .saturating_sub(seconds_to_samples(CUTSCENE_DURATION_SECONDS, sample_rate)),
            ),
            countdown_cursor,
            assets: None,
            played_audio: [false; CUTSCENE_AUDIO_CUES.len()],
        }
    }

    pub(crate) fn load_assets_or_warn(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &mut HashMap<AssetId, Texture>,
    ) {
        if self.assets.is_some() {
            return;
        }
        match DarnellIntroCutsceneAssets::load(device, queue, textures) {
            Ok(assets) => self.assets = Some(assets),
            Err(e) => {
                tracing::warn!(target: "rustic.asset", "Darnell intro cutscene unavailable: {e:#}");
            }
        }
    }

    pub(crate) fn song_start_cursor(&self) -> Samples {
        self.started_at
    }

    pub(crate) fn blocks_input(&self, cursor: Samples) -> bool {
        cursor.0 < self.countdown_cursor.0
    }

    pub(crate) fn active_at(&self, cursor: Samples) -> bool {
        cursor.0 >= self.started_at.0 && cursor.0 < self.countdown_cursor.0
    }

    pub(crate) fn tick_audio_or_warn(
        &mut self,
        mixer: &SharedMixer,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if !self.active_at(cursor) {
            return;
        }
        for (index, cue) in CUTSCENE_AUDIO_CUES.iter().enumerate() {
            if self.played_audio[index] || self.elapsed_seconds(cursor, sample_rate) < cue.at {
                continue;
            }
            self.played_audio[index] = true;
            if let Err(e) = play_cutscene_audio(mixer, cue.sound) {
                tracing::warn!(target: "rustic.audio", "play Darnell cutscene sound: {e:#}");
            }
        }
    }

    pub(crate) fn apply_character_poses(
        &self,
        anim: &mut CharacterAnimState,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if !self.active_at(cursor) {
            return;
        }
        self.play_pose_after(anim, cursor, sample_rate, 0.0, "boyfriend", "intro1");
        self.play_pose_after(anim, cursor, sample_rate, 5.0, "dad", "lightCan");
        self.play_pose_after(anim, cursor, sample_rate, 6.0, "boyfriend", "cock");
        self.play_pose_after(anim, cursor, sample_rate, 6.4, "dad", "kickCan");
        self.play_pose_after(anim, cursor, sample_rate, 6.9, "dad", "kneeCan");
        self.play_pose_after(anim, cursor, sample_rate, 7.1, "boyfriend", "intro2");
        self.play_pose_after(anim, cursor, sample_rate, 7.9, "dad", "laughCutscene");
        self.play_pose_after(
            anim,
            cursor,
            sample_rate,
            8.2,
            "girlfriend",
            "laughCutscene",
        );
    }

    pub(crate) fn apply_camera(
        &self,
        cameras: &mut CameraRegistry,
        camera_fx: &mut CameraFx,
        focus: CameraFocusPoints,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if !self.active_at(cursor) {
            return;
        }
        let elapsed = self.elapsed_seconds(cursor, sample_rate);
        let player = focus.player;
        let opponent = focus.opponent;
        let (position, zoom) = if elapsed < 2.0 {
            (player + glam::vec2(250.0, 0.0), 1.3)
        } else if elapsed < 4.5 {
            let progress = quad_in_out((elapsed - 2.0) as f32 / 2.5);
            (
                (player + glam::vec2(250.0, 0.0)).lerp(opponent + glam::vec2(100.0, 0.0), progress),
                lerp(1.3, 0.66, progress),
            )
        } else if elapsed < 6.0 {
            (opponent + glam::vec2(100.0, 0.0), 0.66)
        } else if elapsed < 6.4 {
            let progress = quad_in_out((elapsed - 6.0) as f32 / 0.4);
            (
                (opponent + glam::vec2(100.0, 0.0))
                    .lerp(opponent + glam::vec2(180.0, 0.0), progress),
                0.66,
            )
        } else if elapsed < 8.0 {
            (opponent + glam::vec2(100.0, 0.0), 0.66)
        } else {
            let progress = quad_in_out((elapsed - 8.0) as f32 / 2.0);
            (
                (opponent + glam::vec2(100.0, 0.0))
                    .lerp(opponent + glam::vec2(180.0, 0.0), progress),
                lerp(0.66, 0.77, progress),
            )
        };
        camera_fx.force_game_camera(cameras, position, zoom);
    }

    pub(crate) fn apply_commands<'a>(
        &self,
        commands: impl Iterator<Item = &'a mut DrawCommand>,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if !self.active_at(cursor) {
            return;
        }
        let elapsed = self.elapsed_seconds(cursor, sample_rate);
        for cmd in commands {
            if matches!(cmd.layer, RenderLayer::Notes | RenderLayer::Hud) {
                cmd.color.w = 0.0;
            }
            if (7.1..8.6).contains(&elapsed) && cmd.layer == RenderLayer::Stage && cmd.z < 300 {
                let fade = ((elapsed - 7.1) / 1.5).clamp(0.0, 1.0) as f32;
                let dark = glam::Vec3::splat(0x22 as f32 / 255.0).lerp(glam::Vec3::ONE, fade);
                cmd.color.x *= dark.x;
                cmd.color.y *= dark.y;
                cmd.color.z *= dark.z;
            }
        }
    }

    pub(crate) fn append_commands(
        &self,
        sprites: &mut RenderCommandList,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if !self.active_at(cursor) {
            return;
        }
        let elapsed = self.elapsed_seconds(cursor, sample_rate);
        if let Some(alpha) = black_overlay_alpha(elapsed) {
            sprites.push(black_overlay_command(alpha));
        }
        if let Some((clip, started_at)) = can_clip_at(elapsed, sample_rate) {
            if let Some(assets) = self.assets.as_ref() {
                assets.append_can_command(
                    sprites,
                    clip,
                    cursor,
                    Samples(self.started_at.0.saturating_add(started_at.0)),
                    sample_rate,
                );
            }
        }
    }

    fn elapsed_seconds(&self, cursor: Samples, sample_rate: u32) -> f64 {
        cursor.0.saturating_sub(self.started_at.0).max(0) as f64 / f64::from(sample_rate.max(1))
    }

    fn play_pose_after(
        &self,
        anim: &mut CharacterAnimState,
        cursor: Samples,
        sample_rate: u32,
        at_seconds: f64,
        target: &'static str,
        pose: &'static str,
    ) {
        let started_at = self.cursor_at(at_seconds, sample_rate);
        if cursor >= started_at {
            anim.play_chart_animation(target, pose, started_at, false);
        }
    }

    fn cursor_at(&self, seconds: f64, sample_rate: u32) -> Samples {
        Samples(
            self.started_at
                .0
                .saturating_add(seconds_to_samples(seconds, sample_rate)),
        )
    }
}

impl DarnellIntroCutsceneAssets {
    fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &mut HashMap<AssetId, Texture>,
    ) -> Result<Self> {
        let resolver = app_asset_resolver();
        let atlas_path = AssetPath::new(CAN_ATLAS_PATH)?;
        let atlas = load_sparrow(&resolver, &atlas_path).context("load Darnell can atlas")?;
        let texture_path = atlas_path.sibling(&atlas.image_path)?;
        let image = load_png(&resolver, &texture_path).context("load Darnell can texture")?;
        let texture_id = asset_id_for_path(&texture_path);
        let texture_width = image.width;
        let texture_height = image.height;
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
        Ok(Self {
            texture_id,
            texture_width,
            texture_height,
            kicked_up: animation_frames(&atlas, "can kicked up0")?,
            kicked_forward: animation_frames(&atlas, "can kick quick0")?,
        })
    }

    fn append_can_command(
        &self,
        sprites: &mut RenderCommandList,
        clip: CanClip,
        cursor: Samples,
        started_at: Samples,
        sample_rate: u32,
    ) {
        let frames = match clip {
            CanClip::KickedUp => &self.kicked_up,
            CanClip::KickedForward => &self.kicked_forward,
        };
        let Some(frame) = frame_for_cursor(frames, cursor, started_at, sample_rate) else {
            return;
        };
        sprites.push(sparrow_command(
            self.texture_id,
            self.texture_width,
            self.texture_height,
            frame,
            CUTSCENE_CAN_POSITION,
        ));
    }
}

pub(crate) fn should_play_darnell_intro_cutscene(
    selection: PreviewSelection,
    story_mode: bool,
) -> bool {
    story_mode && selection.song == PreviewSong::DARNELL
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanClip {
    KickedUp,
    KickedForward,
}

#[derive(Debug, Clone, Copy)]
struct AudioCue {
    at: f64,
    sound: DarnellCutsceneAudio,
}

const CUTSCENE_AUDIO_CUES: [AudioCue; 8] = [
    AudioCue {
        at: 0.7,
        sound: DarnellCutsceneAudio::Music,
    },
    AudioCue {
        at: 5.0,
        sound: DarnellCutsceneAudio::Lighter,
    },
    AudioCue {
        at: 6.0,
        sound: DarnellCutsceneAudio::GunPrep,
    },
    AudioCue {
        at: 6.4,
        sound: DarnellCutsceneAudio::KickCanUp,
    },
    AudioCue {
        at: 6.9,
        sound: DarnellCutsceneAudio::KickCanForward,
    },
    AudioCue {
        at: 7.1,
        sound: DarnellCutsceneAudio::Shot,
    },
    AudioCue {
        at: 7.9,
        sound: DarnellCutsceneAudio::DarnellLaugh,
    },
    AudioCue {
        at: 8.2,
        sound: DarnellCutsceneAudio::NeneLaugh,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DarnellCutsceneAudio {
    Music,
    Lighter,
    GunPrep,
    KickCanUp,
    KickCanForward,
    Shot,
    DarnellLaugh,
    NeneLaugh,
}

static MUSIC_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static LIGHTER_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static GUN_PREP_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static KICK_UP_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static KICK_FORWARD_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static SHOT_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static DARNELL_LAUGH_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static NENE_LAUGH_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();

impl DarnellCutsceneAudio {
    fn path(self) -> &'static str {
        match self {
            Self::Music => "music/darnellCanCutscene/darnellCanCutscene.ogg",
            Self::Lighter => "sounds/Darnell_Lighter.ogg",
            Self::GunPrep => "sounds/Gun_Prep.ogg",
            Self::KickCanUp => "sounds/Kick_Can_UP.ogg",
            Self::KickCanForward => "sounds/Kick_Can_FORWARD.ogg",
            Self::Shot => "sounds/shot1.ogg",
            Self::DarnellLaugh => "sounds/cutscene/darnell_laugh.ogg",
            Self::NeneLaugh => "sounds/cutscene/nene_laugh.ogg",
        }
    }

    fn cache(self) -> &'static OnceLock<Option<Arc<[u8]>>> {
        match self {
            Self::Music => &MUSIC_BYTES,
            Self::Lighter => &LIGHTER_BYTES,
            Self::GunPrep => &GUN_PREP_BYTES,
            Self::KickCanUp => &KICK_UP_BYTES,
            Self::KickCanForward => &KICK_FORWARD_BYTES,
            Self::Shot => &SHOT_BYTES,
            Self::DarnellLaugh => &DARNELL_LAUGH_BYTES,
            Self::NeneLaugh => &NENE_LAUGH_BYTES,
        }
    }
}

fn play_cutscene_audio(mixer: &SharedMixer, sound: DarnellCutsceneAudio) -> Result<()> {
    let Some(bytes) = cached_audio(sound) else {
        return Ok(());
    };
    let source = streaming_vorbis_source(bytes.clone())
        .with_context(|| format!("decode {}", sound.path()))?;
    mixer
        .edit(|mixer| mixer.add_source(Stem::Sfx, source))
        .with_context(|| format!("queue {}", sound.path()))?;
    Ok(())
}

fn cached_audio(sound: DarnellCutsceneAudio) -> Option<&'static Arc<[u8]>> {
    sound
        .cache()
        .get_or_init(|| match load_audio_bytes(sound.path()) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!(target: "rustic.audio", "Darnell cutscene audio unavailable: {e:#}");
                None
            }
        })
        .as_ref()
}

fn load_audio_bytes(path: &str) -> Result<Arc<[u8]>> {
    let resolver = app_asset_resolver();
    let path = AssetPath::new(path)?;
    load_bytes(&resolver, &path).with_context(|| format!("load {}", path.as_str()))
}

fn can_clip_at(elapsed: f64, sample_rate: u32) -> Option<(CanClip, Samples)> {
    if (6.4..6.9).contains(&elapsed) {
        return Some((
            CanClip::KickedUp,
            Samples(seconds_to_samples(6.4, sample_rate)),
        ));
    }
    if (6.9..7.1).contains(&elapsed) {
        return Some((
            CanClip::KickedForward,
            Samples(seconds_to_samples(6.9, sample_rate)),
        ));
    }
    None
}

fn black_overlay_alpha(elapsed: f64) -> Option<f32> {
    if elapsed < 1.0 {
        return Some(1.0);
    }
    if elapsed < 3.0 {
        return Some((1.0 - (elapsed - 1.0) / 2.0) as f32);
    }
    None
}

fn animation_frames(atlas: &rustic_asset::SparrowAtlas, prefix: &str) -> Result<Vec<SparrowFrame>> {
    let frames: Vec<_> = atlas
        .animation_frames(prefix, &[])
        .into_iter()
        .cloned()
        .collect();
    if frames.is_empty() {
        anyhow::bail!("Darnell cutscene can animation {prefix} has no frames");
    }
    Ok(frames)
}

fn frame_for_cursor(
    frames: &[SparrowFrame],
    cursor: Samples,
    started_at: Samples,
    sample_rate: u32,
) -> Option<&SparrowFrame> {
    if frames.is_empty() {
        return None;
    }
    let elapsed = cursor.0.saturating_sub(started_at.0).max(0);
    let index = (elapsed * i64::from(CUTSCENE_FPS) / i64::from(sample_rate.max(1))).max(0) as usize;
    frames.get(index.min(frames.len() - 1))
}

fn sparrow_command(
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frame: &SparrowFrame,
    position: glam::Vec2,
) -> DrawCommand {
    let draw_pos = position - frame_trim_offset(frame);
    let mut cmd = DrawCommand::sprite(texture_id, draw_pos, frame_draw_size(frame));
    cmd.camera = CameraId(0);
    cmd.layer = RenderLayer::Stage;
    cmd.z = CUTSCENE_CAN_Z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Linear;
    (cmd.uv_min, cmd.uv_max) = frame_uv(frame, texture_width, texture_height);
    cmd.uv_rotated = frame.rotated;
    cmd
}

fn black_overlay_command(alpha: f32) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(
        PAUSE_OVERLAY_TEXTURE_ID,
        glam::vec2(-100.0, -100.0),
        glam::vec2(2000.0, 2500.0),
    );
    cmd.camera = CameraId(2);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = 10_025;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = glam::vec4(0.0, 0.0, 0.0, alpha);
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

fn seconds_to_samples(seconds: f64, sample_rate: u32) -> i64 {
    (seconds.max(0.0) * f64::from(sample_rate.max(1))).round() as i64
}

fn quad_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) * 0.5
    }
}

fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_song::PreviewDifficulty;

    #[test]
    fn only_story_darnell_uses_intro_cutscene() {
        let darnell = PreviewSelection::new(PreviewSong::DARNELL, PreviewDifficulty::Hard);
        let lit_up = PreviewSelection::new(PreviewSong::LIT_UP, PreviewDifficulty::Hard);

        assert!(should_play_darnell_intro_cutscene(darnell, true));
        assert!(!should_play_darnell_intro_cutscene(darnell, false));
        assert!(!should_play_darnell_intro_cutscene(lit_up, true));
    }

    #[test]
    fn cutscene_extends_song_clock_before_countdown() {
        let cutscene = DarnellIntroCutsceneState::new(Samples(-90_000), 48_000);

        assert_eq!(cutscene.song_start_cursor(), Samples(-570_000));
        assert!(cutscene.blocks_input(Samples(-90_001)));
        assert!(!cutscene.blocks_input(Samples(-90_000)));
    }

    #[test]
    fn can_clip_tracks_scripted_kick_windows() {
        assert_eq!(can_clip_at(6.39, 48_000), None);
        assert_eq!(
            can_clip_at(6.4, 48_000).map(|clip| clip.0),
            Some(CanClip::KickedUp)
        );
        assert_eq!(
            can_clip_at(6.95, 48_000).map(|clip| clip.0),
            Some(CanClip::KickedForward)
        );
        assert_eq!(can_clip_at(7.1, 48_000), None);
    }

    #[test]
    fn apply_commands_hides_gameplay_ui_and_darkens_stage_props_after_shot() {
        let cutscene = DarnellIntroCutsceneState::new(Samples(0), 48_000);
        let mut hud = DrawCommand::sprite(1.into(), glam::Vec2::ZERO, glam::Vec2::ONE);
        hud.layer = RenderLayer::Hud;
        let mut stage = DrawCommand::sprite(2.into(), glam::Vec2::ZERO, glam::Vec2::ONE);
        stage.layer = RenderLayer::Stage;
        stage.z = 20;
        let mut character = DrawCommand::sprite(3.into(), glam::Vec2::ZERO, glam::Vec2::ONE);
        character.layer = RenderLayer::Characters;
        let mut commands = vec![hud, stage, character];

        cutscene.apply_commands(commands.iter_mut(), Samples(-480_000 + 8 * 48_000), 48_000);

        assert_eq!(commands[0].color.w, 0.0);
        assert!(commands[1].color.x < 0.8);
        assert_eq!(commands[2].color, glam::Vec4::ONE);
    }

    #[test]
    fn source_can_atlas_has_cutscene_animations() {
        let resolver = app_asset_resolver();
        let atlas = load_sparrow(&resolver, &AssetPath::new(CAN_ATLAS_PATH).unwrap()).unwrap();

        assert!(!animation_frames(&atlas, "can kicked up0")
            .unwrap()
            .is_empty());
        assert!(!animation_frames(&atlas, "can kick quick0")
            .unwrap()
            .is_empty());
    }
}
