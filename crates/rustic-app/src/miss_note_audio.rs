//! OG miss-note sound playback.
//!
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:2941
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:3156
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:3197

use crate::asset_roots::app_asset_resolver;
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_audio::{streaming_vorbis_source, SharedMixer, SoundSource, Stem};
use rustic_core::time::Samples;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

static MISS_NOTE_SOUNDS: OnceLock<Option<MissNoteSounds>> = OnceLock::new();
static MISS_NOTE_ROLL: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissNoteKind {
    Scoreable,
    Ghost,
}

impl MissNoteKind {
    fn gain_range(self) -> (f32, f32) {
        match self {
            Self::Scoreable => (0.5, 0.6),
            Self::Ghost => (0.1, 0.2),
        }
    }
}

pub fn play_miss_note_or_warn(mixer: &SharedMixer, cursor: Samples, kind: MissNoteKind) {
    if let Err(e) = play_miss_note(mixer, cursor, kind) {
        tracing::warn!(target: "rustic.audio", "play miss note sound: {e:#}");
    }
}

fn play_miss_note(mixer: &SharedMixer, cursor: Samples, kind: MissNoteKind) -> Result<()> {
    let Some(sounds) = miss_note_sounds() else {
        return Ok(());
    };
    let roll = miss_note_roll(cursor);
    let source = sounds.source(roll)?;
    let gain = miss_note_gain(kind, roll);
    mixer
        .edit(|mixer| {
            let id = mixer.add_source(Stem::Sfx, source)?;
            mixer.set_voice_gain(id, gain);
            Ok(())
        })
        .context("queue miss note sound")?;
    Ok(())
}

fn miss_note_sounds() -> Option<&'static MissNoteSounds> {
    MISS_NOTE_SOUNDS
        .get_or_init(|| match MissNoteSounds::load_default() {
            Ok(sounds) => Some(sounds),
            Err(e) => {
                tracing::warn!(target: "rustic.audio", "miss note sounds unavailable: {e:#}");
                None
            }
        })
        .as_ref()
}

#[derive(Debug, Clone)]
struct MissNoteSounds {
    variants: [Arc<[u8]>; 3],
}

impl MissNoteSounds {
    fn load_default() -> Result<Self> {
        let resolver = app_asset_resolver();
        Ok(Self {
            variants: [
                load_sound(&resolver, 0)?,
                load_sound(&resolver, 1)?,
                load_sound(&resolver, 2)?,
            ],
        })
    }

    fn source(&self, roll: u64) -> Result<SoundSource> {
        let index = miss_note_index(roll);
        streaming_vorbis_source(self.variants[index].clone())
            .with_context(|| format!("decode missnote{}", index + 1))
    }
}

fn load_sound(resolver: &OverlayResolver, index: usize) -> Result<Arc<[u8]>> {
    let path = miss_note_path(index)?;
    load_bytes(resolver, &path).with_context(|| format!("load {}", path.as_str()))
}

fn miss_note_path(index: usize) -> Result<AssetPath> {
    AssetPath::new(format!("sounds/missnote{}.ogg", index + 1)).context("miss note sound path")
}

fn miss_note_roll(cursor: Samples) -> u64 {
    let cursor_bits = cursor.0 as u64;
    MISS_NOTE_ROLL
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(cursor_bits.rotate_left(13))
}

fn miss_note_index(roll: u64) -> usize {
    (roll % 3) as usize
}

fn miss_note_gain(kind: MissNoteKind, roll: u64) -> f32 {
    let (min, max) = kind.gain_range();
    let unit = (roll % 101) as f32 / 100.0;
    min + (max - min) * unit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miss_note_paths_match_og_sound_random_names() {
        assert_eq!(miss_note_path(0).unwrap().as_str(), "sounds/missnote1.ogg");
        assert_eq!(miss_note_path(1).unwrap().as_str(), "sounds/missnote2.ogg");
        assert_eq!(miss_note_path(2).unwrap().as_str(), "sounds/missnote3.ogg");
    }

    #[test]
    fn miss_note_index_uses_three_variants() {
        assert_eq!(miss_note_index(0), 0);
        assert_eq!(miss_note_index(1), 1);
        assert_eq!(miss_note_index(2), 2);
        assert_eq!(miss_note_index(3), 0);
    }

    #[test]
    fn miss_note_gain_matches_og_ranges() {
        for roll in [0, 1, 53, 5_300] {
            let note = miss_note_gain(MissNoteKind::Scoreable, roll);
            let ghost = miss_note_gain(MissNoteKind::Ghost, roll);
            assert!((0.5..=0.6).contains(&note));
            assert!((0.1..=0.2).contains(&ghost));
        }
    }
}
