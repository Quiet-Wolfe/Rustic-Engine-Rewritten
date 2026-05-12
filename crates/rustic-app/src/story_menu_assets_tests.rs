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
fn parses_week_background_hex() {
    let color = color_from_story_hex("#F9CF51");
    assert!((color.x - 249.0 / 255.0).abs() < 0.001);
    assert!((color.y - 207.0 / 255.0).abs() < 0.001);
    assert!((color.z - 81.0 / 255.0).abs() < 0.001);
}
