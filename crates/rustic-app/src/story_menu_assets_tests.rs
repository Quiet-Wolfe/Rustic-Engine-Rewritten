use super::*;

#[test]
fn story_difficulty_excludes_erect_variants() {
    let level = match LevelDefinition::parse(
        br#"{
            "name":"DADDY DEAREST",
            "titleAsset":"storymenu/titles/week1",
            "songs":["bopeebo","fresh","dadbattle"]
        }"#,
    ) {
        Ok(level) => level,
        Err(error) => panic!("test fixture should parse: {error}"),
    };
    assert_eq!(
        story_difficulties(&level),
        vec![
            PreviewDifficulty::Easy,
            PreviewDifficulty::Normal,
            PreviewDifficulty::Hard,
        ]
    );
}

#[test]
fn story_beat_uses_menu_music_bpm() {
    assert_eq!(story_beat(Samples(0), 48_000), 0);
    assert_eq!(story_beat(Samples(48_000), 48_000), 1);
}

#[test]
fn story_prop_animation_holds_previous_frame_on_exact_boundary() {
    assert_eq!(
        animation_frame_index(Samples(2_000), 48_000, Samples(0), 24, 3, false),
        0
    );
    assert_eq!(
        animation_frame_index(Samples(2_001), 48_000, Samples(0), 24, 3, false),
        1
    );
    assert_eq!(
        animation_frame_index(Samples(6_000), 48_000, Samples(0), 24, 3, true),
        2
    );
    assert_eq!(
        animation_frame_index(Samples(6_001), 48_000, Samples(0), 24, 3, true),
        0
    );
}

#[test]
fn parses_week_background_hex() {
    let color = color_from_story_hex("#F9CF51");
    assert!((color.x - 249.0 / 255.0).abs() < 0.001);
    assert!((color.y - 207.0 / 255.0).abs() < 0.001);
    assert!((color.z - 81.0 / 255.0).abs() < 0.001);
}

#[test]
fn story_menu_registry_includes_all_vslice_levels() {
    assert_eq!(
        STORY_LEVEL_IDS,
        [
            "tutorial", "week1", "week2", "week3", "week4", "week5", "week6", "week7", "weekend1",
            "sserafim"
        ]
    );
}

#[test]
fn sserafim_level_resolves_spaghetti_playlist() {
    let level = match LevelDefinition::parse(
        br#"{
            "name":"LE SSERAFIM",
            "titleAsset":"storymenu/titles/sserafim",
            "songs":["spaghetti"]
        }"#,
    ) {
        Ok(level) => level,
        Err(error) => panic!("test fixture should parse: {error}"),
    };
    let story = StoryMenuAssets {
        levels: vec![StoryLevel {
            data: level,
            title: StaticTexture {
                texture_id: WHITE_TEXTURE_ID,
                width: 1,
                height: 1,
                filter: FilterMode::Nearest,
            },
            props: Vec::new(),
            difficulties: vec![
                PreviewDifficulty::Easy,
                PreviewDifficulty::Normal,
                PreviewDifficulty::Hard,
            ],
        }],
        arrows: ArrowSkin {
            left_idle: dummy_arrow(),
            right_idle: dummy_arrow(),
        },
        difficulties: HashMap::new(),
        textures: HashMap::new(),
    };

    assert_eq!(
        story.preview_playlist(0),
        Some(vec![PreviewSong::SPAGHETTI])
    );
}

#[test]
fn story_prop_prefers_starting_animation_without_dance_pair() {
    let prop = StoryPropClip {
        texture_id: WHITE_TEXTURE_ID,
        texture_width: 1,
        texture_height: 1,
        position: glam::Vec2::ZERO,
        scale: glam::Vec2::ONE,
        alpha: 1.0,
        starting_animation: Some("spin".to_string()),
        animations: vec![story_clip("idle", false), story_clip("spin", true)],
    };

    assert_eq!(
        prop.animation_for_cursor(Samples(48_000), 48_000)
            .map(|animation| animation.name.as_str()),
        Some("spin")
    );
}

#[test]
fn story_animation_respects_looped_level_metadata() {
    let level = LevelDefinition::parse(
        br#"{
            "name":"LE SSERAFIM",
            "titleAsset":"storymenu/titles/sserafim",
            "props":[{
                "assetPath":"storymenu/props/spaghetti",
                "startingAnimation":"spin",
                "animations":[{
                    "name":"spin",
                    "prefix":"SPL_0",
                    "looped":true
                }]
            }],
            "songs":["spaghetti"]
        }"#,
    )
    .unwrap();
    let atlas = SparrowAtlas::parse(
        br#"<TextureAtlas imagePath="spaghetti.png">
          <SubTexture name="SPL_0000" x="0" y="0" width="20" height="20"/>
          <SubTexture name="SPL_0001" x="20" y="0" width="20" height="20"/>
        </TextureAtlas>"#,
    )
    .unwrap();
    let animation = &level.props[0].animations[0];

    assert!(story_animation(&atlas, animation).unwrap().looped);
}

fn dummy_arrow() -> SparrowStill {
    SparrowStill {
        texture_id: WHITE_TEXTURE_ID,
        texture_width: 1,
        texture_height: 1,
        frame: SparrowFrame::untrimmed("dummy".to_string(), 0, 0, 1, 1),
    }
}

fn story_clip(name: &str, looped: bool) -> StoryAnimationClip {
    StoryAnimationClip {
        name: name.to_string(),
        fps: 24,
        looped,
        offset: glam::Vec2::ZERO,
        frames: vec![SparrowFrame::untrimmed(format!("{name}0000"), 0, 0, 1, 1)],
    }
}
