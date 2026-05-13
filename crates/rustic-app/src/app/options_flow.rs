use super::App;
use crate::menu_audio::MenuSound;
use crate::options_menu_assets::{load_options_menu_assets, OptionsMenuAction, OptionsMenuPage};
use rustic_core::input::InputAction;
use rustic_core::time::Samples;
use rustic_render::{RenderCommandList, TextCommandList};
use winit::event::ElementState;

impl App {
    pub(super) fn load_options_menu(&mut self) {
        self.mode = super::title_flow::AppMode::Options;
        self.title_assets = None;
        self.main_menu_assets = None;
        self.credits_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.options_menu_assets = None;
        self.clear_play_state_for_menu();
        self.options_menu_page = OptionsMenuPage::Root;
        self.options_menu_index = 0;

        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        match load_options_menu_assets(&runtime.rs.device, &runtime.rs.queue) {
            Ok(mut options) => {
                self.atlases = std::mem::take(&mut options.textures);
                self.options_menu_assets = Some(options);
            }
            Err(e) => tracing::warn!(target: "rustic.asset", "options menu unavailable: {e:#}"),
        }
        self.rebuild_options_menu_commands();
    }

    pub(super) fn rebuild_options_menu_commands(&mut self) {
        if let Some(assets) = self.options_menu_assets.as_ref() {
            self.cmds = assets.commands();
            self.text_cmds = assets.text_commands(self.options_menu_page, self.options_menu_index);
        } else {
            self.cmds = RenderCommandList::new();
            self.text_cmds = TextCommandList::new();
        }
        self.append_debug_overlay_commands(Samples(0), crate::scene_assets::SAMPLE_RATE);
    }

    pub(super) fn handle_options_menu_input(
        &mut self,
        action: InputAction,
        state: ElementState,
    ) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        match action {
            InputAction::LaneUp | InputAction::UiUp => self.move_options_selection(-1),
            InputAction::LaneDown | InputAction::UiDown => self.move_options_selection(1),
            InputAction::Confirm => self.confirm_options_item(),
            InputAction::Back => self.back_from_options_menu(),
            _ => {}
        }
        true
    }

    fn move_options_selection(&mut self, delta: isize) {
        let count = self.options_item_count().max(1) as isize;
        self.options_menu_index =
            (self.options_menu_index as isize + delta).rem_euclid(count) as usize;
        self.play_menu_sound(MenuSound::Scroll);
        self.rebuild_options_menu_commands();
    }

    fn confirm_options_item(&mut self) {
        if self.options_menu_page != OptionsMenuPage::Root {
            self.back_from_options_menu();
            return;
        }
        self.play_menu_sound(MenuSound::Confirm);
        let action = self
            .options_menu_assets
            .as_ref()
            .and_then(|assets| assets.action_for_root(self.options_menu_index));
        match action {
            Some(OptionsMenuAction::Page(page)) => {
                self.options_menu_page = page;
                self.options_menu_index = 0;
                self.rebuild_options_menu_commands();
            }
            Some(OptionsMenuAction::Exit) | None => self.load_main_menu(),
        }
    }

    fn back_from_options_menu(&mut self) {
        self.play_menu_sound(MenuSound::Cancel);
        if self.options_menu_page == OptionsMenuPage::Root {
            self.load_main_menu();
        } else {
            self.options_menu_page = OptionsMenuPage::Root;
            self.options_menu_index = 0;
            self.rebuild_options_menu_commands();
        }
    }

    fn options_item_count(&self) -> usize {
        self.options_menu_assets
            .as_ref()
            .map(|assets| assets.item_count(self.options_menu_page))
            .unwrap_or(0)
    }
}
