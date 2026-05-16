use super::*;
use crate::preview_song::{PreviewDifficulty, PreviewSong};

#[test]
fn preview_play_state_uses_selected_difficulty() {
    let easy = load_preview_play_state_for(
        PreviewSelection {
            song: PreviewSong::BOPEEBO,
            difficulty: PreviewDifficulty::Easy,
            variation: None,
        },
        48_000,
    )
    .expect("easy bopeebo chart");
    let hard = load_preview_play_state_for(
        PreviewSelection {
            song: PreviewSong::BOPEEBO,
            difficulty: PreviewDifficulty::Hard,
            variation: None,
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
        variation: None,
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
        variation: None,
    })
    .expect("tutorial chart metadata");

    assert_eq!(chart.chart.player2, "gf");
    assert_eq!(chart.chart.girlfriend, "");
    assert_eq!(chart.chart.stage, "mainStage");
    assert_eq!(stage_asset_id(&chart.chart.stage), "mainStage");
    assert_eq!(character_id(&chart.chart.girlfriend), None);
}

#[test]
fn preview_song_metadata_preserves_pixel_note_style() {
    let chart = load_preview_song_for(PreviewSelection {
        song: PreviewSong::SENPAI,
        difficulty: PreviewDifficulty::Normal,
        variation: None,
    })
    .expect("senpai chart metadata");

    assert_eq!(chart.chart.note_style, "pixel");
}

#[test]
fn preview_song_uses_bf_weekend_variation_when_requested() {
    let chart = load_preview_song_for(
        PreviewSelection::new(PreviewSong::DARNELL, PreviewDifficulty::Normal)
            .with_variation(Some(crate::preview_song::VARIATION_BF)),
    )
    .expect("darnell bf chart metadata");

    assert_eq!(chart.chart.player1, "bf");
    assert_eq!(chart.chart.player2, "darnell");
    assert_eq!(chart.chart.stage, "phillyStreetsErect");
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

#[test]
fn all_registered_story_song_charts_load_for_available_difficulties() {
    for song in PreviewSong::ALL
        .iter()
        .chain(PreviewSong::FREEPLAY_EXTRA.iter())
        .copied()
    {
        for difficulty in song.available_difficulties() {
            let selection = PreviewSelection::new(song, *difficulty);
            load_preview_song_for(selection).unwrap_or_else(|error| {
                panic!(
                    "load {} {} chart: {error:#}",
                    song.folder,
                    difficulty.as_str()
                )
            });
        }
    }
}

#[test]
fn all_imported_story_stages_parse() {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    for stage_id in [
        "mainStage",
        "mainStageErect",
        "spookyMansion",
        "spookyMansionErect",
        "phillyTrain",
        "phillyTrainErect",
        "phillyStreets",
        "phillyStreetsErect",
        "limoRide",
        "limoRideErect",
        "mallXmas",
        "mallXmasErect",
        "mallEvil",
        "school",
        "schoolErect",
        "schoolEvil",
        "schoolEvilErect",
        "tankmanBattlefield",
        "tankmanBattlefieldErect",
        "phillyBlazin",
        "sserafim",
    ] {
        let path = AssetPath::new(format!("data/stages/{stage_id}.json")).unwrap();
        load_stage(&resolver, &path)
            .unwrap_or_else(|error| panic!("load stage {stage_id}: {error:#}"));
    }
}

#[test]
fn sserafim_source_stage_registers_cutscene_and_dust_props() {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .to_path_buf();
    let resolver = OverlayResolver::new().with_baked_root(workspace.join("assets/source"));
    let stage = load_stage(
        &resolver,
        &AssetPath::new("data/stages/sserafim.json").unwrap(),
    )
    .unwrap();
    let ids = stage
        .objects
        .iter()
        .map(|object| object.id.as_str())
        .collect::<Vec<_>>();

    for id in [
        "cutsceneFloor",
        "sserafimCutsceneMain",
        "sserafimGfGetUp",
        "sserafimBfGetUp",
        "sserafimDust1",
        "sserafimDust2",
        "sserafimDust3",
        "sserafimDust4",
        "sserafimEnd1",
        "sserafimEnd2",
        "solidFlash",
    ] {
        assert!(ids.contains(&id), "missing sserafim prop {id}");
    }
}

#[test]
fn sserafim_source_stage_art_uses_full_reference_images() {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .to_path_buf();
    let resolver = OverlayResolver::new().with_baked_root(workspace.join("assets/source"));

    for (path, min_width, min_height) in [
        ("images/sserafim/bg.png", 5_000, 1_000),
        ("images/sserafim/floor.png", 5_000, 700),
        ("images/sserafim/back-tables.png", 4_000, 400),
        ("images/sserafim/truck-stuff.png", 3_000, 1_500),
        ("images/sserafim/lights/truck-light1.png", 2_000, 1_000),
    ] {
        let image = rustic_asset::load_png(&resolver, &AssetPath::new(path).unwrap())
            .unwrap_or_else(|error| panic!("load {path}: {error:#}"));
        assert!(
            image.width >= min_width && image.height >= min_height,
            "{path} should be full-size art, got {}x{}",
            image.width,
            image.height
        );
    }
}

#[test]
fn tankman_bloody_source_character_aliases_heh_pretty_good() {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .to_path_buf();
    let resolver = OverlayResolver::new().with_baked_root(workspace.join("assets/source"));
    let character = load_character(
        &resolver,
        &AssetPath::new("data/characters/tankman-bloody.json").unwrap(),
    )
    .unwrap();

    assert!(character
        .animations
        .iter()
        .any(|animation| animation.name == "hehPrettyGood"));
}

#[test]
fn all_registered_story_scenes_load_available_assets() {
    let render_state =
        pollster::block_on(rustic_render::RenderState::headless()).expect("headless render state");
    for song in PreviewSong::ALL
        .iter()
        .chain(PreviewSong::FREEPLAY_EXTRA.iter())
        .copied()
    {
        for difficulty in song.available_difficulties() {
            let selection = PreviewSelection::new(song, *difficulty);
            load_preview_scene_for(&render_state.device, &render_state.queue, selection)
                .unwrap_or_else(|error| {
                    panic!(
                        "load scene {} {}: {error:#}",
                        song.folder,
                        difficulty.as_str()
                    )
                });
        }
    }
}
