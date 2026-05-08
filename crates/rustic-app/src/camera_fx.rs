//! App-owned camera zoom behavior ported from base FNF `PlayState`.
// LINT-ALLOW: long-file camera zoom, bop, easing, and source-aligned tests stay together.

use rustic_core::ids::CameraId;
use rustic_core::time::Samples;
use rustic_render::CameraRegistry;

const DEFAULT_BOP_INTENSITY: f32 = 1.015;
const DEFAULT_ZOOM_RATE: f32 = 4.0;
const DEFAULT_ZOOM_OFFSET: f32 = 0.0;
const DEFAULT_HUD_CAMERA_ZOOM: f32 = 1.0;
const DEFAULT_CAMERA_FOLLOW_RATE: f32 = 0.04;
const STEPS_PER_BEAT: f32 = 4.0;
const MAX_RELATIVE_CAM_ZOOM: f32 = 1.35;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CameraFx {
    stage_game_zoom: f32,
    current_game_zoom: f32,
    camera_bop_multiplier: f32,
    camera_bop_intensity: f32,
    hud_camera_zoom_intensity: f32,
    camera_zoom_rate: f32,
    camera_zoom_rate_offset: f32,
    zooming: bool,
    last_step: i64,
    last_update_cursor: Option<Samples>,
    follow_target: glam::Vec2,
    follow_initialized: bool,
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
            camera_bop_intensity: DEFAULT_BOP_INTENSITY,
            hud_camera_zoom_intensity: (DEFAULT_BOP_INTENSITY - 1.0) * 2.0,
            camera_zoom_rate: DEFAULT_ZOOM_RATE,
            camera_zoom_rate_offset: DEFAULT_ZOOM_OFFSET,
            zooming: false,
            last_step: -1,
            last_update_cursor: None,
            follow_target: glam::Vec2::new(640.0, 360.0),
            follow_initialized: false,
            zoom_tween: None,
        }
    }
}

impl CameraFx {
    pub(crate) fn reset(&mut self, cameras: &mut CameraRegistry, default_game_zoom: f32) {
        self.stage_game_zoom = default_game_zoom;
        self.current_game_zoom = default_game_zoom;
        self.camera_bop_multiplier = 1.0;
        self.camera_bop_intensity = DEFAULT_BOP_INTENSITY;
        self.hud_camera_zoom_intensity = (DEFAULT_BOP_INTENSITY - 1.0) * 2.0;
        self.camera_zoom_rate = DEFAULT_ZOOM_RATE;
        self.camera_zoom_rate_offset = DEFAULT_ZOOM_OFFSET;
        self.zooming = false;
        self.last_step = -1;
        self.last_update_cursor = None;
        self.follow_target = glam::Vec2::new(640.0, 360.0);
        self.follow_initialized = false;
        self.zoom_tween = None;
        if let Some(camera) = cameras.get_mut(CameraId(0)) {
            camera.zoom = default_game_zoom;
        }
        if let Some(camera) = cameras.get_mut(CameraId(1)) {
            camera.zoom = DEFAULT_HUD_CAMERA_ZOOM;
        }
    }

    pub(crate) fn focus_camera(
        &mut self,
        cameras: &mut CameraRegistry,
        target: glam::Vec2,
        snap: bool,
    ) {
        self.follow_target = target;
        self.follow_initialized = true;
        if snap {
            if let Some(camera) = cameras.get_mut(CameraId(0)) {
                camera.position = target;
            }
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

    pub(crate) fn set_camera_bop(&mut self, event: CameraBopEvent) {
        // ref: bdedc0aa:source/funkin/play/event/SetCameraBopSongEvent.hx:45-53
        self.camera_bop_intensity = (DEFAULT_BOP_INTENSITY - 1.0) * event.intensity + 1.0;
        self.hud_camera_zoom_intensity = (DEFAULT_BOP_INTENSITY - 1.0) * event.intensity * 2.0;
        self.camera_zoom_rate = event.rate;
        self.camera_zoom_rate_offset = event.offset;
    }

    pub(crate) fn update(
        &mut self,
        cameras: &mut CameraRegistry,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        self.update_zoom_tween(cursor);
        let dt_frames = self.update_dt_frames(cursor, sample_rate);
        self.update_follow(cameras, dt_frames);
        // ref: bdedc0aa:source/funkin/play/PlayState.hx:1223-1229,1855-1861
        if self.zooming && self.camera_zoom_rate > 0.0 {
            self.camera_bop_multiplier = camera_bop_decay(self.camera_bop_multiplier, dt_frames);
            self.update_camera_bop(cameras, cursor, sample_rate, bpm, dt_frames);
        }
        self.write_zooms(cameras);
    }

    fn update_dt_frames(&mut self, cursor: Samples, sample_rate: u32) -> f32 {
        let dt_samples = self
            .last_update_cursor
            .map(|last| cursor.0.saturating_sub(last.0).max(0))
            .unwrap_or(0);
        self.last_update_cursor = Some(cursor);
        dt_samples as f32 * 60.0 / sample_rate.max(1) as f32
    }

    fn update_follow(&self, cameras: &mut CameraRegistry, dt_frames: f32) {
        if !self.follow_initialized {
            return;
        }
        let Some(camera) = cameras.get_mut(CameraId(0)) else {
            return;
        };
        // ref: bdedc0aa:source/funkin/util/Constants.hx:640-644
        let ratio = (DEFAULT_CAMERA_FOLLOW_RATE * dt_frames).clamp(0.0, 1.0);
        camera.position = camera.position.lerp(self.follow_target, ratio);
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
        dt_frames: f32,
    ) {
        if let Some(camera) = cameras.get_mut(CameraId(1)) {
            camera.zoom = hud_zoom_decay(camera.zoom, dt_frames);
        }

        let step = camera_step_index(cursor, sample_rate, bpm);
        if step <= self.last_step {
            return;
        }
        self.last_step = step;
        if !should_bop_step(step, self.camera_zoom_rate, self.camera_zoom_rate_offset) {
            return;
        }
        if let Some(camera) = cameras.get_mut(CameraId(1)) {
            if camera.zoom < MAX_RELATIVE_CAM_ZOOM * DEFAULT_HUD_CAMERA_ZOOM {
                self.camera_bop_multiplier = self.camera_bop_intensity;
                camera.zoom += self.hud_camera_zoom_intensity * DEFAULT_HUD_CAMERA_ZOOM;
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

pub(crate) struct CameraBopEvent {
    pub(crate) rate: f32,
    pub(crate) offset: f32,
    pub(crate) intensity: f32,
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

fn camera_bop_decay(current: f32, dt_frames: f32) -> f32 {
    1.0 + (current - 1.0) * 0.95_f32.powf(dt_frames)
}

fn hud_zoom_decay(current: f32, dt_frames: f32) -> f32 {
    1.0 + (current - 1.0) * 0.95_f32.powf(dt_frames)
}

fn camera_step_index(cursor: Samples, sample_rate: u32, bpm: f64) -> i64 {
    let samples_per_step = f64::from(sample_rate.max(1)) * 60.0 / bpm.max(1.0) / 4.0;
    (cursor.0.max(0) as f64 / samples_per_step).floor() as i64
}

fn should_bop_step(step: i64, rate: f32, offset: f32) -> bool {
    if rate <= 0.0 {
        return false;
    }
    let period_steps = rate * STEPS_PER_BEAT;
    let step_with_offset = step as f32 + offset * STEPS_PER_BEAT;
    let remainder = step_with_offset.rem_euclid(period_steps);
    remainder <= f32::EPSILON || (period_steps - remainder) <= f32::EPSILON
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
        let decayed = camera_bop_decay(1.20, 1.0);
        assert!((decayed - 1.19).abs() < 0.0001);

        let half_frame = camera_bop_decay(1.20, 0.5);
        assert!((half_frame - 1.194_936).abs() < 0.0001);
    }

    #[test]
    fn camera_step_index_uses_audio_cursor() {
        assert_eq!(camera_step_index(Samples(-1), 48_000, 100.0), 0);
        assert_eq!(camera_step_index(Samples(7_199), 48_000, 100.0), 0);
        assert_eq!(camera_step_index(Samples(7_200), 48_000, 100.0), 1);
    }

    #[test]
    fn camera_bop_step_respects_rate_and_offset() {
        assert!(should_bop_step(0, 4.0, 0.0));
        assert!(!should_bop_step(15, 4.0, 0.0));
        assert!(should_bop_step(16, 4.0, 0.0));
        assert!(!should_bop_step(2, 1.0, 0.25));
        assert!(should_bop_step(3, 1.0, 0.25));
        assert!(!should_bop_step(0, 0.0, 0.0));
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

    #[test]
    fn set_camera_bop_changes_rate_and_intensity() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut fx = CameraFx::default();
        fx.reset(&mut cameras, 1.0);
        fx.enable_zooming();
        fx.set_camera_bop(CameraBopEvent {
            rate: 1.0,
            offset: 0.0,
            intensity: 2.0,
        });

        fx.update(&mut cameras, Samples(6_000), 48_000, 120.0);
        assert!((cameras.get(CameraId(0)).map_or(0.0, |camera| camera.zoom) - 1.0).abs() < 1e-6);

        fx.update(&mut cameras, Samples(24_000), 48_000, 120.0);

        assert!((cameras.get(CameraId(0)).map_or(0.0, |camera| camera.zoom) - 1.03).abs() < 1e-6);
        assert!((cameras.get(CameraId(1)).map_or(0.0, |camera| camera.zoom) - 1.06).abs() < 1e-6);
    }
}
