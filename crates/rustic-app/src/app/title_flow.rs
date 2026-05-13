use super::App;
// LINT-ALLOW: long-file app flow is being split screen by screen.
use crate::app_text::song_select_text_commands;
use crate::camera_fx::CameraFx;
use crate::freeplay_assets::load_freeplay_assets as load_freeplay_screen_assets;
use crate::main_menu_assets::{load_main_menu_assets, MainMenuAction};
use crate::song_audio::{play_sample_rate, set_vocals_gain};
use crate::story_menu_assets::load_story_menu_assets;
use crate::title_assets::load_title_screen_assets;
use rustic_core::input::InputAction;
use rustic_core::time::Samples;
use rustic_render::{CameraRegistry, RenderCommandList, TextCommandList};
use std::time::Instant;
use winit::event::ElementState;
use winit::event_loop::ActiveEventLoop;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AppMode {
    Title,
    MainMenu,
    Credits,
    Options,
    StoryMenu,
    SongSelect,
    Play,
}

impl App {
    pub(super) fn load_title_screen(&mut self) {
        self.mode = AppMode::Title;
        self.title_start = Instant::now();
        self.title_assets = None;
        self.main_menu_assets = None;
        self.credits_assets = None;
        self.options_menu_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.play_state = None;
        self.static_cmds = RenderCommandList::new();
        self.cmds = RenderCommandList::new();
        self.text_cmds = TextCommandList::new();
        self.cameras = CameraRegistry::with_default_fnf();
        self.camera_fx = CameraFx::default();
        self.update_window_title();

        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        match load_title_screen_assets(&runtime.rs.device, &runtime.rs.queue) {
            Ok(mut title) => {
                self.atlases = std::mem::take(&mut title.textures);
                self.title_assets = Some(title);
                self.rebuild_title_commands();
            }
            Err(e) => {
                tracing::warn!(target: "rustic.asset", "title screen unavailable: {e:#}");
                self.enter_play();
            }
        }
    }

    pub(super) fn rebuild_title_commands(&mut self) {
        self.text_cmds = TextCommandList::new();
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        self.cmds = self
            .title_assets
            .as_ref()
            .map(|assets| assets.commands(cursor, sample_rate))
            .unwrap_or_default();
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    pub(super) fn rebuild_song_select_commands(&mut self) {
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        if let Some(assets) = self.freeplay_assets.as_ref() {
            self.cmds = assets.commands(self.preview_selection, cursor, sample_rate);
            self.text_cmds = assets.text_commands(self.preview_selection);
        } else {
            self.cmds = RenderCommandList::new();
            self.text_cmds = song_select_text_commands(self.preview_selection, true);
        }
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    pub(super) fn load_main_menu(&mut self) {
        self.mode = AppMode::MainMenu;
        self.title_start = Instant::now();
        self.title_assets = None;
        self.main_menu_assets = None;
        self.credits_assets = None;
        self.options_menu_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.clear_play_state_for_menu();
        self.update_window_title();

        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        match load_main_menu_assets(&runtime.rs.device, &runtime.rs.queue) {
            Ok(mut menu) => {
                self.atlases = std::mem::take(&mut menu.textures);
                self.main_menu_index = self
                    .main_menu_index
                    .min(menu.item_count().saturating_sub(1));
                self.main_menu_assets = Some(menu);
                self.rebuild_main_menu_commands();
            }
            Err(e) => {
                tracing::warn!(target: "rustic.asset", "main menu unavailable: {e:#}");
                self.enter_song_select();
            }
        }
    }

    pub(super) fn rebuild_main_menu_commands(&mut self) {
        self.text_cmds = TextCommandList::new();
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        self.cmds = self
            .main_menu_assets
            .as_ref()
            .map(|assets| assets.commands(self.main_menu_index, cursor, sample_rate))
            .unwrap_or_default();
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    pub(super) fn load_story_menu(&mut self) {
        self.mode = AppMode::StoryMenu;
        self.title_start = Instant::now();
        self.title_assets = None;
        self.main_menu_assets = None;
        self.credits_assets = None;
        self.options_menu_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.clear_play_state_for_menu();
        self.update_window_title();

        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        match load_story_menu_assets(&runtime.rs.device, &runtime.rs.queue) {
            Ok(mut story) => {
                self.atlases = std::mem::take(&mut story.textures);
                self.story_menu_index = self
                    .story_menu_index
                    .min(story.item_count().saturating_sub(1));
                self.story_menu_difficulty =
                    story.difficulty_for_level(self.story_menu_index, self.story_menu_difficulty);
                self.story_menu_assets = Some(story);
                self.rebuild_story_menu_commands();
            }
            Err(e) => {
                tracing::warn!(target: "rustic.asset", "story menu unavailable: {e:#}");
                self.enter_song_select();
            }
        }
    }

    pub(super) fn rebuild_story_menu_commands(&mut self) {
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        if let Some(assets) = self.story_menu_assets.as_ref() {
            self.cmds = assets.commands(
                self.story_menu_index,
                self.story_menu_difficulty,
                cursor,
                sample_rate,
            );
            self.text_cmds =
                assets.text_commands(self.story_menu_index, self.story_menu_difficulty);
        } else {
            self.cmds = RenderCommandList::new();
            self.text_cmds = TextCommandList::new();
        }
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    pub(super) fn input_cursor(&mut self) -> Samples {
        if let Some(cursor) = self.pause_cursor() {
            return cursor;
        }
        match self.mode {
            AppMode::Title
            | AppMode::MainMenu
            | AppMode::Credits
            | AppMode::Options
            | AppMode::StoryMenu
            | AppMode::SongSelect => self.title_cursor(play_sample_rate(&self.mixer)),
            AppMode::Play => self.advance_song_clock(),
        }
    }

    pub(super) fn handle_mode_input(
        &mut self,
        action: InputAction,
        state: ElementState,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        match self.mode {
            AppMode::Title => self.handle_title_input(action, state, event_loop),
            AppMode::MainMenu => self.handle_main_menu_input(action, state),
            AppMode::Credits => self.handle_credits_input(action, state),
            AppMode::Options => self.handle_options_menu_input(action, state),
            AppMode::StoryMenu => self.handle_story_menu_input(action, state),
            AppMode::SongSelect => self.handle_song_select_input(action, state),
            AppMode::Play => false,
        }
    }

    fn handle_title_input(
        &mut self,
        action: InputAction,
        state: ElementState,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        // ref: bdedc0aa:source/funkin/ui/title/TitleState.hx:249-302
        match action {
            InputAction::Confirm => self.load_main_menu(),
            InputAction::Back => event_loop.exit(),
            _ => {}
        }
        true
    }

    fn handle_main_menu_input(&mut self, action: InputAction, state: ElementState) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        // ref: bdedc0aa:source/funkin/ui/mainmenu/MainMenuState.hx:145-232
        let item_count = self
            .main_menu_assets
            .as_ref()
            .map(|assets| assets.item_count())
            .unwrap_or(0)
            .max(1);
        match action {
            InputAction::LaneUp | InputAction::UiUp => {
                self.main_menu_index = (self.main_menu_index + item_count - 1) % item_count;
                self.rebuild_main_menu_commands();
            }
            InputAction::LaneDown | InputAction::UiDown => {
                self.main_menu_index = (self.main_menu_index + 1) % item_count;
                self.rebuild_main_menu_commands();
            }
            InputAction::Confirm => self.confirm_main_menu_item(),
            InputAction::Back => self.load_title_screen(),
            _ => {}
        }
        true
    }

    fn confirm_main_menu_item(&mut self) {
        let action = self
            .main_menu_assets
            .as_ref()
            .and_then(|assets| assets.action_for(self.main_menu_index));
        match action {
            Some(MainMenuAction::StoryMode) => self.load_story_menu(),
            Some(MainMenuAction::Freeplay) => self.enter_song_select(),
            Some(MainMenuAction::Options) => self.load_options_menu(),
            Some(MainMenuAction::Credits) => self.load_credits_screen(),
            None => {}
        }
    }

    fn handle_story_menu_input(&mut self, action: InputAction, state: ElementState) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        // ref: bdedc0aa:source/funkin/ui/story/StoryMenuState.hx:357-428
        let item_count = self
            .story_menu_assets
            .as_ref()
            .map(|assets| assets.item_count())
            .unwrap_or(0)
            .max(1);
        let old_index = self.story_menu_index;
        let old_difficulty = self.story_menu_difficulty;
        match action {
            InputAction::LaneUp | InputAction::UiUp => {
                self.story_menu_index = (self.story_menu_index + item_count - 1) % item_count;
                self.clamp_story_difficulty();
            }
            InputAction::LaneDown | InputAction::UiDown => {
                self.story_menu_index = (self.story_menu_index + 1) % item_count;
                self.clamp_story_difficulty();
            }
            InputAction::LaneLeft | InputAction::UiLeft => {
                if let Some(assets) = self.story_menu_assets.as_ref() {
                    self.story_menu_difficulty = assets
                        .previous_difficulty(self.story_menu_index, self.story_menu_difficulty);
                }
            }
            InputAction::LaneRight | InputAction::UiRight => {
                if let Some(assets) = self.story_menu_assets.as_ref() {
                    self.story_menu_difficulty =
                        assets.next_difficulty(self.story_menu_index, self.story_menu_difficulty);
                }
            }
            InputAction::Confirm => self.confirm_story_menu_item(),
            InputAction::Back => self.load_main_menu(),
            _ => {}
        }
        if self.mode == AppMode::StoryMenu
            && (self.story_menu_index != old_index || self.story_menu_difficulty != old_difficulty)
        {
            self.rebuild_story_menu_commands();
        }
        true
    }

    fn clamp_story_difficulty(&mut self) {
        if let Some(assets) = self.story_menu_assets.as_ref() {
            self.story_menu_difficulty =
                assets.difficulty_for_level(self.story_menu_index, self.story_menu_difficulty);
        }
    }

    fn confirm_story_menu_item(&mut self) {
        let Some(assets) = self.story_menu_assets.as_ref() else {
            return;
        };
        let difficulty =
            assets.difficulty_for_level(self.story_menu_index, self.story_menu_difficulty);
        if let Some(songs) = assets.preview_playlist(self.story_menu_index) {
            self.start_story_playlist(songs, difficulty);
        } else if let Some(selection) =
            assets.preview_selection(self.story_menu_index, self.story_menu_difficulty)
        {
            self.preview_selection = selection;
            self.enter_play();
        }
    }

    fn handle_song_select_input(&mut self, action: InputAction, state: ElementState) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:1815-1868
        let old = self.preview_selection;
        match action {
            InputAction::LaneUp | InputAction::UiUp => {
                self.preview_selection = self.preview_selection.previous_song();
            }
            InputAction::LaneDown | InputAction::UiDown => {
                self.preview_selection = self.preview_selection.next_song();
            }
            InputAction::LaneLeft | InputAction::UiLeft => {
                self.preview_selection = self.preview_selection.previous_difficulty();
            }
            InputAction::LaneRight | InputAction::UiRight => {
                self.preview_selection = self.preview_selection.next_difficulty();
            }
            InputAction::Confirm => self.enter_play(),
            InputAction::Back => self.load_main_menu(),
            _ => {}
        }
        if self.mode == AppMode::SongSelect && self.preview_selection != old {
            self.update_window_title();
            self.rebuild_song_select_commands();
        }
        true
    }

    pub(super) fn enter_song_select(&mut self) {
        self.mode = AppMode::SongSelect;
        self.title_assets = None;
        self.main_menu_assets = None;
        self.credits_assets = None;
        self.options_menu_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.pause_menu = None;
        self.clear_play_state_for_menu();
        self.load_freeplay_assets();
        self.rebuild_song_select_commands();
    }

    fn load_freeplay_assets(&mut self) {
        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        match load_freeplay_screen_assets(&runtime.rs.device, &runtime.rs.queue) {
            Ok(mut freeplay) => {
                self.atlases = std::mem::take(&mut freeplay.textures);
                self.freeplay_assets = Some(freeplay);
            }
            Err(e) => tracing::warn!(target: "rustic.asset", "freeplay assets unavailable: {e:#}"),
        }
    }

    pub(super) fn clear_play_state_for_menu(&mut self) {
        self.play_state = None;
        self.game_over = None;
        self.pause_menu = None;
        self.pause_music.stop(&self.mixer);
        self.characters = None;
        self.bitmap_text_skin = None;
        self.note_skin = None;
        self.note_splash_skin = None;
        self.hold_cover_skin = None;
        self.hud_skin = None;
        self.popup_skin = None;
        self.countdown_skin = None;
        self.score_popups = Default::default();
        self.note_splashes = Default::default();
        self.hold_covers = Default::default();
        self.active_holds = Default::default();
        self.held_lanes = Default::default();
        self.opponent_receptors = Default::default();
        self.clear_story_playlist();
        self.static_cmds = RenderCommandList::new();
        self.atlases.clear();
        set_vocals_gain(&self.mixer, 1.0);
        if let Err(e) = self.mixer.edit(|mixer| {
            mixer.pause();
            mixer.seek(Samples(0))?;
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "pause song select audio: {e:#}");
        }
        self.update_window_title();
    }

    pub(super) fn enter_play(&mut self) {
        self.mode = AppMode::Play;
        self.title_assets = None;
        self.main_menu_assets = None;
        self.credits_assets = None;
        self.options_menu_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.load_selected_song();
    }

    pub(super) fn title_cursor(&self, sample_rate: u32) -> Samples {
        let elapsed = self.title_start.elapsed().as_secs_f64();
        Samples((elapsed * f64::from(sample_rate)).round() as i64)
    }
}
