use super::App;
// LINT-ALLOW: long-file app flow is being split screen by screen.
use crate::app_text::song_select_text_commands;
use crate::camera_fx::CameraFx;
use crate::freeplay_assets::{
    load_freeplay_assets_for_style as load_freeplay_screen_assets, FreeplayStyle,
};
use crate::main_menu_assets::{load_main_menu_assets, MainMenuAction};
use crate::menu_audio::MenuSound;
use crate::preview_song::{PreviewSelection, PreviewSong, VARIATION_BF, VARIATION_PICO};
use crate::song_audio::{play_sample_rate, set_vocals_gain};
use crate::story_menu_assets::load_story_menu_assets;
use crate::title_assets::load_title_screen_assets;
use rustic_audio::Stem;
use rustic_core::input::InputAction;
use rustic_core::time::Samples;
use rustic_render::{CameraRegistry, RenderCommandList, TextCommandList};
use std::process::Command;
use std::time::Instant;
use winit::event::ElementState;
use winit::event_loop::ActiveEventLoop;

const MAIN_MENU_CONFIRM_DELAY_SECONDS: i64 = 1;
const MERCH_URL_FALLBACK: &str = "https://needlejuicerecords.com/en-ca/pages/friday-night-funkin";

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
        self.main_menu_confirm_started_at = None;
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
        self.start_menu_music();

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
        self.update_freeplay_preview();
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        if let Some(at) = self.freeplay_confirm_at {
            if cursor.0 >= at.0 {
                self.freeplay_confirm_at = None;
                self.enter_play();
                return;
            }
        }
        if let Some(assets) = self.freeplay_assets.as_mut() {
            assets.tick(cursor, sample_rate);
        }
        if let Some(assets) = self.freeplay_assets.as_ref() {
            self.cmds = assets.commands(
                self.preview_selection,
                self.freeplay_selected_index,
                cursor,
                sample_rate,
            );
            self.text_cmds =
                assets.text_commands(self.freeplay_selected_index, cursor, sample_rate);
        } else {
            self.cmds = RenderCommandList::new();
            self.text_cmds = song_select_text_commands(self.preview_selection, true);
        }
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    pub(super) fn start_menu_music(&mut self) {
        if self.audio_output.is_some() {
            self.menu_music.start_or_warn(&self.mixer);
        }
    }

    pub(super) fn stop_menu_music(&mut self) {
        self.menu_music.stop(&self.mixer);
    }

    pub(super) fn start_freeplay_preview(&mut self) {
        if self.audio_output.is_some() {
            self.stop_menu_music();
            if self
                .freeplay_assets
                .as_ref()
                .is_some_and(|assets| assets.is_random_at(self.freeplay_selected_index))
            {
                self.freeplay_preview.start_random_or_warn(&self.mixer);
            } else {
                self.freeplay_preview
                    .start_selection_or_warn(&self.mixer, self.preview_selection);
            }
        }
    }

    pub(super) fn update_freeplay_preview(&mut self) {
        if self.audio_output.is_some() {
            self.freeplay_preview.update(&self.mixer);
        }
    }

    pub(super) fn stop_freeplay_preview(&mut self) {
        self.freeplay_preview.stop(&self.mixer);
    }

    pub(super) fn load_main_menu(&mut self) {
        self.mode = AppMode::MainMenu;
        self.title_start = Instant::now();
        self.title_assets = None;
        self.main_menu_assets = None;
        self.main_menu_confirm_started_at = None;
        self.credits_assets = None;
        self.options_menu_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.clear_play_state_for_menu();
        self.start_menu_music();
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
        if let Some(started_at) = self.main_menu_confirm_started_at {
            let delay = i64::from(sample_rate.max(1)) * MAIN_MENU_CONFIRM_DELAY_SECONDS;
            if cursor.0.saturating_sub(started_at.0) >= delay {
                self.main_menu_confirm_started_at = None;
                self.start_confirmed_main_menu_item();
                return;
            }
        }
        self.cmds = self
            .main_menu_assets
            .as_ref()
            .map(|assets| {
                assets.commands(
                    self.main_menu_index,
                    cursor,
                    sample_rate,
                    self.main_menu_confirm_started_at,
                )
            })
            .unwrap_or_default();
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    pub(super) fn load_story_menu(&mut self) {
        self.mode = AppMode::StoryMenu;
        self.title_start = Instant::now();
        self.title_assets = None;
        self.main_menu_assets = None;
        self.main_menu_confirm_started_at = None;
        self.credits_assets = None;
        self.options_menu_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.story_menu_confirm_started_at = None;
        self.clear_play_state_for_menu();
        self.start_menu_music();
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
        if let Some(started_at) = self.story_menu_confirm_started_at {
            if cursor.0 >= started_at.0.saturating_add(i64::from(sample_rate.max(1))) {
                self.story_menu_confirm_started_at = None;
                self.start_confirmed_story_playlist();
                return;
            }
        }
        if let Some(assets) = self.story_menu_assets.as_ref() {
            self.cmds = assets.commands(
                self.story_menu_index,
                self.story_menu_difficulty,
                cursor,
                sample_rate,
                self.story_menu_confirm_started_at,
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
            AppMode::Play if self.dialogue.is_some() => self.song_start_cursor,
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
            InputAction::Confirm => {
                self.play_menu_sound(MenuSound::TitleConfirm);
                self.load_main_menu();
            }
            InputAction::Back => {
                self.play_menu_sound(MenuSound::Cancel);
                event_loop.exit();
            }
            _ => {}
        }
        true
    }

    fn handle_main_menu_input(&mut self, action: InputAction, state: ElementState) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        if self.main_menu_confirm_started_at.is_some() {
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
                self.play_menu_sound(MenuSound::Scroll);
                self.rebuild_main_menu_commands();
            }
            InputAction::LaneDown | InputAction::UiDown => {
                self.main_menu_index = (self.main_menu_index + 1) % item_count;
                self.play_menu_sound(MenuSound::Scroll);
                self.rebuild_main_menu_commands();
            }
            InputAction::Confirm => {
                self.play_menu_sound(MenuSound::Confirm);
                self.confirm_main_menu_item();
            }
            InputAction::Back => {
                self.play_menu_sound(MenuSound::Cancel);
                self.load_title_screen();
            }
            _ => {}
        }
        true
    }

    fn confirm_main_menu_item(&mut self) {
        if self
            .main_menu_assets
            .as_ref()
            .and_then(|assets| assets.action_for(self.main_menu_index))
            .is_none()
        {
            return;
        }
        let sample_rate = play_sample_rate(&self.mixer);
        self.main_menu_confirm_started_at = Some(self.title_cursor(sample_rate));
        self.rebuild_main_menu_commands();
    }

    fn start_confirmed_main_menu_item(&mut self) {
        let action = self
            .main_menu_assets
            .as_ref()
            .and_then(|assets| assets.action_for(self.main_menu_index));
        match action {
            Some(MainMenuAction::StoryMode) => self.load_story_menu(),
            Some(MainMenuAction::Freeplay) => self.enter_song_select(),
            Some(MainMenuAction::Merch) => {
                self.open_merch_url();
                self.rebuild_main_menu_commands();
            }
            Some(MainMenuAction::Options) => self.load_options_menu(),
            Some(MainMenuAction::Credits) => self.load_credits_screen(),
            None => {}
        }
    }

    fn open_merch_url(&self) {
        if let Err(e) = open_url(MERCH_URL_FALLBACK) {
            tracing::warn!(target: "rustic.app", "open merch URL failed: {e}");
        }
    }

    fn handle_story_menu_input(&mut self, action: InputAction, state: ElementState) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        if self.story_menu_confirm_started_at.is_some() {
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
            InputAction::Back => {
                self.play_menu_sound(MenuSound::Cancel);
                self.load_main_menu();
            }
            _ => {}
        }
        if self.mode == AppMode::StoryMenu
            && (self.story_menu_index != old_index || self.story_menu_difficulty != old_difficulty)
        {
            self.play_menu_sound(MenuSound::Scroll);
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
        if assets.preview_playlist(self.story_menu_index).is_some() {
            self.play_menu_sound(MenuSound::Confirm);
            self.start_story_menu_confirm();
        } else if let Some(selection) =
            assets.preview_selection(self.story_menu_index, self.story_menu_difficulty)
        {
            self.play_menu_sound(MenuSound::Confirm);
            self.preview_selection = selection;
            self.enter_play();
        }
    }

    fn start_story_menu_confirm(&mut self) {
        let sample_rate = play_sample_rate(&self.mixer);
        self.story_menu_confirm_started_at = Some(self.title_cursor(sample_rate));
        self.rebuild_story_menu_commands();
    }

    fn start_confirmed_story_playlist(&mut self) {
        let Some(assets) = self.story_menu_assets.as_ref() else {
            return;
        };
        let difficulty =
            assets.difficulty_for_level(self.story_menu_index, self.story_menu_difficulty);
        if let Some(songs) = assets.preview_playlist(self.story_menu_index) {
            self.start_story_playlist(songs, difficulty);
        }
    }

    fn handle_song_select_input(&mut self, action: InputAction, state: ElementState) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        // While the freeplay Confirm animation is playing, ignore further input
        // so the player can't double-confirm or back out mid-transition.
        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:2738-2745 (busy flag)
        if self.freeplay_confirm_at.is_some() {
            return true;
        }
        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:1815-1868
        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:2578-2634 (random confirm)
        let old_index = self.freeplay_selected_index;
        let old = self.preview_selection;
        if is_freeplay_player_action(action) {
            let sample_rate = play_sample_rate(&self.mixer);
            let cursor = self.title_cursor(sample_rate);
            if let Some(assets) = self.freeplay_assets.as_mut() {
                assets.dj_on_player_action(cursor);
            }
        }
        match action {
            InputAction::LaneUp | InputAction::UiUp => {
                self.move_freeplay_selection(-1);
            }
            InputAction::LaneDown | InputAction::UiDown => {
                self.move_freeplay_selection(1);
            }
            InputAction::LaneLeft | InputAction::UiLeft => {
                self.change_freeplay_difficulty(-1);
            }
            InputAction::LaneRight | InputAction::UiRight => {
                self.change_freeplay_difficulty(1);
            }
            InputAction::UiSelect => {
                self.toggle_freeplay_character();
            }
            InputAction::UiJumpTop => {
                self.jump_freeplay_selection(0);
            }
            InputAction::UiJumpBottom => {
                self.jump_freeplay_selection_to_bottom();
            }
            InputAction::Confirm => {
                self.play_menu_sound(MenuSound::Confirm);
                if self.current_freeplay_is_random() && !self.choose_random_freeplay_song() {
                    self.play_menu_sound(MenuSound::Cancel);
                    return true;
                }
                self.start_freeplay_confirm();
            }
            InputAction::Back => {
                self.play_menu_sound(MenuSound::Cancel);
                self.load_main_menu();
            }
            _ => {}
        }
        if self.mode == AppMode::SongSelect
            && (self.preview_selection != old || self.freeplay_selected_index != old_index)
        {
            self.play_menu_sound(MenuSound::Scroll);
            self.start_freeplay_preview();
            self.update_window_title();
            self.rebuild_song_select_commands();
        }
        true
    }

    pub(super) fn enter_song_select(&mut self) {
        self.mode = AppMode::SongSelect;
        self.freeplay_selected_index = 0;
        self.freeplay_confirm_at = None;
        self.title_assets = None;
        self.main_menu_assets = None;
        self.main_menu_confirm_started_at = None;
        self.credits_assets = None;
        self.options_menu_assets = None;
        self.freeplay_assets = None;
        self.story_menu_assets = None;
        self.pause_menu = None;
        self.clear_play_state_for_menu();
        self.load_freeplay_assets();
        self.sync_freeplay_selection_to_preview();
        // Drop the DJ into its Intro animation anchored at "now" so the eject-in
        // plays once before settling into Idle.
        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:848-857
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        if let Some(assets) = self.freeplay_assets.as_mut() {
            assets.reset_dj_intro(cursor);
        }
        self.start_freeplay_preview();
        self.rebuild_song_select_commands();
    }

    /// Start the confirm sequence: trigger the DJ Confirm animation and gate
    /// `enter_play()` until the start-delay elapses. The Confirm sound was
    /// already played by the caller.
    /// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:2744-2846
    fn start_freeplay_confirm(&mut self) {
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        let delay_secs = if let Some(assets) = self.freeplay_assets.as_mut() {
            assets.dj_enter_confirm(cursor);
            assets.start_delay_secs
        } else {
            1.0
        };
        let delay_samples = (f64::from(sample_rate) * delay_secs) as i64;
        self.freeplay_confirm_at = Some(Samples(cursor.0 + delay_samples));
        // No further input until the delay elapses; the per-frame
        // rebuild_song_select_commands hook calls enter_play() at that point.
    }

    fn sync_freeplay_selection_to_preview(&mut self) {
        let Some(assets) = self.freeplay_assets.as_ref() else {
            return;
        };
        let Some(index) = assets.index_of(self.preview_selection.song) else {
            return;
        };
        self.freeplay_selected_index = index;
        if let Some(song) = assets.song_at(index) {
            self.preview_selection =
                self.freeplay_selection_for_song(song, self.preview_selection.difficulty);
        }
    }

    fn move_freeplay_selection(&mut self, delta: isize) {
        let count = self
            .freeplay_assets
            .as_ref()
            .map(|assets| assets.item_count())
            .unwrap_or(0);
        if count == 0 {
            self.preview_selection = if delta < 0 {
                self.preview_selection.previous_song()
            } else {
                self.preview_selection.next_song()
            }
            .with_variation(Some(self.freeplay_variation));
            return;
        }
        let next = (self.freeplay_selected_index as isize + delta).rem_euclid(count as isize);
        self.freeplay_selected_index = next as usize;
        if let Some(song) = self
            .freeplay_assets
            .as_ref()
            .and_then(|assets| assets.song_at(self.freeplay_selected_index))
        {
            self.preview_selection =
                self.freeplay_selection_for_song(song, self.preview_selection.difficulty);
        }
    }

    fn change_freeplay_difficulty(&mut self, delta: isize) {
        if self.current_freeplay_is_random() {
            self.preview_selection.difficulty = if delta < 0 {
                self.preview_selection.difficulty.previous_freeplay()
            } else {
                self.preview_selection.difficulty.next_freeplay()
            };
        } else if delta < 0 {
            self.preview_selection = self
                .freeplay_assets
                .as_ref()
                .map(|assets| assets.cycle_selection_difficulty(self.preview_selection, -1))
                .unwrap_or_else(|| self.preview_selection.previous_difficulty());
        } else {
            self.preview_selection = self
                .freeplay_assets
                .as_ref()
                .map(|assets| assets.cycle_selection_difficulty(self.preview_selection, 1))
                .unwrap_or_else(|| self.preview_selection.next_difficulty());
        }
    }

    fn current_freeplay_is_random(&self) -> bool {
        self.freeplay_assets
            .as_ref()
            .is_some_and(|assets| assets.is_random_at(self.freeplay_selected_index))
    }

    fn choose_random_freeplay_song(&mut self) -> bool {
        let Some(assets) = self.freeplay_assets.as_ref() else {
            return false;
        };
        let song_count = assets.item_count().saturating_sub(1);
        if song_count == 0 {
            return false;
        }
        let random_offset = (self.boot_instant.elapsed().as_nanos() as usize % song_count) + 1;
        let Some(song) = assets.song_at(random_offset) else {
            return false;
        };
        self.freeplay_selected_index = random_offset;
        self.preview_selection =
            self.freeplay_selection_for_song(song, self.preview_selection.difficulty);
        true
    }

    fn jump_freeplay_selection_to_bottom(&mut self) {
        let Some(last_index) = self
            .freeplay_assets
            .as_ref()
            .and_then(|assets| assets.item_count().checked_sub(1))
        else {
            return;
        };
        self.jump_freeplay_selection(last_index);
    }

    fn jump_freeplay_selection(&mut self, index: usize) {
        let count = self
            .freeplay_assets
            .as_ref()
            .map(|assets| assets.item_count())
            .unwrap_or(0);
        if count == 0 {
            return;
        }
        self.freeplay_selected_index = index.min(count - 1);
        if let Some(song) = self
            .freeplay_assets
            .as_ref()
            .and_then(|assets| assets.song_at(self.freeplay_selected_index))
        {
            self.preview_selection =
                self.freeplay_selection_for_song(song, self.preview_selection.difficulty);
        }
    }

    fn toggle_freeplay_character(&mut self) {
        self.freeplay_variation = match self.freeplay_variation {
            VARIATION_BF => VARIATION_PICO,
            _ => VARIATION_BF,
        };
        let selected_song = self
            .freeplay_assets
            .as_ref()
            .and_then(|assets| assets.song_at(self.freeplay_selected_index));
        if let Some(song) = selected_song {
            self.preview_selection =
                self.freeplay_selection_for_song(song, self.preview_selection.difficulty);
        } else {
            self.preview_selection = self
                .preview_selection
                .with_variation(Some(self.freeplay_variation));
        }
        self.load_freeplay_assets();
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        if let Some(assets) = self.freeplay_assets.as_mut() {
            assets.reset_dj_intro(cursor);
        }
    }

    fn freeplay_selection_for_song(
        &self,
        song: PreviewSong,
        difficulty: crate::preview_song::PreviewDifficulty,
    ) -> PreviewSelection {
        let selection =
            PreviewSelection::new(song, difficulty).with_variation(Some(self.freeplay_variation));
        self.freeplay_assets
            .as_ref()
            .map(|assets| assets.clamp_selection_difficulty(selection))
            .unwrap_or(selection)
    }

    fn load_freeplay_assets(&mut self) {
        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        let style = match self.freeplay_variation {
            VARIATION_PICO => FreeplayStyle::Pico,
            _ => FreeplayStyle::Bf,
        };
        match load_freeplay_screen_assets(&runtime.rs.device, &runtime.rs.queue, style) {
            Ok(mut freeplay) => {
                self.atlases = std::mem::take(&mut freeplay.textures);
                self.freeplay_assets = Some(freeplay);
            }
            Err(e) => tracing::warn!(target: "rustic.asset", "freeplay assets unavailable: {e:#}"),
        }
    }

    pub(super) fn clear_play_state_for_menu(&mut self) {
        self.play_state = None;
        self.subtitle_track = None;
        self.stress_pico_end_cutscene = None;
        self.winter_horrorland_cutscene = None;
        self.game_over = None;
        self.game_over_restart = None;
        self.pause_menu = None;
        self.freeplay_confirm_at = None;
        self.main_menu_confirm_started_at = None;
        self.story_menu_confirm_started_at = None;
        self.pause_music.stop(&self.mixer);
        self.game_over_audio.stop(&self.mixer);
        self.stop_freeplay_preview();
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
            mixer.seek_stems(Samples(0), &[Stem::Instrumental, Stem::Vocals])?;
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "pause song select audio: {e:#}");
        }
        self.update_window_title();
    }

    pub(super) fn enter_play(&mut self) {
        self.mode = AppMode::Play;
        self.death_counter = 0;
        self.practice_mode = false;
        self.stop_menu_music();
        self.stop_freeplay_preview();
        self.title_assets = None;
        self.main_menu_assets = None;
        self.main_menu_confirm_started_at = None;
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

fn is_freeplay_player_action(action: InputAction) -> bool {
    matches!(
        action,
        InputAction::LaneUp
            | InputAction::UiUp
            | InputAction::LaneDown
            | InputAction::UiDown
            | InputAction::LaneLeft
            | InputAction::UiLeft
            | InputAction::LaneRight
            | InputAction::UiRight
            | InputAction::UiSelect
            | InputAction::UiJumpTop
            | InputAction::UiJumpBottom
            | InputAction::Confirm
            | InputAction::Back
    )
}

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        let _ = url;
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "opening URLs is not supported on this platform",
        ));
    }

    command.spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freeplay_player_actions_reset_dj_idle_timer() {
        assert!(is_freeplay_player_action(InputAction::UiUp));
        assert!(is_freeplay_player_action(InputAction::UiSelect));
        assert!(is_freeplay_player_action(InputAction::Confirm));
        assert!(!is_freeplay_player_action(InputAction::Debug));
    }
}
