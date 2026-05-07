//! App-owned loading of vanilla song stems into the shared mixer.

use crate::preview_song::PreviewSong;
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem};
use rustic_core::time::Samples;

pub fn load_preview_stems(mixer: &SharedMixer, start_cursor: Samples) -> Result<()> {
    let resolver = OverlayResolver::new().with_baked_root("assets/baked");
    let song = PreviewSong::from_env();
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

fn load_stem(resolver: &OverlayResolver, path: &str) -> Result<rustic_audio::SoundSource> {
    let path = AssetPath::new(path)?;
    let bytes = load_bytes(resolver, &path).with_context(|| format!("load {}", path.as_str()))?;
    streaming_vorbis_source(bytes).with_context(|| format!("decode {}", path.as_str()))
}
