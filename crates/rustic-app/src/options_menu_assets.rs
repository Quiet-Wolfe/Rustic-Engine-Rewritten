//! Options-menu shell based on Funkin' v0.8.5 `OptionsState`.
//!
//! This wires the main options codex pages and stateful desktop preference rows.
//!
//! ref: bdedc0aa:source/funkin/ui/options/OptionsState.hx:46-101,150-332

use crate::asset_roots::baked_assets_root;
use crate::options_preferences::{OptionsPreferences, PREFERENCE_ITEM_COUNT};
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
        self.text_commands_with_preferences(page, selected_index, OptionsPreferences::default())
    }

    pub(crate) fn text_commands_with_preferences(
        &self,
        page: OptionsMenuPage,
        selected_index: usize,
        preferences: OptionsPreferences,
    ) -> TextCommandList {
        let mut commands = TextCommandList::new();
        push_title(&mut commands, page.title());
        let item_count = self.item_count(page);
        for index in 0..item_count {
            let Some(item) = item_label(page, index, preferences) else {
                continue;
            };
            push_item(
                &mut commands,
                item.as_str(),
                index,
                selected_index,
                item_count,
            );
        }
        commands
    }

    pub fn item_count(&self, page: OptionsMenuPage) -> usize {
        match page {
            OptionsMenuPage::Preferences => PREFERENCE_ITEM_COUNT,
            _ => page.items().len(),
        }
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
            Self::Preferences => &[],
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
const CONTROL_ITEMS: [&str; 5] = [
    "LEFT        A / LEFT",
    "DOWN        S / DOWN",
    "UP          W / UP",
    "RIGHT       D / RIGHT",
    "BACK",
];
const SAVE_DATA_ITEMS: [&str; 2] = ["CLEAR SAVE DATA", "BACK"];

fn item_label(
    page: OptionsMenuPage,
    index: usize,
    preferences: OptionsPreferences,
) -> Option<String> {
    match page {
        OptionsMenuPage::Preferences => preferences.label_for(index),
        _ => page.items().get(index).map(|item| (*item).to_string()),
    }
}

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

fn push_item(
    commands: &mut TextCommandList,
    label: &str,
    index: usize,
    selected_index: usize,
    item_count: usize,
) {
    let selected = index == selected_index;
    let dense = item_count > 7;
    let y = if dense {
        150.0 + index as f32 * 48.0
    } else {
        190.0 + index as f32 * 76.0
    };
    let mut cmd = TextCommand::new(
        format!("{}{}", if selected { "> " } else { "  " }, label),
        glam::vec2(380.0, y),
        if selected && dense {
            34.0
        } else if selected {
            42.0
        } else if dense {
            30.0
        } else {
            34.0
        },
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
    fn preferences_include_desktop_og_toggle_rows() {
        let assets = OptionsMenuAssets {
            background: OptionsBackground {
                texture_id: AssetId::new(1),
                size: glam::vec2(1280.0, 720.0),
            },
            textures: HashMap::new(),
        };
        let preferences = OptionsPreferences::default();

        assert_eq!(assets.item_count(OptionsMenuPage::Preferences), 11);
        assert_eq!(
            item_label(OptionsMenuPage::Preferences, 0, preferences).as_deref(),
            Some("DOWNSCROLL             OFF")
        );
        assert_eq!(
            item_label(OptionsMenuPage::Preferences, 1, preferences).as_deref(),
            Some("STRUMLINE BACKGROUND   0%")
        );
        assert_eq!(
            item_label(OptionsMenuPage::Preferences, 7, preferences).as_deref(),
            Some("VSYNC                  ON")
        );
        assert_eq!(
            item_label(OptionsMenuPage::Preferences, 10, preferences).as_deref(),
            Some("BACK")
        );
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
