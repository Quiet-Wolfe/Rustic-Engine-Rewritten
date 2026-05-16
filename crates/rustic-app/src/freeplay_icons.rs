//! Pixelated Freeplay capsule icons.
//!
//! ref: bdedc0aa:source/funkin/ui/PixelatedIcon.hx
//! ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:207

use super::helpers::{asset_id_for_path, sparrow_scaled_command, SparrowAtlasHandle};
use anyhow::{Context, Result};
use rustic_asset::{load_png, load_sparrow, AssetPath, OverlayResolver, SparrowFrame};
use rustic_core::time::Samples;
use rustic_render::{FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;

const ICON_ANIM_FPS: u16 = 10;
const ICON_X: f32 = 160.0;
const ICON_Y: f32 = 35.0;
const ICON_SCALE: f32 = 2.0;

const KNOWN_FREEPLAY_ICON_IDS: &[&str] = &[
    "bfpixel",
    "dadpixel",
    "darnellpixel",
    "gfpixel",
    "mompixel",
    "monsterpixel",
    "parents-christmaspixel",
    "picopixel",
    "senpaipixel",
    "spiritpixel",
    "spookypixel",
    "sserafim-kazuhapixel",
    "tankmanpixel",
];

#[derive(Debug, Default)]
pub(super) struct FreeplayIconAssets {
    icons: HashMap<String, FreeplayIcon>,
}

impl FreeplayIconAssets {
    pub(super) fn push_capsule_icon(
        &self,
        commands: &mut RenderCommandList,
        character_id: Option<&str>,
        pos: glam::Vec2,
        alpha: f32,
        cursor: Samples,
        sample_rate: u32,
        z: i32,
    ) {
        let Some(character_id) = character_id else {
            return;
        };
        let icon_key = freeplay_icon_key_for_character(character_id);
        let Some(icon) = self.icons.get(&icon_key) else {
            return;
        };
        let Some(frame) = icon.frame(cursor, sample_rate) else {
            return;
        };
        let mut command = sparrow_scaled_command(
            icon.atlas.texture_id,
            icon.atlas.width,
            icon.atlas.height,
            frame,
            pos + icon.position(character_id),
            glam::Vec2::splat(ICON_SCALE),
            glam::vec4(1.0, 1.0, 1.0, alpha),
            z,
        );
        command.filter = FilterMode::Nearest;
        commands.push(command);
    }
}

#[derive(Debug)]
struct FreeplayIcon {
    atlas: SparrowAtlasHandle,
    idle_frames: Vec<SparrowFrame>,
}

impl FreeplayIcon {
    fn frame(&self, cursor: Samples, sample_rate: u32) -> Option<&SparrowFrame> {
        if self.idle_frames.is_empty() {
            return None;
        }
        let elapsed = cursor.0.max(0) as u128;
        let index = (elapsed * u128::from(ICON_ANIM_FPS) / u128::from(sample_rate.max(1))) as usize;
        self.idle_frames.get(index % self.idle_frames.len())
    }

    fn position(&self, character_id: &str) -> glam::Vec2 {
        glam::vec2(ICON_X - icon_origin_x(character_id), ICON_Y)
    }
}

pub(super) fn load_freeplay_icons(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<rustic_core::ids::AssetId, Texture>,
) -> FreeplayIconAssets {
    let mut icons = HashMap::new();
    for id in KNOWN_FREEPLAY_ICON_IDS {
        match load_freeplay_icon(device, queue, resolver, textures, id) {
            Ok(icon) => {
                icons.insert((*id).to_string(), icon);
            }
            Err(error) => {
                tracing::warn!(
                    target: "rustic.asset",
                    "freeplay icon {id} unavailable: {error:#}"
                );
            }
        }
    }
    FreeplayIconAssets { icons }
}

fn load_freeplay_icon(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<rustic_core::ids::AssetId, Texture>,
    id: &str,
) -> Result<FreeplayIcon> {
    let xml_path = AssetPath::new(format!("images/freeplay/icons/{id}.xml"))?;
    let atlas = load_sparrow(resolver, &xml_path).with_context(|| format!("load {xml_path}"))?;
    let png_path = AssetPath::new(format!("images/freeplay/icons/{id}.png"))?;
    let image = load_png(resolver, &png_path).with_context(|| format!("load {png_path}"))?;
    let texture_id = asset_id_for_path(&png_path);
    let (width, height) = (image.width, image.height);
    textures.insert(
        texture_id,
        Texture::from_png_image(
            device,
            queue,
            &image,
            FilterMode::Nearest,
            Some(png_path.as_str()),
        ),
    );
    let idle_frames = atlas
        .animation_frames("idle", &[])
        .into_iter()
        .cloned()
        .collect();
    Ok(FreeplayIcon {
        atlas: SparrowAtlasHandle {
            texture_id,
            width,
            height,
        },
        idle_frames,
    })
}

pub(super) fn freeplay_icon_key_for_character(character_id: &str) -> String {
    let root = match character_id {
        "darnell-blazin" => "darnell",
        "mom-car" => "mom",
        "monster-christmas" => "monster",
        "pico-playable" | "pico-pixel" => "pico",
        "senpai-angry" => "senpai",
        "spooky-dark" => "spooky",
        "tankman-bloody" => "tankman",
        other => other.strip_suffix("-pixel").unwrap_or(other),
    };
    format!("{root}pixel")
}

fn icon_origin_x(character_id: &str) -> f32 {
    match character_id {
        "parents-christmas" => 140.0,
        "sserafim-kazuha" => 195.0,
        _ => 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_ids_map_to_existing_freeplay_icon_names() {
        assert_eq!(freeplay_icon_key_for_character("bf-pixel"), "bfpixel");
        assert_eq!(
            freeplay_icon_key_for_character("pico-playable"),
            "picopixel"
        );
        assert_eq!(
            freeplay_icon_key_for_character("spooky-dark"),
            "spookypixel"
        );
        assert_eq!(
            freeplay_icon_key_for_character("tankman-bloody"),
            "tankmanpixel"
        );
        assert_eq!(
            freeplay_icon_key_for_character("sserafim-kazuha"),
            "sserafim-kazuhapixel"
        );
    }

    #[test]
    fn known_freeplay_icon_assets_exist_in_source_tree() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest_dir
            .parent()
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| manifest_dir.to_path_buf());
        let source_root = workspace.join("assets/source/images/freeplay/icons");
        let mut missing = Vec::new();
        for id in KNOWN_FREEPLAY_ICON_IDS {
            for extension in ["png", "xml"] {
                let path = source_root.join(format!("{id}.{extension}"));
                if !path.exists() {
                    missing.push(path.display().to_string());
                }
            }
        }
        assert!(
            missing.is_empty(),
            "freeplay icon assets missing:\n{}",
            missing.join("\n")
        );
    }
}
