//! App-owned loading of vanilla song stems into the shared mixer.

use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem};
use rustic_core::time::Samples;

pub fn load_bopeebo_stems(mixer: &SharedMixer, start_cursor: Samples) -> Result<()> {
    let resolver = OverlayResolver::new().with_baked_root("assets/baked");
    let inst = load_stem(&resolver, "music/Bopeebo_Inst.ogg")?;
    let vocals = load_stem(&resolver, "music/Bopeebo_Voices.ogg")?;

    mixer
        .edit(|mixer| {
            mixer.clear();
            mixer.add_source(Stem::Instrumental, inst)?;
            mixer.add_source(Stem::Vocals, vocals)?;
            mixer.seek(start_cursor)?;
            Ok(())
        })
        .context("configure Bopeebo stems")?;
    Ok(())
}

fn load_stem(resolver: &OverlayResolver, path: &str) -> Result<rustic_audio::SoundSource> {
    let path = AssetPath::new(path)?;
    let bytes = load_bytes(resolver, &path).with_context(|| format!("load {}", path.as_str()))?;
    streaming_vorbis_source(bytes).with_context(|| format!("decode {}", path.as_str()))
}
