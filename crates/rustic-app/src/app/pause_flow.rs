use super::App;
use crate::pause_menu::{
    ensure_pause_overlay_texture, PauseMenuAction, PauseMenuState, PAUSE_OVERLAY_TEXTURE_ID,
};
use crate::song_audio::set_vocals_gain;
use rustic_core::input::InputAction;
use rustic_core::time::Samples;
use rustic_render::{RenderCommandList, TextCommandList};
use std::time::Instant;
use winit::event::ElementState;

impl App {
    pub(super) fn pause_cursor(&self) -> Option<Samples> {
        self.pause_menu.as_ref().map(PauseMenuState::cursor)
    }

    pub(super) fn handle_pause_input(
        &mut self,
        action: InputAction,
        state: ElementState,
        cursor: Samples,
    ) -> bool {
        if self.pause_menu.is_none() {
            if state == ElementState::Pressed
                && self.mode == super::title_flow::AppMode::Play
                && self.game_over.is_none()
                // ref: bdedc0aa:source/funkin/input/Controls.hx:792-793
                && matches!(
                    action,
                    InputAction::Pause | InputAction::Back | InputAction::Confirm
                )
            {
                self.enter_pause_menu(cursor);
                return true;
            }
            return false;
        }

        if state != ElementState::Pressed {
            return true;
        }

        let pause_action = self
            .pause_menu
            .as_mut()
            .map(|menu| menu.input(action, self.preview_selection))
            .unwrap_or(PauseMenuAction::None);
        match pause_action {
            PauseMenuAction::Resume => self.resume_from_pause(),
            PauseMenuAction::RestartSong => self.restart_song_from_pause(),
            PauseMenuAction::ChangeDifficulty(difficulty) => {
                self.change_difficulty_from_pause(difficulty);
            }
            PauseMenuAction::ExitToMenu => {
                self.pause_menu = None;
                self.pause_music.stop(&self.mixer);
                self.return_to_play_menu();
            }
            PauseMenuAction::None => self.rebuild_pause_commands(),
        }
        true
    }

    fn enter_pause_menu(&mut self, cursor: Samples) {
        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        ensure_pause_overlay_texture(&runtime.rs.device, &runtime.rs.queue, &mut self.atlases);
        self.pause_menu = Some(PauseMenuState::new(cursor));
        if let Err(e) = self.mixer.edit(|mixer| {
            mixer.pause();
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "pause gameplay audio: {e:#}");
        }
        if self.audio_output.is_some() {
            self.pause_music.start_or_warn(&self.mixer);
        }
        self.rebuild_pause_commands();
    }

    pub(super) fn rebuild_pause_commands(&mut self) {
        let Some(menu) = self.pause_menu.as_ref() else {
            return;
        };
        let mut sprites = RenderCommandList::new();
        for cmd in self.cmds.iter() {
            if cmd.texture != PAUSE_OVERLAY_TEXTURE_ID {
                sprites.push(cmd.clone());
            }
        }

        let mut text = TextCommandList::new();
        for cmd in self.text_cmds.iter() {
            if cmd.z < 10_000 {
                text.push(cmd.clone());
            }
        }

        self.pause_music.update_gain(&self.mixer);
        menu.append_commands(&mut sprites, &mut text, self.preview_selection);
        self.cmds = sprites;
        self.text_cmds = text;
    }

    fn resume_from_pause(&mut self) {
        let Some(menu) = self.pause_menu.take() else {
            return;
        };
        self.pause_music.stop(&self.mixer);
        self.resume_song_clock_from(menu.cursor());
        if self.song_started {
            if let Err(e) = self.mixer.edit(|mixer| {
                mixer.resume();
                Ok(())
            }) {
                tracing::warn!(target: "rustic.audio", "resume gameplay audio: {e:#}");
            }
        }
        self.rebuild_frame_commands();
    }

    fn restart_song_from_pause(&mut self) {
        self.pause_menu = None;
        self.pause_music.stop(&self.mixer);
        self.load_selected_song();
    }

    fn change_difficulty_from_pause(&mut self, difficulty: crate::preview_song::PreviewDifficulty) {
        self.pause_menu = None;
        self.pause_music.stop(&self.mixer);
        self.preview_selection =
            crate::preview_song::PreviewSelection::new(self.preview_selection.song, difficulty);
        if !self.story_playlist.is_empty() {
            self.story_playlist_difficulty = self.preview_selection.difficulty;
        }
        self.load_selected_song();
    }

    fn resume_song_clock_from(&mut self, cursor: Samples) {
        if !self.song_started || self.audio_output.is_none() {
            self.song_start_cursor = cursor;
            self.song_start = Instant::now();
        }
        set_vocals_gain(&self.mixer, 1.0);
    }
}
