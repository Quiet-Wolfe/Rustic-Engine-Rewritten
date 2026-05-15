//! Runtime note layout preferences backed by the Options preferences page.

use crate::pause_menu::PAUSE_OVERLAY_TEXTURE_ID;
use rustic_core::ids::CameraId;
use rustic_core::render::RenderLayer;
use rustic_render::{DrawCommand, FilterMode};

const FNF_HEIGHT: f32 = 720.0;
const STRUMLINE_X: f32 = 48.0;
const STRUMLINE_Y: f32 = 24.0;
const STRUMLINE_SIZE: f32 = 104.0;
const NOTE_SPACING: f32 = STRUMLINE_SIZE + 8.0;
const PLAYER_OFFSET: f32 = 640.0;
const BACKGROUND_PAD: f32 = 8.0;

pub(crate) fn apply_downscroll(cmd: &mut DrawCommand, downscroll: bool) {
    if downscroll {
        cmd.world_pos.y = FNF_HEIGHT - cmd.world_pos.y - cmd.size.y;
    }
}

pub(crate) fn strumline_background_commands(
    percent: u8,
    downscroll: bool,
    show_opponent: bool,
    player_x_offset: f32,
) -> Vec<DrawCommand> {
    if percent == 0 {
        return Vec::new();
    }
    let alpha = f32::from(percent.min(100)) / 100.0 * 0.55;
    let mut commands = Vec::with_capacity(2);
    if show_opponent {
        commands.push(strumline_background(0.0, downscroll, alpha));
    }
    commands.push(strumline_background(
        PLAYER_OFFSET + player_x_offset,
        downscroll,
        alpha,
    ));
    commands
}

fn strumline_background(x_offset: f32, downscroll: bool, alpha: f32) -> DrawCommand {
    let width = NOTE_SPACING * 4.0 - 8.0 + BACKGROUND_PAD * 2.0;
    let height = STRUMLINE_SIZE + BACKGROUND_PAD * 2.0;
    let y = if downscroll {
        FNF_HEIGHT - STRUMLINE_Y - STRUMLINE_SIZE - BACKGROUND_PAD
    } else {
        STRUMLINE_Y - BACKGROUND_PAD
    };
    let mut cmd = DrawCommand::sprite(
        PAUSE_OVERLAY_TEXTURE_ID,
        glam::vec2(STRUMLINE_X + x_offset - BACKGROUND_PAD, y),
        glam::vec2(width, height),
    );
    cmd.camera = CameraId(1);
    cmd.layer = RenderLayer::Notes;
    cmd.z = -20;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = glam::vec4(0.0, 0.0, 0.0, alpha);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustic_core::ids::AssetId;

    #[test]
    fn downscroll_mirrors_command_y_inside_reference_height() {
        let mut cmd = DrawCommand::sprite(
            AssetId::new(1),
            glam::vec2(20.0, 24.0),
            glam::vec2(10.0, 30.0),
        );
        apply_downscroll(&mut cmd, true);
        assert_eq!(cmd.world_pos.y, 666.0);
    }

    #[test]
    fn strumline_background_respects_percent_and_downscroll() {
        assert!(strumline_background_commands(0, false, true, 0.0).is_empty());
        let commands = strumline_background_commands(50, true, false, -272.0);
        assert_eq!(commands.len(), 1);
        assert!((commands[0].color.w - 0.275).abs() < 0.001);
        assert!(commands[0].world_pos.y > 500.0);
    }
}
