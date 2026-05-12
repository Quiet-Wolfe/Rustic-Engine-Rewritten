use super::App;
use crate::camera_fx::CameraFx;
use crate::song_audio::play_sample_rate;
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

    pub(super) fn input_cursor(&mut self) -> Samples {
        match self.mode {
            AppMode::Title => self.title_cursor(play_sample_rate(&self.mixer)),
            AppMode::Play => self.advance_song_clock(),
        }
    }

    pub(super) fn handle_title_input(
        &mut self,
        action: InputAction,
        state: ElementState,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        if self.mode != AppMode::Title {
            return false;
        }
        if state != ElementState::Pressed {
            return true;
        }
        match action {
            InputAction::Confirm => self.enter_play(),
            InputAction::Back => event_loop.exit(),
            _ => {}
        }
        true
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
