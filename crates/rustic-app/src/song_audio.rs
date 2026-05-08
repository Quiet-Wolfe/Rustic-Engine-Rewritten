//! App-owned loading of vanilla song stems into the shared mixer.

use crate::{asset_roots::baked_assets_root, preview_song::PreviewSelection};
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem};
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
    let inst = load_stem(&resolver, &song.inst_path())?;
    let vocals = load_stem(&resolver, &song.voices_path())?;

    mixer
        .edit(|mixer| {
            mixer.clear();
            mixer.add_source(Stem::Instrumental, inst)?;
            mixer.add_source(Stem::Vocals, vocals)?;
            mixer.seek(start_cursor)?;
            Ok(())
        })
        .with_context(|| format!("configure {} stems", song.folder))?;
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
