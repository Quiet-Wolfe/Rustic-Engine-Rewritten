//! SRT-backed gameplay subtitles for vanilla cutscene/lyric data.
//!
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:2031-2043,2616-2623
//! ref: bdedc0aa:source/funkin/play/components/Subtitles.hx:25-125
//! ref: bdedc0aa:source/funkin/util/SRTUtil.hx:38-93

use crate::asset_roots::app_asset_resolver;
use crate::preview_song::{PreviewSelection, PreviewSong, VARIATION_PICO};
use anyhow::{anyhow, Context, Result};
use rustic_asset::{load_bytes, AssetPath};
use rustic_core::time::Samples;
use rustic_render::{TextCommand, TextCommandList};

#[derive(Debug, Clone)]
pub(crate) struct SubtitleTrack {
    cues: Vec<SubtitleCue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SubtitleCue {
    start_ms: i64,
    end_ms: i64,
    text: String,
}

impl SubtitleTrack {
    pub(crate) fn load_for_selection(selection: PreviewSelection) -> Result<Option<Self>> {
        let Some(path) = subtitle_path_for_selection(selection) else {
            return Ok(None);
        };
        Ok(Some(Self::load_path(&path)?))
    }

    pub(crate) fn load_stress_pico_end_cutscene() -> Result<Self> {
        Self::load_path(stress_pico_end_cutscene_subtitle_path())
    }

    fn load_path(path: &str) -> Result<Self> {
        let resolver = app_asset_resolver();
        let path = AssetPath::new(path)?;
        let bytes = load_bytes(&resolver, &path).with_context(|| format!("load {path}"))?;
        let cues = parse_srt(&bytes).with_context(|| format!("parse {path}"))?;
        Ok(Self { cues })
    }

    pub(crate) fn append_commands(
        &self,
        commands: &mut TextCommandList,
        cursor: Samples,
        sample_rate: u32,
    ) {
        let Some(text) = self.active_text(cursor, sample_rate) else {
            return;
        };
        push_subtitle(commands, text);
    }

    fn active_text(&self, cursor: Samples, sample_rate: u32) -> Option<&str> {
        let cursor_ms = cursor.0.saturating_mul(1_000) / i64::from(sample_rate.max(1));
        self.cues
            .iter()
            .find(|cue| cursor_ms >= cue.start_ms && cursor_ms < cue.end_ms)
            .map(|cue| cue.text.as_str())
    }
}

fn subtitle_path_for_selection(selection: PreviewSelection) -> Option<String> {
    if selection.song != PreviewSong::STRESS {
        return None;
    }
    let file = match selection.effective_variation_suffix() {
        Some(VARIATION_PICO) => "song-lyrics-pico.srt",
        _ => "song-lyrics.srt",
    };
    Some(format!(
        "data/songs/{}/subtitles/{file}",
        selection.song.folder
    ))
}

fn stress_pico_end_cutscene_subtitle_path() -> &'static str {
    "data/songs/stress/subtitles/end-cutscene-pico.srt"
}

fn parse_srt(bytes: &[u8]) -> Result<Vec<SubtitleCue>> {
    let text = std::str::from_utf8(bytes)?.replace("\r\n", "\n");
    let text = text.trim_start_matches('\u{feff}');
    text.split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .map(parse_srt_block)
        .collect()
}

fn parse_srt_block(block: &str) -> Result<SubtitleCue> {
    let lines: Vec<_> = block
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let timing_index = lines
        .iter()
        .position(|line| line.contains("-->"))
        .ok_or_else(|| anyhow!("SRT cue missing timing line"))?;
    let timing = lines[timing_index];
    let (start, end) = timing
        .split_once("-->")
        .ok_or_else(|| anyhow!("SRT cue has invalid timing line"))?;
    let start_ms = parse_timestamp_ms(start.trim())?;
    let end_ms = parse_timestamp_ms(end.trim())?;
    let text = lines
        .iter()
        .skip(timing_index + 1)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        return Err(anyhow!("SRT cue missing text"));
    }
    Ok(SubtitleCue {
        start_ms,
        end_ms,
        text,
    })
}

fn parse_timestamp_ms(value: &str) -> Result<i64> {
    let (hours, rest) = value
        .split_once(':')
        .ok_or_else(|| anyhow!("invalid SRT timestamp hours"))?;
    let (minutes, rest) = rest
        .split_once(':')
        .ok_or_else(|| anyhow!("invalid SRT timestamp minutes"))?;
    let (seconds, millis) = rest
        .split_once(',')
        .or_else(|| rest.split_once('.'))
        .ok_or_else(|| anyhow!("invalid SRT timestamp millis"))?;
    let hours: i64 = hours.parse()?;
    let minutes: i64 = minutes.parse()?;
    let seconds: i64 = seconds.parse()?;
    let millis: i64 = millis.parse()?;
    Ok((((hours * 60) + minutes) * 60 + seconds) * 1_000 + millis)
}

fn push_subtitle(commands: &mut TextCommandList, text: &str) {
    let mut shadow = TextCommand::new(text, glam::vec2(112.0, 614.0), 32.0);
    shadow.max_width = Some(1_056.0);
    shadow.color = glam::vec4(0.0, 0.0, 0.0, 0.85);
    shadow.z = 980;
    commands.push(shadow);

    let mut subtitle = TextCommand::new(text, glam::vec2(110.0, 612.0), 32.0);
    subtitle.max_width = Some(1_056.0);
    subtitle.color = glam::vec4(1.0, 1.0, 1.0, 0.96);
    subtitle.z = 981;
    commands.push(subtitle);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_song::{PreviewDifficulty, VARIATION_BF};

    #[test]
    fn parses_srt_cue_times_and_text() {
        let cues = parse_srt(
            b"\xef\xbb\xbf1\n00:01:02,000 --> 00:01:03,360\nHeh.\n\n2\n00:01:03,760 --> 00:01:04,710\nPretty good!\n",
        )
        .unwrap();

        assert_eq!(
            cues[0],
            SubtitleCue {
                start_ms: 62_000,
                end_ms: 63_360,
                text: "Heh.".to_string(),
            }
        );
        assert_eq!(cues[1].text, "Pretty good!");
    }

    #[test]
    fn active_text_uses_half_open_cue_window() {
        let track = SubtitleTrack {
            cues: vec![SubtitleCue {
                start_ms: 1_000,
                end_ms: 2_000,
                text: "line".to_string(),
            }],
        };

        assert_eq!(track.active_text(Samples(47_999), 48_000), None);
        assert_eq!(track.active_text(Samples(48_000), 48_000), Some("line"));
        assert_eq!(track.active_text(Samples(96_000), 48_000), None);
    }

    #[test]
    fn stress_uses_pico_subtitle_variant_when_selected() {
        let pico = PreviewSelection::new(PreviewSong::STRESS, PreviewDifficulty::Hard)
            .with_variation(Some(VARIATION_PICO));
        let bf = PreviewSelection::new(PreviewSong::STRESS, PreviewDifficulty::Hard)
            .with_variation(Some(VARIATION_BF));

        assert_eq!(
            subtitle_path_for_selection(pico).as_deref(),
            Some("data/songs/stress/subtitles/song-lyrics-pico.srt")
        );
        assert_eq!(
            subtitle_path_for_selection(bf).as_deref(),
            Some("data/songs/stress/subtitles/song-lyrics.srt")
        );
        assert_eq!(
            subtitle_path_for_selection(PreviewSelection::new(
                PreviewSong::BOPEEBO,
                PreviewDifficulty::Normal
            )),
            None
        );
    }

    #[test]
    fn stress_pico_end_cutscene_uses_upstream_srt_path() {
        assert_eq!(
            stress_pico_end_cutscene_subtitle_path(),
            "data/songs/stress/subtitles/end-cutscene-pico.srt"
        );
    }

    #[test]
    fn subtitle_commands_include_shadow_and_foreground() {
        let track = SubtitleTrack {
            cues: vec![SubtitleCue {
                start_ms: 1_000,
                end_ms: 2_000,
                text: "Heh.".to_string(),
            }],
        };
        let mut commands = TextCommandList::new();

        track.append_commands(&mut commands, Samples(48_000), 48_000);

        assert_eq!(commands.len(), 2);
        assert_eq!(commands.as_slice()[1].text, "Heh.");
        assert!(commands.as_slice()[1].z > commands.as_slice()[0].z);
    }
}
