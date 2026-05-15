//! App-owned loading of vanilla song stems into the shared mixer.

use crate::scene_assets::load_preview_song_for;
use crate::{asset_roots::baked_assets_root, preview_song::PreviewSelection};
use anyhow::{anyhow, Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver, ParsedSong};
use rustic_audio::{streaming_vorbis_source, SharedMixer, SoundSource, Stem};
use rustic_core::time::Samples;

pub fn load_preview_stems(mixer: &SharedMixer, start_cursor: Samples) -> Result<()> {
    load_preview_stems_for(mixer, PreviewSelection::from_env(), start_cursor)
}

pub fn load_preview_stems_for(
    mixer: &SharedMixer,
    selection: PreviewSelection,
    start_cursor: Samples,
) -> Result<()> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let song = selection.song;
    let parsed = load_preview_song_for(selection).ok();
    let inst = load_first_stem(&resolver, &inst_paths(selection))?;
    let vocals = load_optional_vocals(
        &resolver,
        &vocal_path_groups(selection, parsed.as_ref()),
        &selection.song.voices_path(),
    );
    configure_song_stems(mixer, inst, vocals, start_cursor, song.folder)
}

fn configure_song_stems(
    mixer: &SharedMixer,
    inst: SoundSource,
    vocals: Vec<SoundSource>,
    start_cursor: Samples,
    song_id: &str,
) -> Result<()> {
    mixer
        .edit(|mixer| {
            mixer.clear();
            mixer.add_source(Stem::Instrumental, inst)?;
            for vocals in vocals {
                mixer.add_source(Stem::Vocals, vocals)?;
            }
            mixer.seek(start_cursor)?;
            Ok(())
        })
        .with_context(|| format!("configure {song_id} stems"))?;
    Ok(())
}

pub fn play_sample_rate(mixer: &SharedMixer) -> u32 {
    mixer.sample_rate().max(1)
}

pub fn set_vocals_gain(mixer: &SharedMixer, gain: f32) {
    if let Err(e) = mixer.edit(|mixer| {
        mixer.set_stem_gain(Stem::Vocals, gain);
        Ok(())
    }) {
        tracing::warn!(target: "rustic.audio", "set vocals gain: {e:#}");
    }
}

fn load_stem(resolver: &OverlayResolver, path: &str) -> Result<rustic_audio::SoundSource> {
    let path = AssetPath::new(path)?;
    let bytes = load_bytes(resolver, &path).with_context(|| format!("load {}", path.as_str()))?;
    streaming_vorbis_source(bytes).with_context(|| format!("decode {}", path.as_str()))
}

fn load_first_stem(resolver: &OverlayResolver, paths: &[String]) -> Result<SoundSource> {
    let mut errors = Vec::new();
    for path in paths {
        match load_stem(resolver, path) {
            Ok(stem) => return Ok(stem),
            Err(e) => errors.push(format!("{path}: {e:#}")),
        }
    }
    Err(anyhow!("no usable stem in [{}]", errors.join("; ")))
}

fn load_optional_vocals(
    resolver: &OverlayResolver,
    groups: &[Vec<String>],
    legacy_path: &str,
) -> Vec<SoundSource> {
    let mut vocals = Vec::new();
    let mut errors = Vec::new();
    for paths in groups {
        match load_first_stem(resolver, paths) {
            Ok(stem) => vocals.push(stem),
            Err(e) => errors.push(format!("{e:#}")),
        }
    }
    if vocals.is_empty() {
        match load_stem(resolver, legacy_path) {
            Ok(stem) => vocals.push(stem),
            Err(e) => errors.push(format!("{legacy_path}: {e:#}")),
        }
    }
    if vocals.is_empty() {
        tracing::warn!(target: "rustic.audio", "optional vocals unavailable: {}", errors.join("; "));
    }
    vocals
}

fn inst_paths(selection: PreviewSelection) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(suffix) = selection.effective_variation_suffix() {
        paths.push(format!("songs/{}/Inst-{suffix}.ogg", selection.song.folder));
    }
    paths.push(format!("songs/{}/Inst.ogg", selection.song.folder));
    paths.push(selection.song.inst_path());
    paths
}

#[cfg(test)]
fn vocal_paths(selection: PreviewSelection, parsed: Option<&ParsedSong>) -> Vec<String> {
    let mut paths: Vec<_> = vocal_path_groups(selection, parsed)
        .into_iter()
        .flatten()
        .collect();
    paths.push(selection.song.voices_path());
    paths
}

fn vocal_path_groups(selection: PreviewSelection, parsed: Option<&ParsedSong>) -> Vec<Vec<String>> {
    let suffix = selection.effective_variation_suffix();
    parsed
        .map(|parsed| {
            vocal_character_ids(parsed)
                .into_iter()
                .map(|id| {
                    let mut paths = Vec::new();
                    if let Some(suffix) = suffix {
                        paths.push(format!(
                            "songs/{}/Voices-{id}-{suffix}.ogg",
                            selection.song.folder
                        ));
                    }
                    paths.push(format!("songs/{}/Voices-{id}.ogg", selection.song.folder));
                    paths
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn vocal_character_ids(parsed: &ParsedSong) -> Vec<&str> {
    let mut ids = Vec::new();
    for id in [
        parsed.chart.player1.as_str(),
        parsed.chart.player2.as_str(),
        parsed.chart.girlfriend.as_str(),
    ] {
        let id = id.trim();
        if !id.is_empty() && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustic_audio::Mixer;
    use std::sync::Arc;

    #[test]
    fn configure_song_stems_allows_missing_optional_vocals() {
        let mixer = SharedMixer::new(Mixer::new(48_000));
        configure_song_stems(
            &mixer,
            SoundSource::Pcm(Arc::from([0.0_f32, 0.0])),
            Vec::new(),
            Samples(12),
            "tutorial",
        )
        .unwrap();

        mixer
            .edit(|mixer| {
                assert_eq!(mixer.voice_count(), 1);
                assert_eq!(mixer.sample_cursor(), Samples(12));
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn preview_stem_paths_prefer_vslice_split_vocals_then_legacy() {
        const CHART: &str = r#"{"scrollSpeed":{"normal":1},"notes":{"normal":[]}}"#;
        const METADATA: &str = r#"{
            "songName": "Tutorial",
            "playData": {
                "characters": { "player": "bf", "opponent": "gf", "girlfriend": "" }
            },
            "timeChanges": [{ "bpm": 100 }]
        }"#;
        let selection = PreviewSelection::from_keys(Some("tutorial"), Some("normal"));
        let parsed =
            ParsedSong::parse_vslice(CHART.as_bytes(), METADATA.as_bytes(), "normal").unwrap();

        assert_eq!(
            inst_paths(selection),
            vec!["songs/tutorial/Inst.ogg", "music/Tutorial_Inst.ogg"]
        );
        assert_eq!(
            vocal_paths(selection, Some(&parsed)),
            vec![
                "songs/tutorial/Voices-bf.ogg",
                "songs/tutorial/Voices-gf.ogg",
                "music/Tutorial_Voices.ogg"
            ]
        );
    }

    #[test]
    fn preview_stem_paths_prefer_erect_variant_stems() {
        const CHART: &str = r#"{"scrollSpeed":{"erect":1},"notes":{"erect":[]}}"#;
        const METADATA: &str = r#"{
            "songName": "DadBattle Erect",
            "playData": {
                "characters": { "player": "bf", "opponent": "dad", "girlfriend": "gf" },
                "stage": "mainStageErect"
            },
            "timeChanges": [{ "bpm": 190 }]
        }"#;
        let selection = PreviewSelection::from_keys(Some("dadbattle"), Some("erect"));
        let parsed =
            ParsedSong::parse_vslice(CHART.as_bytes(), METADATA.as_bytes(), "erect").unwrap();

        assert_eq!(
            inst_paths(selection),
            vec![
                "songs/dadbattle/Inst-erect.ogg",
                "songs/dadbattle/Inst.ogg",
                "music/Dadbattle_Inst.ogg"
            ]
        );
        assert_eq!(
            vocal_paths(selection, Some(&parsed)),
            vec![
                "songs/dadbattle/Voices-bf-erect.ogg",
                "songs/dadbattle/Voices-bf.ogg",
                "songs/dadbattle/Voices-dad-erect.ogg",
                "songs/dadbattle/Voices-dad.ogg",
                "songs/dadbattle/Voices-gf-erect.ogg",
                "songs/dadbattle/Voices-gf.ogg",
                "music/Dadbattle_Voices.ogg"
            ]
        );
    }

    #[test]
    fn preview_stem_paths_prefer_character_variant_stems() {
        const CHART: &str = r#"{"scrollSpeed":{"hard":1},"notes":{"hard":[]}}"#;
        const METADATA: &str = r#"{
            "songName": "Bopeebo Pico",
            "playData": {
                "characters": {
                    "player": "pico-playable",
                    "opponent": "dad",
                    "girlfriend": "gf"
                }
            },
            "timeChanges": [{ "bpm": 100 }]
        }"#;
        let selection = PreviewSelection::from_keys(Some("bopeebo"), Some("hard"))
            .with_variation(Some(crate::preview_song::VARIATION_PICO));
        let parsed =
            ParsedSong::parse_vslice(CHART.as_bytes(), METADATA.as_bytes(), "hard").unwrap();

        assert_eq!(
            inst_paths(selection),
            vec![
                "songs/bopeebo/Inst-pico.ogg",
                "songs/bopeebo/Inst.ogg",
                "music/Bopeebo_Inst.ogg"
            ]
        );
        assert_eq!(
            vocal_paths(selection, Some(&parsed)),
            vec![
                "songs/bopeebo/Voices-pico-playable-pico.ogg",
                "songs/bopeebo/Voices-pico-playable.ogg",
                "songs/bopeebo/Voices-dad-pico.ogg",
                "songs/bopeebo/Voices-dad.ogg",
                "songs/bopeebo/Voices-gf-pico.ogg",
                "songs/bopeebo/Voices-gf.ogg",
                "music/Bopeebo_Voices.ogg"
            ]
        );
    }

    #[test]
    fn tutorial_vslice_stems_load_as_split_sources() {
        let mixer = SharedMixer::new(Mixer::new(48_000));
        let selection = PreviewSelection::from_keys(Some("tutorial"), Some("normal"));

        load_preview_stems_for(&mixer, selection, Samples(0)).unwrap();

        mixer
            .edit(|mixer| {
                assert_eq!(mixer.voice_count(), 3);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn dadbattle_erect_stems_load_as_variant_split_sources() {
        let mixer = SharedMixer::new(Mixer::new(48_000));
        let selection = PreviewSelection::from_keys(Some("dadbattle"), Some("erect"));

        load_preview_stems_for(&mixer, selection, Samples(0)).unwrap();

        mixer
            .edit(|mixer| {
                assert_eq!(mixer.voice_count(), 3);
                Ok(())
            })
            .unwrap();
    }
}
