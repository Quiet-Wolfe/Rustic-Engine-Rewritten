use super::*;
use crate::preview_song::{
    PreviewDifficulty, PreviewSelection, PreviewSong, VARIATION_BF, VARIATION_PICO,
};

fn source_resolver() -> OverlayResolver {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir.to_path_buf());
    OverlayResolver::new().with_baked_root(workspace.join("assets/source"))
}

#[test]
fn reads_album_id_from_song_metadata() {
    let resolver = source_resolver();
    let album = load_song_album_id(
        &resolver,
        PreviewSong::BOPEEBO.metadata_path_for(PreviewDifficulty::Erect),
    )
    .unwrap();

    assert_eq!(album, "volume3");
}

#[test]
fn song_album_map_uses_base_and_variant_metadata() {
    let resolver = source_resolver();
    let metadata = load_song_metadata_map(&resolver);

    let bopeebo = metadata.get(&PreviewSong::BOPEEBO.id).unwrap();
    assert_eq!(
        bopeebo.album_id_for_selection(PreviewSelection::new(
            PreviewSong::BOPEEBO,
            PreviewDifficulty::Normal
        )),
        "volume1"
    );
    assert_eq!(
        bopeebo.album_id_for_selection(PreviewSelection::new(
            PreviewSong::BOPEEBO,
            PreviewDifficulty::Erect
        )),
        "volume3"
    );
    assert_eq!(
        bopeebo.album_id_for_selection(
            PreviewSelection::new(PreviewSong::BOPEEBO, PreviewDifficulty::Normal)
                .with_variation(Some(VARIATION_PICO))
        ),
        "volume4"
    );

    let darnell = metadata.get(&PreviewSong::DARNELL.id).unwrap();
    assert_eq!(
        darnell.album_id_for_selection(PreviewSelection::new(
            PreviewSong::DARNELL,
            PreviewDifficulty::Normal
        )),
        "volume3"
    );
    assert_eq!(
        darnell.album_id_for_selection(PreviewSelection::new(
            PreviewSong::DARNELL,
            PreviewDifficulty::Erect
        )),
        "expansion2"
    );
    assert_eq!(
        darnell.album_id_for_selection(
            PreviewSelection::new(PreviewSong::DARNELL, PreviewDifficulty::Normal)
                .with_variation(Some(VARIATION_BF))
        ),
        "volume4"
    );

    let spaghetti = metadata.get(&PreviewSong::SPAGHETTI.id).unwrap();
    assert_eq!(
        spaghetti.album_id_for_selection(PreviewSelection::new(
            PreviewSong::SPAGHETTI,
            PreviewDifficulty::Normal
        )),
        "spaghetti"
    );
}

#[test]
fn song_metadata_map_uses_variant_difficulty_ratings() {
    let resolver = source_resolver();
    let metadata = load_song_metadata_map(&resolver);
    let darnell = metadata.get(&PreviewSong::DARNELL.id).unwrap();
    let bf_normal = PreviewSelection::new(PreviewSong::DARNELL, PreviewDifficulty::Normal)
        .with_variation(Some(VARIATION_BF));

    assert_eq!(darnell.rating_for_selection(bf_normal), Some(4));
    assert_eq!(
        darnell.rating_for_selection(PreviewSelection::new(
            PreviewSong::DARNELL,
            PreviewDifficulty::Normal
        )),
        Some(3)
    );
}

#[test]
fn song_metadata_map_exposes_freeplay_icon_character_ids() {
    let resolver = source_resolver();
    let metadata = load_song_metadata_map(&resolver);

    let tutorial = metadata.get(&PreviewSong::TUTORIAL.id).unwrap();
    assert_eq!(
        tutorial.icon_id_for_selection(PreviewSelection::new(
            PreviewSong::TUTORIAL,
            PreviewDifficulty::Normal
        )),
        Some("gf")
    );

    let stress = metadata.get(&PreviewSong::STRESS.id).unwrap();
    assert_eq!(
        stress.icon_id_for_selection(
            PreviewSelection::new(PreviewSong::STRESS, PreviewDifficulty::Normal)
                .with_variation(Some(VARIATION_PICO))
        ),
        Some("tankman-bloody")
    );
}

#[test]
fn song_metadata_difficulties_follow_active_character_variation() {
    let resolver = source_resolver();
    let metadata = load_song_metadata_map(&resolver);
    let bopeebo = metadata.get(&PreviewSong::BOPEEBO.id).unwrap();

    assert_eq!(
        bopeebo.difficulties_for_variation(None),
        vec![
            PreviewDifficulty::Easy,
            PreviewDifficulty::Normal,
            PreviewDifficulty::Hard,
            PreviewDifficulty::Erect,
            PreviewDifficulty::Nightmare
        ]
    );
    assert_eq!(
        bopeebo.difficulties_for_variation(Some(VARIATION_PICO)),
        vec![
            PreviewDifficulty::Easy,
            PreviewDifficulty::Normal,
            PreviewDifficulty::Hard
        ]
    );
}

#[test]
fn freeplay_style_data_drives_pico_paths_and_confirm_delay() {
    let resolver = source_resolver();
    let style = load_freeplay_style_config(&resolver, FreeplayStyle::Pico).unwrap();

    assert_eq!(
        style.bg_image_path(),
        "images/freeplay/freeplayBGweek1-pico.png"
    );
    assert_eq!(
        style.selector_atlas_path(),
        "images/freeplay/freeplaySelector/freeplaySelector_pico.xml"
    );
    assert_eq!(
        style.capsule_atlas_path(),
        "images/freeplay/freeplayCapsule/capsule/freeplayCapsule_pico.xml"
    );
    assert_eq!(style.start_delay, 1.45);
    assert_eq!(
        style.capsule_text_colors,
        [
            glam::vec4(0xCC as f32 / 255.0, 0x66 as f32 / 255.0, 0.0, 1.0),
            glam::vec4(0xCC as f32 / 255.0, 0x99 as f32 / 255.0, 0.0, 1.0)
        ]
    );
}
