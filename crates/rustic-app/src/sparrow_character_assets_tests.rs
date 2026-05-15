#![allow(clippy::unwrap_used)]

use super::*;
use rustic_asset::{CharacterDefinition, SparrowAtlas, StageDefinition};

#[test]
fn non_looping_animation_frames_clamp_to_last_frame() {
    // ref: bdedc0aa:source/funkin/graphics/FunkinAnimationController.hx:28-39
    assert_eq!(
        flixel_frame_index(Samples(0), 48_000, Samples(0), 24, 3, false),
        0
    );
    assert_eq!(
        flixel_frame_index(Samples(2_000), 48_000, Samples(0), 24, 3, false),
        0
    );
    assert_eq!(
        flixel_frame_index(Samples(2_001), 48_000, Samples(0), 24, 3, false),
        1
    );
    assert_eq!(
        flixel_frame_index(Samples(96_000), 48_000, Samples(0), 24, 3, false),
        2
    );
}

#[test]
fn looping_animation_frames_wrap() {
    // ref: bdedc0aa:source/funkin/graphics/FunkinAnimationController.hx:28-39
    assert_eq!(
        flixel_frame_index(Samples(6_000), 48_000, Samples(0), 24, 3, true),
        2
    );
    assert_eq!(
        flixel_frame_index(Samples(6_001), 48_000, Samples(0), 24, 3, true),
        0
    );
    assert_eq!(
        flixel_frame_index(Samples(8_000), 48_000, Samples(0), 24, 3, true),
        0
    );
    assert_eq!(
        flixel_frame_index(Samples(8_001), 48_000, Samples(0), 24, 3, true),
        1
    );
}

#[test]
fn animation_frame_index_uses_pose_start_cursor() {
    assert_eq!(
        flixel_frame_index(Samples(12_000), 48_000, Samples(10_000), 24, 3, false),
        0
    );
    assert_eq!(
        flixel_frame_index(Samples(12_001), 48_000, Samples(10_000), 24, 3, false),
        1
    );
}

#[test]
fn animation_duration_uses_frame_count_and_fps() {
    assert_eq!(animation_duration_samples(48_000, 24, 12), Samples(24_000));
    assert_eq!(animation_duration_samples(48_000, 0, 0), Samples(48_000));
}

#[test]
fn sparrow_character_position_uses_stage_feet_origin() {
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="test.png">
          <SubTexture name="idle0000" x="0" y="0" width="80" height="90"
            frameX="-5" frameY="-7" frameWidth="100" frameHeight="200"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let character = CharacterDefinition::parse(
        br#"{
          "id": "test",
          "atlas": "images/test.xml",
          "offsets": [10, 20],
          "scale": 2,
          "animations": [{ "name": "idle", "prefix": "idle", "offsets": [1, 2] }]
        }"#,
    )
    .unwrap();
    let stage =
        StageDefinition::parse(br#"{"id":"stage","boyfriend":{"position":{"x":300,"y":400}}}"#)
            .unwrap();

    let pos = character_frame_pos(
        &character,
        &character.animations[0],
        &atlas.frames[0],
        stage.boyfriend,
    );

    assert_eq!(pos, glam::vec2(219.0, 32.0));
}

#[test]
fn sparrow_camera_focus_uses_idle_visual_center() {
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="test.png">
          <SubTexture name="idle0000" x="0" y="0" width="80" height="90"
            frameX="-5" frameY="-7" frameWidth="100" frameHeight="200"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let character = CharacterDefinition::parse(
        br#"{
          "id": "test",
          "atlas": "images/test.xml",
          "cameraOffsets": [7, -9],
          "death": { "cameraOffsets": [-73, 42], "cameraZoom": 1.2 },
          "offsets": [10, 20],
          "scale": 2,
          "animations": [{ "name": "idle", "prefix": "idle" }]
        }"#,
    )
    .unwrap();
    let stage = StageDefinition::parse(
        br#"{"id":"stage","boyfriend":{
          "position":{"x":300,"y":400},
          "cameraOffset":{"x":-100,"y":-100}
        }}"#,
    )
    .unwrap();
    let animation = character.animations[0].clone();
    let sprite = SparrowCharacterSprite {
        character,
        slot: stage.boyfriend,
        is_player: false,
        z: 0,
        filter: FilterMode::Nearest,
        assets: vec![LoadedSparrowAtlas {
            texture_id: AssetId::new(1),
            width: 1,
            height: 1,
            atlas: atlas.clone(),
        }],
        poses: vec![CharacterPose {
            animation,
            asset_index: 0,
            frames: vec![atlas.frames[0].clone()],
        }],
        initial_pose: 0,
    };

    assert_eq!(sprite.camera_focus_point(), glam::vec2(217.0, 111.0));
}
