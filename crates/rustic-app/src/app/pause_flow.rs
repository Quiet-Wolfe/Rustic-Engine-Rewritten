use super::App;
use crate::menu_audio::MenuSound;
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
    pub(super) fn pause_on_unfocus(&mut self, cursor: Samples) -> bool {
        if !should_pause_on_unfocus(
            self.options_preferences.pause_on_unfocus,
            self.mode,
            self.pause_menu.is_some(),
            self.game_over.is_some(),
            self.dialogue.is_some(),
            self.darnell_intro_cutscene
                .as_ref()
                .is_some_and(|cutscene| cutscene.blocks_input(cursor))
                || self
                    .winter_horrorland_cutscene
                    .as_ref()
                    .is_some_and(|cutscene| cutscene.blocks_input(cursor)),
        ) {
            return false;
        }
        self.enter_pause_menu(cursor);
        true
    }

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
                && self.stress_pico_end_cutscene.is_none()
                && !self
                    .darnell_intro_cutscene
                    .as_ref()
                    .is_some_and(|cutscene| cutscene.blocks_input(cursor))
                && !self
                    .winter_horrorland_cutscene
                    .as_ref()
                    .is_some_and(|cutscene| cutscene.blocks_input(cursor))
                // ref: bdedc0aa:source/funkin/input/Controls.hx:792-793
                && is_gameplay_pause_action(action)
            {
                self.enter_pause_menu(cursor);
                return true;
            }
            return false;
        }

        if state != ElementState::Pressed {
            if let Some(menu) = self.pause_menu.as_mut() {
                menu.release(action);
            }
            return true;
        }

        self.play_pause_menu_sound(action);
        let pause_action = self
            .pause_menu
            .as_mut()
            .map(|menu| menu.input(action, self.preview_selection, self.practice_mode))
            .unwrap_or(PauseMenuAction::None);
        match pause_action {
            PauseMenuAction::Resume => self.resume_from_pause(),
            PauseMenuAction::RestartSong => self.restart_song_from_pause(),
            PauseMenuAction::EnablePracticeMode => {
                // ref: bdedc0aa:source/funkin/play/PauseSubState.hx:1084-1090
                self.practice_mode = true;
                self.rebuild_pause_commands();
            }
            PauseMenuAction::ChangeDifficulty(difficulty) => {
                self.change_difficulty_from_pause(difficulty);
            }
            PauseMenuAction::AdjustGlobalOffset(delta) => {
                self.adjust_global_offset_from_pause(delta);
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
        menu.append_commands(
            &mut sprites,
            &mut text,
            self.preview_selection,
            self.practice_mode,
            self.death_counter,
            self.options_preferences.global_offset_ms,
        );
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
        self.restart_loaded_song_from_start();
    }

    fn change_difficulty_from_pause(&mut self, difficulty: crate::preview_song::PreviewDifficulty) {
        self.pause_menu = None;
        self.pause_music.stop(&self.mixer);
        self.preview_selection =
            crate::preview_song::PreviewSelection::new(self.preview_selection.song, difficulty)
                .with_variation(self.preview_selection.variation);
        if !self.story_playlist.is_empty() {
            self.story_playlist_difficulty = self.preview_selection.difficulty;
        }
        self.load_selected_song();
    }

    fn resume_song_clock_from(&mut self, cursor: Samples) {
        let now = Instant::now();
        self.song_start_cursor = cursor;
        self.song_start = now;
        self.audio_clock.resume_from_pause(cursor, now);
        set_vocals_gain(&self.mixer, 1.0);
    }

    fn play_pause_menu_sound(&self, action: InputAction) {
        match action {
            InputAction::LaneUp
            | InputAction::UiUp
            | InputAction::LaneDown
            | InputAction::UiDown => self.play_menu_sound(MenuSound::Scroll),
            InputAction::Confirm => self.play_menu_sound(MenuSound::Confirm),
            InputAction::Back | InputAction::Pause => self.play_menu_sound(MenuSound::Cancel),
            _ => {}
        }
    }

    fn adjust_global_offset_from_pause(&mut self, delta: i16) {
        if self.options_preferences.adjust_global_offset_ms(delta)
            == crate::options_preferences::PreferenceChange::Changed
        {
            self.persist_options_preferences();
        }
        self.rebuild_pause_commands();
    }
}

fn should_pause_on_unfocus(
    enabled: bool,
    mode: super::title_flow::AppMode,
    pause_menu_active: bool,
    game_over_active: bool,
    dialogue_active: bool,
    cutscene_active: bool,
) -> bool {
    enabled
        && mode == super::title_flow::AppMode::Play
        && !pause_menu_active
        && !game_over_active
        && !dialogue_active
        && !cutscene_active
}

fn is_gameplay_pause_action(action: InputAction) -> bool {
    matches!(action, InputAction::Pause | InputAction::Back)
}

#[cfg(test)]
mod tests {
    use super::super::title_flow::AppMode;
    use super::*;

    #[test]
    fn pause_on_unfocus_only_applies_to_live_gameplay() {
        assert!(should_pause_on_unfocus(
            true,
            AppMode::Play,
            false,
            false,
            false,
            false
        ));
        assert!(!should_pause_on_unfocus(
            false,
            AppMode::Play,
            false,
            false,
            false,
            false
        ));
        assert!(!should_pause_on_unfocus(
            true,
            AppMode::SongSelect,
            false,
            false,
            false,
            false
        ));
        assert!(!should_pause_on_unfocus(
            true,
            AppMode::Play,
            true,
            false,
            false,
            false
        ));
        assert!(!should_pause_on_unfocus(
            true,
            AppMode::Play,
            false,
            true,
            false,
            false
        ));
        assert!(!should_pause_on_unfocus(
            true,
            AppMode::Play,
            false,
            false,
            true,
            false
        ));
        assert!(!should_pause_on_unfocus(
            true,
            AppMode::Play,
            false,
            false,
            false,
            true
        ));
    }

    #[test]
    fn gameplay_pause_hotkeys_exclude_confirm_action() {
        assert!(is_gameplay_pause_action(InputAction::Pause));
        assert!(is_gameplay_pause_action(InputAction::Back));
        assert!(!is_gameplay_pause_action(InputAction::Confirm));
    }
}
