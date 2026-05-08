//! App-owned loading of vanilla song stems into the shared mixer.

use crate::{asset_roots::baked_assets_root, preview_song::PreviewSelection};
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
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
    let inst = load_stem(&resolver, &song.inst_path())?;
    let vocals = match load_stem(&resolver, &song.voices_path()) {
        Ok(vocals) => Some(vocals),
        Err(e) => {
            tracing::warn!(target: "rustic.audio", "optional vocals unavailable: {e:#}");
            None
        }
    };
    configure_song_stems(mixer, inst, vocals, start_cursor, song.folder)
}

fn configure_song_stems(
    mixer: &SharedMixer,
    inst: SoundSource,
    vocals: Option<SoundSource>,
    start_cursor: Samples,
    song_id: &str,
) -> Result<()> {
    mixer
        .edit(|mixer| {
            mixer.clear();
            mixer.add_source(Stem::Instrumental, inst)?;
            if let Some(vocals) = vocals {
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
            None,
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
}
