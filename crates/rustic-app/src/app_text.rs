//! Runtime text overlays for prototype-only app feedback.

use crate::preview_song::{PreviewSelection, PreviewSong};
use rustic_render::{TextCommand, TextCommandList};

pub(crate) struct DebugOverlayLine {
    text: String,
}

impl DebugOverlayLine {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

pub(crate) fn preview_text_commands(selection: PreviewSelection) -> TextCommandList {
    let mut commands = TextCommandList::new();
    let song = selection.song.display_name();
    let diff = selection.difficulty.as_str();

    let mut header = TextCommand::new(format!("{song} ({diff})"), glam::vec2(24.0, 24.0), 32.0);
    header.color = glam::vec4(1.0, 1.0, 1.0, 0.85);
    header.z = 100;
    commands.push(header);

    let mut hint = TextCommand::new("<- song   -> difficulty", glam::vec2(24.0, 60.0), 18.0);
    hint.color = glam::vec4(0.85, 0.85, 0.85, 0.7);
    hint.z = 100;
    commands.push(hint);

    commands
}

pub fn song_select_text_commands(
    selection: PreviewSelection,
    include_difficulty_label: bool,
) -> TextCommandList {
    let mut commands = TextCommandList::new();

    let mut title = TextCommand::new("Freeplay", glam::vec2(78.0, 62.0), 54.0);
    title.color = glam::vec4(1.0, 0.84, 0.26, 0.98);
    title.z = 90;
    commands.push(title);

    if include_difficulty_label {
        let mut difficulty = TextCommand::new(
            format!("< {} >", selection.difficulty.as_str()),
            glam::vec2(82.0, 130.0),
            28.0,
        );
        difficulty.color = glam::vec4(0.92, 0.95, 1.0, 0.82);
        difficulty.z = 90;
        commands.push(difficulty);
    }

    for (index, song) in PreviewSong::ALL.iter().enumerate() {
        let selected = *song == selection.song;
        let prefix = if selected { ">" } else { " " };
        let mut cmd = TextCommand::new(
            format!("{prefix} {}", song.display_name()),
            glam::vec2(112.0, 220.0 + index as f32 * 58.0),
            if selected { 40.0 } else { 30.0 },
        );
        cmd.color = if selected {
            glam::vec4(1.0, 1.0, 1.0, 0.98)
        } else {
            glam::vec4(0.72, 0.76, 0.86, 0.72)
        };
        cmd.z = 90;
        commands.push(cmd);
    }

    commands
}

pub(crate) fn push_debug_overlay_text(
    commands: &mut TextCommandList,
    lines: impl IntoIterator<Item = DebugOverlayLine>,
) {
    let mut y = 94.0;
    for line in lines {
        let mut cmd = TextCommand::new(line.text, glam::vec2(24.0, y), 16.0);
        cmd.color = glam::vec4(0.95, 1.0, 0.92, 0.82);
        cmd.z = 120;
        commands.push(cmd);
        y += 18.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_overlay_lines_stack_vertically_above_preview_hints() {
        let mut commands = TextCommandList::new();
        push_debug_overlay_text(
            &mut commands,
            [
                DebugOverlayLine::new("fps 60.0"),
                DebugOverlayLine::new("camGame pos 640,360"),
            ],
        );

        assert_eq!(commands.len(), 2);
        assert_eq!(commands.as_slice()[0].position.y, 94.0);
        assert_eq!(commands.as_slice()[1].position.y, 112.0);
        assert_eq!(commands.as_slice()[0].z, 120);
    }

    #[test]
    fn song_select_highlights_current_preview_song() {
        let commands = song_select_text_commands(
            PreviewSelection::from_keys(Some("fresh"), Some("hard")),
            true,
        );
        let selected = commands
            .iter()
            .find(|cmd| cmd.text.starts_with(">"))
            .map(|cmd| cmd.text.as_str());
        assert_eq!(selected, Some("> Fresh"));
    }
}
