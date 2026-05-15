#![allow(clippy::unwrap_used)]

use super::*;
use crate::asset_roots::baked_assets_root;
use rustic_asset::load_character;

fn character_with_hold() -> CharacterDefinition {
    CharacterDefinition::parse(
        br#"{
            "id": "dad",
            "renderType": "animateatlas",
            "assetPath": "shared:characters/dad",
            "animations": [
                { "name": "singUP", "prefix": "Up", "frameRate": 24 },
                { "name": "singUP-hold", "prefix": "Up", "frameRate": 24,
                  "looped": true, "frameIndices": [3, 4, 5] }
            ]
        }"#,
    )
    .unwrap()
}

fn pose(animation: CharacterAnimation, frame_count: usize) -> AnimateCharacterPose {
    AnimateCharacterPose {
        animation,
        asset_index: 0,
        source: AnimatePoseSource::FrameLabel,
        frame_count,
    }
}

fn baked_resolver() -> OverlayResolver {
    OverlayResolver::new().with_baked_root(baked_assets_root())
}

fn test_animate_atlas(resolver: &OverlayResolver, asset_path: &AssetPath) -> LoadedAnimateAtlas {
    let animation_path = animate_asset_file(asset_path, "Animation.json").unwrap();
    let spritemap_path = animate_asset_file(asset_path, "spritemap1.json").unwrap();
    let animation = load_animate_animation(resolver, &animation_path).unwrap();
    let atlas = load_animate_spritemap(resolver, &spritemap_path).unwrap();
    LoadedAnimateAtlas {
        texture_id: AssetId::new(1),
        flat_labels: FlatLabelAtlas::new(&animation, &atlas),
        animation,
        atlas,
    }
}

#[test]
fn finished_animate_pose_switches_to_matching_hold_animation() {
    let character = character_with_hold();
    let sprite = AnimateCharacterSprite {
        poses: vec![
            pose(character.animations[0].clone(), 2),
            pose(character.animations[1].clone(), 3),
        ],
        character,
        slot: StageCharacterSlot::default(),
        origin: glam::Vec2::ZERO,
        visual_height: 0.0,
        is_player: false,
        z: 0,
        filter: FilterMode::Nearest,
        assets: Vec::new(),
        initial_pose: 0,
        mixed_sparrow: None,
        lip_sync: None,
    };
    let request = CharacterPoseRequest {
        name: "singUP",
        started_at: Samples(100),
    };

    let (before, before_started) = sprite.resolve_pose_for_request(request, Samples(4_099), 48_000);
    assert_eq!(before.animation.name, "singUP");
    assert_eq!(before_started, Samples(100));

    let (after, after_started) = sprite.resolve_pose_for_request(request, Samples(4_100), 48_000);
    assert_eq!(after.animation.name, "singUP-hold");
    assert_eq!(after_started, Samples(4_100));
}

#[test]
fn flat_label_indices_are_relative_to_the_label_prefix() {
    let animation = AnimateAnimation::parse(
        br#"{
            "AN": {
                "N": "Test",
                "SN": "Root",
                "TL": {
                    "L": [
                        { "LN": "labels", "FR": [
                            { "N": "Idle", "I": 0, "DU": 3, "E": [] },
                            { "N": "Up", "I": 3, "DU": 3, "E": [] }
                        ] },
                        { "LN": "art", "FR": [
                            { "I": 0, "DU": 6, "E": [] }
                        ] }
                    ]
                }
            }
        }"#,
    )
    .unwrap();
    let atlas = AnimateAtlas::parse_spritemap(
        br#"{
            "ATLAS": {
                "SPRITES": [
                    { "SPRITE": { "name": "idle0", "x": 0, "y": 0, "w": 10, "h": 10 } },
                    { "SPRITE": { "name": "idle1", "x": 10, "y": 0, "w": 10, "h": 10 } },
                    { "SPRITE": { "name": "idle2", "x": 20, "y": 0, "w": 10, "h": 10 } },
                    { "SPRITE": { "name": "up0", "x": 30, "y": 0, "w": 10, "h": 10 } },
                    { "SPRITE": { "name": "up1", "x": 40, "y": 0, "w": 10, "h": 10 } },
                    { "SPRITE": { "name": "up2", "x": 50, "y": 0, "w": 10, "h": 10 } }
                ],
                "meta": { "size": { "w": 60, "h": 10 } }
            }
        }"#,
    )
    .unwrap();
    let flat = FlatLabelAtlas::new(&animation, &atlas).unwrap();
    let character = CharacterDefinition::parse(
        br#"{
            "id": "test",
            "assetPath": "shared:characters/test",
            "animations": [
                { "name": "singUP-hold", "prefix": "Up", "frameIndices": [1] }
            ]
        }"#,
    )
    .unwrap();

    let parts = flat.parts(&animation, &character.animations[0], 1).unwrap();
    assert_eq!(parts[0].frame_name, "up1");
}

#[test]
fn gf_dance_left_uses_animate_symbols_instead_of_flat_spritemap_slices() {
    let resolver = baked_resolver();
    let character = load_character(
        &resolver,
        &AssetPath::new("data/characters/gf.json").unwrap(),
    )
    .unwrap();
    let asset_path = animation_asset_path(&character, &character.animations[0]).unwrap();
    let loaded = test_animate_atlas(&resolver, &asset_path);
    assert!(loaded.flat_labels.is_none());
    let mut asset_indices = HashMap::new();
    asset_indices.insert(asset_path.as_str().to_owned(), 0);
    let poses =
        animate_character_poses(&character, std::slice::from_ref(&loaded), &asset_indices).unwrap();
    let dance_left = poses
        .iter()
        .find(|pose| pose.animation.name == "danceLeft")
        .unwrap();
    let parts = dance_left
        .parts(&loaded, Samples(0), SAMPLE_RATE, Samples(0))
        .unwrap();
    let frame_names = parts
        .iter()
        .map(|part| part.frame_name.as_str())
        .collect::<Vec<_>>();

    assert!(frame_names.len() > 4);
    assert!(frame_names.contains(&"16"));
    assert!(!frame_names
        .iter()
        .any(|name| ["36", "37", "38"].contains(name)));
}

#[test]
fn animate_origin_uses_bottom_center_of_flattened_bounds() {
    let asset = test_animate_atlas(
        &baked_resolver(),
        &AssetPath::new("shared:characters/dad").unwrap(),
    );
    let character = character_with_hold();
    let mut asset_indices = HashMap::new();
    asset_indices.insert("shared:characters/dad".to_owned(), 0);
    let poses =
        animate_character_poses(&character, std::slice::from_ref(&asset), &asset_indices).unwrap();
    let (origin, visual_height) = animate_character_origin(&poses[0], &[asset], 1.0).unwrap();
    assert!(origin.x.is_finite());
    assert!(origin.y > 0.0);
    assert!(visual_height > 0.0);
}

#[test]
fn pico_playable_censor_poses_use_mixed_sparrow_asset() {
    let render_state =
        pollster::block_on(rustic_render::RenderState::headless()).expect("headless render state");
    let resolver = baked_resolver();
    let character = load_character(
        &resolver,
        &AssetPath::new("data/characters/pico-playable.json").unwrap(),
    )
    .unwrap();
    let mut textures = HashMap::new();
    let sprite = load_animate_character_sprite(
        &render_state.device,
        &render_state.queue,
        &resolver,
        character,
        StageCharacterSlot::default(),
        true,
        0,
        &mut textures,
    )
    .unwrap();

    let commands = sprite.commands(
        CharacterPoseRequest {
            name: "singRIGHT-censor",
            started_at: Samples(0),
        },
        Samples(0),
        SAMPLE_RATE,
    );

    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].texture,
        asset_id_for_path(&AssetPath::new("images/characters/Pico_Censored.png").unwrap())
    );
}
