//! Runtime hooks for the Week 3 Philly train stage script.

use crate::character_anim::{CharacterPoseNames, CharacterPoseRequest};
use crate::preview_song::PreviewSong;
use crate::stage_scripted_motion::{philly_train_start, philly_train_x};
use rustic_core::time::Samples;

const TRAIN_MOVE_DELAY_TENTHS: i64 = 47;
const TRAIN_RESET_FRAMES: i64 = 96;
const HAIR_FALL_FRAMES: i64 = 12;
const ANIMATION_FPS: i64 = 24;

pub(crate) fn philly_train_pose_overrides(
    song: PreviewSong,
    poses: CharacterPoseNames,
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
) -> CharacterPoseNames {
    if !is_philly_train_song(song) {
        return poses;
    }
    let Some(started_at) = philly_train_start(cursor, sample_rate, bpm) else {
        return poses;
    };
    let move_start = train_move_start(started_at, sample_rate);
    if cursor.0 < move_start.0 {
        return poses;
    }

    // ref: bdedc0aa:assets/preload/scripts/stages/phillyTrainErect.hxc:490-537
    if philly_train_x(cursor, sample_rate, bpm).is_some() {
        return CharacterPoseNames {
            girlfriend: CharacterPoseRequest {
                name: "hairBlow",
                started_at: move_start,
            },
            ..poses
        };
    }

    let reset = train_reset_cursor(started_at, sample_rate);
    let fall_duration = frame_duration_samples(HAIR_FALL_FRAMES, sample_rate);
    if cursor.0 >= reset.0 && cursor.0 < reset.0 + fall_duration {
        return CharacterPoseNames {
            girlfriend: CharacterPoseRequest {
                name: "hairFall",
                started_at: reset,
            },
            ..poses
        };
    }

    poses
}

fn is_philly_train_song(song: PreviewSong) -> bool {
    matches!(
        song,
        PreviewSong::PICO | PreviewSong::PHILLY_NICE | PreviewSong::BLAMMED
    )
}

fn train_move_start(started_at: Samples, sample_rate: u32) -> Samples {
    Samples(started_at.0 + i64::from(sample_rate.max(1)) * TRAIN_MOVE_DELAY_TENTHS / 10)
}

fn train_reset_cursor(started_at: Samples, sample_rate: u32) -> Samples {
    Samples(
        train_move_start(started_at, sample_rate).0
            + frame_duration_samples(TRAIN_RESET_FRAMES, sample_rate),
    )
}

fn frame_duration_samples(frames: i64, sample_rate: u32) -> i64 {
    let sample_rate = i64::from(sample_rate.max(1));
    (frames * sample_rate + ANIMATION_FPS - 1) / ANIMATION_FPS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poses() -> CharacterPoseNames {
        CharacterPoseNames {
            girlfriend: CharacterPoseRequest {
                name: "danceLeft",
                started_at: Samples(1),
            },
            opponent: CharacterPoseRequest {
                name: "idle",
                started_at: Samples(2),
            },
            player: CharacterPoseRequest {
                name: "singRIGHT",
                started_at: Samples(3),
            },
        }
    }

    #[test]
    fn philly_train_sets_girlfriend_hair_blow_while_moving() {
        let poses = philly_train_pose_overrides(
            PreviewSong::PICO,
            poses(),
            Samples(513_600),
            48_000,
            120.0,
        );

        assert_eq!(poses.girlfriend.name, "hairBlow");
        assert_eq!(poses.girlfriend.started_at, Samples(513_600));
        assert_eq!(poses.opponent.name, "idle");
        assert_eq!(poses.player.name, "singRIGHT");
    }

    #[test]
    fn philly_train_sets_girlfriend_hair_fall_at_reset() {
        let poses = philly_train_pose_overrides(
            PreviewSong::PICO,
            poses(),
            Samples(705_600),
            48_000,
            120.0,
        );

        assert_eq!(poses.girlfriend.name, "hairFall");
        assert_eq!(poses.girlfriend.started_at, Samples(705_600));
    }

    #[test]
    fn philly_train_hair_fall_returns_to_regular_pose_after_animation() {
        let poses = philly_train_pose_overrides(
            PreviewSong::PICO,
            poses(),
            Samples(729_600),
            48_000,
            120.0,
        );

        assert_eq!(poses.girlfriend.name, "danceLeft");
    }

    #[test]
    fn non_week3_songs_keep_regular_girlfriend_pose() {
        let poses = philly_train_pose_overrides(
            PreviewSong::SPOOKEEZ,
            poses(),
            Samples(513_600),
            48_000,
            120.0,
        );

        assert_eq!(poses.girlfriend.name, "danceLeft");
    }
}
