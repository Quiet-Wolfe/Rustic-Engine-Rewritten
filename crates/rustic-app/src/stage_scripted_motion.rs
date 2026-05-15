//! Deterministic ports of small stage scripts that affect prop timing/motion.
// LINT-ALLOW: long-file stage script ports keep timing helpers and regression tests together.

use rustic_core::time::Samples;

pub(crate) fn philly_traffic_light_state(
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
) -> Option<(&'static str, Samples)> {
    let beat = stage_beat(cursor, sample_rate, bpm);
    if beat < 0 {
        return None;
    }
    let segment = philly_traffic_segment(beat);
    let beat_samples = beat_samples(sample_rate, bpm);
    let animation = if segment.lights_stop {
        "tored"
    } else {
        "togreen"
    };
    Some((animation, Samples(segment.started_beat * beat_samples)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhillyCarLane {
    Forward,
    Back,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PhillyCarPose {
    pub(crate) animation: &'static str,
    pub(crate) started_at: Samples,
    pub(crate) position: glam::Vec2,
    pub(crate) rotation: f32,
}

pub(crate) fn philly_car_pose(
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
    lane: PhillyCarLane,
) -> Option<PhillyCarPose> {
    let beat = stage_beat(cursor, sample_rate, bpm);
    if beat < 0 {
        return None;
    }
    let segment = philly_traffic_segment(beat);
    let beat_samples = beat_samples(sample_rate, bpm);
    match lane {
        PhillyCarLane::Forward => {
            philly_forward_car_pose(cursor, sample_rate, beat_samples, segment)
        }
        PhillyCarLane::Back => philly_back_car_pose(cursor, sample_rate, beat_samples, segment),
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PhillyBlazinLightningState {
    pub(crate) started_at: Samples,
    pub(crate) x: f32,
    pub(crate) alpha: f32,
}

pub(crate) fn philly_blazin_lightning_state(
    cursor: Samples,
    sample_rate: u32,
) -> Option<PhillyBlazinLightningState> {
    let sample_rate = i64::from(sample_rate.max(1));
    let first_strike = sample_rate * 3;
    if cursor.0 < first_strike {
        return None;
    }
    let cycle = sample_rate * 11;
    let strike_index = (cursor.0 - first_strike).div_euclid(cycle.max(1));
    let started_at = first_strike + strike_index * cycle.max(1);
    let elapsed = cursor.0 - started_at;
    let full_duration = sample_rate * 3 / 2;
    if elapsed >= full_duration {
        return None;
    }
    let fade_start = sample_rate * 6 / 5;
    let alpha = if elapsed <= fade_start {
        1.0
    } else {
        let fade_len = (full_duration - fade_start).max(1) as f32;
        1.0 - (elapsed - fade_start) as f32 / fade_len
    };
    let x = if strike_index % 3 == 0 { -220.0 } else { 840.0 };
    Some(PhillyBlazinLightningState {
        started_at: Samples(started_at),
        x,
        alpha: alpha.clamp(0.0, 1.0),
    })
}

pub(crate) fn philly_blazin_lightning_start(cursor: Samples, sample_rate: u32) -> Option<Samples> {
    philly_blazin_lightning_state(cursor, sample_rate).map(|state| state.started_at)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TankRollingPose {
    pub(crate) position: glam::Vec2,
    pub(crate) rotation: f32,
}

pub(crate) fn tank_rolling_pose(cursor: Samples, sample_rate: u32) -> TankRollingPose {
    let seconds = cursor.0.max(0) as f32 / sample_rate.max(1) as f32;
    let angle_degrees = -90.0 + seconds * 6.0;
    let radians = (angle_degrees + 180.0_f32).to_radians();
    TankRollingPose {
        position: glam::vec2(
            400.0 + radians.cos() * 1500.0,
            1300.0 + radians.sin() * 1100.0,
        ),
        rotation: (angle_degrees - 75.0).to_radians(),
    }
}

fn philly_forward_car_pose(
    cursor: Samples,
    sample_rate: u32,
    beat_samples: i64,
    segment: PhillyTrafficSegment,
) -> Option<PhillyCarPose> {
    if !segment.lights_stop && segment.started_beat > 0 {
        let start = Samples(segment.started_beat * beat_samples);
        let variant = philly_car_animation(segment.started_beat + 2, PhillyCarLane::Forward);
        if let Some(pose) = philly_car_path_pose(
            cursor,
            start,
            philly_car_duration_samples(sample_rate, variant, PhillyCarRun::FinishLight),
            variant,
            PHILLY_CAR_LIGHT_FINISH_PATH,
            (-5.0, 18.0),
            PhillyCarEase::SineIn,
        ) {
            return Some(pose);
        }
    }

    if segment.lights_stop {
        let start_beat = segment.started_beat + 4;
        let start = Samples(start_beat * beat_samples);
        let variant = philly_car_animation(start_beat, PhillyCarLane::Forward);
        let duration = philly_car_duration_samples(sample_rate, variant, PhillyCarRun::StopLight);
        if let Some(pose) = philly_car_path_pose(
            cursor,
            start,
            duration,
            variant,
            PHILLY_CAR_LIGHT_STOP_PATH,
            (-7.0, -5.0),
            PhillyCarEase::CubeOut,
        ) {
            return Some(pose);
        }
        let wait_until = Samples(segment.next_beat * beat_samples);
        if cursor.0 >= start.0 + duration && cursor.0 < wait_until.0 {
            return Some(PhillyCarPose {
                animation: variant,
                started_at: start,
                position: PHILLY_CAR_LIGHT_STOP_PATH[2],
                rotation: (-5.0_f32).to_radians(),
            });
        }
        return None;
    }

    let start_beat = segment.started_beat + if segment.started_beat == 0 { 4 } else { 8 };
    if start_beat >= segment.next_beat {
        return None;
    }
    let start = Samples(start_beat * beat_samples);
    let variant = philly_car_animation(start_beat, PhillyCarLane::Forward);
    philly_car_path_pose(
        cursor,
        start,
        philly_car_duration_samples(sample_rate, variant, PhillyCarRun::Through),
        variant,
        PHILLY_CAR_FORWARD_PATH,
        (-8.0, 18.0),
        PhillyCarEase::Linear,
    )
}

fn philly_back_car_pose(
    cursor: Samples,
    sample_rate: u32,
    beat_samples: i64,
    segment: PhillyTrafficSegment,
) -> Option<PhillyCarPose> {
    if segment.lights_stop {
        return None;
    }
    let start_beat = segment.started_beat + if segment.started_beat == 0 { 2 } else { 14 };
    if start_beat >= segment.next_beat {
        return None;
    }
    let start = Samples(start_beat * beat_samples);
    let variant = philly_car_animation(start_beat, PhillyCarLane::Back);
    philly_car_path_pose(
        cursor,
        start,
        philly_car_duration_samples(sample_rate, variant, PhillyCarRun::Through),
        variant,
        PHILLY_CAR_BACK_PATH,
        (18.0, -8.0),
        PhillyCarEase::Linear,
    )
}

// ref: bdedc0aa:assets/preload/scripts/stages/phillyStreets.hxc:185-339
const PHILLY_CAR_FORWARD_PATH: [glam::Vec2; 3] = [
    glam::vec2(1263.4, 850.7),
    glam::vec2(2093.4, 761.7),
    glam::vec2(2795.4, 1058.7),
];
const PHILLY_CAR_LIGHT_STOP_PATH: [glam::Vec2; 3] = [
    glam::vec2(1173.4, 860.7),
    glam::vec2(1383.4, 835.7),
    glam::vec2(1563.4, 826.7),
];
const PHILLY_CAR_LIGHT_FINISH_PATH: [glam::Vec2; 3] = [
    glam::vec2(1563.4, 826.7),
    glam::vec2(2093.4, 761.7),
    glam::vec2(2795.4, 1058.7),
];
const PHILLY_CAR_BACK_PATH: [glam::Vec2; 3] = [
    glam::vec2(2795.4, 1018.7),
    glam::vec2(2093.4, 781.7),
    glam::vec2(1263.4, 870.7),
];

#[derive(Debug, Clone, Copy)]
struct PhillyTrafficSegment {
    lights_stop: bool,
    started_beat: i64,
    next_beat: i64,
}

fn philly_traffic_segment(beat: i64) -> PhillyTrafficSegment {
    let mut started_beat = 0;
    let mut interval = 8;
    let mut lights_stop = false;
    while beat >= started_beat + interval {
        started_beat += interval;
        lights_stop = !lights_stop;
        interval = if lights_stop { 20 } else { 30 };
    }
    PhillyTrafficSegment {
        lights_stop,
        started_beat,
        next_beat: started_beat + interval,
    }
}

#[derive(Debug, Clone, Copy)]
enum PhillyCarRun {
    Through,
    StopLight,
    FinishLight,
}

#[derive(Debug, Clone, Copy)]
enum PhillyCarEase {
    Linear,
    CubeOut,
    SineIn,
}

fn philly_car_path_pose(
    cursor: Samples,
    started_at: Samples,
    duration: i64,
    animation: &'static str,
    path: [glam::Vec2; 3],
    rotations: (f32, f32),
    ease: PhillyCarEase,
) -> Option<PhillyCarPose> {
    let elapsed = cursor.0 - started_at.0;
    if elapsed < 0 || elapsed >= duration {
        return None;
    }
    let progress = elapsed as f32 / duration.max(1) as f32;
    let eased = match ease {
        PhillyCarEase::Linear => progress,
        PhillyCarEase::CubeOut => 1.0 - (1.0 - progress).powi(3),
        PhillyCarEase::SineIn => 1.0 - (progress * std::f32::consts::FRAC_PI_2).cos(),
    };
    Some(PhillyCarPose {
        animation,
        started_at,
        position: quad_bezier(path, eased),
        rotation: (rotations.0 + (rotations.1 - rotations.0) * eased).to_radians(),
    })
}

fn quad_bezier(path: [glam::Vec2; 3], progress: f32) -> glam::Vec2 {
    let one_minus = 1.0 - progress;
    path[0] * one_minus * one_minus
        + path[1] * 2.0 * one_minus * progress
        + path[2] * progress * progress
}

fn philly_car_duration_samples(
    sample_rate: u32,
    animation: &'static str,
    run: PhillyCarRun,
) -> i64 {
    let seconds = match run {
        PhillyCarRun::FinishLight => 2.4,
        PhillyCarRun::StopLight if animation == "car2" => 1.2,
        PhillyCarRun::Through if animation == "car2" => 0.9,
        _ if animation == "car1" => 1.35,
        _ => 2.0,
    };
    (seconds * sample_rate.max(1) as f32).round() as i64
}

fn philly_car_animation(start_beat: i64, lane: PhillyCarLane) -> &'static str {
    let lane_offset = if lane == PhillyCarLane::Back { 2 } else { 0 };
    match (start_beat + lane_offset).rem_euclid(4) {
        0 => "car1",
        1 => "car2",
        2 => "car3",
        _ => "car4",
    }
}

pub(crate) fn philly_train_x(cursor: Samples, sample_rate: u32, bpm: f64) -> Option<f32> {
    let start = philly_train_start(cursor, sample_rate, bpm)?;
    let move_start = start.0 + i64::from(sample_rate.max(1)) * 47 / 10;
    if cursor.0 < move_start {
        return None;
    }
    let frames = ((cursor.0 - move_start).max(0) * 24 / i64::from(sample_rate.max(1))) as i32;
    let car_frames = 11;
    let cars = frames / car_frames;
    if cars < 8 {
        return Some(2000.0 - (frames % car_frames) as f32 * 400.0);
    }
    let finishing_x = -1150.0 - (frames - 8 * car_frames) as f32 * 400.0;
    (finishing_x >= -4000.0).then_some(finishing_x)
}

pub(crate) fn philly_train_start(cursor: Samples, sample_rate: u32, bpm: f64) -> Option<Samples> {
    let beat = stage_beat(cursor, sample_rate, bpm);
    if beat < 12 {
        return None;
    }
    let beat_samples = beat_samples(sample_rate, bpm);
    let start_beat = 12 + (beat - 12).div_euclid(32) * 32;
    Some(Samples(start_beat * beat_samples))
}

pub(crate) fn limo_fast_car_position(
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
) -> Option<glam::Vec2> {
    let start = limo_fast_car_start(cursor, sample_rate, bpm)?;
    let duration = i64::from(sample_rate.max(1)) * 2;
    let elapsed = cursor.0 - start.0;
    if elapsed < 0 || elapsed >= duration {
        return None;
    }
    let progress = elapsed as f32 / duration.max(1) as f32;
    let beat_samples = beat_samples(sample_rate, bpm);
    let run_index = (start.0 / beat_samples - 8).div_euclid(16);
    let y = 195.0 + (run_index % 3 - 1) as f32 * 35.0;
    Some(glam::vec2(-12600.0 + progress * 16_000.0, y))
}

pub(crate) fn limo_fast_car_start(cursor: Samples, sample_rate: u32, bpm: f64) -> Option<Samples> {
    let beat = stage_beat(cursor, sample_rate, bpm);
    if beat < 8 {
        return None;
    }
    let beat_samples = beat_samples(sample_rate, bpm);
    let start_beat = 8 + (beat - 8).div_euclid(16) * 16;
    Some(Samples(start_beat * beat_samples))
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LimoShootingStarState {
    pub(crate) started_at: Samples,
    pub(crate) position: glam::Vec2,
}

pub(crate) fn limo_shooting_star_state(
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
) -> Option<LimoShootingStarState> {
    let beat = stage_beat(cursor, sample_rate, bpm);
    if beat < 8 {
        return None;
    }
    let beat_samples = beat_samples(sample_rate, bpm);
    let start_beat = 8 + (beat - 8).div_euclid(16) * 16;
    let started_at = Samples(start_beat * beat_samples);
    let duration = i64::from(sample_rate.max(1)) * 6 / 5;
    if cursor.0 < started_at.0 || cursor.0 >= started_at.0 + duration {
        return None;
    }
    let run = (start_beat - 8).div_euclid(16);
    Some(LimoShootingStarState {
        started_at,
        position: glam::vec2(
            50.0 + (run * 271).rem_euclid(850) as f32,
            -10.0 + (run * 13).rem_euclid(31) as f32,
        ),
    })
}

fn stage_beat(cursor: Samples, sample_rate: u32, bpm: f64) -> i64 {
    cursor.0.max(0).div_euclid(beat_samples(sample_rate, bpm))
}

fn beat_samples(sample_rate: u32, bpm: f64) -> i64 {
    (f64::from(sample_rate.max(1)) * 60.0 / bpm.max(1.0))
        .round()
        .max(1.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn philly_train_waits_then_moves_in_scripted_cycle() {
        assert_eq!(philly_train_x(Samples(0), 48_000, 120.0), None);
        assert_eq!(
            philly_train_start(Samples(288_000), 48_000, 120.0),
            Some(Samples(288_000))
        );
        assert_eq!(philly_train_x(Samples(500_000), 48_000, 120.0), None);
        assert_eq!(
            philly_train_x(Samples(513_600), 48_000, 120.0),
            Some(2000.0)
        );
        assert_eq!(
            philly_train_x(Samples(515_600), 48_000, 120.0),
            Some(1600.0)
        );
    }

    #[test]
    fn limo_fast_car_has_short_visible_runs() {
        assert_eq!(limo_fast_car_position(Samples(0), 48_000, 120.0), None);
        assert_eq!(
            limo_fast_car_start(Samples(192_000), 48_000, 120.0),
            Some(Samples(192_000))
        );
        let pos = limo_fast_car_position(Samples(192_000), 48_000, 120.0).unwrap();
        assert!((pos.x + 12_600.0).abs() < 0.01);
        assert_eq!(
            limo_fast_car_position(Samples(340_000), 48_000, 120.0),
            None
        );
    }

    #[test]
    fn philly_cars_drive_stop_and_resume_with_traffic_cycle() {
        let green_car =
            philly_car_pose(Samples(96_000), 48_000, 120.0, PhillyCarLane::Forward).unwrap();
        assert_eq!(green_car.animation, "car1");
        assert!((green_car.position.x - 1263.4).abs() < 0.01);

        let red_car =
            philly_car_pose(Samples(288_000), 48_000, 120.0, PhillyCarLane::Forward).unwrap();
        assert_eq!(red_car.animation, "car1");
        assert!((red_car.position.x - 1173.4).abs() < 0.01);

        let waiting =
            philly_car_pose(Samples(528_000), 48_000, 120.0, PhillyCarLane::Forward).unwrap();
        assert_eq!(waiting.position, glam::vec2(1563.4, 826.7));

        let resumed =
            philly_car_pose(Samples(672_000), 48_000, 120.0, PhillyCarLane::Forward).unwrap();
        assert_eq!(resumed.started_at, Samples(672_000));
        assert!(resumed.rotation < 0.0);
    }

    #[test]
    fn philly_back_cars_only_drive_on_green_lights() {
        assert!(philly_car_pose(Samples(48_000), 48_000, 120.0, PhillyCarLane::Back).is_some());
        assert!(philly_car_pose(Samples(288_000), 48_000, 120.0, PhillyCarLane::Back).is_none());
    }

    #[test]
    fn limo_shooting_star_appears_in_short_scripted_windows() {
        assert!(limo_shooting_star_state(Samples(180_000), 48_000, 120.0).is_none());
        let state = limo_shooting_star_state(Samples(192_000), 48_000, 120.0).unwrap();
        assert_eq!(state.started_at, Samples(192_000));
        assert_eq!(state.position, glam::vec2(50.0, -10.0));
        assert!(limo_shooting_star_state(Samples(260_000), 48_000, 120.0).is_none());
    }
}
