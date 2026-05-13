//! OG countdown sound scheduling.

use crate::asset_roots::baked_assets_root;
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_audio::{streaming_vorbis_source, SharedMixer, SoundSource, Stem};
use rustic_core::time::Samples;
use std::sync::Arc;

const COUNTDOWN_VOLUME: f32 = 0.6;

#[derive(Debug, Clone, Default)]
pub struct CountdownAudio {
    sounds: Option<CountdownSounds>,
    last_step: Option<CountdownStep>,
}

impl CountdownAudio {
    pub fn load_default_or_warn(audio_enabled: bool) -> Self {
        if !audio_enabled {
            return Self::default();
        }
        match Self::load_default() {
            Ok(audio) => audio,
            Err(e) => {
                tracing::warn!(target: "rustic.audio", "countdown sounds unavailable: {e:#}");
                Self::default()
            }
        }
    }

    pub fn reset(&mut self) {
        self.last_step = None;
    }

    pub fn tick_or_warn(
        &mut self,
        mixer: &SharedMixer,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        if let Err(e) = self.tick(mixer, cursor, sample_rate, bpm) {
            tracing::warn!(target: "rustic.audio", "play countdown sound: {e:#}");
        }
    }

    fn load_default() -> Result<Self> {
        let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
        Ok(Self {
            sounds: Some(CountdownSounds::load(&resolver)?),
            last_step: None,
        })
    }

    fn tick(
        &mut self,
        mixer: &SharedMixer,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) -> Result<()> {
        let Some(step) = self.next_step(cursor, sample_rate, bpm) else {
            return Ok(());
        };
        let Some(sounds) = &self.sounds else {
            return Ok(());
        };
        let source = sounds.source(step)?;
        mixer
            .edit(|mixer| {
                let id = mixer.add_source(Stem::Sfx, source)?;
                mixer.set_voice_gain(id, COUNTDOWN_VOLUME);
                Ok(())
            })
            .with_context(|| format!("queue {step:?}"))?;
        Ok(())
    }

    fn next_step(&mut self, cursor: Samples, sample_rate: u32, bpm: f64) -> Option<CountdownStep> {
        let step = countdown_step(cursor, sample_rate, bpm);
        if step == self.last_step {
            return None;
        }
        self.last_step = step;
        step
    }
}

#[derive(Debug, Clone)]
struct CountdownSounds {
    three: Arc<[u8]>,
    two: Arc<[u8]>,
    one: Arc<[u8]>,
    go: Arc<[u8]>,
}

impl CountdownSounds {
    fn load(resolver: &OverlayResolver) -> Result<Self> {
        Ok(Self {
            three: load_sound(resolver, "introTHREE")?,
            two: load_sound(resolver, "introTWO")?,
            one: load_sound(resolver, "introONE")?,
            go: load_sound(resolver, "introGO")?,
        })
    }

    fn source(&self, step: CountdownStep) -> Result<SoundSource> {
        let bytes = match step {
            CountdownStep::Three => &self.three,
            CountdownStep::Two => &self.two,
            CountdownStep::One => &self.one,
            CountdownStep::Go => &self.go,
        };
        streaming_vorbis_source(bytes.clone()).with_context(|| format!("decode {step:?}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CountdownStep {
    Three,
    Two,
    One,
    Go,
}

fn load_sound(resolver: &OverlayResolver, name: &str) -> Result<Arc<[u8]>> {
    let path = AssetPath::new(format!("sounds/gameplay/countdown/funkin/{name}.ogg"))?;
    load_bytes(resolver, &path).with_context(|| format!("load {}", path.as_str()))
}

fn countdown_step(cursor: Samples, sample_rate: u32, bpm: f64) -> Option<CountdownStep> {
    let crochet = crochet_samples(sample_rate, bpm);
    if cursor.0 < -4 * crochet || cursor.0 >= 0 {
        return None;
    }
    if cursor.0 < -3 * crochet {
        Some(CountdownStep::Three)
    } else if cursor.0 < -2 * crochet {
        Some(CountdownStep::Two)
    } else if cursor.0 < -crochet {
        Some(CountdownStep::One)
    } else {
        Some(CountdownStep::Go)
    }
}

fn crochet_samples(sample_rate: u32, bpm: f64) -> i64 {
    (f64::from(sample_rate) * 60.0 / bpm.max(1.0)).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_sound_windows_follow_og_steps() {
        assert_eq!(countdown_step(Samples(-144_000), 48_000, 100.0), None);
        assert_eq!(
            countdown_step(Samples(-115_200), 48_000, 100.0),
            Some(CountdownStep::Three)
        );
        assert_eq!(
            countdown_step(Samples(-86_400), 48_000, 100.0),
            Some(CountdownStep::Two)
        );
        assert_eq!(
            countdown_step(Samples(-57_600), 48_000, 100.0),
            Some(CountdownStep::One)
        );
        assert_eq!(
            countdown_step(Samples(-28_800), 48_000, 100.0),
            Some(CountdownStep::Go)
        );
        assert_eq!(countdown_step(Samples(0), 48_000, 100.0), None);
    }

    #[test]
    fn countdown_audio_debounces_each_step_window() {
        let mut audio = CountdownAudio::default();

        assert_eq!(
            audio.next_step(Samples(-115_200), 48_000, 100.0),
            Some(CountdownStep::Three)
        );
        assert_eq!(audio.next_step(Samples(-100_000), 48_000, 100.0), None);
        assert_eq!(
            audio.next_step(Samples(-86_400), 48_000, 100.0),
            Some(CountdownStep::Two)
        );
    }
}
