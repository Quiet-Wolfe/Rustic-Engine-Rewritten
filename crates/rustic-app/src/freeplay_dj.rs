//! Freeplay DJ sprite from Funkin' v0.8.5.
//!
//! Drives BF/Pico's core Freeplay DJ frame labels.
//! FistPump, FistPumpLoss, NewUnlock, and CharSelect states are deferred.
//!
//! ref: bdedc0aa:source/funkin/ui/freeplay/dj/BaseFreeplayDJ.hx
//! ref: bdedc0aa:source/funkin/ui/freeplay/dj/AnimateAtlasFreeplayDJ.hx
//! ref: bdedc0aa:assets/preload/data/players/bf.json

use crate::asset_roots::baked_assets_root;
use anyhow::{Context, Result};
use rustic_asset::{
    load_animate_animation, load_animate_spritemap, load_png, AnimateAnimation, AnimateAtlas,
    AnimateDrawPart, AssetPath, OverlayResolver,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, Texture};

// ref: bdedc0aa:source/funkin/ui/freeplay/dj/BaseFreeplayDJ.hx:221-233 (resetPosition non-widescreen branch)
const DJ_FPS: u16 = 24;
// ref: bdedc0aa:source/funkin/ui/freeplay/dj/BaseFreeplayDJ.hx:10-59 FreeplayDJState enum
const DJ_INTRO_LABEL: &str = "Intro";
const DJ_IDLE_LABEL: &str = "Idle";
const DJ_CONFIRM_LABEL: &str = "Confirm";
const DJ_AFK_LABEL: &str = "AFK";
const DJ_CARTOON_LABEL: &str = "Watching TV";
const DJ_IDLE_EGG_SECONDS: f64 = 60.0;
const DJ_IDLE_CARTOON_SECONDS: f64 = 120.0;

/// Logical paths to assets the BF DJ depends on.
pub const REQUIRED_DJ_ASSETS: &[&str] = &[
    "images/freeplay/freeplay-boyfriend/Animation.json",
    "images/freeplay/freeplay-boyfriend/spritemap1.json",
    "images/freeplay/freeplay-boyfriend/spritemap1.png",
];

/// Subset of FreeplayDJState we drive in Phase 1.
/// ref: bdedc0aa:source/funkin/ui/freeplay/dj/BaseFreeplayDJ.hx:10-59
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjState {
    Intro,
    Idle,
    IdleEasterEgg,
    Cartoon,
    Confirm,
}

#[derive(Debug)]
pub struct FreeplayDJ {
    texture_id: AssetId,
    animation: AnimateAnimation,
    atlas: AnimateAtlas,
    intro_frame_count: u32,
    idle_frame_count: u32,
    confirm_frame_count: u32,
    afk_frame_count: u32,
    cartoon_frame_count: u32,
    state: DjState,
    state_started_at: Samples,
    idle_started_at: Samples,
    seen_idle_easter_egg: bool,
    pub texture: Option<Texture>,
}

impl FreeplayDJ {
    pub fn commands(&self, cursor: Samples, sample_rate: u32) -> Vec<DrawCommand> {
        let (label, frame_count, looped) = match self.state {
            DjState::Intro => (DJ_INTRO_LABEL, self.intro_frame_count, false),
            DjState::Idle => (DJ_IDLE_LABEL, self.idle_frame_count, true),
            DjState::IdleEasterEgg => (DJ_AFK_LABEL, self.afk_frame_count, false),
            DjState::Cartoon => (DJ_CARTOON_LABEL, self.cartoon_frame_count, true),
            DjState::Confirm => (DJ_CONFIRM_LABEL, self.confirm_frame_count, false),
        };
        let frame_offset = label_frame_offset(
            cursor,
            self.state_started_at,
            sample_rate,
            DJ_FPS,
            frame_count,
            looped,
        );
        let Ok(parts) = self.animation.flatten_label_frame(label, frame_offset) else {
            return Vec::new();
        };
        parts
            .iter()
            .filter_map(|part| self.command_for_part(part))
            .collect()
    }

    /// Re-enter the Intro animation anchored at `cursor`. Auto-transitions to
    /// Idle once `tick()` sees the intro frame budget exhausted.
    /// ref: bdedc0aa:source/funkin/ui/freeplay/dj/BaseFreeplayDJ.hx:14-19
    pub fn reset_intro(&mut self, cursor: Samples) {
        self.state = DjState::Intro;
        self.state_started_at = cursor;
        self.idle_started_at = cursor;
        self.seen_idle_easter_egg = false;
    }

    /// Trigger the Confirm animation (plays once and holds last frame).
    /// ref: bdedc0aa:source/funkin/ui/freeplay/dj/BaseFreeplayDJ.hx:36-39
    pub fn enter_confirm(&mut self, cursor: Samples) {
        self.state = DjState::Confirm;
        self.state_started_at = cursor;
    }

    /// Reset the AFK timer after any Freeplay player action.
    /// ref: bdedc0aa:source/funkin/ui/freeplay/dj/BaseFreeplayDJ.hx:117-128
    pub fn on_player_action(&mut self, cursor: Samples) {
        if matches!(
            self.state,
            DjState::Idle | DjState::IdleEasterEgg | DjState::Cartoon
        ) {
            self.state = DjState::Idle;
            self.state_started_at = cursor;
            self.idle_started_at = cursor;
            self.seen_idle_easter_egg = false;
        }
    }

    /// Advance the DJ state machine. Currently only used to drop Intro → Idle
    /// when the Intro label finishes and Idle → AFK → Idle around the easter
    /// egg timer; Confirm terminates via frame-offset clamping.
    pub fn tick(&mut self, cursor: Samples, sample_rate: u32) {
        match self.state {
            DjState::Intro => {
                if self.state_frame(cursor, sample_rate) >= self.intro_frame_count {
                    let intro_samples =
                        animation_duration_samples(self.intro_frame_count, sample_rate, DJ_FPS);
                    self.state = DjState::Idle;
                    self.state_started_at = Samples(self.state_started_at.0 + intro_samples);
                    self.idle_started_at = self.state_started_at;
                    self.seen_idle_easter_egg = false;
                }
            }
            DjState::Idle => {
                let idle_elapsed = cursor.0.saturating_sub(self.idle_started_at.0);
                if self.cartoon_frame_count > 0
                    && idle_elapsed >= seconds_to_samples(DJ_IDLE_CARTOON_SECONDS, sample_rate)
                {
                    self.state = DjState::Cartoon;
                    self.state_started_at = cursor;
                } else if self.afk_frame_count > 0
                    && !self.seen_idle_easter_egg
                    && idle_elapsed >= seconds_to_samples(DJ_IDLE_EGG_SECONDS, sample_rate)
                {
                    self.state = DjState::IdleEasterEgg;
                    self.state_started_at = cursor;
                    self.seen_idle_easter_egg = true;
                }
            }
            DjState::IdleEasterEgg => {
                if self.state_frame(cursor, sample_rate) >= self.afk_frame_count {
                    let afk_samples =
                        animation_duration_samples(self.afk_frame_count, sample_rate, DJ_FPS);
                    self.state = DjState::Idle;
                    self.state_started_at = Samples(self.state_started_at.0 + afk_samples);
                }
            }
            DjState::Cartoon => {}
            DjState::Confirm => {}
        }
    }

    fn state_frame(&self, cursor: Samples, sample_rate: u32) -> u32 {
        elapsed_frame(cursor, self.state_started_at, sample_rate, DJ_FPS)
    }

    fn command_for_part(&self, part: &AnimateDrawPart) -> Option<DrawCommand> {
        let frame = self.atlas.frame(&part.frame_name)?;
        let mut cmd = DrawCommand::sprite(
            self.texture_id,
            glam::vec2(0.0, 0.0),
            glam::vec2(frame.size.x, frame.size.y),
        );
        cmd.camera = CameraId(1);
        cmd.layer = RenderLayer::Characters;
        cmd.z = 100;
        cmd.pivot = glam::Vec2::ZERO;
        cmd.filter = FilterMode::Linear;
        cmd.affine = part.matrix;
        cmd.uv_min = frame.uv_min;
        cmd.uv_max = frame.uv_max;
        cmd.uv_rotated = frame.rotated;
        cmd.color = glam::Vec4::from_array(part.color);
        cmd.color_offset = glam::Vec4::from_array(part.color_offset);
        Some(cmd)
    }

    pub fn take_texture(&mut self) -> Option<(AssetId, Texture)> {
        self.texture.take().map(|tex| (self.texture_id, tex))
    }
}

pub fn load_freeplay_dj(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<FreeplayDJ> {
    load_freeplay_dj_for_asset(device, queue, "images/freeplay/freeplay-boyfriend")
}

pub fn load_freeplay_dj_for_asset(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    asset_path: &str,
) -> Result<FreeplayDJ> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let animation_path = AssetPath::new(format!("{asset_path}/Animation.json"))?;
    let spritemap_path = AssetPath::new(format!("{asset_path}/spritemap1.json"))?;
    let texture_path = AssetPath::new(format!("{asset_path}/spritemap1.png"))?;
    let animation = load_animate_animation(&resolver, &animation_path)
        .with_context(|| format!("load {animation_path}"))?;
    let atlas = load_animate_spritemap(&resolver, &spritemap_path)
        .with_context(|| format!("load {spritemap_path}"))?;
    let image =
        load_png(&resolver, &texture_path).with_context(|| format!("load {texture_path}"))?;
    let texture_id = asset_id_for_path(&texture_path);
    let texture = Texture::from_png_image(
        device,
        queue,
        &image,
        FilterMode::Linear,
        Some(texture_path.as_str()),
    );
    let intro_frame_count = animation
        .label(DJ_INTRO_LABEL)
        .map(|label| label.duration.max(1))
        .unwrap_or(1);
    let idle_frame_count = animation
        .label(DJ_IDLE_LABEL)
        .map(|label| label.duration.max(1))
        .unwrap_or(1);
    let confirm_frame_count = animation
        .label(DJ_CONFIRM_LABEL)
        .map(|label| label.duration.max(1))
        .unwrap_or(1);
    let afk_frame_count = animation
        .label(DJ_AFK_LABEL)
        .map(|label| label.duration.max(1))
        .unwrap_or(0);
    let cartoon_frame_count = animation
        .label(DJ_CARTOON_LABEL)
        .map(|label| label.duration.max(1))
        .unwrap_or(0);
    Ok(FreeplayDJ {
        texture_id,
        animation,
        atlas,
        intro_frame_count,
        idle_frame_count,
        confirm_frame_count,
        afk_frame_count,
        cartoon_frame_count,
        // Caller calls `reset_intro(cursor)` from `enter_song_select` so the
        // intro plays from the moment the screen mounts. Default to Idle so a
        // stale/uninitialized DJ never gets stuck on a frozen first intro frame.
        state: DjState::Idle,
        state_started_at: Samples(0),
        idle_started_at: Samples(0),
        seen_idle_easter_egg: false,
        texture: Some(texture),
    })
}

fn label_frame_offset(
    cursor: Samples,
    state_started_at: Samples,
    sample_rate: u32,
    fps: u16,
    frame_count: u32,
    looped: bool,
) -> u32 {
    if frame_count <= 1 {
        return 0;
    }
    let frame = elapsed_frame(cursor, state_started_at, sample_rate, fps);
    if looped {
        frame % frame_count.max(1)
    } else {
        frame.min(frame_count.saturating_sub(1))
    }
}

fn elapsed_frame(cursor: Samples, state_started_at: Samples, sample_rate: u32, fps: u16) -> u32 {
    let elapsed = cursor.0.saturating_sub(state_started_at.0).max(0) as u128;
    (elapsed * u128::from(fps) / u128::from(sample_rate.max(1))) as u32
}

fn animation_duration_samples(frame_count: u32, sample_rate: u32, fps: u16) -> i64 {
    (u128::from(frame_count.max(1)) * u128::from(sample_rate.max(1)) / u128::from(fps.max(1)))
        as i64
}

fn seconds_to_samples(seconds: f64, sample_rate: u32) -> i64 {
    (seconds.max(0.0) * f64::from(sample_rate.max(1))).round() as i64
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

    #[test]
    fn label_frame_offset_loops_when_idle() {
        // 48000 / 24 = 2000 samples per frame; anchor 0 with looped=true.
        assert_eq!(
            label_frame_offset(Samples(0), Samples(0), 48_000, 24, 10, true),
            0
        );
        assert_eq!(
            label_frame_offset(Samples(2_000), Samples(0), 48_000, 24, 10, true),
            1
        );
        assert_eq!(
            label_frame_offset(Samples(2_000 * 10), Samples(0), 48_000, 24, 10, true),
            0
        );
    }

    #[test]
    fn label_frame_offset_clamps_when_one_shot() {
        // Non-looped (Intro / Confirm) holds the last frame past the duration.
        assert_eq!(
            label_frame_offset(Samples(2_000 * 4), Samples(0), 48_000, 24, 10, false),
            4
        );
        assert_eq!(
            label_frame_offset(Samples(2_000 * 100), Samples(0), 48_000, 24, 10, false),
            9
        );
    }

    #[test]
    fn label_frame_offset_is_relative_to_state_start() {
        // Anchoring at a non-zero start cursor keeps the intro starting from frame 0.
        assert_eq!(
            label_frame_offset(
                Samples(2_000 * 5),
                Samples(2_000 * 5),
                48_000,
                24,
                17,
                false
            ),
            0
        );
        assert_eq!(
            label_frame_offset(
                Samples(2_000 * 7),
                Samples(2_000 * 5),
                48_000,
                24,
                17,
                false
            ),
            2
        );
    }

    #[test]
    fn elapsed_frame_and_duration_use_freeplay_dj_rate() {
        assert_eq!(elapsed_frame(Samples(2_000), Samples(0), 48_000, 24), 1);
        assert_eq!(animation_duration_samples(24, 48_000, 24), 48_000);
        assert_eq!(seconds_to_samples(DJ_IDLE_EGG_SECONDS, 48_000), 2_880_000);
    }

    #[test]
    fn bf_idle_egg_only_plays_once_before_cartoon_state() {
        let mut dj = load_test_dj("images/freeplay/freeplay-boyfriend");

        dj.tick(
            Samples(seconds_to_samples(DJ_IDLE_EGG_SECONDS, 48_000)),
            48_000,
        );
        assert_eq!(dj.state, DjState::IdleEasterEgg);

        let afk_end = Samples(
            seconds_to_samples(DJ_IDLE_EGG_SECONDS, 48_000)
                + animation_duration_samples(dj.afk_frame_count, 48_000, DJ_FPS),
        );
        dj.tick(afk_end, 48_000);
        assert_eq!(dj.state, DjState::Idle);
        assert!(dj.seen_idle_easter_egg);

        dj.tick(
            Samples(seconds_to_samples(DJ_IDLE_CARTOON_SECONDS, 48_000)),
            48_000,
        );
        assert_eq!(dj.state, DjState::Cartoon);
    }

    #[test]
    fn pico_without_cartoon_label_stays_on_idle_egg_path() {
        let mut dj = load_test_dj("images/freeplay/freeplay-pico");
        assert_eq!(dj.cartoon_frame_count, 0);

        dj.tick(
            Samples(seconds_to_samples(DJ_IDLE_CARTOON_SECONDS, 48_000)),
            48_000,
        );

        assert_eq!(dj.state, DjState::IdleEasterEgg);
    }

    #[test]
    fn player_action_leaves_cartoon_and_resets_idle_timer() {
        let mut dj = load_test_dj("images/freeplay/freeplay-boyfriend");
        dj.tick(
            Samples(seconds_to_samples(DJ_IDLE_CARTOON_SECONDS, 48_000)),
            48_000,
        );
        assert_eq!(dj.state, DjState::Cartoon);

        dj.on_player_action(Samples(6_000_000));

        assert_eq!(dj.state, DjState::Idle);
        assert_eq!(dj.idle_started_at, Samples(6_000_000));
        assert!(!dj.seen_idle_easter_egg);
    }

    /// Locks the DJ source asset inventory.
    #[test]
    fn required_assets_present() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest_dir
            .parent()
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| manifest_dir.to_path_buf());
        let source_root = workspace.join("assets/source");
        let mut missing = Vec::new();
        for logical in REQUIRED_DJ_ASSETS {
            let path = source_root.join(logical);
            if !path.exists() {
                missing.push(path.display().to_string());
            }
        }
        assert!(
            missing.is_empty(),
            "freeplay DJ assets missing - DO NOT DELETE these files, they are required for the OG-fidelity DJ:\n{}",
            missing.join("\n"),
        );
    }

    fn load_test_dj(asset_path: &str) -> FreeplayDJ {
        let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
        let animation_path = AssetPath::new(format!("{asset_path}/Animation.json")).unwrap();
        let spritemap_path = AssetPath::new(format!("{asset_path}/spritemap1.json")).unwrap();
        let animation = load_animate_animation(&resolver, &animation_path).unwrap();
        let atlas = load_animate_spritemap(&resolver, &spritemap_path).unwrap();
        let intro_frame_count = animation
            .label(DJ_INTRO_LABEL)
            .map(|label| label.duration.max(1))
            .unwrap_or(1);
        let idle_frame_count = animation
            .label(DJ_IDLE_LABEL)
            .map(|label| label.duration.max(1))
            .unwrap_or(1);
        let confirm_frame_count = animation
            .label(DJ_CONFIRM_LABEL)
            .map(|label| label.duration.max(1))
            .unwrap_or(1);
        let afk_frame_count = animation
            .label(DJ_AFK_LABEL)
            .map(|label| label.duration.max(1))
            .unwrap_or(0);
        let cartoon_frame_count = animation
            .label(DJ_CARTOON_LABEL)
            .map(|label| label.duration.max(1))
            .unwrap_or(0);
        FreeplayDJ {
            texture_id: AssetId::new(1),
            animation,
            atlas,
            intro_frame_count,
            idle_frame_count,
            confirm_frame_count,
            afk_frame_count,
            cartoon_frame_count,
            state: DjState::Idle,
            state_started_at: Samples(0),
            idle_started_at: Samples(0),
            seen_idle_easter_egg: false,
            texture: None,
        }
    }
}
