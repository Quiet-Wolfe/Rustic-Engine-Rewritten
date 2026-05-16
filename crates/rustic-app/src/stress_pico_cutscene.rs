//! Runtime slice for the Stress Pico end cutscene.
//!
//! ref: bdedc0aa:assets/preload/scripts/songs/stress-pico.hxc:133-226

use crate::character_anim::CharacterAnimState;
use crate::pause_menu::PAUSE_OVERLAY_TEXTURE_ID;
use crate::preview_song::{PreviewSelection, PreviewSong, VARIATION_PICO};
use crate::subtitle_track::SubtitleTrack;
use rustic_core::ids::CameraId;
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, TextCommandList};

const CUTSCENE_FPS: f64 = 24.0;
const BOYFRIEND_LAUGH_FRAME: f64 = 176.0;
const BLACK_FADE_FRAME: f64 = 270.0;
const END_FRAME: f64 = 320.0;
const BLACK_FADE_SECONDS: f64 = 2.0;

#[derive(Debug, Clone)]
pub(crate) struct StressPicoEndCutsceneState {
    started_at: Samples,
    subtitle_track: Option<SubtitleTrack>,
}

impl StressPicoEndCutsceneState {
    pub(crate) fn load_or_warn(started_at: Samples) -> Self {
        let subtitle_track = match SubtitleTrack::load_stress_pico_end_cutscene() {
            Ok(track) => Some(track),
            Err(e) => {
                tracing::warn!(
                    target: "rustic.asset",
                    "Stress Pico end cutscene subtitles unavailable: {e:#}"
                );
                None
            }
        };
        Self {
            started_at,
            subtitle_track,
        }
    }

    #[cfg(test)]
    pub(crate) fn new(started_at: Samples, subtitle_track: Option<SubtitleTrack>) -> Self {
        Self {
            started_at,
            subtitle_track,
        }
    }

    pub(crate) fn finished(&self, cursor: Samples, sample_rate: u32) -> bool {
        cursor.0.saturating_sub(self.started_at.0) >= frame_samples(END_FRAME, sample_rate)
    }

    pub(crate) fn apply_character_poses(
        &self,
        anim: &mut CharacterAnimState,
        cursor: Samples,
        sample_rate: u32,
    ) {
        anim.play_chart_animation("dad", "stressPicoEnding", self.started_at, false);
        let laugh_at = self.frame_cursor(BOYFRIEND_LAUGH_FRAME, sample_rate);
        if cursor >= laugh_at {
            anim.play_chart_animation("boyfriend", "laughEnd", laugh_at, false);
        }
    }

    pub(crate) fn apply_commands<'a>(&self, commands: impl Iterator<Item = &'a mut DrawCommand>) {
        for cmd in commands {
            if matches!(cmd.layer, RenderLayer::Notes | RenderLayer::Hud) {
                cmd.color.w = 0.0;
            }
        }
    }

    pub(crate) fn append_commands(
        &self,
        sprites: &mut RenderCommandList,
        text: &mut TextCommandList,
        cursor: Samples,
        sample_rate: u32,
        subtitles_enabled: bool,
    ) {
        let alpha = self.black_alpha(cursor, sample_rate);
        if alpha > 0.0 {
            sprites.push(black_overlay_command(alpha));
        }
        if subtitles_enabled {
            if let Some(track) = self.subtitle_track.as_ref() {
                track.append_commands(text, self.elapsed_cursor(cursor), sample_rate);
            }
        }
    }

    fn elapsed_cursor(&self, cursor: Samples) -> Samples {
        Samples(cursor.0.saturating_sub(self.started_at.0).max(0))
    }

    fn frame_cursor(&self, frame: f64, sample_rate: u32) -> Samples {
        Samples(
            self.started_at
                .0
                .saturating_add(frame_samples(frame, sample_rate)),
        )
    }

    fn black_alpha(&self, cursor: Samples, sample_rate: u32) -> f32 {
        let elapsed = self.elapsed_cursor(cursor).0;
        let fade_start = frame_samples(BLACK_FADE_FRAME, sample_rate);
        if elapsed <= fade_start {
            return 0.0;
        }
        let fade_duration = seconds_to_samples(BLACK_FADE_SECONDS, sample_rate).max(1);
        ((elapsed - fade_start) as f32 / fade_duration as f32).clamp(0.0, 1.0)
    }
}

pub(crate) fn should_play_stress_pico_end_cutscene(selection: PreviewSelection) -> bool {
    selection.song == PreviewSong::STRESS
        && selection.effective_variation_suffix() == Some(VARIATION_PICO)
}

fn black_overlay_command(alpha: f32) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(
        PAUSE_OVERLAY_TEXTURE_ID,
        glam::vec2(0.0, 0.0),
        glam::vec2(1280.0, 720.0),
    );
    cmd.camera = CameraId(2);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = 10_050;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = glam::vec4(0.0, 0.0, 0.0, alpha);
    cmd
}

fn frame_samples(frame: f64, sample_rate: u32) -> i64 {
    seconds_to_samples(frame / CUTSCENE_FPS, sample_rate)
}

fn seconds_to_samples(seconds: f64, sample_rate: u32) -> i64 {
    (seconds.max(0.0) * f64::from(sample_rate.max(1))).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_song::PreviewDifficulty;

    #[test]
    fn only_stress_pico_uses_the_scripted_end_cutscene() {
        let stress_pico = PreviewSelection::new(PreviewSong::STRESS, PreviewDifficulty::Hard)
            .with_variation(Some(VARIATION_PICO));
        let stress_base = PreviewSelection::new(PreviewSong::STRESS, PreviewDifficulty::Hard);
        let blazin_pico = PreviewSelection::new(PreviewSong::BLAZIN, PreviewDifficulty::Hard)
            .with_variation(Some(VARIATION_PICO));

        assert!(should_play_stress_pico_end_cutscene(stress_pico));
        assert!(!should_play_stress_pico_end_cutscene(stress_base));
        assert!(!should_play_stress_pico_end_cutscene(blazin_pico));
    }

    #[test]
    fn cutscene_finishes_on_the_upstream_frame() {
        let cutscene = StressPicoEndCutsceneState::new(Samples(10_000), None);

        assert!(!cutscene.finished(Samples(649_999), 48_000));
        assert!(cutscene.finished(Samples(650_000), 48_000));
    }

    #[test]
    fn cutscene_hides_hud_and_notes_but_not_characters() {
        let cutscene = StressPicoEndCutsceneState::new(Samples(0), None);
        let mut hud = DrawCommand::sprite(1.into(), glam::Vec2::ZERO, glam::Vec2::ONE);
        hud.layer = RenderLayer::Hud;
        let mut notes = DrawCommand::sprite(2.into(), glam::Vec2::ZERO, glam::Vec2::ONE);
        notes.layer = RenderLayer::Notes;
        let mut characters = DrawCommand::sprite(3.into(), glam::Vec2::ZERO, glam::Vec2::ONE);
        characters.layer = RenderLayer::Characters;
        let mut commands = vec![hud, notes, characters];

        cutscene.apply_commands(commands.iter_mut());

        assert_eq!(commands[0].color.w, 0.0);
        assert_eq!(commands[1].color.w, 0.0);
        assert_eq!(commands[2].color.w, 1.0);
    }

    #[test]
    fn black_overlay_fades_after_frame_270() {
        let cutscene = StressPicoEndCutsceneState::new(Samples(0), None);
        let mut sprites = RenderCommandList::new();
        let mut text = TextCommandList::new();

        cutscene.append_commands(&mut sprites, &mut text, Samples(539_999), 48_000, true);
        assert!(sprites.is_empty());

        cutscene.append_commands(&mut sprites, &mut text, Samples(636_000), 48_000, true);
        assert_eq!(sprites.len(), 1);
        assert!(sprites.as_slice()[0].color.w > 0.0);
    }
}
