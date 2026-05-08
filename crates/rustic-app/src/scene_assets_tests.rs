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
        texture_id: AssetId::new(1),
        texture_width: 1,
        texture_height: 1,
        character,
        slot: stage.boyfriend,
        is_player: false,
        z: 0,
        filter: FilterMode::Nearest,
        poses: vec![CharacterPose {
            animation,
            frames: vec![atlas.frames[0].clone()],
        }],
        initial_pose: 0,
    };

    assert_eq!(sprite.camera_focus_point(), glam::vec2(217.0, 111.0));

    let (death_focus, death_zoom) = CharacterSprite::Sparrow(sprite).game_over_camera(1.1);
    assert_eq!(death_focus, glam::vec2(237.0, 262.0));
    assert!((death_zoom - 1.32).abs() < 1e-6);
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

#[test]
fn preview_play_state_uses_erect_variant_files() {
    let erect = load_preview_song_for(PreviewSelection {
        song: PreviewSong::DADBATTLE,
        difficulty: PreviewDifficulty::Erect,
    })
    .expect("erect dadbattle chart");

    assert_eq!(erect.chart.stage, "mainStageErect");
    assert_eq!(erect.chart.bpm, 190.0);
    assert!(erect.chart.notes.iter().any(|note| note.time_ms > 60_000.0));
}

#[test]
fn preview_song_metadata_preserves_tutorial_gf_opponent() {
    let chart = load_preview_song_for(PreviewSelection {
        song: PreviewSong::TUTORIAL,
        difficulty: PreviewDifficulty::Normal,
    })
    .expect("tutorial chart metadata");

    assert_eq!(chart.chart.player2, "gf");
    assert_eq!(chart.chart.girlfriend, "");
    assert_eq!(chart.chart.stage, "mainStage");
    assert_eq!(stage_asset_id(&chart.chart.stage), "mainStage");
    assert_eq!(character_id(&chart.chart.girlfriend), None);
}

#[test]
fn stage_asset_id_keeps_vslice_stage_variants() {
    assert_eq!(stage_asset_id("mainStageErect"), "mainStageErect");
}

#[test]
fn baked_main_stage_erect_preserves_animated_crowd_prop() {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let stage = load_stage(
        &resolver,
        &AssetPath::new("data/stages/mainStageErect.json").unwrap(),
    )
    .expect("mainStageErect stage");

    let crowd = stage
        .objects
        .iter()
        .find(|object| object.id == "crowd")
        .expect("crowd prop");
    assert_eq!(crowd.image.as_str(), "images/erect/crowd.png");
    assert_eq!(
        crowd
            .animation
            .as_ref()
            .map(|animation| animation.prefix.as_str()),
        Some("idle0")
    );
}
