//! Main-menu asset wiring from Funkin' v0.8.5.
//!
//! ref: bdedc0aa:source/funkin/ui/mainmenu/MainMenuState.hx:115-232

use crate::asset_roots::baked_assets_root;
use anyhow::{Context, Result};
use rustic_asset::{load_png, load_sparrow, AssetPath, OverlayResolver, SparrowFrame};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;

const MENU_ANIMATION_FPS: u16 = 24;
const MENU_ITEM_SPACING: f32 = 160.0;
const MENU_ITEM_X: f32 = 1280.0 * 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuAction {
    StoryMode,
    Freeplay,
    Options,
    Credits,
}

#[derive(Debug)]
pub struct MainMenuAssets {
    background: MenuBackground,
    items: Vec<MenuItemClip>,
    pub textures: HashMap<AssetId, Texture>,
}

impl MainMenuAssets {
    pub fn commands(
        &self,
        selected_index: usize,
        cursor: Samples,
        sample_rate: u32,
    ) -> RenderCommandList {
        let mut commands = RenderCommandList::new();
        commands.push(self.background.command());
        for (index, item) in self.items.iter().enumerate() {
            if let Some(cmd) = item.command(
                index,
                self.items.len(),
                index == selected_index,
                cursor,
                sample_rate,
            ) {
                commands.push(cmd);
            }
        }
        commands
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn action_for(&self, index: usize) -> Option<MainMenuAction> {
        self.items.get(index).map(|item| item.action)
    }
}

#[derive(Debug)]
struct MenuBackground {
    texture_id: AssetId,
    size: glam::Vec2,
}

impl MenuBackground {
    fn command(&self) -> DrawCommand {
        let scale = 1280.0 * 1.2 / self.size.x.max(1.0);
        let draw_size = self.size * scale;
        let mut cmd = DrawCommand::sprite(
            self.texture_id,
            glam::vec2((1280.0 - draw_size.x) * 0.5, (720.0 - draw_size.y) * 0.5),
            draw_size,
        );
        cmd.camera = CameraId(1);
        cmd.layer = RenderLayer::Background;
        cmd.z = -10;
        cmd.pivot = glam::Vec2::ZERO;
        cmd.filter = FilterMode::Linear;
        cmd
    }
}

#[derive(Debug)]
struct MenuItemClip {
    action: MainMenuAction,
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    idle: Vec<SparrowFrame>,
    selected: Vec<SparrowFrame>,
}

impl MenuItemClip {
    fn command(
        &self,
        index: usize,
        item_count: usize,
        is_selected: bool,
        cursor: Samples,
        sample_rate: u32,
    ) -> Option<DrawCommand> {
        let frames = if is_selected {
            &self.selected
        } else {
            &self.idle
        };
        let frame = frames.get(animation_frame_index(cursor, sample_rate, frames.len()))?;
        let center = glam::vec2(
            MENU_ITEM_X,
            menu_top(item_count) + MENU_ITEM_SPACING * index as f32,
        );
        Some(centered_sparrow_command(
            self.texture_id,
            self.texture_width,
            self.texture_height,
            frame,
            center,
            index as i32,
        ))
    }
}

pub fn load_main_menu_assets(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<MainMenuAssets> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let mut textures = HashMap::new();
    let background = load_background(device, queue, &resolver, &mut textures)?;
    let items = vec![
        load_menu_item(
            device,
            queue,
            &resolver,
            &mut textures,
            MainMenuAction::StoryMode,
            "images/mainmenu/storymode.xml",
            "storymode",
        )?,
        load_menu_item(
            device,
            queue,
            &resolver,
            &mut textures,
            MainMenuAction::Freeplay,
            "images/mainmenu/freeplay.xml",
            "freeplay",
        )?,
        load_menu_item(
            device,
            queue,
            &resolver,
            &mut textures,
            MainMenuAction::Options,
            "images/mainmenu/options.xml",
            "options",
        )?,
        load_menu_item(
            device,
            queue,
            &resolver,
            &mut textures,
            MainMenuAction::Credits,
            "images/mainmenu/credits.xml",
            "credits",
        )?,
    ];
    Ok(MainMenuAssets {
        background,
        items,
        textures,
    })
}

fn load_background(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<MenuBackground> {
    let path = AssetPath::new("images/menuBG.png")?;
    let image = load_png(resolver, &path).with_context(|| format!("load {path}"))?;
    let texture_id = asset_id_for_path(&path);
    let size = glam::vec2(image.width as f32, image.height as f32);
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
    Ok(MenuBackground { texture_id, size })
}

fn load_menu_item(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    action: MainMenuAction,
    atlas_path: &str,
    prefix: &str,
) -> Result<MenuItemClip> {
    let atlas_path = AssetPath::new(atlas_path)?;
    let atlas = load_sparrow(resolver, &atlas_path)?;
    let texture_path = atlas_path.sibling(&atlas.image_path)?;
    let image =
        load_png(resolver, &texture_path).with_context(|| format!("load {texture_path}"))?;
    let texture_id = asset_id_for_path(&texture_path);
    let (texture_width, texture_height) = (image.width, image.height);
    textures.insert(
        texture_id,
        Texture::from_png_image(
            device,
            queue,
            &image,
            FilterMode::Linear,
            Some(texture_path.as_str()),
        ),
    );
    let idle = atlas
        .animation_frames(&format!("{prefix} idle"), &[])
        .into_iter()
        .cloned()
        .collect();
    let selected = atlas
        .animation_frames(&format!("{prefix} selected"), &[])
        .into_iter()
        .cloned()
        .collect();
    Ok(MenuItemClip {
        action,
        texture_id,
        texture_width,
        texture_height,
        idle,
        selected,
    })
}

fn centered_sparrow_command(
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frame: &SparrowFrame,
    center: glam::Vec2,
    z: i32,
) -> DrawCommand {
    let frame_size = glam::vec2(frame.frame_width as f32, frame.frame_height as f32);
    let mut cmd = DrawCommand::sprite(
        texture_id,
        center - frame_size * 0.5 - glam::vec2(frame.frame_x as f32, frame.frame_y as f32),
        frame_draw_size(frame),
    );
    cmd.camera = CameraId(1);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Linear;
    (cmd.uv_min, cmd.uv_max) = frame_uv(frame, texture_width, texture_height);
    cmd.uv_rotated = frame.rotated;
    cmd
}

fn menu_top(count: usize) -> f32 {
    (720.0 - MENU_ITEM_SPACING * count.saturating_sub(1) as f32) * 0.5
}

fn animation_frame_index(cursor: Samples, sample_rate: u32, frame_count: usize) -> usize {
    if frame_count <= 1 {
        return 0;
    }
    let elapsed = cursor.0.max(0) as u128;
    (elapsed * u128::from(MENU_ANIMATION_FPS) / u128::from(sample_rate.max(1))) as usize
        % frame_count
}

fn frame_draw_size(frame: &SparrowFrame) -> glam::Vec2 {
    if frame.rotated {
        glam::vec2(frame.height as f32, frame.width as f32)
    } else {
        glam::vec2(frame.width as f32, frame.height as f32)
    }
}

fn frame_uv(
    frame: &SparrowFrame,
    texture_width: u32,
    texture_height: u32,
) -> (glam::Vec2, glam::Vec2) {
    let width = texture_width.max(1) as f32;
    let height = texture_height.max(1) as f32;
    (
        glam::vec2(frame.x as f32 / width, frame.y as f32 / height),
        glam::vec2(
            (frame.x as f32 + frame.width as f32) / width,
            (frame.y as f32 + frame.height as f32) / height,
        ),
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

    #[test]
    fn menu_item_top_matches_og_spacing() {
        assert_eq!(menu_top(4), 120.0);
    }
}
