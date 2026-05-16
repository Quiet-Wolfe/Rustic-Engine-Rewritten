//! Runtime hooks for the Spaghetti Sserafim stage script.
// LINT-ALLOW: long-file Sserafim stage script state and tests stay together.

use crate::character_anim::{CharacterPoseNames, CharacterPoseRequest};
use crate::preview_song::PreviewSong;
use crate::stage_object_asset_helpers::{asset_id_for_path, stage_beat, stage_beat_start};
use rustic_asset::{AssetPath, ChartEventKind, SserafimEvent};
use rustic_core::ids::CameraId;
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::DrawCommand;

const BASE_VISIBLE: [bool; 5] = [true, false, false, false, false];
const BASE_SINGING: [bool; 6] = [false, false, false, false, false, false];
const FLASH_OVERLAY_Z: i32 = 10_001;

#[derive(Debug, Clone)]
pub(crate) struct SserafimStageState {
    active: bool,
    visible: [bool; 5],
    singing: [bool; 6],
    cover_visible: bool,
    beautiful: bool,
    dark: TweenValue,
    truck_lights: TimedFlash,
    pulse: PulseLights,
    yunjin_intro: YunjinIntro,
    dust_clear: Option<Samples>,
    flash: TimedFlash,
    end_started_at: Option<Samples>,
}

impl Default for SserafimStageState {
    fn default() -> Self {
        Self {
            active: false,
            visible: BASE_VISIBLE,
            singing: BASE_SINGING,
            cover_visible: false,
            beautiful: false,
            dark: TweenValue::default(),
            truck_lights: TimedFlash::default(),
            pulse: PulseLights::default(),
            yunjin_intro: YunjinIntro::default(),
            dust_clear: None,
            flash: TimedFlash::default(),
            end_started_at: None,
        }
    }
}

impl SserafimStageState {
    pub(crate) fn reset_for_song(&mut self, song: PreviewSong) {
        *self = Self::default();
        self.active = song == PreviewSong::SPAGHETTI;
    }

    pub(crate) fn active(&self) -> bool {
        self.active
    }

    pub(crate) fn member_sings(&self, member: SserafimMember) -> bool {
        self.member_sings_player(member)
    }

    pub(crate) fn finish_cursor_override(&self) -> Option<Samples> {
        let started_at = self.end_started_at?;
        Some(Samples(
            started_at.0.saturating_add(seconds_to_samples(9.0)),
        ))
    }

    pub(crate) fn end_started_at(&self) -> Option<Samples> {
        self.end_started_at
    }

    pub(crate) fn apply_event(&mut self, kind: &ChartEventKind, cursor: Samples) -> bool {
        let ChartEventKind::Sserafim(event) = kind else {
            return false;
        };
        if !self.active {
            return true;
        }
        match event {
            SserafimEvent::Show { visible } => {
                copy_bool_array(visible, &mut self.visible);
            }
            SserafimEvent::Sing { singing } => {
                copy_bool_array(singing, &mut self.singing);
            }
            SserafimEvent::Dark { amount, duration } => {
                self.dark
                    .begin(self.dark.value_at(cursor), *amount, *duration, cursor);
            }
            SserafimEvent::Lights { amount, duration } => {
                self.truck_lights = TimedFlash::new(*amount, *duration, cursor);
            }
            SserafimEvent::PulseLights {
                enabled,
                colors,
                durations,
                intensities,
            } => {
                self.pulse = PulseLights::new(*enabled, colors, durations, intensities);
            }
            SserafimEvent::Cover { visible } => self.cover_visible = *visible,
            SserafimEvent::Beautiful { beautiful } => self.beautiful = *beautiful,
            SserafimEvent::Kick { final_kick } => {
                self.yunjin_intro = YunjinIntro::kick(*final_kick, cursor);
                if *final_kick {
                    self.dust_clear = Some(cursor);
                }
            }
            SserafimEvent::Flash { duration } => {
                self.flash = TimedFlash::new(1.0, *duration, cursor);
            }
            SserafimEvent::End => {
                self.end_started_at = Some(cursor);
            }
            SserafimEvent::GuitarVibration { .. } => {}
            _ => {}
        }
        true
    }

    pub(crate) fn pose_for_member(
        &self,
        member: SserafimMember,
        poses: CharacterPoseNames,
        cursor: Samples,
    ) -> Option<CharacterPoseRequest> {
        if !self.active || !self.member_visible(member) {
            return None;
        }
        if member == SserafimMember::Yunjin {
            if let Some(request) = self.yunjin_intro.pose_request(cursor) {
                return Some(request);
            }
        }
        let request = if self.member_sings_player(member) {
            poses.player
        } else {
            poses.opponent
        };
        Some(if member == SserafimMember::Girlfriend && self.beautiful {
            CharacterPoseRequest {
                name: beautiful_girlfriend_pose(request.name),
                started_at: request.started_at,
            }
        } else {
            request
        })
    }

    pub(crate) fn apply_commands<'a>(
        &self,
        commands: impl Iterator<Item = &'a mut DrawCommand>,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        if !self.active {
            return;
        }
        let dark = self.dark.value_at(cursor).clamp(0.0, 1.0);
        let truck_alpha = self.truck_lights.alpha_at(cursor);
        let pulse = self.pulse.value_at(cursor, sample_rate, bpm);
        for cmd in commands {
            if self.end_cutscene_active(cursor)
                && matches!(cmd.layer, RenderLayer::Notes | RenderLayer::Hud)
            {
                cmd.color.w = 0.0;
            }
            if matches!(cmd.layer, RenderLayer::Stage | RenderLayer::Characters) && dark > 0.0 {
                cmd.color.x *= 1.0 - dark;
                cmd.color.y *= 1.0 - dark;
                cmd.color.z *= 1.0 - dark;
            }
            if cmd.texture == sserafim_texture_id("generated/stage/solid-000000.png") {
                cmd.color.w = if self.cover_visible || self.end_cover_visible(cursor) {
                    1.0
                } else {
                    0.0
                };
            } else if is_flash_overlay(cmd) {
                apply_sserafim_overlay_camera(cmd);
                cmd.color = glam::vec4(1.0, 1.0, 1.0, self.flash.alpha_at(cursor));
            } else if cmd.texture == sserafim_texture_id("images/sserafim/lights/truck-light1.png")
                || cmd.texture == sserafim_texture_id("images/sserafim/lights/truck-light2.png")
            {
                cmd.color.w = truck_alpha;
            } else if cmd.texture
                == sserafim_texture_id("images/sserafim/lights/back-light-color.png")
            {
                cmd.color = pulse.color;
                cmd.color.w = pulse.alpha;
            } else if cmd.texture
                == sserafim_texture_id("images/sserafim/lights/back-light-white.png")
            {
                cmd.color.w = pulse.alpha * 0.8;
            } else if cmd.texture == sserafim_texture_id("images/sserafim/end/end1.png") {
                apply_sserafim_overlay_camera(cmd);
                cmd.color.w = self.end_card_alpha(cursor, EndCard::First);
            } else if cmd.texture == sserafim_texture_id("images/sserafim/end/end2.png") {
                apply_sserafim_overlay_camera(cmd);
                cmd.color.w = self.end_card_alpha(cursor, EndCard::Second);
            }
            if let Some(started_at) = self.dust_clear {
                apply_dust_clear(cmd, started_at, cursor);
            }
        }
    }

    fn member_visible(&self, member: SserafimMember) -> bool {
        match member.visible_index() {
            Some(index) => self.visible[index],
            None => true,
        }
    }

    fn member_sings_player(&self, member: SserafimMember) -> bool {
        self.singing[member.singing_index()]
    }

    fn end_cover_visible(&self, cursor: Samples) -> bool {
        self.end_elapsed(cursor)
            .is_some_and(|elapsed| elapsed >= seconds_to_samples(0.05))
    }

    fn end_cutscene_active(&self, cursor: Samples) -> bool {
        self.end_cover_visible(cursor)
    }

    fn end_card_alpha(&self, cursor: Samples, card: EndCard) -> f32 {
        let Some(elapsed) = self.end_elapsed(cursor) else {
            return 0.0;
        };
        let first_start = seconds_to_samples(0.05);
        let second_start = seconds_to_samples(4.0);
        let hide_all = seconds_to_samples(8.0);
        match card {
            EndCard::First if (first_start..second_start).contains(&elapsed) => 1.0,
            EndCard::Second if (second_start..hide_all).contains(&elapsed) => 1.0,
            _ => 0.0,
        }
    }

    fn end_elapsed(&self, cursor: Samples) -> Option<i64> {
        let started_at = self.end_started_at?;
        Some(cursor.0.saturating_sub(started_at.0).max(0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SserafimMember {
    Yunjin,
    Kazuha,
    Chaewon,
    Eunchae,
    Sakura,
    Girlfriend,
}

#[derive(Debug, Clone, Copy)]
enum EndCard {
    First,
    Second,
}

impl SserafimMember {
    fn visible_index(self) -> Option<usize> {
        match self {
            Self::Yunjin => Some(0),
            Self::Kazuha => Some(1),
            Self::Chaewon => Some(2),
            Self::Eunchae => Some(3),
            Self::Sakura => Some(4),
            Self::Girlfriend => None,
        }
    }

    fn singing_index(self) -> usize {
        match self {
            Self::Yunjin => 0,
            Self::Kazuha => 1,
            Self::Chaewon => 2,
            Self::Eunchae => 3,
            Self::Sakura => 4,
            Self::Girlfriend => 5,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TweenValue {
    from: f32,
    to: f32,
    started_at: Samples,
    duration_samples: i64,
}

impl Default for TweenValue {
    fn default() -> Self {
        Self {
            from: 0.0,
            to: 0.0,
            started_at: Samples(0),
            duration_samples: 0,
        }
    }
}

impl TweenValue {
    fn begin(&mut self, from: f32, to: f32, duration_seconds: f32, started_at: Samples) {
        self.from = from;
        self.to = to;
        self.started_at = started_at;
        self.duration_samples = seconds_to_samples(duration_seconds);
    }

    fn value_at(self, cursor: Samples) -> f32 {
        if self.duration_samples <= 0 {
            return self.to;
        }
        let elapsed = cursor.0.saturating_sub(self.started_at.0).max(0);
        let progress = (elapsed as f32 / self.duration_samples as f32).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * progress
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TimedFlash {
    amount: f32,
    duration_samples: i64,
    started_at: Samples,
}

impl TimedFlash {
    fn new(amount: f32, duration_seconds: f32, started_at: Samples) -> Self {
        Self {
            amount,
            duration_samples: seconds_to_samples(duration_seconds),
            started_at,
        }
    }

    fn alpha_at(self, cursor: Samples) -> f32 {
        if self.duration_samples <= 0 {
            return 0.0;
        }
        let elapsed = cursor.0.saturating_sub(self.started_at.0).max(0);
        if elapsed >= self.duration_samples {
            return 0.0;
        }
        self.amount * (1.0 - elapsed as f32 / self.duration_samples as f32)
    }
}

#[derive(Debug, Clone, Default)]
struct PulseLights {
    enabled: bool,
    colors: Vec<glam::Vec4>,
    durations: Vec<f32>,
    intensities: Vec<f32>,
}

impl PulseLights {
    fn new(enabled: bool, colors: &[String], durations: &[f32], intensities: &[f32]) -> Self {
        Self {
            enabled,
            colors: colors
                .iter()
                .map(|color| parse_sserafim_color(color))
                .collect(),
            durations: durations.to_vec(),
            intensities: intensities.to_vec(),
        }
    }

    fn value_at(&self, cursor: Samples, sample_rate: u32, bpm: f64) -> PulseValue {
        if !self.enabled {
            return PulseValue::default();
        }
        let beat = stage_beat(cursor, sample_rate, bpm);
        let index = beat.max(0) as usize;
        let beat_start = stage_beat_start(cursor, sample_rate, bpm);
        let duration = self
            .durations
            .get(index % self.durations.len().max(1))
            .copied()
            .unwrap_or(0.5);
        let duration_samples = seconds_to_samples(duration).max(1);
        let elapsed = cursor.0.saturating_sub(beat_start.0).max(0);
        let fade = (1.0 - elapsed as f32 / duration_samples as f32).clamp(0.0, 1.0);
        let intensity = self
            .intensities
            .get(index % self.intensities.len().max(1))
            .copied()
            .unwrap_or(1.0);
        let color = self
            .colors
            .get(index % self.colors.len().max(1))
            .copied()
            .unwrap_or(glam::Vec4::ONE);
        PulseValue {
            color,
            alpha: (intensity * fade).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PulseValue {
    color: glam::Vec4,
    alpha: f32,
}

impl Default for PulseValue {
    fn default() -> Self {
        Self {
            color: glam::Vec4::ONE,
            alpha: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct YunjinIntro {
    pose: &'static str,
    started_at: Samples,
    final_kick: bool,
    kick: bool,
}

impl Default for YunjinIntro {
    fn default() -> Self {
        Self {
            pose: "doorclosed",
            started_at: Samples(0),
            final_kick: false,
            kick: false,
        }
    }
}

impl YunjinIntro {
    fn kick(final_kick: bool, started_at: Samples) -> Self {
        Self {
            pose: if final_kick { "kick2" } else { "kick1" },
            started_at,
            final_kick,
            kick: true,
        }
    }

    fn pose_request(self, cursor: Samples) -> Option<CharacterPoseRequest> {
        let kick_hold = seconds_to_samples(1.2);
        if !self.kick || cursor.0.saturating_sub(self.started_at.0) < kick_hold {
            return Some(CharacterPoseRequest {
                name: self.pose,
                started_at: self.started_at,
            });
        }
        if self.final_kick {
            None
        } else {
            Some(CharacterPoseRequest {
                name: "doorclosed",
                started_at: Samples(self.started_at.0.saturating_add(kick_hold)),
            })
        }
    }
}

fn copy_bool_array(values: &[bool], target: &mut [bool]) {
    for (slot, value) in target.iter_mut().zip(values.iter().copied()) {
        *slot = value;
    }
}

fn beautiful_girlfriend_pose(name: &'static str) -> &'static str {
    match name {
        "danceLeft" => "danceLeft-beautiful",
        "danceRight" => "danceRight-beautiful",
        "singLEFT" => "singLEFT-beautiful",
        "singDOWN" => "singDOWN-beautiful",
        "singUP" => "singUP-beautiful",
        "singRIGHT" => "singRIGHT-beautiful",
        "singLEFTmiss" => "singLEFTmiss-beautiful",
        "singDOWNmiss" => "singDOWNmiss-beautiful",
        "singUPmiss" => "singUPmiss-beautiful",
        "singRIGHTmiss" => "singRIGHTmiss-beautiful",
        _ => name,
    }
}

fn seconds_to_samples(seconds: f32) -> i64 {
    (seconds.max(0.0) * 48_000.0).round() as i64
}

fn parse_sserafim_color(value: &str) -> glam::Vec4 {
    let hex = value
        .trim()
        .trim_start_matches('#')
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    let parsed = u32::from_str_radix(hex, 16).unwrap_or(0xffff_ffff);
    let (red, green, blue, alpha) = if hex.len() > 6 {
        (
            ((parsed >> 16) & 0xff) as u8,
            ((parsed >> 8) & 0xff) as u8,
            (parsed & 0xff) as u8,
            ((parsed >> 24) & 0xff) as u8,
        )
    } else {
        (
            ((parsed >> 16) & 0xff) as u8,
            ((parsed >> 8) & 0xff) as u8,
            (parsed & 0xff) as u8,
            0xff,
        )
    };
    glam::vec4(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        f32::from(alpha) / 255.0,
    )
}

#[derive(Debug, Clone, Copy)]
struct DustClearSpec {
    duration_samples: i64,
    y_offset: f32,
}

fn apply_dust_clear(cmd: &mut DrawCommand, started_at: Samples, cursor: Samples) {
    let Some(spec) = dust_clear_spec(cmd) else {
        return;
    };
    let elapsed = cursor.0.saturating_sub(started_at.0).max(0);
    let progress = (elapsed as f32 / spec.duration_samples as f32).clamp(0.0, 1.0);
    cmd.color.w *= 1.0 - progress;
    cmd.world_pos.y += spec.y_offset * progress;
}

fn is_flash_overlay(cmd: &DrawCommand) -> bool {
    cmd.texture == sserafim_texture_id("generated/stage/solid-FFFFFF.png")
        && cmd.layer == RenderLayer::Overlay
        && cmd.z == FLASH_OVERLAY_Z
}

fn apply_sserafim_overlay_camera(cmd: &mut DrawCommand) {
    cmd.camera = CameraId(2);
    cmd.layer = RenderLayer::Overlay;
}

fn dust_clear_spec(cmd: &DrawCommand) -> Option<DustClearSpec> {
    if cmd.texture == sserafim_texture_id("images/sserafim/dust/dustMid.png") {
        return Some(if cmd.world_pos.y > -300.0 {
            DustClearSpec {
                duration_samples: seconds_to_samples(20.0),
                y_offset: 100.0,
            }
        } else {
            DustClearSpec {
                duration_samples: seconds_to_samples(24.0),
                y_offset: 150.0,
            }
        });
    }
    if cmd.texture == sserafim_texture_id("images/sserafim/dust/dustBack.png") {
        return Some(if cmd.world_pos.y > -800.0 {
            DustClearSpec {
                duration_samples: seconds_to_samples(16.0),
                y_offset: 200.0,
            }
        } else {
            DustClearSpec {
                duration_samples: seconds_to_samples(16.0),
                y_offset: 100.0,
            }
        });
    }
    None
}

fn sserafim_texture_id(path: &str) -> rustic_core::ids::AssetId {
    asset_id_for_path(&AssetPath::new(path).expect("valid built-in Sserafim asset path"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poses() -> CharacterPoseNames {
        CharacterPoseNames {
            girlfriend: CharacterPoseRequest {
                name: "danceLeft",
                started_at: Samples(1),
            },
            opponent: CharacterPoseRequest {
                name: "singRIGHT",
                started_at: Samples(2),
            },
            player: CharacterPoseRequest {
                name: "singLEFT",
                started_at: Samples(3),
            },
        }
    }

    #[test]
    fn show_and_sing_events_route_visible_members() {
        let mut state = SserafimStageState::default();
        state.reset_for_song(PreviewSong::SPAGHETTI);
        state.apply_event(
            &ChartEventKind::Sserafim(SserafimEvent::Show {
                visible: vec![true, true, false, false, true],
            }),
            Samples(0),
        );
        state.apply_event(
            &ChartEventKind::Sserafim(SserafimEvent::Sing {
                singing: vec![false, true, false, false, true, false],
            }),
            Samples(0),
        );

        assert_eq!(
            state
                .pose_for_member(SserafimMember::Kazuha, poses(), Samples(0))
                .unwrap()
                .name,
            "singLEFT"
        );
        assert!(state
            .pose_for_member(SserafimMember::Chaewon, poses(), Samples(0))
            .is_none());
    }

    #[test]
    fn beautiful_event_suffixes_girlfriend_poses() {
        let mut state = SserafimStageState::default();
        state.reset_for_song(PreviewSong::SPAGHETTI);
        state.apply_event(
            &ChartEventKind::Sserafim(SserafimEvent::Beautiful { beautiful: true }),
            Samples(0),
        );

        assert_eq!(
            state
                .pose_for_member(SserafimMember::Girlfriend, poses(), Samples(0))
                .unwrap()
                .name,
            "singRIGHT-beautiful"
        );
    }

    #[test]
    fn final_kick_clears_sserafim_dust() {
        let mut state = SserafimStageState::default();
        state.reset_for_song(PreviewSong::SPAGHETTI);
        state.apply_event(
            &ChartEventKind::Sserafim(SserafimEvent::Kick { final_kick: true }),
            Samples(0),
        );

        let mut dust = DrawCommand::sprite(
            sserafim_texture_id("images/sserafim/dust/dustMid.png"),
            glam::vec2(-650.0, -200.0),
            glam::vec2(100.0, 100.0),
        );
        dust.layer = RenderLayer::Stage;
        dust.color.w = 0.8;

        state.apply_commands(
            std::iter::once(&mut dust),
            Samples(10 * 48_000),
            48_000,
            100.0,
        );

        assert!(dust.color.w < 0.8);
        assert!(dust.world_pos.y > -200.0);
    }

    #[test]
    fn flash_event_only_drives_flash_overlay() {
        let mut state = SserafimStageState::default();
        state.reset_for_song(PreviewSong::SPAGHETTI);
        state.apply_event(
            &ChartEventKind::Sserafim(SserafimEvent::Flash { duration: 1.6 }),
            Samples(0),
        );

        let mut flash = DrawCommand::sprite(
            sserafim_texture_id("generated/stage/solid-FFFFFF.png"),
            glam::Vec2::ZERO,
            glam::vec2(1280.0, 720.0),
        );
        flash.layer = RenderLayer::Overlay;
        flash.z = FLASH_OVERLAY_Z;
        flash.color.w = 0.0;
        let mut white_background = DrawCommand::sprite(
            sserafim_texture_id("generated/stage/solid-FFFFFF.png"),
            glam::vec2(-5000.0, -3000.0),
            glam::vec2(10_000.0, 10_000.0),
        );
        white_background.layer = RenderLayer::Stage;
        white_background.color.w = 1.0;

        state.apply_commands(
            [&mut flash, &mut white_background].into_iter(),
            Samples(38_400),
            48_000,
            100.0,
        );

        assert!((flash.color.w - 0.5).abs() < 0.001);
        assert_eq!(flash.camera, CameraId(2));
        assert_eq!(white_background.color.w, 1.0);
    }

    #[test]
    fn end_event_shows_sserafim_cards_and_cover_on_timers() {
        let mut state = SserafimStageState::default();
        state.reset_for_song(PreviewSong::SPAGHETTI);
        state.apply_event(
            &ChartEventKind::Sserafim(SserafimEvent::End),
            Samples(1_000),
        );
        let mut cover = DrawCommand::sprite(
            sserafim_texture_id("generated/stage/solid-000000.png"),
            glam::Vec2::ZERO,
            glam::Vec2::ONE,
        );
        let mut first = DrawCommand::sprite(
            sserafim_texture_id("images/sserafim/end/end1.png"),
            glam::Vec2::ZERO,
            glam::Vec2::ONE,
        );
        let mut second = DrawCommand::sprite(
            sserafim_texture_id("images/sserafim/end/end2.png"),
            glam::Vec2::ZERO,
            glam::Vec2::ONE,
        );

        state.apply_commands(
            [&mut cover, &mut first, &mut second].into_iter(),
            Samples(49_000),
            48_000,
            100.0,
        );

        assert_eq!(cover.color.w, 1.0);
        assert_eq!(first.color.w, 1.0);
        assert_eq!(first.camera, CameraId(2));
        assert_eq!(second.color.w, 0.0);

        state.apply_commands(
            [&mut cover, &mut first, &mut second].into_iter(),
            Samples(5 * 48_000 + 1_000),
            48_000,
            100.0,
        );

        assert_eq!(first.color.w, 0.0);
        assert_eq!(second.color.w, 1.0);
        assert_eq!(second.camera, CameraId(2));
    }

    #[test]
    fn end_cutscene_hides_gameplay_layers_until_scripted_finish() {
        let mut state = SserafimStageState::default();
        state.reset_for_song(PreviewSong::SPAGHETTI);
        state.apply_event(
            &ChartEventKind::Sserafim(SserafimEvent::End),
            Samples(1_000),
        );
        let mut note = DrawCommand::sprite(
            sserafim_texture_id("images/NOTE_assets.png"),
            glam::Vec2::ZERO,
            glam::Vec2::ONE,
        );
        note.layer = RenderLayer::Notes;
        let mut hud = DrawCommand::sprite(
            sserafim_texture_id("images/healthBar.png"),
            glam::Vec2::ZERO,
            glam::Vec2::ONE,
        );
        hud.layer = RenderLayer::Hud;

        state.apply_commands(
            [&mut note, &mut hud].into_iter(),
            Samples(4_000),
            48_000,
            100.0,
        );

        assert_eq!(note.color.w, 0.0);
        assert_eq!(hud.color.w, 0.0);
        assert_eq!(state.finish_cursor_override(), Some(Samples(433_000)));
    }
}
