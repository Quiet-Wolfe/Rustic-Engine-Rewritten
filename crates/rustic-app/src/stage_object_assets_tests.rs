// LINT-ALLOW: long-file stage prop animation parity tests share fixtures and helpers.

use super::*;
use crate::countdown_assets::countdown_start_cursor;
use crate::stage_scripted_motion::tank_rolling_pose;
use rustic_asset::StageDefinition;

#[test]
fn stage_object_atlas_path_replaces_png_extension() {
    let path = AssetPath::new("images/erect/crowd.png").unwrap();

    assert_eq!(
        stage_object_atlas_path(&path).unwrap().as_str(),
        "images/erect/crowd.xml"
    );
}

#[test]
fn solid_stage_prop_uses_scale_as_draw_size() {
    let stage = StageDefinition::parse(
        br##"{
          "name": "test",
          "props": [{
            "name": "solid",
            "assetPath": "#222026",
            "position": [-500, -1000],
            "scale": [2400, 2000],
            "scroll": [0, 0],
            "zIndex": 0
          }]
        }"##,
    )
    .unwrap();
    let object = &stage.objects[0];
    let cmd = base_stage_command(
        object,
        AssetId::new(9),
        FilterMode::Linear,
        glam::vec2(object.position.x, object.position.y),
        glam::vec2(object.scale.x, object.scale.y),
    );

    assert_eq!(cmd.world_pos, glam::vec2(-500.0, -1000.0));
    assert_eq!(cmd.size, glam::vec2(2400.0, 2000.0));
    assert_eq!(cmd.scroll_factor, glam::Vec2::ZERO);
}

#[test]
fn animated_stage_prop_world_pos_applies_trim_offset() {
    let stage = StageDefinition::parse(
        br##"{
          "name": "test",
          "props": [{
            "name": "crowd",
            "assetPath": "erect/crowd",
            "position": [682, 290],
            "scale": [2, 3],
            "scroll": [1, 1],
            "zIndex": 5,
            "startingAnimation": "idle",
            "animations": [{
              "name": "idle",
              "prefix": "idle0",
              "frameRate": 12,
              "looped": true
            }]
          }]
        }"##,
    )
    .unwrap();
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="crowd.png">
          <SubTexture name="idle0000" x="0" y="0" width="100" height="80"
            frameX="-5" frameY="7" frameWidth="120" frameHeight="100"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let object = &stage.objects[0];
    let frame = atlas.first_animation_frame("idle0", &[]).unwrap();

    assert_eq!(stage_frame_pos(object, frame), glam::vec2(692.0, 269.0));
}

#[test]
fn stage_prop_sprite_advances_looping_frames() {
    let stage = StageDefinition::parse(
        br##"{
          "name": "test",
          "props": [{
            "name": "crowd",
            "assetPath": "erect/crowd",
            "position": [0, 0],
            "scale": [1, 1],
            "scroll": [1, 1],
            "zIndex": 5,
            "startingAnimation": "idle",
            "animations": [{
              "name": "idle",
              "prefix": "idle0",
              "frameRate": 12,
              "looped": true
            }]
          }]
        }"##,
    )
    .unwrap();
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="crowd.png">
          <SubTexture name="idle0000" x="0" y="0" width="100" height="80"/>
          <SubTexture name="idle0001" x="100" y="0" width="100" height="80"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let object = stage.objects[0].clone();
    let frames: Vec<_> = atlas
        .animation_frames("idle0", &[])
        .into_iter()
        .cloned()
        .collect();
    let prop = SparrowStagePropSprite {
        texture_id: AssetId::new(7),
        texture_width: 200,
        texture_height: 80,
        object,
        animations: vec![LoadedSparrowStageAnimation {
            name: "idle".to_string(),
            frames,
            frame_rate: 12,
            looped: true,
        }],
        starting_animation: 0,
        dance_left: None,
        dance_right: None,
        filter: FilterMode::Linear,
    };

    assert_eq!(prop.command(Samples(0), 48_000, 100.0).uv_min.x, 0.0);
    assert_eq!(prop.command(Samples(4_000), 48_000, 100.0).uv_min.x, 0.0);
    assert_eq!(prop.command(Samples(4_001), 48_000, 100.0).uv_min.x, 0.5);
    assert_eq!(prop.command(Samples(8_000), 48_000, 100.0).uv_min.x, 0.5);
    assert_eq!(prop.command(Samples(8_001), 48_000, 100.0).uv_min.x, 0.0);
}

#[test]
fn dancing_stage_prop_alternates_left_and_right_on_beats() {
    let stage = StageDefinition::parse(
        br##"{
          "name": "test",
          "props": [{
            "name": "freaks",
            "assetPath": "weeb/bgFreaks",
            "position": [0, 0],
            "scale": [1, 1],
            "scroll": [1, 1],
            "zIndex": 5,
            "danceEvery": 1,
            "startingAnimation": "danceLeft",
            "animations": [
              { "name": "danceLeft", "prefix": "left", "frameRate": 12 },
              { "name": "danceRight", "prefix": "right", "frameRate": 12 }
            ]
          }]
        }"##,
    )
    .unwrap();
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="bgFreaks.png">
          <SubTexture name="left0000" x="0" y="0" width="100" height="80"/>
          <SubTexture name="right0000" x="100" y="0" width="100" height="80"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let object = stage.objects[0].clone();
    let animations =
        load_sparrow_stage_animations(&object, object.animation.as_ref().unwrap(), &atlas).unwrap();
    let prop = SparrowStagePropSprite {
        texture_id: AssetId::new(7),
        texture_width: 200,
        texture_height: 80,
        object,
        starting_animation: starting_animation_index(&animations, "danceLeft"),
        dance_left: animation_index(&animations, "danceLeft"),
        dance_right: animation_index(&animations, "danceRight"),
        animations,
        filter: FilterMode::Linear,
    };

    assert_eq!(prop.command(Samples(0), 48_000, 100.0).uv_min.x, 0.0);
    assert_eq!(prop.command(Samples(28_800), 48_000, 100.0).uv_min.x, 0.5);
    assert_eq!(prop.command(Samples(57_600), 48_000, 100.0).uv_min.x, 0.0);
}

#[test]
fn roses_school_freaks_use_scared_dance_suffix() {
    let stage = StageDefinition::parse(
        br##"{
          "name": "test",
          "props": [{
            "name": "freaks",
            "assetPath": "weeb/bgFreaks",
            "position": [0, 0],
            "scale": [1, 1],
            "scroll": [1, 1],
            "zIndex": 5,
            "danceEvery": 1,
            "startingAnimation": "danceLeft",
            "animations": [
              { "name": "danceLeft", "prefix": "left", "frameRate": 12 },
              { "name": "danceRight", "prefix": "right", "frameRate": 12 },
              { "name": "danceLeft-scared", "prefix": "leftScared", "frameRate": 12 },
              { "name": "danceRight-scared", "prefix": "rightScared", "frameRate": 12 }
            ]
          }]
        }"##,
    )
    .unwrap();
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="bgFreaks.png">
          <SubTexture name="left0000" x="0" y="0" width="100" height="80"/>
          <SubTexture name="leftScared0000" x="100" y="0" width="100" height="80"/>
          <SubTexture name="right0000" x="200" y="0" width="100" height="80"/>
          <SubTexture name="rightScared0000" x="300" y="0" width="100" height="80"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let object = stage.objects[0].clone();
    let animations =
        load_sparrow_stage_animations(&object, object.animation.as_ref().unwrap(), &atlas).unwrap();
    let prop = SparrowStagePropSprite {
        texture_id: AssetId::new(7),
        texture_width: 400,
        texture_height: 80,
        object,
        starting_animation: starting_animation_index(&animations, "danceLeft"),
        dance_left: animation_index(&animations, "danceLeft"),
        dance_right: animation_index(&animations, "danceRight"),
        animations,
        filter: FilterMode::Linear,
    };

    assert_eq!(prop.command(Samples(0), 48_000, 100.0).uv_min.x, 0.0);
    assert_eq!(
        prop.command_for_song(Samples(0), 48_000, 100.0, Some(PreviewSong::ROSES))
            .uv_min
            .x,
        0.25
    );
    assert_eq!(
        prop.command_for_song(Samples(28_800), 48_000, 100.0, Some(PreviewSong::ROSES))
            .uv_min
            .x,
        0.75
    );
}

#[test]
fn spooky_mansion_background_plays_lightning_strike_animation() {
    let stage = StageDefinition::parse(
        br##"{
          "name": "test",
          "props": [{
            "name": "halloweenBG",
            "assetPath": "halloween_bg",
            "animType": "animateatlas",
            "position": [0, 0],
            "scale": [1, 1],
            "scroll": [1, 1],
            "zIndex": 5,
            "startingAnimation": "idle",
            "animations": [
              { "name": "idle", "prefix": "static", "frameRate": 12 },
              { "name": "lightning", "prefix": "thunder", "frameRate": 12 }
            ]
          }]
        }"##,
    )
    .unwrap();
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="halloween_bg.png">
          <SubTexture name="static0000" x="0" y="0" width="100" height="80"/>
          <SubTexture name="thunder0000" x="100" y="0" width="100" height="80"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let object = stage.objects[0].clone();
    let animations =
        load_sparrow_stage_animations(&object, object.animation.as_ref().unwrap(), &atlas).unwrap();
    let prop = SparrowStagePropSprite {
        texture_id: AssetId::new(7),
        texture_width: 200,
        texture_height: 80,
        object,
        starting_animation: starting_animation_index(&animations, "idle"),
        dance_left: None,
        dance_right: None,
        animations,
        filter: FilterMode::Linear,
    };

    assert_eq!(prop.command(Samples(0), 48_000, 100.0).uv_min.x, 0.0);
    assert_eq!(prop.command(Samples(115_200), 48_000, 100.0).uv_min.x, 0.5);
    assert_eq!(prop.command(Samples(172_800), 48_000, 100.0).uv_min.x, 0.0);
}

#[test]
fn weekend_philly_traffic_cycles_red_and_green_on_scripted_beats() {
    let stage = StageDefinition::parse(
        br##"{
          "name": "test",
          "props": [{
            "name": "phillyTraffic",
            "assetPath": "phillyStreets/phillyTraffic",
            "position": [0, 0],
            "scale": [1, 1],
            "scroll": [1, 1],
            "zIndex": 5,
            "startingAnimation": "togreen",
            "animations": [
              { "name": "togreen", "prefix": "green", "frameRate": 12 },
              { "name": "tored", "prefix": "red", "frameRate": 12 }
            ]
          }]
        }"##,
    )
    .unwrap();
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="phillyTraffic.png">
          <SubTexture name="green0000" x="0" y="0" width="100" height="80"/>
          <SubTexture name="red0000" x="100" y="0" width="100" height="80"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let object = stage.objects[0].clone();
    let animations =
        load_sparrow_stage_animations(&object, object.animation.as_ref().unwrap(), &atlas).unwrap();
    let prop = SparrowStagePropSprite {
        texture_id: AssetId::new(7),
        texture_width: 200,
        texture_height: 80,
        object,
        starting_animation: starting_animation_index(&animations, "togreen"),
        dance_left: None,
        dance_right: None,
        animations,
        filter: FilterMode::Linear,
    };

    assert_eq!(prop.command(Samples(0), 48_000, 120.0).uv_min.x, 0.0);
    assert_eq!(prop.command(Samples(192_000), 48_000, 120.0).uv_min.x, 0.5);
    assert_eq!(prop.command(Samples(672_000), 48_000, 120.0).uv_min.x, 0.0);
}

#[test]
fn weekend_philly_cars_select_variant_and_follow_scripted_path() {
    let stage = StageDefinition::parse(
        br##"{"name":"test","props":[{
          "name":"phillyCars","assetPath":"phillyStreets/phillyCars",
          "position":[1200,818],"scale":[1,1],"scroll":[1,1],"zIndex":5,
          "startingAnimation":"car1",
          "animations":[
            {"name":"car1","prefix":"car1","frameRate":24},
            {"name":"car2","prefix":"car2","frameRate":24},
            {"name":"car3","prefix":"car3","frameRate":24},
            {"name":"car4","prefix":"car4","frameRate":24}
          ]}]}"##,
    )
    .unwrap();
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="phillyCars.png">
          <SubTexture name="car10000" x="0" y="0" width="100" height="80"/>
          <SubTexture name="car20000" x="100" y="0" width="100" height="80"/>
          <SubTexture name="car30000" x="200" y="0" width="100" height="80"/>
          <SubTexture name="car40000" x="300" y="0" width="100" height="80"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let object = stage.objects[0].clone();
    let animations =
        load_sparrow_stage_animations(&object, object.animation.as_ref().unwrap(), &atlas).unwrap();
    let prop = SparrowStagePropSprite {
        texture_id: AssetId::new(7),
        texture_width: 400,
        texture_height: 80,
        object,
        starting_animation: starting_animation_index(&animations, "car1"),
        dance_left: None,
        dance_right: None,
        animations,
        filter: FilterMode::Linear,
    };

    let cmd = prop
        .commands(Samples(96_000), 48_000, 120.0, None)
        .remove(0);
    assert_eq!(cmd.uv_min.x, 0.0);
    assert!((cmd.world_pos.x - 1263.4).abs() < 0.01);
    assert!(cmd.rotation < 0.0);
}

#[test]
fn blazin_lightning_prop_only_draws_during_scripted_strikes() {
    let stage = StageDefinition::parse(
        br##"{
          "name": "test",
          "props": [{
            "name": "lightning",
            "assetPath": "phillyBlazin/lightning",
            "position": [50, -300],
            "scale": [1, 1],
            "scroll": [0, 0],
            "zIndex": 5,
            "startingAnimation": "strike",
            "animations": [
              { "name": "strike", "prefix": "lightning", "frameRate": 12 }
            ]
          }]
        }"##,
    )
    .unwrap();
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="lightning.png">
          <SubTexture name="lightning0000" x="0" y="0" width="100" height="80"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let object = stage.objects[0].clone();
    let animations =
        load_sparrow_stage_animations(&object, object.animation.as_ref().unwrap(), &atlas).unwrap();
    let prop = SparrowStagePropSprite {
        texture_id: AssetId::new(7),
        texture_width: 100,
        texture_height: 80,
        object,
        starting_animation: starting_animation_index(&animations, "strike"),
        dance_left: None,
        dance_right: None,
        animations,
        filter: FilterMode::Linear,
    };

    assert!(prop
        .commands(Samples(95_999), 48_000, 180.0, None)
        .is_empty());
    let strike = prop.commands(Samples(144_000), 48_000, 180.0, None);
    assert_eq!(strike.len(), 1);
    assert_eq!(strike[0].world_pos.x, -220.0);
    assert!(prop
        .commands(Samples(216_000), 48_000, 180.0, None)
        .is_empty());
}

#[test]
fn sserafim_getup_cutscene_sprites_continue_into_gameplay() {
    let getup_start = sserafim_intro_event_cursor(710.0, 48_000, 120.0);

    assert_eq!(
        sserafim_cutscene_animate_started_at("sserafimGfGetUp", getup_start, 48_000, 120.0),
        Some(getup_start)
    );
    assert_eq!(
        sserafim_cutscene_animate_started_at(
            "sserafimBfGetUp",
            countdown_start_cursor(48_000, 120.0),
            48_000,
            120.0
        ),
        Some(getup_start)
    );
    assert_eq!(
        sserafim_cutscene_animate_started_at("sserafimCutsceneMain", getup_start, 48_000, 120.0),
        None
    );
}

#[test]
fn week7_tank_rolling_motion_follows_scripted_circle() {
    let pose_start = tank_rolling_pose(Samples(0), 48_000);
    let pose_later = tank_rolling_pose(Samples(48_000), 48_000);

    assert!((pose_start.position.x - 400.0).abs() < 0.01);
    assert!((pose_start.position.y - 2400.0).abs() < 0.01);
    assert!(pose_later.position.x < pose_start.position.x);
    assert!(pose_later.rotation > pose_start.rotation);
}
