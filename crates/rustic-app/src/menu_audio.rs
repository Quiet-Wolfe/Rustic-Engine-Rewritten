//! Shared menu navigation sound effects.
//!
//! ref: bdedc0aa:source/funkin/ui/MenuList.hx:145,172,208,214,290
//! ref: bdedc0aa:source/funkin/ui/title/TitleState.hx:256-260

use crate::asset_roots::app_asset_resolver;
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem};
use std::sync::{Arc, OnceLock};

static SCROLL_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static CONFIRM_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static CANCEL_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSound {
    Scroll,
    Confirm,
    TitleConfirm,
    Cancel,
}

impl MenuSound {
    fn path(self) -> &'static str {
        match self {
            Self::Scroll => "sounds/scrollMenu.ogg",
            Self::Confirm | Self::TitleConfirm => "sounds/confirmMenu.ogg",
            Self::Cancel => "sounds/cancelMenu.ogg",
        }
    }

    fn gain(self) -> f32 {
        match self {
            Self::Scroll => 0.4,
            Self::TitleConfirm => 0.7,
            Self::Confirm | Self::Cancel => 1.0,
        }
    }

    fn cache(self) -> &'static OnceLock<Option<Arc<[u8]>>> {
        match self {
            Self::Scroll => &SCROLL_BYTES,
            Self::Confirm | Self::TitleConfirm => &CONFIRM_BYTES,
            Self::Cancel => &CANCEL_BYTES,
        }
    }
}

pub fn play_menu_sound_or_warn(mixer: &SharedMixer, sound: MenuSound) {
    if let Err(e) = play_menu_sound(mixer, sound) {
        tracing::warn!(target: "rustic.audio", "play menu sound: {e:#}");
    }
}

fn play_menu_sound(mixer: &SharedMixer, sound: MenuSound) -> Result<()> {
    let Some(bytes) = menu_sound_bytes(sound) else {
        return Ok(());
    };
    let source = streaming_vorbis_source(bytes.clone())
        .with_context(|| format!("decode {}", sound.path()))?;
    mixer
        .edit(|mixer| {
            let voice = mixer.add_source(Stem::Sfx, source)?;
            mixer.set_voice_gain(voice, sound.gain());
            Ok(())
        })
        .with_context(|| format!("queue {}", sound.path()))?;
    Ok(())
}

fn menu_sound_bytes(sound: MenuSound) -> Option<&'static Arc<[u8]>> {
    sound
        .cache()
        .get_or_init(|| match load_menu_sound_bytes(sound.path()) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!(
                    target: "rustic.audio",
                    "menu sound unavailable {}: {e:#}",
                    sound.path()
                );
                None
            }
        })
        .as_ref()
}

fn load_menu_sound_bytes(path: &str) -> Result<Arc<[u8]>> {
    let resolver = app_asset_resolver();
    let path = AssetPath::new(path)?;
    load_bytes(&resolver, &path).with_context(|| format!("load {}", path.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_sound_paths_match_og_assets() {
        assert_eq!(MenuSound::Scroll.path(), "sounds/scrollMenu.ogg");
        assert_eq!(MenuSound::Confirm.path(), "sounds/confirmMenu.ogg");
        assert_eq!(MenuSound::Cancel.path(), "sounds/cancelMenu.ogg");
    }

    #[test]
    fn menu_sound_gains_match_og_call_sites() {
        assert_eq!(MenuSound::Scroll.gain(), 0.4);
        assert_eq!(MenuSound::TitleConfirm.gain(), 0.7);
        assert_eq!(MenuSound::Confirm.gain(), 1.0);
        assert_eq!(MenuSound::Cancel.gain(), 1.0);
    }
}
