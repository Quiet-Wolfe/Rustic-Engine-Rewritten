//! Game-over sound effect and death-loop music.
//!
//! ref: bdedc0aa:source/funkin/play/GameOverSubState.hx:248-263,311-315,600-614

use crate::asset_roots::baked_assets_root;
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem, VoiceId};
use std::sync::{Arc, OnceLock};

const LOSS_SOUND_PATH: &str = "sounds/gameplay/gameover/fnf_loss_sfx.ogg";
const LOOP_MUSIC_PATH: &str = "music/gameplay/gameover/gameOver.ogg";
const CONFIRM_MUSIC_PATH: &str = "music/gameplay/gameover/gameOverEnd.ogg";

static LOSS_SOUND_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static LOOP_MUSIC_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static CONFIRM_MUSIC_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();

#[derive(Debug, Default)]
pub struct GameOverAudio {
    loss_voice: Option<VoiceId>,
    loop_voice: Option<VoiceId>,
    confirm_voice: Option<VoiceId>,
}

impl GameOverAudio {
    pub fn play_loss_sfx_or_warn(&mut self, mixer: &SharedMixer) {
        if let Err(e) = self.play_loss_sfx(mixer) {
            tracing::warn!(target: "rustic.audio", "play game over loss sound: {e:#}");
        }
    }

    pub fn start_loop_music_or_warn(&mut self, mixer: &SharedMixer) {
        if let Err(e) = self.start_loop_music(mixer) {
            tracing::warn!(target: "rustic.audio", "play game over music: {e:#}");
        }
    }

    pub fn play_confirm_music_or_warn(&mut self, mixer: &SharedMixer) {
        if let Err(e) = self.play_confirm_music(mixer) {
            tracing::warn!(target: "rustic.audio", "play game over confirm music: {e:#}");
        }
    }

    pub fn stop(&mut self, mixer: &SharedMixer) {
        let voices = [
            self.loss_voice.take(),
            self.loop_voice.take(),
            self.confirm_voice.take(),
        ];
        if voices.iter().all(Option::is_none) {
            return;
        }
        if let Err(e) = mixer.edit(|mixer| {
            for voice in voices.into_iter().flatten() {
                mixer.stop_voice(voice);
            }
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "stop game over audio: {e:#}");
        }
    }

    fn play_loss_sfx(&mut self, mixer: &SharedMixer) -> Result<()> {
        let Some(bytes) = cached_bytes(&LOSS_SOUND_BYTES, LOSS_SOUND_PATH, "loss sound") else {
            return Ok(());
        };
        let source =
            streaming_vorbis_source(bytes.clone()).context("decode game over loss sound")?;
        let voice = mixer
            .edit(|mixer| mixer.add_source(Stem::Sfx, source))
            .context("queue game over loss sound")?;
        self.loss_voice = Some(voice);
        Ok(())
    }

    fn start_loop_music(&mut self, mixer: &SharedMixer) -> Result<()> {
        if self.loop_voice.is_some() {
            return Ok(());
        }
        let Some(bytes) = cached_bytes(&LOOP_MUSIC_BYTES, LOOP_MUSIC_PATH, "loop music") else {
            return Ok(());
        };
        let source = streaming_vorbis_source(bytes.clone()).context("decode game over music")?;
        let voice = mixer
            .edit(|mixer| mixer.add_looped_source(Stem::Sfx, source))
            .context("queue game over music")?;
        self.loop_voice = Some(voice);
        Ok(())
    }

    fn play_confirm_music(&mut self, mixer: &SharedMixer) -> Result<()> {
        self.stop(mixer);
        let Some(bytes) = cached_bytes(&CONFIRM_MUSIC_BYTES, CONFIRM_MUSIC_PATH, "confirm music")
        else {
            return Ok(());
        };
        let source = streaming_vorbis_source(bytes.clone()).context("decode game over confirm")?;
        let voice = mixer
            .edit(|mixer| mixer.add_source(Stem::Sfx, source))
            .context("queue game over confirm")?;
        self.confirm_voice = Some(voice);
        Ok(())
    }
}

fn cached_bytes(
    cache: &'static OnceLock<Option<Arc<[u8]>>>,
    path: &'static str,
    label: &'static str,
) -> Option<&'static Arc<[u8]>> {
    cache
        .get_or_init(|| match load_audio_bytes(path) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!(target: "rustic.audio", "game over {label} unavailable: {e:#}");
                None
            }
        })
        .as_ref()
}

fn load_audio_bytes(path: &str) -> Result<Arc<[u8]>> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let path = AssetPath::new(path)?;
    load_bytes(&resolver, &path).with_context(|| format!("load {}", path.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_over_audio_paths_match_og_assets() {
        assert_eq!(LOSS_SOUND_PATH, "sounds/gameplay/gameover/fnf_loss_sfx.ogg");
        assert_eq!(LOOP_MUSIC_PATH, "music/gameplay/gameover/gameOver.ogg");
        assert_eq!(
            CONFIRM_MUSIC_PATH,
            "music/gameplay/gameover/gameOverEnd.ogg"
        );
    }
}
