//! Minimal gameplay pause substate shaped after Funkin' v0.8.5.
//!
//! This first slice covers the standard menu flow: resume, restart, change
//! difficulty, and exit to the active play menu. Pause music and stickers are
//! deferred until the audio/menu asset layer has those source assets imported.
//!
//! ref: bdedc0aa:source/funkin/play/PauseSubState.hx:59-70,381-464,638-674,970-1168

use crate::preview_song::{PreviewDifficulty, PreviewSelection};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::input::InputAction;
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{
    DrawCommand, FilterMode, RenderCommandList, TextCommand, TextCommandList, Texture,
};
use std::collections::HashMap;

pub const PAUSE_OVERLAY_TEXTURE_ID: AssetId = AssetId::new(0x7061_7573_655f_0001);

const MENU_X: f32 = 90.0;
const MENU_Y: f32 = 300.0;
const MENU_SPACING: f32 = 44.0;
const META_RIGHT_X: f32 = 20.0;
const META_WIDTH: f32 = 1220.0;
const OVERLAY_ALPHA: f32 = 0.60;

#[derive(Debug, Clone)]
pub struct PauseMenuState {
    cursor: Samples,
    mode: PauseMenuMode,
    selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PauseMenuMode {
    Standard,
    Difficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseMenuAction {
    Resume,
    RestartSong,
    ChangeDifficulty(PreviewDifficulty),
    ExitToMenu,
    None,
}

impl PauseMenuState {
    pub fn new(cursor: Samples) -> Self {
        Self {
            cursor,
            mode: PauseMenuMode::Standard,
            selected: 0,
        }
    }

    pub fn cursor(&self) -> Samples {
        self.cursor
    }

    pub fn input(&mut self, action: InputAction, selection: PreviewSelection) -> PauseMenuAction {
        match action {
            InputAction::LaneUp | InputAction::UiUp => {
                self.change_selection(-1, selection);
                PauseMenuAction::None
            }
            InputAction::LaneDown | InputAction::UiDown => {
                self.change_selection(1, selection);
                PauseMenuAction::None
            }
            InputAction::Confirm => self.confirm(selection),
            InputAction::Back | InputAction::Pause => {
                if self.mode == PauseMenuMode::Difficulty {
                    self.mode = PauseMenuMode::Standard;
                    self.selected = 0;
                    PauseMenuAction::None
                } else {
                    PauseMenuAction::Resume
                }
            }
            _ => PauseMenuAction::None,
        }
    }

    pub fn append_commands(
        &self,
        sprites: &mut RenderCommandList,
        text: &mut TextCommandList,
        selection: PreviewSelection,
    ) {
        sprites.push(overlay_command());
        push_metadata_text(text, selection);
        push_menu_text(text, self.entries(selection), self.selected);
    }

    fn confirm(&mut self, selection: PreviewSelection) -> PauseMenuAction {
        match self.mode {
            PauseMenuMode::Standard => match self.selected {
                0 => PauseMenuAction::Resume,
                1 => PauseMenuAction::RestartSong,
                2 => {
                    self.mode = PauseMenuMode::Difficulty;
                    self.selected = 0;
                    PauseMenuAction::None
                }
                3 => PauseMenuAction::ExitToMenu,
                _ => PauseMenuAction::None,
            },
            PauseMenuMode::Difficulty => {
                let entries = difficulty_entries(selection);
                if self.selected == 0 {
                    self.mode = PauseMenuMode::Standard;
                    self.selected = 0;
                    return PauseMenuAction::None;
                }
                entries
                    .get(self.selected - 1)
                    .copied()
                    .map(PauseMenuAction::ChangeDifficulty)
                    .unwrap_or(PauseMenuAction::None)
            }
        }
    }

    fn change_selection(&mut self, delta: isize, selection: PreviewSelection) {
        let count = self.entries(selection).len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = (self.selected as isize + delta).rem_euclid(count as isize) as usize;
    }

    fn entries(&self, selection: PreviewSelection) -> Vec<String> {
        match self.mode {
            PauseMenuMode::Standard => standard_entries(),
            PauseMenuMode::Difficulty => {
                let mut entries = vec!["Back".to_string()];
                entries.extend(
                    difficulty_entries(selection)
                        .iter()
                        .map(|difficulty| difficulty_title(*difficulty)),
                );
                entries
            }
        }
    }
}

pub fn ensure_pause_overlay_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    textures: &mut HashMap<AssetId, Texture>,
) {
    textures.entry(PAUSE_OVERLAY_TEXTURE_ID).or_insert_with(|| {
        Texture::from_rgba8(
            device,
            queue,
            &[255, 255, 255, 255],
            1,
            1,
            FilterMode::Nearest,
            Some("rustic.pause.white"),
        )
    });
}

fn standard_entries() -> Vec<String> {
    vec![
        "Resume".to_string(),
        "Restart Song".to_string(),
        "Change Difficulty".to_string(),
        "Exit to Menu".to_string(),
    ]
}

fn difficulty_entries(selection: PreviewSelection) -> Vec<PreviewDifficulty> {
    selection.song.available_difficulties().to_vec()
}

fn difficulty_title(difficulty: PreviewDifficulty) -> String {
    match difficulty {
        PreviewDifficulty::Easy => "Easy",
        PreviewDifficulty::Normal => "Normal",
        PreviewDifficulty::Hard => "Hard",
        PreviewDifficulty::Erect => "Erect",
        PreviewDifficulty::Nightmare => "Nightmare",
    }
    .to_string()
}

fn overlay_command() -> DrawCommand {
    let mut cmd = DrawCommand::sprite(
        PAUSE_OVERLAY_TEXTURE_ID,
        glam::vec2(0.0, 0.0),
        glam::vec2(1280.0, 720.0),
    );
    cmd.camera = CameraId(2);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = 10_000;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = glam::vec4(0.0, 0.0, 0.0, OVERLAY_ALPHA);
    cmd
}

fn push_metadata_text(commands: &mut TextCommandList, selection: PreviewSelection) {
    let rows = [
        selection.song.display_name().to_string(),
        "Artist: Kawai Sprite".to_string(),
        format!("Difficulty: {}", difficulty_title(selection.difficulty)),
        "0 Blue Balls".to_string(),
    ];
    for (index, row) in rows.into_iter().enumerate() {
        let mut cmd = TextCommand::new(
            row,
            glam::vec2(META_RIGHT_X, 15.0 + index as f32 * 32.0),
            32.0,
        );
        cmd.max_width = Some(META_WIDTH);
        cmd.color = glam::vec4(1.0, 1.0, 1.0, 0.95);
        cmd.z = 10_100 + index as i32;
        commands.push(cmd);
    }
}

fn push_menu_text(commands: &mut TextCommandList, entries: Vec<String>, selected: usize) {
    for (index, entry) in entries.into_iter().enumerate() {
        let selected = index == selected;
        let mut cmd = TextCommand::new(
            entry,
            glam::vec2(MENU_X, MENU_Y + index as f32 * MENU_SPACING),
            if selected { 36.0 } else { 32.0 },
        );
        cmd.color = if selected {
            glam::vec4(1.0, 1.0, 1.0, 1.0)
        } else {
            glam::vec4(0.75, 0.75, 0.75, 0.85)
        };
        cmd.z = 10_200 + index as i32;
        commands.push(cmd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_song::PreviewSong;

    #[test]
    fn standard_pause_confirm_actions_match_menu_order() {
        let selection = PreviewSelection::new(PreviewSong::BOPEEBO, PreviewDifficulty::Normal);
        let mut menu = PauseMenuState::new(Samples(12_000));

        assert_eq!(menu.confirm(selection), PauseMenuAction::Resume);
        menu.change_selection(1, selection);
        assert_eq!(menu.confirm(selection), PauseMenuAction::RestartSong);
        menu.change_selection(1, selection);
        assert_eq!(menu.confirm(selection), PauseMenuAction::None);
        assert_eq!(menu.mode, PauseMenuMode::Difficulty);
    }

    #[test]
    fn difficulty_menu_lists_song_variations() {
        let selection = PreviewSelection::new(PreviewSong::BOPEEBO, PreviewDifficulty::Normal);
        let mut menu = PauseMenuState::new(Samples(0));

        menu.input(InputAction::LaneDown, selection);
        menu.input(InputAction::LaneDown, selection);
        menu.input(InputAction::Confirm, selection);

        let entries = menu.entries(selection);
        assert_eq!(
            entries,
            vec!["Back", "Easy", "Normal", "Hard", "Erect", "Nightmare"]
        );
    }

    #[test]
    fn back_from_difficulty_returns_to_standard_menu() {
        let selection = PreviewSelection::new(PreviewSong::TUTORIAL, PreviewDifficulty::Normal);
        let mut menu = PauseMenuState::new(Samples(0));

        menu.input(InputAction::LaneDown, selection);
        menu.input(InputAction::LaneDown, selection);
        menu.input(InputAction::Confirm, selection);
        assert_eq!(menu.mode, PauseMenuMode::Difficulty);

        let action = menu.input(InputAction::Back, selection);

        assert_eq!(action, PauseMenuAction::None);
        assert_eq!(menu.mode, PauseMenuMode::Standard);
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn overlay_uses_cam_other_and_full_reference_size() {
        let command = overlay_command();

        assert_eq!(command.camera, CameraId(2));
        assert_eq!(command.world_pos, glam::Vec2::ZERO);
        assert_eq!(command.size, glam::vec2(1280.0, 720.0));
        assert_eq!(command.color.w, OVERLAY_ALPHA);
    }
}
