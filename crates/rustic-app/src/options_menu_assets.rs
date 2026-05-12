//! Options-menu shell based on Funkin' v0.8.5 `OptionsState`.
//!
//! This first pass wires the main options codex pages so the main-menu
//! Options entry is playable instead of a no-op. Preference persistence and
//! full controls editing can attach to these pages later.
//!
//! ref: bdedc0aa:source/funkin/ui/options/OptionsState.hx:46-101,150-332

use crate::asset_roots::baked_assets_root;
use anyhow::{Context, Result};
use rustic_asset::{load_png, AssetPath, OverlayResolver};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_render::{
    DrawCommand, FilterMode, RenderCommandList, TextCommand, TextCommandList, Texture,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsMenuPage {
    Root,
    Preferences,
    Controls,
    SaveData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsMenuAction {
    Page(OptionsMenuPage),
    Exit,
}

#[derive(Debug)]
pub struct OptionsMenuAssets {
    background: OptionsBackground,
    pub textures: HashMap<AssetId, Texture>,
}

impl OptionsMenuAssets {
    pub fn commands(&self) -> RenderCommandList {
        let mut commands = RenderCommandList::new();
        commands.push(self.background.command());
        commands
    }

    pub fn text_commands(&self, page: OptionsMenuPage, selected_index: usize) -> TextCommandList {
        let mut commands = TextCommandList::new();
        push_title(&mut commands, page.title());
        for (index, item) in page.items().iter().enumerate() {
            push_item(&mut commands, item, index, selected_index);
        }
        commands
    }

    pub fn item_count(&self, page: OptionsMenuPage) -> usize {
        page.items().len()
    }

    pub fn action_for_root(&self, index: usize) -> Option<OptionsMenuAction> {
        ROOT_ACTIONS.get(index).copied()
    }
}

#[derive(Debug)]
struct OptionsBackground {
    texture_id: AssetId,
    size: glam::Vec2,
}

impl OptionsBackground {
    fn command(&self) -> DrawCommand {
        let scale = 1280.0 * 1.1 / self.size.x.max(1.0);
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
        cmd.color = glam::vec4(0.45, 0.72, 0.95, 1.0);
        cmd
    }
}

pub fn load_options_menu_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<OptionsMenuAssets> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let mut textures = HashMap::new();
    let background = load_background(device, queue, &resolver, &mut textures)?;
    Ok(OptionsMenuAssets {
        background,
        textures,
    })
}

impl OptionsMenuPage {
    fn title(self) -> &'static str {
        match self {
            Self::Root => "OPTIONS",
            Self::Preferences => "PREFERENCES",
            Self::Controls => "CONTROLS",
            Self::SaveData => "SAVE DATA OPTIONS",
        }
    }

    fn items(self) -> &'static [&'static str] {
        match self {
            Self::Root => ROOT_ITEMS.as_slice(),
            Self::Preferences => PREFERENCE_ITEMS.as_slice(),
            Self::Controls => CONTROL_ITEMS.as_slice(),
            Self::SaveData => SAVE_DATA_ITEMS.as_slice(),
        }
    }
}

const ROOT_ITEMS: [&str; 4] = ["PREFERENCES", "CONTROLS", "SAVE DATA OPTIONS", "EXIT"];
const ROOT_ACTIONS: [OptionsMenuAction; 4] = [
    OptionsMenuAction::Page(OptionsMenuPage::Preferences),
    OptionsMenuAction::Page(OptionsMenuPage::Controls),
    OptionsMenuAction::Page(OptionsMenuPage::SaveData),
    OptionsMenuAction::Exit,
];
const PREFERENCE_ITEMS: [&str; 5] = [
    "DOWN SCROLL        OFF",
    "FLASHING LIGHTS    ON",
    "CAMERA ZOOM        ON",
    "AUTO PAUSE         ON",
    "BACK",
];
const CONTROL_ITEMS: [&str; 5] = [
    "LEFT        A / LEFT",
    "DOWN        S / DOWN",
    "UP          W / UP",
    "RIGHT       D / RIGHT",
    "BACK",
];
const SAVE_DATA_ITEMS: [&str; 2] = ["CLEAR SAVE DATA", "BACK"];

fn load_background(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<OptionsBackground> {
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
    Ok(OptionsBackground { texture_id, size })
}

fn push_title(commands: &mut TextCommandList, title: &str) {
    let mut cmd = TextCommand::new(title, glam::vec2(0.0, 76.0), 64.0);
    cmd.max_width = Some(1280.0);
    cmd.color = glam::vec4(1.0, 1.0, 1.0, 0.95);
    cmd.z = 100;
    commands.push(cmd);
}

fn push_item(commands: &mut TextCommandList, label: &str, index: usize, selected_index: usize) {
    let selected = index == selected_index;
    let mut cmd = TextCommand::new(
        format!("{}{}", if selected { "> " } else { "  " }, label),
        glam::vec2(420.0, 190.0 + index as f32 * 76.0),
        if selected { 42.0 } else { 34.0 },
    );
    cmd.color = if selected {
        glam::vec4(1.0, 0.86, 0.24, 1.0)
    } else {
        glam::vec4(0.95, 0.95, 1.0, 0.78)
    };
    cmd.z = 110;
    commands.push(cmd);
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
    fn root_options_match_og_codex_order() {
        let assets = OptionsMenuAssets {
            background: OptionsBackground {
                texture_id: AssetId::new(1),
                size: glam::vec2(1280.0, 720.0),
            },
            textures: HashMap::new(),
        };

        assert_eq!(assets.item_count(OptionsMenuPage::Root), 4);
        assert_eq!(
            assets.action_for_root(0),
            Some(OptionsMenuAction::Page(OptionsMenuPage::Preferences))
        );
        assert_eq!(assets.action_for_root(3), Some(OptionsMenuAction::Exit));
    }

    #[test]
    fn text_commands_highlight_selected_item() {
        let assets = OptionsMenuAssets {
            background: OptionsBackground {
                texture_id: AssetId::new(1),
                size: glam::vec2(1280.0, 720.0),
            },
            textures: HashMap::new(),
        };

        let commands = assets.text_commands(OptionsMenuPage::Root, 1);
        let selected = commands
            .iter()
            .find(|cmd| cmd.text.starts_with(">"))
            .map(|cmd| cmd.text.as_str());
        assert_eq!(selected, Some("> CONTROLS"));
    }
}
