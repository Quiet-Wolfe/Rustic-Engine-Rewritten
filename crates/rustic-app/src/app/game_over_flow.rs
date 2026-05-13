use super::App;
use crate::game_over::{GameOverRestart, GameOverState};
use crate::song_audio::{play_sample_rate, set_vocals_gain};
use rustic_core::time::Samples;

impl App {
    pub(super) fn restart_song_after_game_over(&mut self, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/GameOverSubState.hx:409-424
        if self.game_over.is_none() || self.game_over_restart.is_some() {
            return;
        }
        let cursor = self
            .game_over
            .map(|state| state.cursor(play_sample_rate(&self.mixer)))
            .unwrap_or(cursor);
        self.character_anim.player_death_confirm(cursor);
        self.game_over_restart = Some(GameOverRestart::new());
        if self.audio_output.is_some() {
            self.game_over_audio.play_confirm_music_or_warn(&self.mixer);
        }
    }

    pub(super) fn enter_game_over(&mut self, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/PlayState.hx:1441-1472
        if self.game_over.is_some() {
            return;
        }
        self.game_over_restart = None;
        let sample_rate = play_sample_rate(&self.mixer);
        let loop_after = self
            .characters
            .as_ref()
            .and_then(|characters| characters.player_animation_duration("firstDeath", sample_rate))
            .unwrap_or(Samples(i64::from(sample_rate)));
        self.character_anim.player_first_death(cursor);
        if let Some(characters) = &self.characters {
            let (target, zoom) = characters.player_game_over_camera(self.base_camera_zoom);
            self.camera_fx
                .focus_game_over_camera(&mut self.cameras, target, zoom);
        }
        (
            self.held_lanes,
            self.opponent_receptors,
            self.hold_covers,
            self.active_holds,
        ) = Default::default();
        set_vocals_gain(&self.mixer, 0.0);
        if let Err(e) = self.mixer.edit(|mixer| {
            mixer.pause();
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "pause game over audio: {e:#}");
        }
        if self.audio_output.is_some() {
            self.game_over_audio.play_loss_sfx_or_warn(&self.mixer);
        }
        self.game_over = Some(GameOverState::new(cursor, loop_after));
    }

    pub(super) fn rebuild_game_over_commands(&mut self, cursor: Samples, sample_rate: u32) {
        if self.finish_game_over_restart_if_due() {
            return;
        }
        let Some(game_over) = self.game_over.as_mut() else {
            return;
        };
        if let Some(loop_at) = game_over.start_loop_if_due(cursor) {
            self.character_anim.player_death_loop(loop_at);
            if self.audio_output.is_some() {
                self.game_over_audio.start_loop_music_or_warn(&self.mixer);
            }
        }
        self.camera_fx
            .update(&mut self.cameras, cursor, sample_rate, 100.0);

        self.cmds.clear();
        if let Some(characters) = &self.characters {
            for cmd in
                characters.player_commands(self.character_anim.poses().player, cursor, sample_rate)
            {
                self.cmds.push(cmd);
            }
        }
    }

    fn finish_game_over_restart_if_due(&mut self) -> bool {
        let Some(restart) = self.game_over_restart else {
            return false;
        };
        if !restart.is_due() {
            return false;
        }
        self.game_over_restart = None;
        self.game_over.take();
        self.game_over_audio.stop(&self.mixer);
        self.load_selected_song();
        true
    }
}
