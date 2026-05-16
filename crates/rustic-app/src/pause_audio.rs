//! Pause-menu music playback.
//!
//! ref: bdedc0aa:source/funkin/play/PauseSubState.hx:105-112,351-363

use crate::asset_roots::app_asset_resolver;
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem, VoiceId};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

const PAUSE_MUSIC_PATH: &str = "music/breakfast/breakfast.ogg";
const MUSIC_FADE_IN_TIME: Duration = Duration::from_secs(5);
const MUSIC_FINAL_VOLUME: f32 = 0.75;

static PAUSE_MUSIC_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();

#[derive(Debug, Default)]
pub struct PauseMusic {
    voice: Option<VoiceId>,
    started_at: Option<Instant>,
}

impl PauseMusic {
    pub fn start_or_warn(&mut self, mixer: &SharedMixer) {
        if let Err(e) = self.start(mixer) {
            tracing::warn!(target: "rustic.audio", "play pause music: {e:#}");
        }
    }

    pub fn stop(&mut self, mixer: &SharedMixer) {
        let Some(voice) = self.voice.take() else {
            self.started_at = None;
            return;
        };
        if let Err(e) = mixer.edit(|mixer| {
            mixer.stop_voice(voice);
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "stop pause music: {e:#}");
        }
        self.started_at = None;
    }

    pub fn update_gain(&self, mixer: &SharedMixer) {
        let (Some(voice), Some(started_at)) = (self.voice, self.started_at) else {
            return;
        };
        let gain = pause_music_gain(started_at.elapsed());
        if let Err(e) = mixer.edit(|mixer| {
            mixer.set_voice_gain(voice, gain);
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "fade pause music: {e:#}");
        }
    }

    fn start(&mut self, mixer: &SharedMixer) -> Result<()> {
        if self.voice.is_some() {
            self.update_gain(mixer);
            return Ok(());
        }
        let Some(bytes) = pause_music_bytes() else {
            return Ok(());
        };
        let source = streaming_vorbis_source(bytes.clone()).context("decode pause music")?;
        let started_at = Instant::now();
        let voice = mixer
            .edit(|mixer| {
                let voice = mixer.add_looped_source(Stem::Sfx, source)?;
                mixer.set_voice_gain(voice, pause_music_gain(Duration::ZERO));
                Ok(voice)
            })
            .context("queue pause music")?;
        self.voice = Some(voice);
        self.started_at = Some(started_at);
        Ok(())
    }
}

fn pause_music_bytes() -> Option<&'static Arc<[u8]>> {
    PAUSE_MUSIC_BYTES
        .get_or_init(|| match load_pause_music_bytes() {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!(target: "rustic.audio", "pause music unavailable: {e:#}");
                None
            }
        })
        .as_ref()
}

fn load_pause_music_bytes() -> Result<Arc<[u8]>> {
    let resolver = app_asset_resolver();
    let path = AssetPath::new(PAUSE_MUSIC_PATH)?;
    load_bytes(&resolver, &path).with_context(|| format!("load {}", path.as_str()))
}

fn pause_music_gain(elapsed: Duration) -> f32 {
    let fade = MUSIC_FADE_IN_TIME.as_secs_f32();
    if fade <= 0.0 {
        return MUSIC_FINAL_VOLUME;
    }
    let t = (elapsed.as_secs_f32() / fade).clamp(0.0, 1.0);
    MUSIC_FINAL_VOLUME * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_music_path_matches_og_breakfast_asset() {
        assert_eq!(PAUSE_MUSIC_PATH, "music/breakfast/breakfast.ogg");
    }

    #[test]
    fn pause_music_fades_to_og_final_volume() {
        assert_eq!(pause_music_gain(Duration::ZERO), 0.0);
        assert!((pause_music_gain(Duration::from_millis(2_500)) - 0.375).abs() < 0.001);
        assert_eq!(pause_music_gain(Duration::from_secs(5)), 0.75);
        assert_eq!(pause_music_gain(Duration::from_secs(6)), 0.75);
    }
}
