//! Winter Horrorland opening lights cutscene.
//!
//! ref: bdedc0aa:assets/preload/scripts/songs/winter-horrorland.hxc:45-93

use crate::camera_fx::CameraFx;
use crate::pause_menu::PAUSE_OVERLAY_TEXTURE_ID;
use crate::preview_song::{PreviewSelection, PreviewSong};
use crate::stage_sfx::play_winter_horrorland_lights_sound_or_warn;
use rustic_audio::SharedMixer;
use rustic_core::ids::CameraId;
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{CameraRegistry, DrawCommand, FilterMode, RenderCommandList};

const BLACK_DELAY_SECONDS: f64 = 0.1;
const LIGHTS_TURN_ON_SECONDS: f64 = 4.643_129;
const HUD_FADE_SECONDS: f64 = 2.0;
const CUTSCENE_CAMERA_POSITION: glam::Vec2 = glam::vec2(400.0, -2050.0);
const CUTSCENE_CAMERA_ZOOM: f32 = 2.5;

#[derive(Debug, Clone)]
pub(crate) struct WinterHorrorlandCutsceneState {
    start_cursor: Samples,
    countdown_cursor: Samples,
    lights_played: bool,
}

impl WinterHorrorlandCutsceneState {
    pub(crate) fn new(countdown_cursor: Samples, sample_rate: u32) -> Self {
        let lead_in = seconds_to_samples(BLACK_DELAY_SECONDS + LIGHTS_TURN_ON_SECONDS, sample_rate);
        Self {
            start_cursor: Samples(countdown_cursor.0.saturating_sub(lead_in)),
            countdown_cursor,
            lights_played: false,
        }
    }

    pub(crate) fn song_start_cursor(&self) -> Samples {
        self.start_cursor
    }

    pub(crate) fn tick_audio_or_warn(
        &mut self,
        mixer: &SharedMixer,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if self.lights_played || !self.lights_should_play(cursor, sample_rate) {
            return;
        }
        self.lights_played = true;
        play_winter_horrorland_lights_sound_or_warn(mixer);
    }

    pub(crate) fn apply_camera(
        &self,
        cameras: &mut CameraRegistry,
        camera_fx: &mut CameraFx,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if !self.stage_lights_on(cursor, sample_rate) || cursor.0 >= 0 {
            return;
        }
        camera_fx.force_game_camera(cameras, CUTSCENE_CAMERA_POSITION, CUTSCENE_CAMERA_ZOOM);
    }

    pub(crate) fn apply_commands<'a>(
        &self,
        commands: impl Iterator<Item = &'a mut DrawCommand>,
        cursor: Samples,
        sample_rate: u32,
    ) {
        let hud_alpha = self.hud_alpha(cursor, sample_rate);
        if hud_alpha >= 1.0 {
            return;
        }
        for cmd in commands {
            if matches!(
                cmd.layer,
                RenderLayer::Notes | RenderLayer::Hud | RenderLayer::Overlay
            ) && cmd.camera == CameraId(1)
            {
                cmd.color.w *= hud_alpha;
            }
        }
    }

    pub(crate) fn append_commands(
        &self,
        sprites: &mut RenderCommandList,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if self.black_screen_visible(cursor, sample_rate) {
            sprites.push(black_overlay_command());
        }
    }

    pub(crate) fn blocks_input(&self, cursor: Samples) -> bool {
        cursor.0 < self.countdown_cursor.0
    }

    fn black_screen_visible(&self, cursor: Samples, sample_rate: u32) -> bool {
        self.elapsed(cursor).0 < seconds_to_samples(BLACK_DELAY_SECONDS, sample_rate)
    }

    fn stage_lights_on(&self, cursor: Samples, sample_rate: u32) -> bool {
        cursor.0 >= self.light_on_cursor(sample_rate).0
    }

    fn lights_should_play(&self, cursor: Samples, sample_rate: u32) -> bool {
        self.stage_lights_on(cursor, sample_rate) && cursor.0 < self.countdown_cursor.0
    }

    fn hud_alpha(&self, cursor: Samples, sample_rate: u32) -> f32 {
        if cursor.0 < self.countdown_cursor.0 {
            return 0.0;
        }
        let fade_samples = seconds_to_samples(HUD_FADE_SECONDS, sample_rate).max(1);
        let elapsed = cursor.0.saturating_sub(self.countdown_cursor.0);
        quad_in_out(elapsed as f32 / fade_samples as f32)
    }

    fn elapsed(&self, cursor: Samples) -> Samples {
        Samples(cursor.0.saturating_sub(self.start_cursor.0).max(0))
    }

    fn light_on_cursor(&self, sample_rate: u32) -> Samples {
        Samples(
            self.start_cursor
                .0
                .saturating_add(seconds_to_samples(BLACK_DELAY_SECONDS, sample_rate)),
        )
    }
}

pub(crate) fn should_play_winter_horrorland_cutscene(selection: PreviewSelection) -> bool {
    selection.song == PreviewSong::WINTER_HORRORLAND
}

fn black_overlay_command() -> DrawCommand {
    let mut cmd = DrawCommand::sprite(
        PAUSE_OVERLAY_TEXTURE_ID,
        glam::vec2(-200.0, -200.0),
        glam::vec2(2560.0, 1440.0),
    );
    cmd.camera = CameraId(2);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = 10_100;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = glam::vec4(0.0, 0.0, 0.0, 1.0);
    cmd
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_song::PreviewDifficulty;

    #[test]
    fn only_winter_horrorland_uses_the_lights_cutscene() {
        assert!(should_play_winter_horrorland_cutscene(
            PreviewSelection::new(PreviewSong::WINTER_HORRORLAND, PreviewDifficulty::Hard)
        ));
        assert!(!should_play_winter_horrorland_cutscene(
            PreviewSelection::new(PreviewSong::EGGNOG, PreviewDifficulty::Hard)
        ));
    }

    #[test]
    fn cutscene_extends_song_clock_before_countdown() {
        let cutscene = WinterHorrorlandCutsceneState::new(Samples(-90_565), 48_000);

        assert_eq!(cutscene.song_start_cursor(), Samples(-318_235));
        assert!(cutscene.blocks_input(Samples(-90_566)));
        assert!(!cutscene.blocks_input(Samples(-90_565)));
    }

    #[test]
    fn black_screen_only_covers_the_initial_delay() {
        let cutscene = WinterHorrorlandCutsceneState::new(Samples(-90_565), 48_000);
        let mut sprites = RenderCommandList::new();

        cutscene.append_commands(&mut sprites, cutscene.song_start_cursor(), 48_000);
        assert_eq!(sprites.len(), 1);

        let mut sprites = RenderCommandList::new();
        cutscene.append_commands(&mut sprites, Samples(-313_434), 48_000);
        assert!(sprites.is_empty());
    }

    #[test]
    fn hud_is_hidden_until_countdown_and_then_fades() {
        let cutscene = WinterHorrorlandCutsceneState::new(Samples(-90_565), 48_000);

        assert_eq!(cutscene.hud_alpha(Samples(-90_566), 48_000), 0.0);
        let mid = cutscene.hud_alpha(Samples(-42_565), 48_000);
        assert!(mid > 0.45 && mid < 0.55);
        assert_eq!(cutscene.hud_alpha(Samples(5_435), 48_000), 1.0);
    }

    #[test]
    fn apply_commands_only_fades_hud_camera_ui() {
        let cutscene = WinterHorrorlandCutsceneState::new(Samples(0), 48_000);
        let mut hud = DrawCommand::sprite(1.into(), glam::Vec2::ZERO, glam::Vec2::ONE);
        hud.camera = CameraId(1);
        hud.layer = RenderLayer::Hud;
        let mut stage = DrawCommand::sprite(2.into(), glam::Vec2::ZERO, glam::Vec2::ONE);
        stage.camera = CameraId(0);
        stage.layer = RenderLayer::Stage;
        let mut commands = vec![hud, stage];

        cutscene.apply_commands(commands.iter_mut(), Samples(-1), 48_000);

        assert_eq!(commands[0].color.w, 0.0);
        assert_eq!(commands[1].color.w, 1.0);
    }
}
