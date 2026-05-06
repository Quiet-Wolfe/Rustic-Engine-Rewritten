//! App-owned camera zoom behavior ported from base FNF `PlayState`.

use rustic_core::ids::CameraId;
use rustic_core::time::Samples;
use rustic_render::CameraRegistry;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CameraFx {
    default_game_zoom: f32,
    zooming: bool,
    last_beat: i64,
}

impl Default for CameraFx {
    fn default() -> Self {
        Self {
            default_game_zoom: 1.0,
            zooming: false,
            last_beat: -1,
        }
    }
}

impl CameraFx {
    pub(crate) fn reset(&mut self, cameras: &mut CameraRegistry, default_game_zoom: f32) {
        self.default_game_zoom = default_game_zoom;
        self.zooming = false;
        self.last_beat = -1;
        if let Some(camera) = cameras.get_mut(CameraId(0)) {
            camera.zoom = default_game_zoom;
        }
        if let Some(camera) = cameras.get_mut(CameraId(1)) {
            camera.zoom = 1.0;
        }
    }

    pub(crate) fn enable_zooming(&mut self) {
        // ref: 50fccded:source/PlayState.hx:1528-1530
        self.zooming = true;
    }

    pub(crate) fn update(
        &mut self,
        cameras: &mut CameraRegistry,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        // ref: 50fccded:source/PlayState.hx:1408-1411,2284-2287
        if !self.zooming {
            return;
        }
        if let Some(camera) = cameras.get_mut(CameraId(0)) {
            camera.zoom = camera_zoom_decay(camera.zoom, self.default_game_zoom);
        }
        if let Some(camera) = cameras.get_mut(CameraId(1)) {
            camera.zoom = camera_zoom_decay(camera.zoom, 1.0);
        }

        let beat = camera_beat_index(cursor, sample_rate, bpm);
        if beat <= self.last_beat {
            return;
        }
        self.last_beat = beat;
        if beat.rem_euclid(4) != 0 {
            return;
        }
        let mut bumped = false;
        if let Some(camera) = cameras.get_mut(CameraId(0)) {
            if camera.zoom < 1.35 {
                camera.zoom += 0.015;
                bumped = true;
            }
        }
        if bumped {
            if let Some(camera) = cameras.get_mut(CameraId(1)) {
                camera.zoom += 0.03;
            }
        }
    }
}

fn camera_zoom_decay(current: f32, target: f32) -> f32 {
    // ref: 50fccded:source/PlayState.hx:1410-1411
    target + (current - target) * 0.95
}

fn camera_beat_index(cursor: Samples, sample_rate: u32, bpm: f64) -> i64 {
    let samples_per_beat = f64::from(sample_rate.max(1)) * 60.0 / bpm.max(1.0);
    (cursor.0.max(0) as f64 / samples_per_beat).floor() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_zoom_decay_matches_og_lerp_direction() {
        let decayed = camera_zoom_decay(1.20, 1.05);
        assert!((decayed - 1.1925).abs() < 0.0001);
    }

    #[test]
    fn camera_beat_index_uses_audio_cursor() {
        assert_eq!(camera_beat_index(Samples(-1), 48_000, 100.0), 0);
        assert_eq!(camera_beat_index(Samples(28_799), 48_000, 100.0), 0);
        assert_eq!(camera_beat_index(Samples(28_800), 48_000, 100.0), 1);
    }

    #[test]
    fn reset_sets_base_camera_zooms() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut fx = CameraFx::default();

        fx.reset(&mut cameras, 1.05);

        assert!((cameras.get(CameraId(0)).map_or(0.0, |camera| camera.zoom) - 1.05).abs() < 1e-6);
        assert!((cameras.get(CameraId(1)).map_or(0.0, |camera| camera.zoom) - 1.0).abs() < 1e-6);
    }
}
