use super::*;
use crate::preview_song::{PreviewDifficulty, PreviewSong};

#[test]
fn non_looping_animation_frames_clamp_to_last_frame() {
    // ref: bdedc0aa:source/funkin/graphics/FunkinAnimationController.hx:28-39
    assert_eq!(
        animation_frame_index(Samples(0), 48_000, Samples(0), 24, 3, false),
        0
    );
    assert_eq!(
        animation_frame_index(Samples(2_000), 48_000, Samples(0), 24, 3, false),
        1
    );
    assert_eq!(
        animation_frame_index(Samples(96_000), 48_000, Samples(0), 24, 3, false),
        2
    );
}

#[test]
fn looping_animation_frames_wrap() {
    // ref: bdedc0aa:source/funkin/graphics/FunkinAnimationController.hx:28-39
    assert_eq!(
        animation_frame_index(Samples(6_000), 48_000, Samples(0), 24, 3, true),
        0
    );
    assert_eq!(
        animation_frame_index(Samples(8_000), 48_000, Samples(0), 24, 3, true),
        1
    );
}

#[test]
fn animation_frame_index_uses_pose_start_cursor() {
    assert_eq!(
        animation_frame_index(Samples(12_000), 48_000, Samples(10_000), 24, 3, false),
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
fn preview_play_state_uses_selected_difficulty() {
    let easy = load_preview_play_state_for(
        PreviewSelection {
            song: PreviewSong::BOPEEBO,
            difficulty: PreviewDifficulty::Easy,
        },
        48_000,
    )
    .expect("easy bopeebo chart");
    let hard = load_preview_play_state_for(
        PreviewSelection {
            song: PreviewSong::BOPEEBO,
            difficulty: PreviewDifficulty::Hard,
        },
        48_000,
    )
    .expect("hard bopeebo chart");
    assert_eq!(easy.scroll_speed, 1.2);
    assert_eq!(hard.scroll_speed, 1.6);
    assert!(hard.notes.len() > easy.notes.len());
}
