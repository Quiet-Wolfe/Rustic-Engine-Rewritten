//! Freeplay selected-song preview music.
//!
//! ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:2966-3038

use crate::asset_roots::app_asset_resolver;
use crate::preview_song::PreviewSelection;
use anyhow::{anyhow, Context, Result};
use rustic_asset::{load_bytes, AssetPath};
use rustic_audio::{streaming_vorbis_source, SharedMixer, Stem, VoiceId};
use std::sync::Arc;
use std::time::{Duration, Instant};

const PREVIEW_FINAL_VOLUME: f32 = 0.7;
const PREVIEW_FADE_IN: Duration = Duration::from_secs(2);
const PREVIEW_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug, Default)]
pub struct FreeplayPreviewMusic {
    voice: Option<VoiceId>,
    target: Option<PreviewTarget>,
    queued: Option<QueuedPreview>,
    fade_started: Option<Instant>,
}

impl FreeplayPreviewMusic {
    pub fn start_selection_or_warn(&mut self, mixer: &SharedMixer, selection: PreviewSelection) {
        self.queue(mixer, PreviewTarget::Song(selection));
    }

    pub fn start_random_or_warn(&mut self, mixer: &SharedMixer) {
        self.queue(mixer, PreviewTarget::Random);
    }

    pub fn update(&mut self, mixer: &SharedMixer) {
        if let Some(queued) = self.queued {
            if preview_delay_elapsed(queued.queued_at.elapsed()) {
                self.queued = None;
                if let Err(e) = self.start_now(mixer, queued.target) {
                    tracing::warn!(target: "rustic.audio", "play freeplay preview: {e:#}");
                }
            }
        }
        let (Some(voice), Some(started)) = (self.voice, self.fade_started) else {
            return;
        };
        let gain = preview_fade_gain(started.elapsed());
        if gain >= PREVIEW_FINAL_VOLUME {
            self.fade_started = None;
        }
        if let Err(e) = mixer.edit(|mixer| {
            mixer.set_voice_gain(voice, gain);
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "fade freeplay preview: {e:#}");
        }
    }

    pub fn stop(&mut self, mixer: &SharedMixer) {
        self.queued = None;
        self.target = None;
        self.fade_started = None;
        let Some(voice) = self.voice.take() else {
            return;
        };
        if let Err(e) = mixer.edit(|mixer| {
            mixer.stop_voice(voice);
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "stop freeplay preview: {e:#}");
        }
    }

    fn queue(&mut self, mixer: &SharedMixer, target: PreviewTarget) {
        if self.target == Some(target) || self.queued.is_some_and(|queued| queued.target == target)
        {
            return;
        }
        self.stop(mixer);
        self.queued = Some(QueuedPreview {
            target,
            queued_at: Instant::now(),
        });
    }

    fn start_now(&mut self, mixer: &SharedMixer, target: PreviewTarget) -> Result<()> {
        let bytes = load_first_preview_bytes(target)?;
        let source = streaming_vorbis_source(bytes).context("decode freeplay preview")?;
        let voice = mixer
            .edit(|mixer| {
                let voice = mixer.add_looped_source(Stem::Sfx, source)?;
                mixer.set_voice_gain(voice, 0.0);
                Ok(voice)
            })
            .context("queue freeplay preview")?;
        self.voice = Some(voice);
        self.target = Some(target);
        self.fade_started = Some(Instant::now());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewTarget {
    Random,
    Song(PreviewSelection),
}

#[derive(Debug, Clone, Copy)]
struct QueuedPreview {
    target: PreviewTarget,
    queued_at: Instant,
}

fn load_first_preview_bytes(target: PreviewTarget) -> Result<Arc<[u8]>> {
    let resolver = app_asset_resolver();
    let mut errors = Vec::new();
    for path in preview_paths(target) {
        let asset = AssetPath::new(path.clone())?;
        match load_bytes(&resolver, &asset) {
            Ok(bytes) => return Ok(bytes),
            Err(e) => errors.push(format!("{path}: {e:#}")),
        }
    }
    Err(anyhow!(
        "no usable freeplay preview in [{}]",
        errors.join("; ")
    ))
}

fn preview_paths(target: PreviewTarget) -> Vec<String> {
    let PreviewTarget::Song(selection) = target else {
        return vec!["music/freeplayRandom/freeplayRandom.ogg".to_string()];
    };
    let mut paths = Vec::new();
    if let Some(suffix) = selection.effective_variation_suffix() {
        paths.push(format!("songs/{}/Inst-{suffix}.ogg", selection.song.folder));
    }
    paths.push(format!("songs/{}/Inst.ogg", selection.song.folder));
    paths.push(selection.song.inst_path());
    paths
}

fn preview_fade_gain(elapsed: Duration) -> f32 {
    let progress = elapsed.as_secs_f32() / PREVIEW_FADE_IN.as_secs_f32();
    PREVIEW_FINAL_VOLUME * progress.clamp(0.0, 1.0)
}

fn preview_delay_elapsed(elapsed: Duration) -> bool {
    elapsed >= PREVIEW_DELAY
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_song::{PreviewDifficulty, PreviewSong};

    #[test]
    fn erect_preview_prefers_variant_inst_then_base_then_legacy() {
        let paths = preview_paths(PreviewTarget::Song(PreviewSelection::new(
            PreviewSong::BOPEEBO,
            PreviewDifficulty::Erect,
        )));

        assert_eq!(
            paths,
            vec![
                "songs/bopeebo/Inst-erect.ogg",
                "songs/bopeebo/Inst.ogg",
                "music/Bopeebo_Inst.ogg"
            ]
        );
    }

    #[test]
    fn pico_preview_prefers_character_variant_inst() {
        let paths = preview_paths(PreviewTarget::Song(
            PreviewSelection::new(PreviewSong::BOPEEBO, PreviewDifficulty::Hard)
                .with_variation(Some(crate::preview_song::VARIATION_PICO)),
        ));

        assert_eq!(
            paths,
            vec![
                "songs/bopeebo/Inst-pico.ogg",
                "songs/bopeebo/Inst.ogg",
                "music/Bopeebo_Inst.ogg"
            ]
        );
    }

    #[test]
    fn random_preview_uses_og_freeplay_random_track() {
        assert_eq!(
            preview_paths(PreviewTarget::Random),
            vec!["music/freeplayRandom/freeplayRandom.ogg"]
        );
    }

    #[test]
    fn preview_fade_reaches_og_target_volume() {
        assert_eq!(preview_fade_gain(Duration::ZERO), 0.0);
        assert!((preview_fade_gain(Duration::from_secs(1)) - 0.35).abs() < 0.001);
        assert!((preview_fade_gain(Duration::from_secs(3)) - 0.7).abs() < 0.001);
    }

    #[test]
    fn preview_waits_before_loading_like_og_timer() {
        assert!(!preview_delay_elapsed(Duration::from_millis(249)));
        assert!(preview_delay_elapsed(Duration::from_millis(250)));
    }
}
