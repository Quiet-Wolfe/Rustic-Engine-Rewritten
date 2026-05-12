use super::App;
use crate::app_text::song_select_text_commands;
use crate::camera_fx::CameraFx;
use crate::song_audio::{play_sample_rate, set_vocals_gain};
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
    SongSelect,
    Play,
}

impl App {
    pub(super) fn load_title_screen(&mut self) {
        self.mode = AppMode::Title;
        self.title_start = Instant::now();
        self.title_assets = None;
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
            .unwrap_or_else(RenderCommandList::new);
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    pub(super) fn rebuild_song_select_commands(&mut self) {
        self.cmds = RenderCommandList::new();
        self.text_cmds = song_select_text_commands(self.preview_selection);
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = self.title_cursor(sample_rate);
        self.append_debug_overlay_commands(cursor, sample_rate);
    }

    pub(super) fn input_cursor(&mut self) -> Samples {
        match self.mode {
            AppMode::Title | AppMode::SongSelect => {
                self.title_cursor(play_sample_rate(&self.mixer))
            }
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
            InputAction::Confirm => self.enter_song_select(),
            InputAction::Back => event_loop.exit(),
            _ => {}
        }
        true
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
            InputAction::Back => self.load_title_screen(),
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
        self.play_state = None;
        self.game_over = None;
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
        self.title_start = Instant::now();
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
        self.rebuild_song_select_commands();
    }

    fn enter_play(&mut self) {
        self.mode = AppMode::Play;
        self.title_assets = None;
        self.load_selected_song();
    }

    fn title_cursor(&self, sample_rate: u32) -> Samples {
        let elapsed = self.title_start.elapsed().as_secs_f64();
        Samples((elapsed * f64::from(sample_rate)).round() as i64)
    }
}
