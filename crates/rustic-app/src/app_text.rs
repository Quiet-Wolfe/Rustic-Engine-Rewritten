//! Runtime text overlays for prototype-only app feedback.

use crate::preview_song::PreviewSelection;
use rustic_render::{TextCommand, TextCommandList};

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
