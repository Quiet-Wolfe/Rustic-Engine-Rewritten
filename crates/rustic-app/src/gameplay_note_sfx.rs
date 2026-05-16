//! Scripted note-kind sound effects from vanilla Weekend 1 character scripts.

use crate::asset_roots::app_asset_resolver;
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem};
use rustic_core::time::Samples;
use rustic_game::NoteKind;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

static GUN_PREP_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static PICO_BONK_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static SHOT1_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static SHOT2_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static SHOT3_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static SHOT4_BYTES: OnceLock<Option<Arc<[u8]>>> = OnceLock::new();
static SHOT_ROLL: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameplayNoteSfx {
    GunPrep,
    PicoBonk,
    Shot(usize),
}

impl GameplayNoteSfx {
    fn path(self) -> &'static str {
        match self {
            Self::GunPrep => "sounds/Gun_Prep.ogg",
            Self::PicoBonk => "sounds/Pico_Bonk.ogg",
            Self::Shot(0) => "sounds/shot1.ogg",
            Self::Shot(1) => "sounds/shot2.ogg",
            Self::Shot(2) => "sounds/shot3.ogg",
            Self::Shot(_) => "sounds/shot4.ogg",
        }
    }

    fn cache(self) -> &'static OnceLock<Option<Arc<[u8]>>> {
        match self {
            Self::GunPrep => &GUN_PREP_BYTES,
            Self::PicoBonk => &PICO_BONK_BYTES,
            Self::Shot(0) => &SHOT1_BYTES,
            Self::Shot(1) => &SHOT2_BYTES,
            Self::Shot(2) => &SHOT3_BYTES,
            Self::Shot(_) => &SHOT4_BYTES,
        }
    }
}

pub(crate) fn play_note_kind_hit_sfx_or_warn(mixer: &SharedMixer, cursor: Samples, kind: NoteKind) {
    if let Err(e) = play_note_kind_hit_sfx(mixer, cursor, kind) {
        tracing::warn!(target: "rustic.audio", "play note-kind hit sound: {e:#}");
    }
}

pub(crate) fn play_note_kind_miss_sfx_or_warn(mixer: &SharedMixer, kind: NoteKind) {
    if let Err(e) = play_note_kind_miss_sfx(mixer, kind) {
        tracing::warn!(target: "rustic.audio", "play note-kind miss sound: {e:#}");
    }
}

fn play_note_kind_hit_sfx(mixer: &SharedMixer, cursor: Samples, kind: NoteKind) -> Result<()> {
    let Some(sound) = hit_sfx_for_kind(kind, cursor) else {
        return Ok(());
    };
    play_sfx(mixer, sound)
}

fn play_note_kind_miss_sfx(mixer: &SharedMixer, kind: NoteKind) -> Result<()> {
    let Some(sound) = miss_sfx_for_kind(kind) else {
        return Ok(());
    };
    play_sfx(mixer, sound)
}

fn hit_sfx_for_kind(kind: NoteKind, cursor: Samples) -> Option<GameplayNoteSfx> {
    match kind {
        // ref: bdedc0aa:assets/preload/scripts/characters/pico-playable.hxc:380-396
        NoteKind::Weekend1CockGun => Some(GameplayNoteSfx::GunPrep),
        // ref: bdedc0aa:assets/preload/scripts/characters/pico-playable.hxc:399-405
        NoteKind::Weekend1FireGun => Some(GameplayNoteSfx::Shot(shot_index(cursor))),
        _ => None,
    }
}

fn miss_sfx_for_kind(kind: NoteKind) -> Option<GameplayNoteSfx> {
    match kind {
        // ref: bdedc0aa:assets/preload/scripts/characters/pico-playable.hxc:407-415
        NoteKind::Weekend1FireGun => Some(GameplayNoteSfx::PicoBonk),
        _ => None,
    }
}

fn play_sfx(mixer: &SharedMixer, sound: GameplayNoteSfx) -> Result<()> {
    let Some(bytes) = sound_bytes(sound) else {
        return Ok(());
    };
    let source = streaming_vorbis_source(bytes.clone())
        .with_context(|| format!("decode {}", sound.path()))?;
    mixer
        .edit(|mixer| {
            mixer.add_source(Stem::Sfx, source)?;
            Ok(())
        })
        .with_context(|| format!("queue {}", sound.path()))?;
    Ok(())
}

fn sound_bytes(sound: GameplayNoteSfx) -> Option<&'static Arc<[u8]>> {
    sound
        .cache()
        .get_or_init(|| match load_sound_bytes(sound.path()) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!(
                    target: "rustic.audio",
                    "note-kind sound unavailable {}: {e:#}",
                    sound.path()
                );
                None
            }
        })
        .as_ref()
}

fn load_sound_bytes(path: &str) -> Result<Arc<[u8]>> {
    let resolver = app_asset_resolver();
    let path = AssetPath::new(path)?;
    load_bytes(&resolver, &path).with_context(|| format!("load {}", path.as_str()))
}

fn shot_index(cursor: Samples) -> usize {
    let roll = SHOT_ROLL
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add((cursor.0 as u64).rotate_left(17));
    (roll % 4) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekend1_gun_sound_paths_match_pico_script_assets() {
        assert_eq!(GameplayNoteSfx::GunPrep.path(), "sounds/Gun_Prep.ogg");
        assert_eq!(GameplayNoteSfx::PicoBonk.path(), "sounds/Pico_Bonk.ogg");
        assert_eq!(GameplayNoteSfx::Shot(0).path(), "sounds/shot1.ogg");
        assert_eq!(GameplayNoteSfx::Shot(3).path(), "sounds/shot4.ogg");
    }

    #[test]
    fn weekend1_firegun_hit_and_miss_choose_scripted_sounds() {
        assert!(matches!(
            hit_sfx_for_kind(NoteKind::Weekend1FireGun, Samples(0)),
            Some(GameplayNoteSfx::Shot(_))
        ));
        assert_eq!(
            miss_sfx_for_kind(NoteKind::Weekend1FireGun),
            Some(GameplayNoteSfx::PicoBonk)
        );
        assert_eq!(
            hit_sfx_for_kind(NoteKind::Weekend1CockGun, Samples(0)),
            Some(GameplayNoteSfx::GunPrep)
        );
    }
}
