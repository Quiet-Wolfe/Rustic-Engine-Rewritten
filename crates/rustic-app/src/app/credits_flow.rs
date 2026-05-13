use super::App;
use crate::credits_assets::load_credits_assets;
use crate::menu_audio::MenuSound;
use crate::song_audio::play_sample_rate;
use rustic_core::input::InputAction;
use rustic_render::{RenderCommandList, TextCommandList};
use winit::event::ElementState;

impl App {
    pub(super) fn load_credits_screen(&mut self) {
        self.mode = super::title_flow::AppMode::Credits;
        self.title_start = std::time::Instant::now();
        self.title_assets = None;
        self.main_menu_assets = None;
        self.credits_assets = None;
        self.options_menu_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.clear_play_state_for_menu();

        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        let mut credits = load_credits_assets(&runtime.rs.device, &runtime.rs.queue);
        self.atlases = std::mem::take(&mut credits.textures);
        self.credits_assets = Some(credits);
        self.rebuild_credits_commands();
    }

    pub(super) fn rebuild_credits_commands(&mut self) {
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        if let Some(assets) = self.credits_assets.as_ref() {
            self.cmds = assets.commands();
            self.text_cmds = assets.text_commands(cursor, sample_rate);
        } else {
            self.cmds = RenderCommandList::new();
            self.text_cmds = TextCommandList::new();
        }
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    pub(super) fn handle_credits_input(
        &mut self,
        action: InputAction,
        state: ElementState,
    ) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        match action {
            InputAction::Confirm | InputAction::Back => {
                self.play_menu_sound(MenuSound::Cancel);
                self.load_main_menu();
            }
            _ => {}
        }
        true
    }
}
