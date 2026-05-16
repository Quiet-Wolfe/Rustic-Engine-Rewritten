//! Credits screen shell based on Funkin' v0.8.5 `CreditsState`.
//!
//! The full game builds the list incrementally from `credits.json`; this
//! first pass renders the opening credits entries and scrolls them at the
//! source base speed.
//!
//! ref: bdedc0aa:source/funkin/ui/credits/CreditsState.hx:16-294

use crate::asset_roots::baked_assets_root;
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{
    DrawCommand, FilterMode, RenderCommandList, TextCommand, TextCommandList, Texture,
};
use serde::Deserialize;
use std::collections::HashMap;

const WHITE_TEXTURE_ID: AssetId = AssetId::new(0x6372_6564_6974_0001);
const CREDITS_SCROLL_BASE_SPEED: f32 = 100.0;
const CREDITS_X: f32 = 24.0;
const CREDITS_WIDTH: f32 = 1280.0 - 48.0;
const HEADER_SIZE: f32 = 32.0;
const BODY_SIZE: f32 = 24.0;

#[derive(Debug)]
pub struct CreditsAssets {
    entries: Vec<CreditEntry>,
    pub textures: HashMap<AssetId, Texture>,
}

impl CreditsAssets {
    pub fn commands(&self) -> RenderCommandList {
        let mut commands = RenderCommandList::new();
        let mut cmd = DrawCommand::sprite(
            WHITE_TEXTURE_ID,
            glam::Vec2::ZERO,
            glam::vec2(1280.0, 720.0),
        );
        cmd.camera = CameraId(1);
        cmd.layer = RenderLayer::Background;
        cmd.z = -10;
        cmd.pivot = glam::Vec2::ZERO;
        cmd.filter = FilterMode::Nearest;
        cmd.color = glam::vec4(0.0, 0.0, 0.0, 1.0);
        commands.push(cmd);
        commands
    }

    pub fn text_commands(&self, cursor: Samples, sample_rate: u32) -> TextCommandList {
        let mut commands = TextCommandList::new();
        let scroll_y =
            720.0 - cursor.0.max(0) as f32 * CREDITS_SCROLL_BASE_SPEED / sample_rate.max(1) as f32;
        let mut y = scroll_y;
        for entry in &self.entries {
            if let Some(header) = entry.header.as_deref() {
                push_line(&mut commands, header, y, true);
                y += HEADER_SIZE * 2.0;
            }
            for line in &entry.body {
                push_line(&mut commands, line, y, false);
                y += BODY_SIZE;
            }
            y += BODY_SIZE * 2.5;
        }
        commands
    }
}

pub fn load_credits_assets(device: &wgpu::Device, queue: &wgpu::Queue) -> CreditsAssets {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let entries = match load_credits_entries(&resolver) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "credits data unavailable: {e:#}");
            fallback_credits()
        }
    };
    let mut textures = HashMap::new();
    textures.insert(
        WHITE_TEXTURE_ID,
        Texture::from_rgba8(
            device,
            queue,
            &[255, 255, 255, 255],
            1,
            1,
            FilterMode::Nearest,
            Some("rustic.credits.white"),
        ),
    );
    CreditsAssets { entries, textures }
}

#[derive(Debug)]
struct CreditEntry {
    header: Option<String>,
    body: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreditsData {
    entries: Vec<CreditsDataEntry>,
}

#[derive(Debug, Deserialize)]
struct CreditsDataEntry {
    #[serde(default)]
    header: Option<String>,
    #[serde(default)]
    body: Vec<CreditsDataLine>,
}

#[derive(Debug, Deserialize)]
struct CreditsDataLine {
    line: String,
}

#[derive(Debug)]
struct StaticCreditEntry {
    header: &'static str,
    body: &'static [&'static str],
}

const FALLBACK_CREDITS: &[StaticCreditEntry] = &[
    StaticCreditEntry {
        header: "Friday Night Funkin'",
        body: &["A video game created by", "The Funkin' Crew Inc."],
    },
    StaticCreditEntry {
        header: "The Funkin' Crew Inc. Shareholders",
        body: &["ninjamuffin99", "PhantomArcade", "Kawai Sprite", "evilsk8r"],
    },
    StaticCreditEntry {
        header: "Direction and Art Lead",
        body: &["PhantomArcade"],
    },
    StaticCreditEntry {
        header: "Music Lead",
        body: &["Isaac \"Kawai Sprite\" Garcia"],
    },
    StaticCreditEntry {
        header: "Co-Direction and Programming Lead",
        body: &["ninjamuffin99"],
    },
    StaticCreditEntry {
        header: "Production Manager",
        body: &["Hundrec"],
    },
    StaticCreditEntry {
        header: "Artists",
        body: &["PhantomArcade", "evilsk8r", "beck"],
    },
];

fn load_credits_entries(resolver: &OverlayResolver) -> anyhow::Result<Vec<CreditEntry>> {
    let path = AssetPath::new("data/credits.json")?;
    let bytes = load_bytes(resolver, &path)?;
    parse_credits_entries(&bytes)
}

fn parse_credits_entries(bytes: &[u8]) -> anyhow::Result<Vec<CreditEntry>> {
    let data: CreditsData = serde_json::from_slice(bytes)?;
    Ok(data
        .entries
        .into_iter()
        .map(|entry| CreditEntry {
            header: entry.header,
            body: entry.body.into_iter().map(|line| line.line).collect(),
        })
        .collect())
}

fn fallback_credits() -> Vec<CreditEntry> {
    FALLBACK_CREDITS
        .iter()
        .map(|entry| CreditEntry {
            header: Some(entry.header.to_string()),
            body: entry.body.iter().map(|line| (*line).to_string()).collect(),
        })
        .collect()
}

fn push_line(commands: &mut TextCommandList, text: &str, y: f32, header: bool) {
    let mut cmd = TextCommand::new(
        text,
        glam::vec2(CREDITS_X, y),
        if header { HEADER_SIZE } else { BODY_SIZE },
    );
    cmd.max_width = Some(CREDITS_WIDTH);
    cmd.color = glam::Vec4::ONE;
    cmd.z = if header { 110 } else { 100 };
    commands.push(cmd);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credits_start_below_screen_and_scroll_up() {
        let assets = CreditsAssets {
            entries: fallback_credits(),
            textures: HashMap::new(),
        };
        let start = assets.text_commands(Samples(0), 48_000);
        let later = assets.text_commands(Samples(48_000), 48_000);

        assert_eq!(start.as_slice()[0].position.y, 720.0);
        assert_eq!(later.as_slice()[0].position.y, 620.0);
    }

    #[test]
    fn parses_upstream_credits_data() {
        let entries = parse_credits_entries(
            br#"{
                "entries": [
                    {
                        "header": "Mobile Lead",
                        "body": [{ "line": "MoonDroid (Zack)" }]
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(entries[0].header.as_deref(), Some("Mobile Lead"));
        assert_eq!(entries[0].body, vec!["MoonDroid (Zack)"]);
    }
}
