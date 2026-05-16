//! Runtime spraycan effects for Weekend 1 `2hot`.
//!
//! ref: bdedc0aa:assets/preload/scripts/songs/2hot.hxc:onNoteHit
//! ref: bdedc0aa:assets/preload/scripts/stages/props/SpraycanAtlasSprite.hxc

use crate::asset_roots::app_asset_resolver;
use crate::preview_song::PreviewSong;
use crate::stage_object_asset_helpers::asset_id_for_path;
use anyhow::{Context, Result};
use rustic_asset::{
    load_animate_animation, load_animate_spritemap, load_png, load_sparrow, AnimateAnimation,
    AnimateAtlas, AnimateDrawPart, AssetPath, OverlayResolver, SparrowFrame,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;

const CAN_FPS: u16 = 24;
const CAN_START_LABEL: &str = "Can Start";
const CAN_HIT_PICO_LABEL: &str = "Hit Pico";
const CAN_SHOT_LABEL: &str = "Can Shot";
const CAN_START_FALLBACK_FRAMES: u32 = 19;
const CAN_HIT_PICO_FALLBACK_FRAMES: u32 = 7;
const CAN_SHOT_FALLBACK_FRAMES: u32 = 17;
const SHOT_EXPLOSION_FRAME: u32 = 3;
const CAN_POSITION: glam::Vec2 = glam::vec2(910.0, 495.0);
const CAN_Z: i32 = 689;
const EXPLOSION_Z: i32 = CAN_Z + 2;
const STAGE_DARKEN_DELAY_FRAMES: u32 = 1;
const STAGE_DARKEN_HOLD_FRAMES: u32 = 1;
const STAGE_DARKEN_TWEEN_SECONDS: f32 = 1.4;

#[derive(Debug, Default)]
pub(crate) struct Weekend1CanEffects {
    active: bool,
    assets: Option<Weekend1CanAssets>,
    cans: Vec<CanInstance>,
    effects: Vec<SparrowEffectInstance>,
    stage_darken_started_at: Option<Samples>,
}

impl Weekend1CanEffects {
    pub(crate) fn reset_for_song(&mut self, song: PreviewSong) {
        self.active = song == PreviewSong::TWO_HOT;
        self.cans.clear();
        self.effects.clear();
        self.stage_darken_started_at = None;
        if !self.active {
            self.assets = None;
        }
    }

    pub(crate) fn clear(&mut self) {
        self.active = false;
        self.assets = None;
        self.cans.clear();
        self.effects.clear();
        self.stage_darken_started_at = None;
    }

    pub(crate) fn load_assets(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &mut HashMap<AssetId, Texture>,
    ) -> Result<()> {
        if !self.active {
            return Ok(());
        }
        let assets = Weekend1CanAssets::load(device, queue, textures)?;
        self.assets = Some(assets);
        Ok(())
    }

    pub(crate) fn spawn_can(&mut self, cursor: Samples) {
        if !self.active {
            return;
        }
        self.cans.push(CanInstance {
            state: CanState::Arcing,
            label: CanLabel::Start,
            position: CAN_POSITION,
            started_at: cursor,
            shot_explosion_spawned: false,
        });
    }

    pub(crate) fn shoot_next_can(&mut self, cursor: Samples, sample_rate: u32) {
        if !self.active {
            return;
        }
        self.tick(cursor, sample_rate);
        if let Some(can) = self
            .cans
            .iter_mut()
            .find(|can| can.state == CanState::Arcing)
        {
            can.state = CanState::Shot;
            can.label = CanLabel::Shot;
            can.started_at = cursor;
            can.shot_explosion_spawned = false;
            self.stage_darken_started_at = Some(Samples(
                cursor
                    .0
                    .saturating_add(frames_to_samples(STAGE_DARKEN_DELAY_FRAMES, sample_rate)),
            ));
        }
    }

    pub(crate) fn impact_next_can(&mut self, cursor: Samples, sample_rate: u32) {
        if !self.active {
            return;
        }
        self.tick(cursor, sample_rate);
        if let Some(can) = self
            .cans
            .iter_mut()
            .find(|can| can.state == CanState::Arcing)
        {
            can.state = CanState::Impacted;
        }
    }

    pub(crate) fn append_commands(
        &mut self,
        commands: &mut RenderCommandList,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if !self.active {
            return;
        }
        self.tick(cursor, sample_rate);
        let Some(assets) = self.assets.as_ref() else {
            return;
        };
        for can in &self.cans {
            let frame_offset = label_frame_offset(
                cursor,
                can.started_at,
                sample_rate,
                can.frame_count(assets),
                false,
            );
            let Ok(parts) = assets
                .can_animation
                .flatten_label_frame(can.label.as_str(), frame_offset)
            else {
                continue;
            };
            for part in parts {
                if let Some(cmd) = assets.command_for_can_part(&part, can.position) {
                    commands.push(cmd);
                }
            }
        }
        for effect in &self.effects {
            if let Some(cmd) = assets.command_for_effect(effect, cursor, sample_rate) {
                commands.push(cmd);
            }
        }
    }

    pub(crate) fn apply_stage_dimming<'a>(
        &self,
        commands: impl Iterator<Item = &'a mut DrawCommand>,
        cursor: Samples,
        sample_rate: u32,
    ) {
        let Some(started_at) = self.stage_darken_started_at else {
            return;
        };
        let factor = stage_darken_factor(cursor, started_at, sample_rate);
        if factor >= 0.999 {
            return;
        }
        for cmd in commands {
            if cmd.layer == RenderLayer::Stage {
                cmd.color.x *= factor;
                cmd.color.y *= factor;
                cmd.color.z *= factor;
            }
        }
    }

    fn tick(&mut self, cursor: Samples, sample_rate: u32) {
        self.tick_cans(cursor, sample_rate);
        self.tick_effects(cursor, sample_rate);
    }

    fn tick_cans(&mut self, cursor: Samples, sample_rate: u32) {
        let durations = CanDurations::from_assets(self.assets.as_ref());
        let mut active = Vec::with_capacity(self.cans.len());
        let mut pending_effects = Vec::new();
        for mut can in self.cans.drain(..) {
            if can.label == CanLabel::Start {
                let start_duration = durations.start_samples(sample_rate);
                if cursor.0 >= can.started_at.0.saturating_add(start_duration) {
                    can.label = CanLabel::HitPico;
                    can.state = CanState::Impacted;
                    can.started_at = Samples(can.started_at.0.saturating_add(start_duration));
                }
            }
            let finished = match can.label {
                CanLabel::Start => false,
                CanLabel::Shot => {
                    maybe_spawn_shot_explosion(&mut can, cursor, sample_rate, &mut pending_effects);
                    cursor.0
                        >= can
                            .started_at
                            .0
                            .saturating_add(durations.shot_samples(sample_rate))
                }
                CanLabel::HitPico => {
                    let hit_end = can
                        .started_at
                        .0
                        .saturating_add(durations.hit_pico_samples(sample_rate));
                    if cursor.0 >= hit_end {
                        pending_effects.push(SparrowEffectInstance {
                            kind: SparrowEffectKind::PicoHitExplosion,
                            position: can.position + glam::vec2(750.0, -100.0),
                            started_at: Samples(hit_end),
                        });
                        true
                    } else {
                        false
                    }
                }
            };
            if !finished {
                active.push(can);
            }
        }
        self.cans = active;
        self.effects.extend(pending_effects);
    }

    fn tick_effects(&mut self, cursor: Samples, sample_rate: u32) {
        let Some(assets) = self.assets.as_ref() else {
            return;
        };
        self.effects.retain(|effect| {
            let clip = assets.effect_clip(effect.kind);
            cursor.0
                < effect
                    .started_at
                    .0
                    .saturating_add(frames_to_samples(clip.frames.len() as u32, sample_rate))
        });
    }

    #[cfg(test)]
    fn active_can_labels(&self) -> Vec<CanLabel> {
        self.cans.iter().map(|can| can.label).collect()
    }

    #[cfg(test)]
    fn active_can_states(&self) -> Vec<CanState> {
        self.cans.iter().map(|can| can.state).collect()
    }
}

#[derive(Debug)]
struct Weekend1CanAssets {
    can_texture_id: AssetId,
    can_animation: AnimateAnimation,
    can_atlas: AnimateAtlas,
    can_start_frames: u32,
    can_hit_pico_frames: u32,
    can_shot_frames: u32,
    shot_explosion: SparrowEffectClip,
    pico_hit_explosion: SparrowEffectClip,
}

impl Weekend1CanAssets {
    fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &mut HashMap<AssetId, Texture>,
    ) -> Result<Self> {
        let resolver = app_asset_resolver();
        let can_animation_path = AssetPath::new("images/spraycanAtlas/Animation.json")?;
        let can_spritemap_path = AssetPath::new("images/spraycanAtlas/spritemap1.json")?;
        let can_texture_path = AssetPath::new("images/spraycanAtlas/spritemap1.png")?;
        let can_animation = load_animate_animation(&resolver, &can_animation_path)
            .with_context(|| format!("load {}", can_animation_path.as_str()))?;
        let can_atlas = load_animate_spritemap(&resolver, &can_spritemap_path)
            .with_context(|| format!("load {}", can_spritemap_path.as_str()))?;
        let can_image = load_png(&resolver, &can_texture_path)
            .with_context(|| format!("load {}", can_texture_path.as_str()))?;
        let can_texture_id = asset_id_for_path(&can_texture_path);
        let can_texture = Texture::from_png_image(
            device,
            queue,
            &can_image,
            FilterMode::Linear,
            Some(can_texture_path.as_str()),
        );
        textures.insert(can_texture_id, can_texture);

        Ok(Self {
            can_start_frames: label_duration_or(
                &can_animation,
                CAN_START_LABEL,
                CAN_START_FALLBACK_FRAMES,
            ),
            can_hit_pico_frames: label_duration_or(
                &can_animation,
                CAN_HIT_PICO_LABEL,
                CAN_HIT_PICO_FALLBACK_FRAMES,
            ),
            can_shot_frames: label_duration_or(
                &can_animation,
                CAN_SHOT_LABEL,
                CAN_SHOT_FALLBACK_FRAMES,
            ),
            can_texture_id,
            can_animation,
            can_atlas,
            shot_explosion: SparrowEffectClip::load(
                device,
                queue,
                textures,
                &resolver,
                "images/SpraypaintExplosion.xml",
                "Explosion 1 movie0",
            )?,
            pico_hit_explosion: SparrowEffectClip::load(
                device,
                queue,
                textures,
                &resolver,
                "images/spraypaintExplosionEZ.xml",
                "explosion round 1 short0",
            )?,
        })
    }

    fn command_for_can_part(
        &self,
        part: &AnimateDrawPart,
        position: glam::Vec2,
    ) -> Option<DrawCommand> {
        let frame = self.can_atlas.frame(&part.frame_name)?;
        let mut cmd = DrawCommand::sprite(
            self.can_texture_id,
            position,
            glam::vec2(frame.size.x, frame.size.y),
        );
        cmd.camera = CameraId(0);
        cmd.layer = RenderLayer::Characters;
        cmd.z = CAN_Z;
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

    fn command_for_effect(
        &self,
        effect: &SparrowEffectInstance,
        cursor: Samples,
        sample_rate: u32,
    ) -> Option<DrawCommand> {
        let clip = self.effect_clip(effect.kind);
        clip.command(effect.position, cursor, effect.started_at, sample_rate)
    }

    fn effect_clip(&self, kind: SparrowEffectKind) -> &SparrowEffectClip {
        match kind {
            SparrowEffectKind::ShotExplosion => &self.shot_explosion,
            SparrowEffectKind::PicoHitExplosion => &self.pico_hit_explosion,
        }
    }
}

#[derive(Debug)]
struct SparrowEffectClip {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frames: Vec<SparrowFrame>,
}

impl SparrowEffectClip {
    fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &mut HashMap<AssetId, Texture>,
        resolver: &OverlayResolver,
        atlas_path: &str,
        prefix: &str,
    ) -> Result<Self> {
        let atlas_path = AssetPath::new(atlas_path)?;
        let atlas =
            load_sparrow(resolver, &atlas_path).with_context(|| format!("load {atlas_path}"))?;
        let texture_path = sparrow_texture_path(&atlas_path, &atlas)?;
        let image =
            load_png(resolver, &texture_path).with_context(|| format!("load {texture_path}"))?;
        let texture_id = asset_id_for_path(&texture_path);
        let texture = Texture::from_png_image(
            device,
            queue,
            &image,
            FilterMode::Linear,
            Some(texture_path.as_str()),
        );
        textures.insert(texture_id, texture);
        let frames: Vec<_> = atlas
            .animation_frames(prefix, &[])
            .into_iter()
            .cloned()
            .collect();
        if frames.is_empty() {
            anyhow::bail!("resolve Weekend 1 spraycan effect frame {prefix}");
        }
        Ok(Self {
            texture_id,
            texture_width: image.width,
            texture_height: image.height,
            frames,
        })
    }

    fn command(
        &self,
        position: glam::Vec2,
        cursor: Samples,
        started_at: Samples,
        sample_rate: u32,
    ) -> Option<DrawCommand> {
        let frame = frame_for_cursor(&self.frames, cursor, started_at, sample_rate)?;
        let draw_pos = position - frame_trim_offset(frame);
        let mut cmd = DrawCommand::sprite(self.texture_id, draw_pos, frame_draw_size(frame));
        cmd.camera = CameraId(0);
        cmd.layer = RenderLayer::Characters;
        cmd.z = EXPLOSION_Z;
        cmd.pivot = glam::Vec2::ZERO;
        cmd.filter = FilterMode::Linear;
        (cmd.uv_min, cmd.uv_max) = frame_uv(frame, self.texture_width, self.texture_height);
        cmd.uv_rotated = frame.rotated;
        Some(cmd)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanState {
    Arcing,
    Shot,
    Impacted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanLabel {
    Start,
    Shot,
    HitPico,
}

impl CanLabel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Start => CAN_START_LABEL,
            Self::Shot => CAN_SHOT_LABEL,
            Self::HitPico => CAN_HIT_PICO_LABEL,
        }
    }
}

#[derive(Debug, Clone)]
struct CanInstance {
    state: CanState,
    label: CanLabel,
    position: glam::Vec2,
    started_at: Samples,
    shot_explosion_spawned: bool,
}

impl CanInstance {
    fn frame_count(&self, assets: &Weekend1CanAssets) -> u32 {
        match self.label {
            CanLabel::Start => assets.can_start_frames,
            CanLabel::Shot => assets.can_shot_frames,
            CanLabel::HitPico => assets.can_hit_pico_frames,
        }
        .max(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SparrowEffectKind {
    ShotExplosion,
    PicoHitExplosion,
}

#[derive(Debug, Clone, Copy)]
struct SparrowEffectInstance {
    kind: SparrowEffectKind,
    position: glam::Vec2,
    started_at: Samples,
}

#[derive(Debug, Clone, Copy)]
struct CanDurations {
    start_frames: u32,
    hit_pico_frames: u32,
    shot_frames: u32,
}

impl CanDurations {
    fn from_assets(assets: Option<&Weekend1CanAssets>) -> Self {
        assets.map_or_else(
            || Self {
                start_frames: CAN_START_FALLBACK_FRAMES,
                hit_pico_frames: CAN_HIT_PICO_FALLBACK_FRAMES,
                shot_frames: CAN_SHOT_FALLBACK_FRAMES,
            },
            |assets| Self {
                start_frames: assets.can_start_frames,
                hit_pico_frames: assets.can_hit_pico_frames,
                shot_frames: assets.can_shot_frames,
            },
        )
    }

    fn start_samples(self, sample_rate: u32) -> i64 {
        frames_to_samples(self.start_frames, sample_rate)
    }

    fn hit_pico_samples(self, sample_rate: u32) -> i64 {
        frames_to_samples(self.hit_pico_frames, sample_rate)
    }

    fn shot_samples(self, sample_rate: u32) -> i64 {
        frames_to_samples(self.shot_frames, sample_rate)
    }
}

fn maybe_spawn_shot_explosion(
    can: &mut CanInstance,
    cursor: Samples,
    sample_rate: u32,
    pending_effects: &mut Vec<SparrowEffectInstance>,
) {
    if can.shot_explosion_spawned {
        return;
    }
    let explosion_at = Samples(
        can.started_at
            .0
            .saturating_add(frames_to_samples(SHOT_EXPLOSION_FRAME, sample_rate)),
    );
    if cursor < explosion_at {
        return;
    }
    can.shot_explosion_spawned = true;
    pending_effects.push(SparrowEffectInstance {
        kind: SparrowEffectKind::ShotExplosion,
        position: can.position + glam::vec2(150.0, -250.0),
        started_at: explosion_at,
    });
}

fn label_duration_or(animation: &AnimateAnimation, label: &str, fallback: u32) -> u32 {
    animation
        .label(label)
        .map(|label| label.duration.max(1))
        .unwrap_or(fallback)
}

fn label_frame_offset(
    cursor: Samples,
    state_started_at: Samples,
    sample_rate: u32,
    frame_count: u32,
    looped: bool,
) -> u32 {
    if frame_count <= 1 {
        return 0;
    }
    let frame = elapsed_frame(cursor, state_started_at, sample_rate);
    if looped {
        frame % frame_count.max(1)
    } else {
        frame.min(frame_count.saturating_sub(1))
    }
}

fn elapsed_frame(cursor: Samples, state_started_at: Samples, sample_rate: u32) -> u32 {
    let elapsed = cursor.0.saturating_sub(state_started_at.0).max(0) as u128;
    (elapsed * u128::from(CAN_FPS) / u128::from(sample_rate.max(1))) as u32
}

fn frames_to_samples(frames: u32, sample_rate: u32) -> i64 {
    (u128::from(frames.max(1)) * u128::from(sample_rate.max(1)) / u128::from(CAN_FPS.max(1))) as i64
}

fn stage_darken_factor(cursor: Samples, started_at: Samples, sample_rate: u32) -> f32 {
    if cursor < started_at {
        return 1.0;
    }
    let hold = frames_to_samples(STAGE_DARKEN_HOLD_FRAMES, sample_rate);
    let elapsed = cursor.0.saturating_sub(started_at.0);
    if elapsed <= hold {
        return 0x11 as f32 / 255.0;
    }
    let tween_start = started_at.0.saturating_add(hold);
    let tween_samples = seconds_to_samples(STAGE_DARKEN_TWEEN_SECONDS, sample_rate).max(1);
    let progress = cursor.0.saturating_sub(tween_start).max(0) as f32 / tween_samples as f32;
    let from = 0x22 as f32 / 255.0;
    from + (1.0 - from) * progress.clamp(0.0, 1.0)
}

fn seconds_to_samples(seconds: f32, sample_rate: u32) -> i64 {
    (seconds.max(0.0) * sample_rate.max(1) as f32).round() as i64
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
    let index = elapsed_frame(cursor, started_at, sample_rate) as usize;
    frames.get(index.min(frames.len().saturating_sub(1)))
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

fn sparrow_texture_path(
    atlas_path: &AssetPath,
    atlas: &rustic_asset::SparrowAtlas,
) -> Result<AssetPath> {
    if atlas.image_path.contains('/') {
        Ok(AssetPath::new(atlas.image_path.clone())?)
    } else {
        Ok(atlas_path.sibling(&atlas.image_path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 48_000;

    #[test]
    fn can_start_auto_impacts_after_scripted_duration() {
        let mut effects = Weekend1CanEffects::default();
        effects.reset_for_song(PreviewSong::TWO_HOT);
        effects.spawn_can(Samples(0));

        effects.tick(
            Samples(frames_to_samples(CAN_START_FALLBACK_FRAMES, SAMPLE_RATE)),
            SAMPLE_RATE,
        );

        assert_eq!(effects.active_can_labels(), vec![CanLabel::HitPico]);
        assert_eq!(effects.active_can_states(), vec![CanState::Impacted]);
    }

    #[test]
    fn firegun_switches_next_arcing_can_to_shot() {
        let mut effects = Weekend1CanEffects::default();
        effects.reset_for_song(PreviewSong::TWO_HOT);
        effects.spawn_can(Samples(0));

        effects.shoot_next_can(Samples(1_000), SAMPLE_RATE);

        assert_eq!(effects.active_can_labels(), vec![CanLabel::Shot]);
        assert_eq!(effects.active_can_states(), vec![CanState::Shot]);
        assert!(effects.stage_darken_started_at.is_some());
    }

    #[test]
    fn missed_firegun_blocks_late_shot_until_hit_pico_finishes() {
        let mut effects = Weekend1CanEffects::default();
        effects.reset_for_song(PreviewSong::TWO_HOT);
        effects.spawn_can(Samples(0));

        effects.impact_next_can(Samples(1_000), SAMPLE_RATE);
        effects.shoot_next_can(Samples(2_000), SAMPLE_RATE);

        assert_eq!(effects.active_can_labels(), vec![CanLabel::Start]);
        assert_eq!(effects.active_can_states(), vec![CanState::Impacted]);
    }

    #[test]
    fn source_spraycan_atlas_has_script_labels() {
        let resolver = app_asset_resolver();
        let path = AssetPath::new("images/spraycanAtlas/Animation.json").unwrap();
        let animation = load_animate_animation(&resolver, &path).unwrap();

        assert_eq!(
            label_duration_or(&animation, CAN_START_LABEL, 0),
            CAN_START_FALLBACK_FRAMES
        );
        assert_eq!(
            label_duration_or(&animation, CAN_HIT_PICO_LABEL, 0),
            CAN_HIT_PICO_FALLBACK_FRAMES
        );
        assert_eq!(
            label_duration_or(&animation, CAN_SHOT_LABEL, 0),
            CAN_SHOT_FALLBACK_FRAMES
        );
    }
}
