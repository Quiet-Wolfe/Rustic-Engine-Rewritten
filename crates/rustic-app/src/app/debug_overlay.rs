use super::App;
use crate::app_text::{push_debug_overlay_text, DebugOverlayLine};
use rustic_core::ids::CameraId;
use rustic_core::time::Samples;
use std::time::Instant;

impl App {
    pub(super) fn toggle_debug_overlay(&mut self) {
        self.debug_overlay = !self.debug_overlay;
    }

    pub(super) fn update_debug_frame_time(&mut self) {
        let now = Instant::now();
        let dt = now.saturating_duration_since(self.last_frame_at);
        self.last_frame_at = now;
        let secs = dt.as_secs_f32();
        if secs > 0.0 {
            let fps = 1.0 / secs;
            self.debug_fps = if self.debug_fps <= 0.0 {
                fps
            } else {
                self.debug_fps * 0.9 + fps * 0.1
            };
        }
    }

    pub(super) fn append_debug_overlay_commands(&mut self, cursor: Samples, sample_rate: u32) {
        if !self.debug_overlay {
            return;
        }

        let cursor_ms = cursor.0 as f64 * 1000.0 / f64::from(sample_rate.max(1));
        let mut lines = vec![
            DebugOverlayLine::new(format!("fps {:.1}", self.debug_fps)),
            DebugOverlayLine::new(format!(
                "{} {} @ {:.1}ms",
                self.preview_selection.song.display_name(),
                self.preview_selection.difficulty.as_str(),
                cursor_ms
            )),
            DebugOverlayLine::new(format!(
                "audio {} mixer {}",
                if self.audio_output.is_some() {
                    "on"
                } else {
                    "fallback"
                },
                self.mixer.sample_cursor().0
            )),
        ];

        if let Some(state) = &self.play_state {
            lines.push(DebugOverlayLine::new(format!(
                "score {} combo {} health {:.3}",
                state.score, state.combo, state.health
            )));
        }
        if let Some(rt) = &self.runtime {
            lines.push(DebugOverlayLine::new(format!(
                "surface {}x{} {:?}",
                rt.surface_cfg.width, rt.surface_cfg.height, rt.surface_cfg.present_mode
            )));
            lines.push(DebugOverlayLine::new(format!(
                "adapter {:?} {}",
                rt.rs.backend, rt.rs.adapter_info.name
            )));
        }
        for id in [CameraId(0), CameraId(1), CameraId(2)] {
            if let Some(camera) = self.cameras.get(id) {
                lines.push(DebugOverlayLine::new(format!(
                    "{} pos {:.1},{:.1} zoom {:.3}",
                    camera.name, camera.position.x, camera.position.y, camera.zoom
                )));
            }
        }
        push_debug_overlay_text(&mut self.text_cmds, lines);
    }
}
