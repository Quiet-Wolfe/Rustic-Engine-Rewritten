//! HUD texture wiring and command generation.

use anyhow::{Context, Result};
use rustic_asset::{load_png, AssetPath, OverlayResolver};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const FNF_WIDTH: f32 = 1280.0;
const FNF_HEIGHT: f32 = 720.0;
const HEALTH_BAR_WIDTH: f32 = 601.0;
const HEALTH_BAR_HEIGHT: f32 = 19.0;
const HEALTH_FILL_INSET_X: f32 = 4.0;
const HEALTH_FILL_INSET_Y: f32 = 4.0;
const HEALTH_FILL_WIDTH: f32 = HEALTH_BAR_WIDTH - 8.0;
const HEALTH_FILL_HEIGHT: f32 = HEALTH_BAR_HEIGHT - 8.0;
const ICON_SIZE: f32 = 150.0;
const ICON_OFFSET: f32 = 26.0;
const WHITE_TEXTURE_ID: AssetId = AssetId::new(0xfef0_0000_0000_0001);

#[derive(Debug, Clone)]
pub struct HudSkin {
    health_bar_texture: AssetId,
    icon_grid_texture: AssetId,
    white_texture: AssetId,
    icon_grid_width: u32,
    icon_grid_height: u32,
}

impl HudSkin {
    pub fn commands(&self, health: f32) -> Vec<DrawCommand> {
        self.commands_with_icon_scale(health, 1.0)
    }

    pub fn commands_with_icon_scale(&self, health: f32, icon_scale: f32) -> Vec<DrawCommand> {
        let mut commands = Vec::with_capacity(5);
        let bar_x = (FNF_WIDTH - HEALTH_BAR_WIDTH) * 0.5;
        let bar_y = FNF_HEIGHT * 0.9;
        let fill_x = bar_x + HEALTH_FILL_INSET_X;
        let fill_y = bar_y + HEALTH_FILL_INSET_Y;
        let health = health.clamp(0.0, 2.0);
        let green_width = HEALTH_FILL_WIDTH * (health / 2.0);

        let mut bg = DrawCommand::sprite(
            self.health_bar_texture,
            glam::vec2(bar_x, bar_y),
            glam::vec2(HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT),
        );
        bg.camera = CameraId(1);
        bg.pivot = glam::Vec2::ZERO;
        bg.layer = RenderLayer::Hud;
        bg.filter = FilterMode::Linear;
        bg.z = 0;
        commands.push(bg);

        commands.push(self.color_rect(
            glam::vec2(fill_x, fill_y),
            glam::vec2(HEALTH_FILL_WIDTH, HEALTH_FILL_HEIGHT),
            glam::vec4(1.0, 0.0, 0.0, 1.0),
            1,
        ));
        commands.push(self.color_rect(
            glam::vec2(fill_x + HEALTH_FILL_WIDTH - green_width, fill_y),
            glam::vec2(green_width, HEALTH_FILL_HEIGHT),
            glam::vec4(0.4, 1.0, 0.2, 1.0),
            2,
        ));

        let marker = fill_x + HEALTH_FILL_WIDTH * (1.0 - health / 2.0);
        let bf_frame = if health < 0.4 { 1 } else { 0 };
        let dad_frame = if health > 1.6 { 13 } else { 12 };
        let icon_size = ICON_SIZE * icon_scale.max(0.01);
        commands.push(self.icon_command(
            bf_frame,
            true,
            glam::vec2(marker - ICON_OFFSET, fill_y - icon_size * 0.5),
            icon_size,
        ));
        commands.push(self.icon_command(
            dad_frame,
            false,
            glam::vec2(marker - (icon_size - ICON_OFFSET), fill_y - icon_size * 0.5),
            icon_size,
        ));
        commands
    }

    fn color_rect(
        &self,
        world_pos: glam::Vec2,
        size: glam::Vec2,
        color: glam::Vec4,
        z: i32,
    ) -> DrawCommand {
        let mut cmd = DrawCommand::sprite(self.white_texture, world_pos, size);
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Hud;
        cmd.filter = FilterMode::Nearest;
        cmd.color = color;
        cmd.z = z;
        cmd
    }

    fn icon_command(
        &self,
        frame_index: u32,
        flip_x: bool,
        world_pos: glam::Vec2,
        icon_size: f32,
    ) -> DrawCommand {
        let mut cmd = DrawCommand::sprite(
            self.icon_grid_texture,
            world_pos,
            glam::vec2(icon_size, icon_size),
        );
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Hud;
        cmd.filter = FilterMode::Linear;
        cmd.z = 3;
        (cmd.uv_min, cmd.uv_max) =
            icon_uv(frame_index, self.icon_grid_width, self.icon_grid_height);
        if flip_x {
            std::mem::swap(&mut cmd.uv_min.x, &mut cmd.uv_max.x);
        }
        cmd
    }
}

pub fn load_hud_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<HudSkin> {
    let health_bar_path = AssetPath::new("images/healthBar.png")?;
    let health_bar = load_png(resolver, &health_bar_path)
        .with_context(|| format!("load {}", health_bar_path.as_str()))?;
    let health_bar_texture = asset_id_for_path(&health_bar_path);
    textures.insert(
        health_bar_texture,
        Texture::from_png_image(
            device,
            queue,
            &health_bar,
            FilterMode::Linear,
            Some(health_bar_path.as_str()),
        ),
    );

    let icon_grid_path = AssetPath::new("images/iconGrid.png")?;
    let icon_grid = load_png(resolver, &icon_grid_path)
        .with_context(|| format!("load {}", icon_grid_path.as_str()))?;
    let icon_grid_texture = asset_id_for_path(&icon_grid_path);
    textures.insert(
        icon_grid_texture,
        Texture::from_png_image(
            device,
            queue,
            &icon_grid,
            FilterMode::Linear,
            Some(icon_grid_path.as_str()),
        ),
    );

    textures.insert(
        WHITE_TEXTURE_ID,
        Texture::from_rgba8(
            device,
            queue,
            &[255, 255, 255, 255],
            1,
            1,
            FilterMode::Nearest,
            Some("rustic.hud.white"),
        ),
    );

    Ok(HudSkin {
        health_bar_texture,
        icon_grid_texture,
        white_texture: WHITE_TEXTURE_ID,
        icon_grid_width: icon_grid.width,
        icon_grid_height: icon_grid.height,
    })
}

fn icon_uv(frame_index: u32, texture_width: u32, texture_height: u32) -> (glam::Vec2, glam::Vec2) {
    let col = frame_index % 10;
    let row = frame_index / 10;
    let width = texture_width.max(1) as f32;
    let height = texture_height.max(1) as f32;
    let x = col as f32 * ICON_SIZE;
    let y = row as f32 * ICON_SIZE;
    (
        glam::vec2(x / width, y / height),
        glam::vec2((x + ICON_SIZE) / width, (y + ICON_SIZE) / height),
    )
}

fn asset_id_for_path(path: &AssetPath) -> AssetId {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in path.as_str().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    AssetId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin() -> HudSkin {
        HudSkin {
            health_bar_texture: AssetId::new(1),
            icon_grid_texture: AssetId::new(2),
            white_texture: AssetId::new(3),
            icon_grid_width: 1500,
            icon_grid_height: 900,
        }
    }

    #[test]
    fn health_bar_uses_og_fill_origin_for_icon_marker() {
        let commands = skin().commands(1.0);
        let fill_x = (FNF_WIDTH - HEALTH_BAR_WIDTH) * 0.5 + HEALTH_FILL_INSET_X;
        let fill_y = FNF_HEIGHT * 0.9 + HEALTH_FILL_INSET_Y;
        let marker = fill_x + HEALTH_FILL_WIDTH * 0.5;

        assert_eq!(commands[0].z, 0);
        assert_eq!(commands[1].z, 1);
        assert_eq!(commands[2].z, 2);
        assert_eq!(commands[3].z, 3);
        assert_eq!(commands[4].z, 3);
        assert_eq!(
            commands[3].world_pos,
            glam::vec2(marker - ICON_OFFSET, fill_y - 75.0)
        );
        assert_eq!(
            commands[4].world_pos,
            glam::vec2(marker - (ICON_SIZE - ICON_OFFSET), fill_y - 75.0)
        );
    }

    #[test]
    fn health_icons_switch_to_og_danger_frames() {
        let low_health = skin().commands(0.39);
        let high_health = skin().commands(1.61);

        let (bf_frame_one_min, bf_frame_one_max) = icon_uv(1, 1500, 900);
        let (dad_frame_thirteen_min, dad_frame_thirteen_max) = icon_uv(13, 1500, 900);

        assert_eq!(low_health[3].uv_min.x, bf_frame_one_max.x);
        assert_eq!(low_health[3].uv_max.x, bf_frame_one_min.x);
        assert_eq!(low_health[3].uv_min.y, bf_frame_one_min.y);
        assert_eq!(low_health[3].uv_max.y, bf_frame_one_max.y);

        assert_eq!(high_health[4].uv_min, dad_frame_thirteen_min);
        assert_eq!(high_health[4].uv_max, dad_frame_thirteen_max);
    }

    #[test]
    fn icon_bump_scales_size_and_dad_side_offset() {
        let commands = skin().commands_with_icon_scale(1.0, 1.2);
        let fill_x = (FNF_WIDTH - HEALTH_BAR_WIDTH) * 0.5 + HEALTH_FILL_INSET_X;
        let fill_y = FNF_HEIGHT * 0.9 + HEALTH_FILL_INSET_Y;
        let marker = fill_x + HEALTH_FILL_WIDTH * 0.5;
        let icon_size = ICON_SIZE * 1.2;

        assert_eq!(commands[3].size, glam::vec2(icon_size, icon_size));
        assert_eq!(
            commands[3].world_pos,
            glam::vec2(marker - ICON_OFFSET, fill_y - icon_size * 0.5)
        );
        assert_eq!(
            commands[4].world_pos,
            glam::vec2(marker - (icon_size - ICON_OFFSET), fill_y - icon_size * 0.5)
        );
    }
}
