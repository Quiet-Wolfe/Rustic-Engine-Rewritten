//! Credits screen shell based on Funkin' v0.8.5 `CreditsState`.
//!
//! The full game builds the list incrementally from `credits.json`; this
//! first pass renders the opening credits entries and scrolls them at the
//! source base speed.
//!
//! ref: bdedc0aa:source/funkin/ui/credits/CreditsState.hx:16-294

use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{
    DrawCommand, FilterMode, RenderCommandList, TextCommand, TextCommandList, Texture,
};
use std::collections::HashMap;

const WHITE_TEXTURE_ID: AssetId = AssetId::new(0x6372_6564_6974_0001);
const CREDITS_SCROLL_BASE_SPEED: f32 = 100.0;
const CREDITS_X: f32 = 24.0;
const CREDITS_WIDTH: f32 = 1280.0 - 48.0;
const HEADER_SIZE: f32 = 32.0;
const BODY_SIZE: f32 = 24.0;

#[derive(Debug)]
pub struct CreditsAssets {
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
        for entry in CREDITS {
            push_line(&mut commands, entry.header, y, true);
            y += HEADER_SIZE * 2.0;
            for line in entry.body {
                push_line(&mut commands, line, y, false);
                y += BODY_SIZE;
            }
            y += BODY_SIZE * 2.5;
        }
        commands
    }
}

pub fn load_credits_assets(device: &wgpu::Device, queue: &wgpu::Queue) -> CreditsAssets {
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
    CreditsAssets { textures }
}

#[derive(Debug)]
struct CreditEntry {
    header: &'static str,
    body: &'static [&'static str],
}

const CREDITS: &[CreditEntry] = &[
    CreditEntry {
        header: "Friday Night Funkin'",
        body: &["A video game created by", "The Funkin' Crew Inc."],
    },
    CreditEntry {
        header: "The Funkin' Crew Inc. Shareholders",
        body: &["ninjamuffin99", "PhantomArcade", "Kawai Sprite", "evilsk8r"],
    },
    CreditEntry {
        header: "Direction and Art Lead",
        body: &["PhantomArcade"],
    },
    CreditEntry {
        header: "Music Lead",
        body: &["Isaac \"Kawai Sprite\" Garcia"],
    },
    CreditEntry {
        header: "Co-Direction and Programming Lead",
        body: &["ninjamuffin99"],
    },
    CreditEntry {
        header: "Production Manager",
        body: &["Hundrec"],
    },
    CreditEntry {
        header: "Artists",
        body: &["PhantomArcade", "evilsk8r", "beck"],
    },
];

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
            textures: HashMap::new(),
        };
        let start = assets.text_commands(Samples(0), 48_000);
        let later = assets.text_commands(Samples(48_000), 48_000);

        assert_eq!(start.as_slice()[0].position.y, 720.0);
        assert_eq!(later.as_slice()[0].position.y, 620.0);
    }
}
