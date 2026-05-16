//! Runtime hooks for the Week 2 spooky mansion stage script.

use crate::character_anim::{CharacterPoseNames, CharacterPoseRequest};
use crate::preview_song::PreviewSong;
use crate::stage_object_asset_helpers::halloween_lightning_start;
use rustic_core::time::Samples;

pub(crate) fn spooky_lightning_pose_overrides(
    song: PreviewSong,
    poses: CharacterPoseNames,
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
) -> CharacterPoseNames {
    if !is_spooky_mansion_song(song) {
        return poses;
    }
    let Some(started_at) = halloween_lightning_start(cursor, sample_rate, bpm) else {
        return poses;
    };
    let scared = CharacterPoseRequest {
        name: "scared",
        started_at,
    };
    CharacterPoseNames {
        girlfriend: scared,
        player: scared,
        ..poses
    }
}

fn is_spooky_mansion_song(song: PreviewSong) -> bool {
    matches!(
        song,
        PreviewSong::SPOOKEEZ | PreviewSong::SOUTH | PreviewSong::MONSTER
    )
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
                name: "singLEFT",
                started_at: Samples(3),
            },
        }
    }

    #[test]
    fn week2_lightning_forces_bf_and_gf_scared_poses() {
        let poses = spooky_lightning_pose_overrides(
            PreviewSong::SPOOKEEZ,
            poses(),
            Samples(96_000),
            48_000,
            120.0,
        );

        assert_eq!(poses.girlfriend.name, "scared");
        assert_eq!(poses.girlfriend.started_at, Samples(96_000));
        assert_eq!(poses.player.name, "scared");
        assert_eq!(poses.opponent.name, "idle");
    }

    #[test]
    fn non_week2_songs_keep_regular_character_poses() {
        let poses = spooky_lightning_pose_overrides(
            PreviewSong::DADBATTLE,
            poses(),
            Samples(96_000),
            48_000,
            120.0,
        );

        assert_eq!(poses.girlfriend.name, "danceLeft");
        assert_eq!(poses.player.name, "singLEFT");
    }
}
