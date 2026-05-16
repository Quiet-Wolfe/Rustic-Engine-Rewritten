use super::App;
use crate::audio_clock::AudioClockDecision;
use crate::preview_song::{PreviewDifficulty, PreviewSelection, PreviewSong};
use crate::song_audio::play_sample_rate;
use crate::stage_sfx::play_stress_pico_end_cutscene_sound_or_warn;
use crate::stress_pico_cutscene::{
    should_play_stress_pico_end_cutscene, StressPicoEndCutsceneState,
};
use rustic_core::time::Samples;
use std::time::Instant;

const SONG_END_TAIL_SECONDS: i64 = 2;

impl App {
    pub(super) fn start_story_playlist(
        &mut self,
        songs: Vec<PreviewSong>,
        difficulty: PreviewDifficulty,
    ) {
        let Some(first) = songs.first().copied() else {
            return;
        };
        self.story_playlist = songs;
        self.story_playlist_index = 0;
        self.story_playlist_difficulty = difficulty;
        self.preview_selection = PreviewSelection::new(first, difficulty);
        self.enter_play();
    }

    pub(super) fn clear_story_playlist(&mut self) {
        self.story_playlist.clear();
        self.story_playlist_index = 0;
    }

    pub(super) fn return_to_play_menu(&mut self) {
        // ref: bdedc0aa:source/funkin/play/GameOverSubState.hx:552
        // ref: bdedc0aa:source/funkin/play/PauseSubState.hx:1152-1157
        self.stress_pico_end_cutscene = None;
        self.winter_horrorland_cutscene = None;
        self.thorns_intro_cutscene = None;
        self.death_counter = 0;
        self.practice_mode = false;
        if self.story_playlist.is_empty() {
            self.enter_song_select();
        } else {
            self.load_story_menu();
        }
    }

    pub(super) fn finish_song_if_due(&mut self, cursor: Samples, sample_rate: u32) -> bool {
        if self.game_over.is_some() || !self.song_started {
            return false;
        }
        let Some(play_state) = self.play_state.as_ref() else {
            return false;
        };
        if let Some(cutscene) = self.stress_pico_end_cutscene.as_ref() {
            if cutscene.finished(cursor, sample_rate) {
                self.stress_pico_end_cutscene = None;
                self.finish_current_song();
                return true;
            }
            return false;
        }
        let tail = i64::from(sample_rate.max(1)) * SONG_END_TAIL_SECONDS;
        let mut finish_at = play_state.chart_end_cursor().0.saturating_add(tail);
        if let Some(sserafim_finish_at) = self.sserafim_stage.finish_cursor_override() {
            finish_at = finish_at.max(sserafim_finish_at.0);
        }
        if cursor.0 < finish_at {
            return false;
        }
        if should_play_stress_pico_end_cutscene(self.preview_selection) {
            self.start_stress_pico_end_cutscene(cursor, sample_rate);
            return false;
        }
        self.finish_current_song();
        true
    }

    fn start_stress_pico_end_cutscene(&mut self, cursor: Samples, sample_rate: u32) {
        let cutscene = StressPicoEndCutsceneState::load_or_warn(cursor);
        cutscene.apply_character_poses(&mut self.character_anim, cursor, sample_rate);
        self.stress_pico_end_cutscene = Some(cutscene);
        if self.audio_output.is_some() {
            play_stress_pico_end_cutscene_sound_or_warn(&self.mixer);
        }
    }

    fn finish_current_song(&mut self) {
        // ref: bdedc0aa:source/funkin/play/PlayState.hx:3435
        self.stress_pico_end_cutscene = None;
        self.winter_horrorland_cutscene = None;
        self.thorns_intro_cutscene = None;
        self.death_counter = 0;
        self.practice_mode = false;
        if self.story_playlist.is_empty() {
            self.enter_song_select();
            return;
        }
        let next_index = self.story_playlist_index + 1;
        if let Some(next_song) = self.story_playlist.get(next_index).copied() {
            self.story_playlist_index = next_index;
            self.preview_selection =
                PreviewSelection::new(next_song, self.story_playlist_difficulty);
            self.load_selected_song();
        } else {
            self.load_story_menu();
        }
    }

    pub(super) fn advance_song_clock(&mut self) -> Samples {
        if !self.song_started {
            let elapsed = self.song_start.elapsed().as_secs_f64();
            let elapsed_samples =
                (elapsed * f64::from(play_sample_rate(&self.mixer))).round() as i64;
            let cursor = Samples(self.song_start_cursor.0 + elapsed_samples);
            if cursor.0 < 0 {
                return cursor;
            }
            self.song_started = true;
            self.song_start = Instant::now();
            self.song_start_cursor = Samples(0);
            self.audio_clock.reset(self.song_start);
            if let Err(e) = self.mixer.edit(|mixer| {
                mixer.seek(Samples(0))?;
                mixer.resume();
                Ok(())
            }) {
                tracing::warn!(target: "rustic.audio", "start countdown audio: {e:#}");
            }
            return Samples(0);
        }

        if self.audio_output.is_some() {
            match self
                .audio_clock
                .observe(self.mixer.sample_cursor(), Instant::now())
            {
                AudioClockDecision::Audio(cursor) => return cursor,
                AudioClockDecision::SwitchToWall(cursor) => {
                    tracing::warn!(
                        target: "rustic.audio",
                        "audio stream cursor stalled, switching gameplay clock to wall time"
                    );
                    self.song_start = Instant::now();
                    self.song_start_cursor = cursor;
                    return cursor;
                }
                AudioClockDecision::Wall => {}
            }
        }
        let elapsed = self.song_start.elapsed().as_secs_f64();
        let elapsed_samples = (elapsed * f64::from(play_sample_rate(&self.mixer))).round() as i64;
        Samples(self.song_start_cursor.0 + elapsed_samples)
    }
}
