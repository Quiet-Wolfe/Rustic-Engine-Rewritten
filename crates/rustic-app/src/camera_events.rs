//! Song-event camera helpers.

use crate::camera_fx::{CameraBopEvent, CameraFx, FocusCameraEvent, ZoomCameraEvent};
use crate::scene_assets::CameraFocusPoints;
use glam::Vec2;
use rustic_asset::ChartEventKind;
use rustic_core::time::Samples;
use rustic_render::CameraRegistry;

pub(crate) fn apply_camera_event(
    cameras: &mut CameraRegistry,
    camera_fx: &mut CameraFx,
    focus: CameraFocusPoints,
    kind: &ChartEventKind,
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
) -> bool {
    match kind {
        ChartEventKind::FocusCamera {
            target,
            x,
            y,
            duration_steps,
            ease,
        } => {
            apply_focus_camera(
                cameras,
                camera_fx,
                focus,
                *target,
                Vec2::new(*x, *y),
                cursor,
                sample_rate,
                bpm,
                *duration_steps,
                ease,
            );
            true
        }
        ChartEventKind::ZoomCamera {
            zoom,
            duration_steps,
            direct,
            ease,
        } => {
            camera_fx.zoom_camera(
                cameras,
                cursor,
                sample_rate,
                bpm,
                ZoomCameraEvent {
                    zoom: *zoom,
                    duration_steps: *duration_steps,
                    direct: *direct,
                    ease,
                },
            );
            true
        }
        ChartEventKind::SetCameraBop {
            rate,
            offset,
            intensity,
        } => {
            camera_fx.set_camera_bop(CameraBopEvent {
                rate: *rate,
                offset: *offset,
                intensity: *intensity,
            });
            true
        }
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_focus_camera(
    cameras: &mut CameraRegistry,
    camera_fx: &mut CameraFx,
    focus: CameraFocusPoints,
    target: Option<i8>,
    offset: Vec2,
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
    duration_steps: f32,
    ease: &str,
) {
    if ease.eq_ignore_ascii_case("CLASSIC") {
        focus_camera(cameras, camera_fx, focus, target, offset, false);
        return;
    }
    let Some(target) = focus_target(focus, target, offset) else {
        return;
    };
    camera_fx.tween_focus_camera(
        cameras,
        target,
        cursor,
        sample_rate,
        bpm,
        FocusCameraEvent {
            duration_steps,
            ease,
        },
    );
}

pub(crate) fn focus_camera(
    cameras: &mut CameraRegistry,
    camera_fx: &mut CameraFx,
    focus: CameraFocusPoints,
    target: Option<i8>,
    offset: Vec2,
    snap: bool,
) {
    // ref: bdedc0aa:source/funkin/play/event/FocusCameraSongEvent.hx:97-145
    if let Some(target) = focus_target(focus, target, offset) {
        camera_fx.focus_camera(cameras, target, snap);
    }
}

pub(crate) fn focus_initial_camera(
    cameras: &mut CameraRegistry,
    camera_fx: &mut CameraFx,
    focus: CameraFocusPoints,
) {
    // ref: bdedc0aa:source/funkin/play/PlayState.hx:2218-2221,939
    focus_camera(cameras, camera_fx, focus, Some(1), Vec2::ZERO, true);
}

fn focus_target(focus: CameraFocusPoints, target: Option<i8>, offset: Vec2) -> Option<Vec2> {
    let base = match target.unwrap_or(0) {
        -1 => Vec2::ZERO,
        0 => focus.player,
        1 => focus.opponent,
        2 => focus.girlfriend?,
        _ => return None,
    };
    Some(base + offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_camera_event_updates_game_camera_to_stage_target() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut camera_fx = CameraFx::default();
        let focus = CameraFocusPoints {
            player: Vec2::new(10.0, 20.0),
            opponent: Vec2::new(30.0, 40.0),
            girlfriend: Some(Vec2::new(50.0, 60.0)),
        };

        focus_camera(
            &mut cameras,
            &mut camera_fx,
            focus,
            Some(1),
            Vec2::new(5.0, -2.0),
            true,
        );

        assert_eq!(
            cameras
                .get(rustic_core::ids::CameraId(0))
                .map(|camera| camera.position),
            Some(Vec2::new(35.0, 38.0))
        );
    }

    #[test]
    fn focus_camera_event_lerps_without_snap() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut camera_fx = CameraFx::default();
        let focus = CameraFocusPoints {
            player: Vec2::new(840.0, 360.0),
            opponent: Vec2::new(640.0, 360.0),
            girlfriend: Some(Vec2::new(640.0, 360.0)),
        };

        focus_camera(
            &mut cameras,
            &mut camera_fx,
            focus,
            Some(0),
            Vec2::ZERO,
            false,
        );
        camera_fx.update(&mut cameras, Samples(0), 48_000, 100.0);
        camera_fx.update(&mut cameras, Samples(12_000), 48_000, 100.0);

        let position = cameras
            .get(rustic_core::ids::CameraId(0))
            .map(|camera| camera.position)
            .unwrap_or_default();
        assert!(position.x > 640.0);
        assert!(position.x < 840.0);
    }

    #[test]
    fn focus_camera_ignores_missing_girlfriend() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut camera_fx = CameraFx::default();
        let before = cameras
            .get(rustic_core::ids::CameraId(0))
            .map(|camera| camera.position);
        let focus = CameraFocusPoints {
            player: Vec2::new(840.0, 360.0),
            opponent: Vec2::new(640.0, 360.0),
            girlfriend: None,
        };

        focus_camera(
            &mut cameras,
            &mut camera_fx,
            focus,
            Some(2),
            Vec2::ZERO,
            true,
        );

        assert_eq!(
            cameras
                .get(rustic_core::ids::CameraId(0))
                .map(|camera| camera.position),
            before
        );
    }

    #[test]
    fn initial_camera_focus_starts_on_opponent() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut camera_fx = CameraFx::default();
        let focus = CameraFocusPoints {
            player: Vec2::new(840.0, 360.0),
            opponent: Vec2::new(320.0, 240.0),
            girlfriend: Some(Vec2::new(640.0, 360.0)),
        };

        focus_initial_camera(&mut cameras, &mut camera_fx, focus);

        assert_eq!(
            cameras
                .get(rustic_core::ids::CameraId(0))
                .map(|camera| camera.position),
            Some(focus.opponent)
        );
    }

    #[test]
    fn focus_camera_event_tweens_when_not_classic() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let mut camera_fx = CameraFx::default();
        let focus = CameraFocusPoints {
            player: Vec2::new(840.0, 360.0),
            opponent: Vec2::new(640.0, 360.0),
            girlfriend: Some(Vec2::new(640.0, 360.0)),
        };

        assert!(apply_camera_event(
            &mut cameras,
            &mut camera_fx,
            focus,
            &ChartEventKind::FocusCamera {
                target: Some(0),
                x: 0.0,
                y: 0.0,
                duration_steps: 4.0,
                ease: "linear".to_string(),
            },
            Samples(0),
            48_000,
            120.0,
        ));
        camera_fx.update(&mut cameras, Samples(12_000), 48_000, 120.0);

        assert_eq!(
            cameras
                .get(rustic_core::ids::CameraId(0))
                .map(|camera| camera.position),
            Some(Vec2::new(740.0, 360.0))
        );
    }
}
