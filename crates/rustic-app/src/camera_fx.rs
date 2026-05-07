//! App-owned camera zoom behavior ported from base FNF `PlayState`.

use rustic_core::ids::CameraId;
use rustic_core::time::Samples;
use rustic_render::CameraRegistry;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CameraFx {
    stage_game_zoom: f32,
    current_game_zoom: f32,
    camera_bop_multiplier: f32,
    zooming: bool,
    last_step: i64,
    zoom_tween: Option<CameraZoomTween>,
}

#[derive(Debug, Clone, Copy)]
struct CameraZoomTween {
    start_zoom: f32,
    target_zoom: f32,
    start_cursor: Samples,
    duration_samples: i64,
    ease: CameraEase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraEase {
    Linear,
    Instant,
    QuadIn,
    QuadOut,
    QuadInOut,
    QuintInOut,
    ExpoIn,
    ExpoOut,
    ExpoInOut,
    SmoothStep,
    CircOut,
    ElasticInOut,
}

impl Default for CameraFx {
    fn default() -> Self {
        Self {
            stage_game_zoom: 1.0,
            current_game_zoom: 1.0,
            camera_bop_multiplier: 1.0,
            zooming: false,
            last_step: -1,
            zoom_tween: None,
        }
    }
}

impl CameraFx {
    pub(crate) fn reset(&mut self, cameras: &mut CameraRegistry, default_game_zoom: f32) {
        self.stage_game_zoom = default_game_zoom;
        self.current_game_zoom = default_game_zoom;
        self.camera_bop_multiplier = 1.0;
        self.zooming = false;
        self.last_step = -1;
        self.zoom_tween = None;
        if let Some(camera) = cameras.get_mut(CameraId(0)) {
            camera.zoom = default_game_zoom;
        }
        if let Some(camera) = cameras.get_mut(CameraId(1)) {
            camera.zoom = 1.0;
        }
    }

    pub(crate) fn enable_zooming(&mut self) {
        // ref: bdedc0aa:source/funkin/play/PlayState.hx:1855-1861
        self.zooming = true;
    }

    pub(crate) fn zoom_camera(
        &mut self,
        cameras: &mut CameraRegistry,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        event: ZoomCameraEvent<'_>,
    ) {
        // ref: bdedc0aa:source/funkin/play/event/ZoomCameraSongEvent.hx:47-77
        // ref: bdedc0aa:source/funkin/play/PlayState.hx:3963-3981
        let target_zoom = event.zoom
            * if event.direct {
                1.0
            } else {
                self.stage_game_zoom
            };
        let ease = CameraEase::from_name(event.ease);
        let duration_samples = steps_to_samples(event.duration_steps, sample_rate, bpm);
        if ease == CameraEase::Instant || duration_samples <= 0 {
            self.current_game_zoom = target_zoom;
            self.zoom_tween = None;
            self.write_zooms(cameras);
            return;
        }
        self.zoom_tween = Some(CameraZoomTween {
            start_zoom: self.current_game_zoom,
            target_zoom,
            start_cursor: cursor,
            duration_samples,
            ease,
        });
    }

    pub(crate) fn update(
        &mut self,
        cameras: &mut CameraRegistry,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        self.update_zoom_tween(cursor);
        // ref: bdedc0aa:source/funkin/play/PlayState.hx:1223-1229,1855-1861
        if self.zooming {
            self.camera_bop_multiplier = camera_bop_decay(self.camera_bop_multiplier);
            self.update_camera_bop(cameras, cursor, sample_rate, bpm);
        }
        self.write_zooms(cameras);
    }

    fn update_zoom_tween(&mut self, cursor: Samples) {
        let Some(tween) = self.zoom_tween else {
            return;
        };
        let elapsed = cursor.0.saturating_sub(tween.start_cursor.0).max(0);
        let progress = elapsed as f32 / tween.duration_samples.max(1) as f32;
        self.current_game_zoom = tween.start_zoom
            + (tween.target_zoom - tween.start_zoom) * ease_progress(tween.ease, progress);
        if progress >= 1.0 {
            self.current_game_zoom = tween.target_zoom;
            self.zoom_tween = None;
        }
    }

    fn update_camera_bop(
        &mut self,
        cameras: &mut CameraRegistry,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        if let Some(camera) = cameras.get_mut(CameraId(1)) {
            camera.zoom = hud_zoom_decay(camera.zoom);
        }

        let step = camera_step_index(cursor, sample_rate, bpm);
        if step <= self.last_step {
            return;
        }
        self.last_step = step;
        if step.rem_euclid(16) != 0 {
            return;
        }
        if let Some(camera) = cameras.get_mut(CameraId(1)) {
            if camera.zoom < 1.35 {
                self.camera_bop_multiplier = 1.015;
                camera.zoom += 0.03;
            }
        }
    }

    fn write_zooms(&mut self, cameras: &mut CameraRegistry) {
        if let Some(camera) = cameras.get_mut(CameraId(0)) {
            camera.zoom = self.current_game_zoom * self.camera_bop_multiplier;
        }
    }
}

pub(crate) struct ZoomCameraEvent<'a> {
    pub(crate) zoom: f32,
    pub(crate) duration_steps: f32,
    pub(crate) direct: bool,
    pub(crate) ease: &'a str,
}

impl CameraEase {
    fn from_name(name: &str) -> Self {
        match name {
            "INSTANT" => Self::Instant,
            "quadIn" | "quad" => Self::QuadIn,
            "quadOut" => Self::QuadOut,
            "quadInOut" => Self::QuadInOut,
            "quintInOut" => Self::QuintInOut,
            "expoIn" | "expo" => Self::ExpoIn,
            "expoOut" => Self::ExpoOut,
            "expoInOut" => Self::ExpoInOut,
            "smoothStep" | "smoothStepIn" => Self::SmoothStep,
            "circOut" => Self::CircOut,
            "elasticInOut" => Self::ElasticInOut,
            _ => Self::Linear,
        }
    }
}

fn camera_bop_decay(current: f32) -> f32 {
    1.0 + (current - 1.0) * 0.95
}

fn hud_zoom_decay(current: f32) -> f32 {
    1.0 + (current - 1.0) * 0.95
}

fn camera_step_index(cursor: Samples, sample_rate: u32, bpm: f64) -> i64 {
    let samples_per_step = f64::from(sample_rate.max(1)) * 60.0 / bpm.max(1.0) / 4.0;
    (cursor.0.max(0) as f64 / samples_per_step).floor() as i64
}

fn steps_to_samples(steps: f32, sample_rate: u32, bpm: f64) -> i64 {
    (f64::from(steps.max(0.0)) * f64::from(sample_rate.max(1)) * 60.0 / bpm.max(1.0) / 4.0).round()
        as i64
}

fn ease_progress(ease: CameraEase, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match ease {
        CameraEase::Instant => 1.0,
        CameraEase::Linear => t,
        CameraEase::QuadIn => t * t,
        CameraEase::QuadOut => 1.0 - (1.0 - t).powi(2),
        CameraEase::QuadInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) * 0.5
            }
        }
        CameraEase::QuintInOut => {
            if t < 0.5 {
                16.0 * t.powi(5)
            } else {
                1.0 - (-2.0 * t + 2.0).powi(5) * 0.5
            }
        }
        CameraEase::ExpoIn => {
            if t == 0.0 {
                0.0
            } else {
                2.0_f32.powf(10.0 * t - 10.0)
            }
        }
        CameraEase::ExpoOut => {
            if t == 1.0 {
                1.0
            } else {
                1.0 - 2.0_f32.powf(-10.0 * t)
            }
        }
        CameraEase::ExpoInOut => {
            if t == 0.0 || t == 1.0 {
                t
            } else if t < 0.5 {
                2.0_f32.powf(20.0 * t - 10.0) * 0.5
            } else {
                (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) * 0.5
            }
        }
        CameraEase::SmoothStep => t * t * (3.0 - 2.0 * t),
        CameraEase::CircOut => (1.0 - (t - 1.0).powi(2)).sqrt(),
        CameraEase::ElasticInOut => elastic_in_out(t),
    }
}

fn elastic_in_out(t: f32) -> f32 {
    if t == 0.0 || t == 1.0 {
        return t;
    }
    let c = std::f32::consts::TAU / 4.5;
    if t < 0.5 {
        -(2.0_f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c).sin()) * 0.5
    } else {
        2.0_f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c).sin() * 0.5 + 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_bop_decay_matches_v085_lerp_direction() {
        let decayed = camera_bop_decay(1.20);
        assert!((decayed - 1.19).abs() < 0.0001);
    }

    #[test]
    fn camera_step_index_uses_audio_cursor() {
        assert_eq!(camera_step_index(Samples(-1), 48_000, 100.0), 0);
        assert_eq!(camera_step_index(Samples(7_199), 48_000, 100.0), 0);
        assert_eq!(camera_step_index(Samples(7_200), 48_000, 100.0), 1);
    }

    #[test]
    fn reset_sets_base_camera_zooms() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut fx = CameraFx::default();

        fx.reset(&mut cameras, 1.05);

        assert!((cameras.get(CameraId(0)).map_or(0.0, |camera| camera.zoom) - 1.05).abs() < 1e-6);
        assert!((cameras.get(CameraId(1)).map_or(0.0, |camera| camera.zoom) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn zoom_camera_event_tweens_by_song_cursor() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut fx = CameraFx::default();
        fx.reset(&mut cameras, 1.05);

        fx.zoom_camera(
            &mut cameras,
            Samples(0),
            48_000,
            120.0,
            ZoomCameraEvent {
                zoom: 1.30,
                duration_steps: 4.0,
                direct: true,
                ease: "linear",
            },
        );
        fx.update(&mut cameras, Samples(12_000), 48_000, 120.0);

        assert!((cameras.get(CameraId(0)).map_or(0.0, |camera| camera.zoom) - 1.175).abs() < 1e-6);

        fx.update(&mut cameras, Samples(24_000), 48_000, 120.0);

        assert!((cameras.get(CameraId(0)).map_or(0.0, |camera| camera.zoom) - 1.30).abs() < 1e-6);
    }

    #[test]
    fn zoom_camera_stage_mode_multiplies_stage_zoom() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut fx = CameraFx::default();
        fx.reset(&mut cameras, 1.05);

        fx.zoom_camera(
            &mut cameras,
            Samples(0),
            48_000,
            120.0,
            ZoomCameraEvent {
                zoom: 1.20,
                duration_steps: 0.0,
                direct: false,
                ease: "linear",
            },
        );

        assert!((cameras.get(CameraId(0)).map_or(0.0, |camera| camera.zoom) - 1.26).abs() < 1e-6);
    }
}
