use super::App;
use crate::countdown_assets::countdown_start_cursor;
use crate::dialogue_state::{DialogueAdvance, DialogueState};
use crate::menu_audio::MenuSound;
use crate::song_audio::play_sample_rate;
use rustic_core::input::InputAction;
use rustic_render::TextCommandList;
use std::time::Instant;
use winit::event::ElementState;

impl App {
    pub(super) fn handle_dialogue_input(
        &mut self,
        action: InputAction,
        state: ElementState,
    ) -> bool {
        if self.dialogue.is_none() {
            return false;
        }
        if state != ElementState::Pressed {
            return true;
        }
        match action {
            InputAction::Confirm => {
                let advance = self
                    .dialogue
                    .as_mut()
                    .map(DialogueState::advance)
                    .unwrap_or(DialogueAdvance::Finished);
                self.play_menu_sound(MenuSound::Confirm);
                match advance {
                    DialogueAdvance::Finished => self.finish_dialogue(),
                    DialogueAdvance::Advanced | DialogueAdvance::Skipped => {
                        self.rebuild_dialogue_commands();
                    }
                }
            }
            InputAction::Back => {
                self.play_menu_sound(MenuSound::Cancel);
                self.finish_dialogue();
            }
            _ => {}
        }
        true
    }

    pub(super) fn rebuild_dialogue_commands(&mut self) {
        self.text_cmds = TextCommandList::new();
        self.cmds = self.static_cmds.clone();
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.song_start_cursor;
        let bpm = self
            .play_state
            .as_ref()
            .map(|play_state| play_state.bpm)
            .unwrap_or(100.0);
        for cmd in
            self.stage_props
                .commands(cursor, sample_rate, bpm, Some(self.preview_selection.song))
        {
            self.cmds.push(cmd);
        }
        if let Some(characters) = &self.characters {
            for cmd in characters.commands(self.character_anim.poses(), cursor, sample_rate) {
                self.cmds.push(cmd);
            }
        }
        if let Some(dialogue) = &self.dialogue {
            dialogue.append_commands(&mut self.cmds, &mut self.text_cmds);
        }
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    fn finish_dialogue(&mut self) {
        self.dialogue = None;
        let sample_rate = play_sample_rate(&self.mixer);
        let bpm = self
            .play_state
            .as_ref()
            .map(|play_state| play_state.bpm)
            .unwrap_or_else(|| {
                f64::from(
                    self.preview_selection
                        .song
                        .starting_bpm_for(self.preview_selection.difficulty),
                )
            });
        self.song_start_cursor = countdown_start_cursor(sample_rate, bpm);
        self.song_start = Instant::now();
        self.song_started = false;
        self.audio_clock.reset(self.song_start);
        self.countdown_audio.reset();
    }
}
