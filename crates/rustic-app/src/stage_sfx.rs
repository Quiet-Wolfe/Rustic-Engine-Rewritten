//! Scripted stage sound effects for vanilla stage props.

use crate::asset_roots::baked_assets_root;
use crate::preview_song::PreviewSong;
use crate::stage_scripted_motion::{
    limo_fast_car_position, limo_fast_car_start, philly_blazin_lightning_start, philly_train_start,
};
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem};
use rustic_core::time::Samples;
use std::sync::{Arc, OnceLock};

const TRAIN_SOUND_PATH: &str = "sounds/train_passes.ogg";
const CAR_PASS_0_PATH: &str = "sounds/carPass0.ogg";
const CAR_PASS_1_PATH: &str = "sounds/carPass1.ogg";
const LIGHTNING_1_PATH: &str = "sounds/Lightning1.ogg";
const LIGHTNING_2_PATH: &str = "sounds/Lightning2.ogg";
const LIGHTNING_3_PATH: &str = "sounds/Lightning3.ogg";

static TRAIN_SOUND_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static CAR_PASS_0_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static CAR_PASS_1_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static LIGHTNING_1_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static LIGHTNING_2_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static LIGHTNING_3_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();

#[derive(Debug, Default)]
pub(crate) struct StageSfx {
    last_train_start: Option<Samples>,
    last_limo_car_start: Option<Samples>,
    last_lightning_start: Option<Samples>,
}

impl StageSfx {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn tick_or_warn(
        &mut self,
        song: PreviewSong,
        mixer: &SharedMixer,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        if let Err(e) = self.tick(song, mixer, cursor, sample_rate, bpm) {
            tracing::warn!(target: "rustic.audio", "play stage sound: {e:#}");
        }
    }

    fn tick(
        &mut self,
        song: PreviewSong,
        mixer: &SharedMixer,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) -> Result<()> {
        if is_philly_train_song(song) {
            if let Some(start) = philly_train_start(cursor, sample_rate, bpm) {
                self.play_once_near_start(mixer, cursor, sample_rate, start, StageSound::Train)?;
            }
        }
        if is_limo_song(song) && limo_fast_car_position(cursor, sample_rate, bpm).is_some() {
            if let Some(start) = limo_fast_car_start(cursor, sample_rate, bpm) {
                self.play_once_near_start(
                    mixer,
                    cursor,
                    sample_rate,
                    start,
                    StageSound::CarPass(if start.0 % 2 == 0 { 0 } else { 1 }),
                )?;
            }
        }
        if song == PreviewSong::BLAZIN {
            if let Some(start) = philly_blazin_lightning_start(cursor, sample_rate) {
                self.play_once_near_start(
                    mixer,
                    cursor,
                    sample_rate,
                    start,
                    StageSound::Lightning((start.0 / i64::from(sample_rate.max(1))) as u8 % 3),
                )?;
            }
        }
        Ok(())
    }

    fn play_once_near_start(
        &mut self,
        mixer: &SharedMixer,
        cursor: Samples,
        sample_rate: u32,
        start: Samples,
        sound: StageSound,
    ) -> Result<()> {
        if !within_trigger_window(cursor, start, sample_rate)
            || self.last_start(sound) == Some(start)
        {
            return Ok(());
        }
        self.set_last_start(sound, start);
        play_stage_sound(mixer, sound)
    }

    fn last_start(&self, sound: StageSound) -> Option<Samples> {
        match sound {
            StageSound::Train => self.last_train_start,
            StageSound::CarPass(_) => self.last_limo_car_start,
            StageSound::Lightning(_) => self.last_lightning_start,
        }
    }

    fn set_last_start(&mut self, sound: StageSound, start: Samples) {
        match sound {
            StageSound::Train => self.last_train_start = Some(start),
            StageSound::CarPass(_) => self.last_limo_car_start = Some(start),
            StageSound::Lightning(_) => self.last_lightning_start = Some(start),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum StageSound {
    Train,
    CarPass(u8),
    Lightning(u8),
}

fn play_stage_sound(mixer: &SharedMixer, sound: StageSound) -> Result<()> {
    let Some(bytes) = cached_stage_sound(sound) else {
        return Ok(());
    };
    let source = streaming_vorbis_source(bytes.clone()).context("decode stage sound")?;
    mixer
        .edit(|mixer| mixer.add_source(Stem::Sfx, source))
        .context("queue stage sound")?;
    Ok(())
}

fn cached_stage_sound(sound: StageSound) -> Option<&'static Arc<[u8]>> {
    let (cache, path) = match sound {
        StageSound::Train => (&TRAIN_SOUND_BYTES, TRAIN_SOUND_PATH),
        StageSound::CarPass(0) => (&CAR_PASS_0_BYTES, CAR_PASS_0_PATH),
        StageSound::CarPass(_) => (&CAR_PASS_1_BYTES, CAR_PASS_1_PATH),
        StageSound::Lightning(0) => (&LIGHTNING_1_BYTES, LIGHTNING_1_PATH),
        StageSound::Lightning(1) => (&LIGHTNING_2_BYTES, LIGHTNING_2_PATH),
        StageSound::Lightning(_) => (&LIGHTNING_3_BYTES, LIGHTNING_3_PATH),
    };
    cache
        .get_or_init(|| match load_stage_sound(path) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!(target: "rustic.audio", "stage sound {path} unavailable: {e:#}");
                None
            }
        })
        .as_ref()
}

fn load_stage_sound(path: &str) -> Result<Arc<[u8]>> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let path = AssetPath::new(path)?;
    load_bytes(&resolver, &path).with_context(|| format!("load {}", path.as_str()))
}

fn within_trigger_window(cursor: Samples, start: Samples, sample_rate: u32) -> bool {
    let elapsed = cursor.0 - start.0;
    elapsed >= 0 && elapsed <= i64::from(sample_rate.max(1)) / 4
}

fn is_philly_train_song(song: PreviewSong) -> bool {
    matches!(
        song,
        PreviewSong::PICO | PreviewSong::PHILLY_NICE | PreviewSong::BLAMMED
    )
}

fn is_limo_song(song: PreviewSong) -> bool {
    matches!(
        song,
        PreviewSong::SATIN_PANTIES | PreviewSong::HIGH | PreviewSong::MILF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_sound_paths_match_script_assets() {
        assert_eq!(TRAIN_SOUND_PATH, "sounds/train_passes.ogg");
        assert_eq!(CAR_PASS_0_PATH, "sounds/carPass0.ogg");
        assert_eq!(LIGHTNING_3_PATH, "sounds/Lightning3.ogg");
    }

    #[test]
    fn stage_sound_trigger_windows_are_stage_specific() {
        assert!(is_philly_train_song(PreviewSong::PHILLY_NICE));
        assert!(is_limo_song(PreviewSong::MILF));
        assert!(within_trigger_window(Samples(10), Samples(0), 48_000));
        assert!(!within_trigger_window(Samples(13_000), Samples(0), 48_000));
    }
}
