//! Minimal v-slice dialogue conversation playback for pre-song cutscenes.

use crate::asset_roots::baked_assets_root;
use crate::pause_menu::PAUSE_OVERLAY_TEXTURE_ID;
use crate::preview_song::{PreviewSelection, PreviewSong, VARIATION_PICO};
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_core::ids::CameraId;
use rustic_core::render::RenderLayer;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, TextCommand, TextCommandList};
use serde::Deserialize;
use std::time::Instant;

const DEFAULT_TEXT_SPEED: f32 = 1.0;
const TYPEWRITER_BASE_DELAY: f32 = 0.05;

#[derive(Debug, Clone)]
pub(crate) struct DialogueState {
    lines: Vec<DialogueLine>,
    selected_line: usize,
    backdrop_color: glam::Vec4,
    line_started_at: Instant,
    skipped_line: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogueLine {
    speaker: String,
    box_id: String,
    text: String,
    speed_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialogueAdvance {
    Skipped,
    Advanced,
    Finished,
}

impl DialogueState {
    pub(crate) fn load_for_selection(selection: PreviewSelection) -> Result<Option<Self>> {
        let Some(id) = conversation_id_for_selection(selection) else {
            return Ok(None);
        };
        Self::load(id).map(Some)
    }

    fn load(id: &'static str) -> Result<Self> {
        let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
        let path = AssetPath::new(format!("data/dialogue/conversations/{id}.json"))?;
        let bytes = load_bytes(&resolver, &path).with_context(|| format!("load {path}"))?;
        let raw: RawConversation = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse dialogue conversation {id}"))?;
        Ok(Self {
            lines: flatten_dialogue(raw.dialogue),
            selected_line: 0,
            line_started_at: Instant::now(),
            skipped_line: false,
            backdrop_color: raw
                .backdrop
                .and_then(|backdrop| backdrop.color)
                .and_then(|color| parse_hex_color(&color))
                .unwrap_or(glam::vec4(0.0, 0.0, 0.0, 0.65)),
        })
    }

    pub(crate) fn advance(&mut self) -> DialogueAdvance {
        if !self.current_line_complete_at(Instant::now()) {
            self.skipped_line = true;
            return DialogueAdvance::Skipped;
        }
        if self.selected_line + 1 >= self.lines.len() {
            return DialogueAdvance::Finished;
        }
        self.selected_line += 1;
        self.line_started_at = Instant::now();
        self.skipped_line = false;
        DialogueAdvance::Advanced
    }

    pub(crate) fn append_commands(
        &self,
        sprites: &mut RenderCommandList,
        text: &mut TextCommandList,
    ) {
        sprites.push(colored_rect(
            glam::vec2(0.0, 0.0),
            glam::vec2(1280.0, 720.0),
            self.backdrop_color,
            20_000,
        ));
        sprites.push(colored_rect(
            glam::vec2(70.0, 438.0),
            glam::vec2(1140.0, 220.0),
            glam::vec4(0.03, 0.03, 0.05, 0.88),
            20_001,
        ));
        if let Some(line) = self.current_line() {
            push_text(
                text,
                line.speaker.to_ascii_uppercase(),
                glam::vec2(108.0, 466.0),
                26.0,
                glam::vec4(1.0, 0.9, 0.62, 1.0),
                20_010,
                Some(1040.0),
            );
            push_text(
                text,
                self.visible_text_at(Instant::now()),
                glam::vec2(108.0, 512.0),
                30.0,
                text_color_for_box(&line.box_id),
                20_011,
                Some(1040.0),
            );
        }
        push_text(
            text,
            "Enter",
            glam::vec2(1110.0, 672.0),
            20.0,
            glam::vec4(1.0, 1.0, 1.0, 0.70),
            20_012,
            None,
        );
    }

    fn current_line(&self) -> Option<&DialogueLine> {
        self.lines.get(self.selected_line)
    }

    fn current_line_complete_at(&self, now: Instant) -> bool {
        self.current_line()
            .is_none_or(|line| self.visible_char_count_at(line, now) >= line.text.chars().count())
    }

    fn visible_text_at(&self, now: Instant) -> String {
        let Some(line) = self.current_line() else {
            return String::new();
        };
        line.text
            .chars()
            .take(self.visible_char_count_at(line, now))
            .collect()
    }

    fn visible_char_count_at(&self, line: &DialogueLine, now: Instant) -> usize {
        if self.skipped_line {
            return line.text.chars().count();
        }
        let delay = f32::from(line.speed_millis).max(1.0) / 1000.0;
        let elapsed = now.duration_since(self.line_started_at).as_secs_f32();
        ((elapsed / delay).floor() as usize + 1).min(line.text.chars().count())
    }

    #[cfg(test)]
    fn line_count(&self) -> usize {
        self.lines.len()
    }
}

pub(crate) fn conversation_id_for_selection(selection: PreviewSelection) -> Option<&'static str> {
    if selection.song == PreviewSong::SENPAI {
        return if selection.variation == Some(VARIATION_PICO) {
            Some("senpai-pico")
        } else {
            Some("senpai")
        };
    }
    if selection.song == PreviewSong::ROSES {
        return if selection.variation == Some(VARIATION_PICO) {
            Some("roses-pico")
        } else {
            Some("roses")
        };
    }
    (selection.song == PreviewSong::THORNS).then_some("thorns")
}

#[derive(Debug, Deserialize)]
struct RawConversation {
    #[serde(default)]
    backdrop: Option<RawBackdrop>,
    dialogue: Vec<RawDialogueEntry>,
}

#[derive(Debug, Deserialize)]
struct RawBackdrop {
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDialogueEntry {
    speaker: Option<String>,
    box_id: Option<String>,
    #[serde(alias = "box")]
    box_alias: Option<String>,
    speed: Option<f32>,
    text: Vec<String>,
}

fn flatten_dialogue(entries: Vec<RawDialogueEntry>) -> Vec<DialogueLine> {
    entries
        .into_iter()
        .flat_map(|entry| {
            let speaker = entry.speaker.unwrap_or_default();
            let box_id = entry.box_id.or(entry.box_alias).unwrap_or_default();
            let speed_millis = ((entry.speed.unwrap_or(DEFAULT_TEXT_SPEED) * TYPEWRITER_BASE_DELAY)
                * 1000.0)
                .round()
                .max(1.0) as u16;
            entry.text.into_iter().map(move |text| DialogueLine {
                speaker: speaker.clone(),
                box_id: box_id.clone(),
                text,
                speed_millis,
            })
        })
        .collect()
}

fn parse_hex_color(value: &str) -> Option<glam::Vec4> {
    let hex = value.trim().strip_prefix('#')?;
    let parsed = u32::from_str_radix(hex, 16).ok()?;
    match hex.len() {
        6 => Some(glam::vec4(
            ((parsed >> 16) & 0xff) as f32 / 255.0,
            ((parsed >> 8) & 0xff) as f32 / 255.0,
            (parsed & 0xff) as f32 / 255.0,
            1.0,
        )),
        8 => Some(glam::vec4(
            ((parsed >> 16) & 0xff) as f32 / 255.0,
            ((parsed >> 8) & 0xff) as f32 / 255.0,
            (parsed & 0xff) as f32 / 255.0,
            ((parsed >> 24) & 0xff) as f32 / 255.0,
        )),
        _ => None,
    }
}

fn colored_rect(position: glam::Vec2, size: glam::Vec2, color: glam::Vec4, z: i32) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(PAUSE_OVERLAY_TEXTURE_ID, position, size);
    cmd.camera = CameraId(2);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = color;
    cmd
}

fn push_text(
    commands: &mut TextCommandList,
    value: impl Into<String>,
    position: glam::Vec2,
    size: f32,
    color: glam::Vec4,
    z: i32,
    max_width: Option<f32>,
) {
    let mut cmd = TextCommand::new(value, position, size);
    cmd.camera = CameraId(2);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.max_width = max_width;
    cmd.color = color;
    commands.push(cmd);
}

fn text_color_for_box(box_id: &str) -> glam::Vec4 {
    match box_id {
        "thorns" => glam::vec4(0.92, 0.96, 1.0, 1.0),
        _ => glam::vec4(1.0, 0.96, 0.90, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_song::PreviewDifficulty;
    use std::time::Duration;

    #[test]
    fn week6_songs_map_to_expected_conversations() {
        assert_eq!(
            conversation_id_for_selection(PreviewSelection::new(
                PreviewSong::SENPAI,
                PreviewDifficulty::Normal
            )),
            Some("senpai")
        );
        assert_eq!(
            conversation_id_for_selection(
                PreviewSelection::new(PreviewSong::ROSES, PreviewDifficulty::Normal)
                    .with_variation(Some(VARIATION_PICO))
            ),
            Some("roses-pico")
        );
        assert_eq!(
            conversation_id_for_selection(PreviewSelection::new(
                PreviewSong::THORNS,
                PreviewDifficulty::Normal
            )),
            Some("thorns")
        );
    }

    #[test]
    fn loads_and_flattens_conversation_text() {
        let mut state = DialogueState::load("senpai").unwrap();
        assert_eq!(state.line_count(), 3);
        assert_eq!(
            state.current_line().map(|line| line.speaker.as_str()),
            Some("senpai")
        );
        state.line_started_at = Instant::now() - Duration::from_secs(60);
        assert_eq!(state.advance(), DialogueAdvance::Advanced);
        state.line_started_at = Instant::now() - Duration::from_secs(60);
        assert_eq!(state.advance(), DialogueAdvance::Advanced);
        state.line_started_at = Instant::now() - Duration::from_secs(60);
        assert_eq!(state.advance(), DialogueAdvance::Finished);
    }

    #[test]
    fn confirm_skips_typewriter_before_advancing() {
        let mut state = DialogueState::load("senpai").unwrap();
        let now = state.line_started_at;

        assert_eq!(state.visible_text_at(now), "A");
        assert_eq!(state.advance(), DialogueAdvance::Skipped);
        assert_eq!(
            state.visible_text_at(now),
            "Ah, a new fair maiden has come in search of true love!"
        );
        assert_eq!(state.advance(), DialogueAdvance::Advanced);
    }

    #[test]
    fn parses_aarrggbb_backdrop_color() {
        let color = parse_hex_color("#BFB3DFD8").unwrap();
        assert!((color.x - 0xB3 as f32 / 255.0).abs() < 0.001);
        assert!((color.w - 0xBF as f32 / 255.0).abs() < 0.001);
    }
}
