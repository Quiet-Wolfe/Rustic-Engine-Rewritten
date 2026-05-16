//! HUD texture wiring and command generation.
// LINT-ALLOW: long-file HUD icon loading and layout tests stay together.

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
    bf_icon: ActiveIcon,
    dad_icon: ActiveIcon,
    icons: HashMap<String, IconTexture>,
    white_texture: AssetId,
}

#[derive(Debug, Clone, Copy)]
struct IconTexture {
    texture: AssetId,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy)]
struct ActiveIcon {
    texture: IconTexture,
    flip_x: bool,
    scale: f32,
    offset: glam::Vec2,
    filter: FilterMode,
}

#[derive(Debug, Clone, Copy)]
pub struct HealthIconEvent {
    pub target: i8,
    pub scale: f32,
    pub flip_x: bool,
    pub is_pixel: bool,
    pub offset: glam::Vec2,
}

impl HudSkin {
    pub fn commands(&self, health: f32) -> Vec<DrawCommand> {
        self.commands_with_icon_scale(health, 1.0)
    }

    pub fn commands_with_icon_scale(&self, health: f32, icon_scale: f32) -> Vec<DrawCommand> {
        self.commands_with_icon_scale_and_visibility(health, icon_scale, true)
    }

    pub fn commands_with_icon_scale_and_visibility(
        &self,
        health: f32,
        icon_scale: f32,
        show_opponent_icon: bool,
    ) -> Vec<DrawCommand> {
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
        let dad_frame = if health > 1.6 { 1 } else { 0 };
        let player_size = icon_size_for(self.bf_icon, icon_scale);
        let opponent_size = icon_size_for(self.dad_icon, icon_scale);
        commands.push(self.icon_command(
            self.bf_icon,
            bf_frame,
            glam::vec2(marker - ICON_OFFSET, fill_y - player_size * 0.5),
            player_size,
        ));
        if show_opponent_icon {
            commands.push(self.icon_command(
                self.dad_icon,
                dad_frame,
                glam::vec2(
                    marker - (opponent_size - ICON_OFFSET),
                    fill_y - opponent_size * 0.5,
                ),
                opponent_size,
            ));
        }
        commands
    }

    pub fn set_health_icon(&mut self, id: &str, event: HealthIconEvent) -> bool {
        let Some(texture) = self.icons.get(id).copied() else {
            tracing::warn!(target: "rustic.asset", "health icon {id} unavailable");
            return false;
        };
        let icon = ActiveIcon {
            texture,
            flip_x: event.flip_x,
            scale: event.scale.max(0.0),
            offset: event.offset,
            filter: if event.is_pixel {
                FilterMode::Nearest
            } else {
                FilterMode::Linear
            },
        };
        match event.target {
            0 => self.bf_icon = icon,
            1 => self.dad_icon = icon,
            _ => return false,
        }
        true
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
        icon: ActiveIcon,
        frame_index: u32,
        world_pos: glam::Vec2,
        icon_size: f32,
    ) -> DrawCommand {
        let mut cmd = DrawCommand::sprite(
            icon.texture.texture,
            world_pos + icon.offset,
            glam::vec2(icon_size, icon_size),
        );
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Hud;
        cmd.filter = icon.filter;
        cmd.z = 3;
        (cmd.uv_min, cmd.uv_max) = icon_uv(frame_index, icon.texture.width, icon.texture.height);
        if icon.flip_x {
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
    load_hud_assets_for_icons(device, queue, resolver, textures, "bf", "dad")
}

pub fn load_hud_assets_for_icons(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    player_icon_id: &str,
    opponent_icon_id: &str,
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

    let icons = load_health_icons(device, queue, resolver, textures)?;
    let bf_icon = icon_or_default(&icons, player_icon_id, "bf")?;
    let dad_icon = icon_or_default(&icons, opponent_icon_id, "dad")?;

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
        bf_icon: ActiveIcon::new(bf_icon, true),
        dad_icon: ActiveIcon::new(dad_icon, false),
        icons,
        white_texture: WHITE_TEXTURE_ID,
    })
}

fn icon_or_default(
    icons: &HashMap<String, IconTexture>,
    id: &str,
    fallback: &str,
) -> Result<IconTexture> {
    icons
        .get(id)
        .or_else(|| icons.get(fallback))
        .copied()
        .with_context(|| format!("load {fallback} health icon"))
}

impl ActiveIcon {
    fn new(texture: IconTexture, flip_x: bool) -> Self {
        Self {
            texture,
            flip_x,
            scale: 1.0,
            offset: glam::Vec2::ZERO,
            filter: FilterMode::Linear,
        }
    }
}

fn load_health_icons(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<HashMap<String, IconTexture>> {
    let mut icons = HashMap::new();
    for id in VANILLA_HEALTH_ICON_IDS {
        let path = AssetPath::new(format!("images/icons/icon-{id}.png"))?;
        let image = match load_png(resolver, &path) {
            Ok(image) => image,
            Err(error) => {
                tracing::warn!(
                    target: "rustic.asset",
                    "health icon {id} unavailable: {error:#}"
                );
                continue;
            }
        };
        let texture_id = asset_id_for_path(&path);
        textures.insert(
            texture_id,
            Texture::from_png_image(
                device,
                queue,
                &image,
                FilterMode::Linear,
                Some(path.as_str()),
            ),
        );
        icons.insert(
            (*id).to_string(),
            IconTexture {
                texture: texture_id,
                width: image.width,
                height: image.height,
            },
        );
    }
    Ok(icons)
}

const VANILLA_HEALTH_ICON_IDS: &[&str] = &[
    "bf",
    "bf-old",
    "bf-pixel",
    "chaewon",
    "dad",
    "darnell",
    "eunchae",
    "face",
    "gf",
    "kazuha",
    "mom",
    "monster",
    "parents",
    "pico",
    "pico-pixel",
    "sakura",
    "senpai",
    "senpai-angry",
    "spirit",
    "spooky",
    "tankman",
    "tankman-bloody",
    "yunjin",
];

fn icon_size_for(icon: ActiveIcon, icon_scale: f32) -> f32 {
    ICON_SIZE * icon_scale.max(0.01) * icon.scale.max(0.01)
}

fn icon_uv(frame_index: u32, texture_width: u32, texture_height: u32) -> (glam::Vec2, glam::Vec2) {
    let frame_count = (texture_width / ICON_SIZE as u32).max(1);
    let frame_index = frame_index.min(frame_count - 1);
    let width = texture_width.max(1) as f32;
    let height = texture_height.max(1) as f32;
    let x = frame_index as f32 * ICON_SIZE;
    let frame_width = ICON_SIZE.min(width);
    let frame_height = ICON_SIZE.min(height);
    (
        glam::vec2(x / width, 0.0),
        glam::vec2((x + frame_width) / width, frame_height / height),
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
        let bf = IconTexture {
            texture: AssetId::new(2),
            width: 300,
            height: 150,
        };
        let dad = IconTexture {
            texture: AssetId::new(4),
            width: 150,
            height: 150,
        };
        let mut icons = HashMap::new();
        icons.insert("bf".to_string(), bf);
        icons.insert("dad".to_string(), dad);
        icons.insert(
            "tankman-bloody".to_string(),
            IconTexture {
                texture: AssetId::new(5),
                width: 300,
                height: 150,
            },
        );
        HudSkin {
            health_bar_texture: AssetId::new(1),
            bf_icon: ActiveIcon::new(bf, true),
            dad_icon: ActiveIcon::new(dad, false),
            icons,
            white_texture: AssetId::new(3),
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

        let (bf_frame_one_min, bf_frame_one_max) = icon_uv(1, 300, 150);
        let (dad_frame_zero_min, dad_frame_zero_max) = icon_uv(1, 150, 150);

        assert_eq!(low_health[3].uv_min.x, bf_frame_one_max.x);
        assert_eq!(low_health[3].uv_max.x, bf_frame_one_min.x);
        assert_eq!(low_health[3].uv_min.y, bf_frame_one_min.y);
        assert_eq!(low_health[3].uv_max.y, bf_frame_one_max.y);

        assert_eq!(high_health[4].uv_min, dad_frame_zero_min);
        assert_eq!(high_health[4].uv_max, dad_frame_zero_max);
    }

    #[test]
    fn opponent_health_icon_can_be_hidden_for_scripted_cutscenes() {
        let commands = skin().commands_with_icon_scale_and_visibility(1.0, 1.0, false);

        assert_eq!(commands.len(), 4);
        assert_eq!(commands[3].texture, AssetId::new(2));
    }

    #[test]
    fn single_frame_icons_clamp_danger_frame_to_idle() {
        assert_eq!(icon_uv(0, 150, 150), icon_uv(1, 150, 150));
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

    #[test]
    fn set_health_icon_updates_target_texture_and_event_shape() {
        let mut skin = skin();

        assert!(skin.set_health_icon(
            "tankman-bloody",
            HealthIconEvent {
                target: 1,
                scale: 1.1,
                flip_x: true,
                is_pixel: true,
                offset: glam::vec2(2.0, -3.0),
            },
        ));

        let commands = skin.commands(1.0);
        assert_eq!(commands[4].texture, AssetId::new(5));
        assert_eq!(
            commands[4].size,
            glam::vec2(ICON_SIZE * 1.1, ICON_SIZE * 1.1)
        );
        assert_eq!(commands[4].filter, FilterMode::Nearest);
        assert!(commands[4].uv_min.x > commands[4].uv_max.x);
    }
}
