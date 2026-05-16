//! Looping title/menu music.
//!
//! ref: bdedc0aa:source/funkin/ui/title/TitleState.hx:336-346

use crate::asset_roots::app_asset_resolver;
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem, VoiceId};
use std::sync::{Arc, OnceLock};

const MENU_MUSIC_PATH: &str = "music/freakyMenu/freakyMenu.ogg";

static MENU_MUSIC_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();

#[derive(Debug, Default)]
pub struct MenuMusic {
    voice: Option<VoiceId>,
}

impl MenuMusic {
    pub fn start_or_warn(&mut self, mixer: &SharedMixer) {
        if let Err(e) = self.start(mixer) {
            tracing::warn!(target: "rustic.audio", "play menu music: {e:#}");
        }
    }

    pub fn stop(&mut self, mixer: &SharedMixer) {
        let Some(voice) = self.voice.take() else {
            return;
        };
        if let Err(e) = mixer.edit(|mixer| {
            mixer.stop_voice(voice);
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "stop menu music: {e:#}");
        }
    }

    fn start(&mut self, mixer: &SharedMixer) -> Result<()> {
        if self.voice.is_some() {
            return Ok(());
        }
        let Some(bytes) = menu_music_bytes() else {
            return Ok(());
        };
        let source = streaming_vorbis_source(bytes.clone()).context("decode menu music")?;
        let voice = mixer
            .edit(|mixer| mixer.add_looped_source(Stem::Sfx, source))
            .context("queue menu music")?;
        self.voice = Some(voice);
        Ok(())
    }
}

fn menu_music_bytes() -> Option<&'static Arc<[u8]>> {
    MENU_MUSIC_BYTES
        .get_or_init(|| match load_menu_music_bytes() {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!(target: "rustic.audio", "menu music unavailable: {e:#}");
                None
            }
        })
        .as_ref()
}

fn load_menu_music_bytes() -> Result<Arc<[u8]>> {
    let resolver = app_asset_resolver();
    let path = AssetPath::new(MENU_MUSIC_PATH)?;
    load_bytes(&resolver, &path).with_context(|| format!("load {}", path.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_music_path_matches_og_freaky_menu_asset() {
        assert_eq!(MENU_MUSIC_PATH, "music/freakyMenu/freakyMenu.ogg");
    }
}
