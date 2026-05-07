//! Song-event camera helpers.

use crate::scene_assets::CameraFocusPoints;
use glam::Vec2;
use rustic_core::ids::CameraId;
use rustic_render::CameraRegistry;

pub fn focus_camera(
    cameras: &mut CameraRegistry,
    focus: CameraFocusPoints,
    target: Option<i8>,
    offset: Vec2,
) {
    // ref: bdedc0aa:source/funkin/play/event/FocusCameraSongEvent.hx:97-145
    let base = match target.unwrap_or(0) {
        -1 => Vec2::ZERO,
        0 => focus.player,
        1 => focus.opponent,
        2 => focus.girlfriend,
        _ => return,
    };
    if let Some(camera) = cameras.get_mut(CameraId(0)) {
        camera.position = base + offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_camera_event_updates_game_camera_to_stage_target() {
        let mut cameras = CameraRegistry::with_default_fnf();
        let focus = CameraFocusPoints {
            player: Vec2::new(10.0, 20.0),
            opponent: Vec2::new(30.0, 40.0),
            girlfriend: Vec2::new(50.0, 60.0),
        };

        focus_camera(&mut cameras, focus, Some(1), Vec2::new(5.0, -2.0));

        assert_eq!(
            cameras.get(CameraId(0)).map(|camera| camera.position),
            Some(Vec2::new(35.0, 38.0))
        );
    }
}
