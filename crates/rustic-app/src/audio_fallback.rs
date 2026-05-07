//! Bounded audio-device open with a wall-clock fallback path.

use crate::scene_assets::SAMPLE_RATE;
use rustic_audio::{AudioOutput, Mixer, SharedMixer};
use std::env;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const AUDIO_OPEN_TIMEOUT_MS: u64 = 750;
const AUDIO_OPEN_TIMEOUT_ENV: &str = "RUSTIC_AUDIO_OPEN_TIMEOUT_MS";
const AUDIO_DISABLE_ENV: &str = "RUSTIC_AUDIO";

pub fn open_audio_output_or_fallback() -> (Option<AudioOutput>, SharedMixer) {
    if audio_disabled() {
        tracing::warn!(
            target: "rustic.audio",
            "{AUDIO_DISABLE_ENV}=off, using wall-clock preview cursor"
        );
        return fallback_audio();
    }

    let timeout = audio_open_timeout(env::var(AUDIO_OPEN_TIMEOUT_ENV).ok().as_deref());
    let (tx, rx) = mpsc::channel();
    let spawn = thread::Builder::new()
        .name("rustic.audio.open".to_string())
        .spawn(move || {
            let _ = tx.send(AudioOutput::open_default());
        });

    if let Err(e) = spawn {
        tracing::warn!(
            target: "rustic.audio",
            "audio output thread unavailable, using wall-clock preview cursor: {e}"
        );
        return fallback_audio();
    }

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
            tracing::info!(
                target: "rustic.audio",
                sample_rate = output.sample_rate(),
                channels = output.channels(),
                sample_format = ?output.sample_format(),
                "opened default output stream"
            );
            let mixer = output.mixer().clone();
            (Some(output), mixer)
        }
        Ok(Err(e)) => {
            tracing::warn!(
                target: "rustic.audio",
                "audio output unavailable, using wall-clock preview cursor: {e:#}"
            );
            fallback_audio()
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            tracing::warn!(
                target: "rustic.audio",
                timeout_ms = timeout.as_millis(),
                "audio output open timed out, using wall-clock preview cursor"
            );
            fallback_audio()
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            tracing::warn!(
                target: "rustic.audio",
                "audio output open thread exited, using wall-clock preview cursor"
            );
            fallback_audio()
        }
    }
}

fn fallback_audio() -> (Option<AudioOutput>, SharedMixer) {
    (None, SharedMixer::new(Mixer::new(SAMPLE_RATE)))
}

fn audio_disabled() -> bool {
    audio_disabled_value(env::var(AUDIO_DISABLE_ENV).ok().as_deref())
}

fn audio_disabled_value(value: Option<&str>) -> bool {
    value
        .map(|value| matches!(value.trim(), "0" | "off" | "false" | "none"))
        .unwrap_or(false)
}

fn audio_open_timeout(value: Option<&str>) -> Duration {
    let millis = value
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .unwrap_or(AUDIO_OPEN_TIMEOUT_MS);
    Duration::from_millis(millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_open_timeout_uses_env_milliseconds() {
        assert_eq!(audio_open_timeout(Some("125")), Duration::from_millis(125));
    }

    #[test]
    fn audio_open_timeout_rejects_zero_and_invalid_values() {
        assert_eq!(
            audio_open_timeout(Some("0")),
            Duration::from_millis(AUDIO_OPEN_TIMEOUT_MS)
        );
        assert_eq!(
            audio_open_timeout(Some("nope")),
            Duration::from_millis(AUDIO_OPEN_TIMEOUT_MS)
        );
    }

    #[test]
    fn audio_disable_env_accepts_off_values() {
        assert!(audio_disabled_value(Some("off")));
        assert!(audio_disabled_value(Some("0")));
        assert!(audio_disabled_value(Some("false")));
        assert!(audio_disabled_value(Some("none")));
        assert!(!audio_disabled_value(Some("on")));
        assert!(!audio_disabled_value(None));
    }
}
