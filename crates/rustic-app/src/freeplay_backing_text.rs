use super::{FreeplayAssets, BACKING_TEXT_GAP, PINKBACK_TARGET_HEIGHT};
use crate::bitmap_text_assets::BitmapTextDraw;
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::RenderCommandList;

impl FreeplayAssets {
    pub(super) fn push_backing_text(
        &self,
        commands: &mut RenderCommandList,
        cursor: Samples,
        sample_rate: u32,
    ) {
        let Some(skin) = self.backing_text_skin.as_ref() else {
            return;
        };
        // ref: bdedc0aa:assets/preload/data/players/bf.json:35-37
        // ref: bdedc0aa:source/funkin/ui/freeplay/BGScrollingText.hx:10-87
        // ref: bdedc0aa:assets/preload/scripts/players/backcards/backcard-bf.hxc:77-113
        const ROWS: [(&str, f32, f32, glam::Vec4, f32); 6] = [
            (
                "HOT BLOODED IN MORE WAYS THAN ONE",
                160.0,
                43.0,
                glam::Vec4::new(1.0, 0xF3 as f32 / 255.0, 0x83 as f32 / 255.0, 0.55),
                6.8,
            ),
            (
                "BOYFRIEND",
                220.0,
                60.0,
                glam::Vec4::new(1.0, 0x99 as f32 / 255.0, 0x63 as f32 / 255.0, 0.52),
                -3.8,
            ),
            (
                "PROTECT YO NUTS",
                285.0,
                43.0,
                glam::Vec4::new(1.0, 1.0, 1.0, 0.75),
                3.5,
            ),
            (
                "BOYFRIEND",
                335.0,
                60.0,
                glam::Vec4::new(1.0, 0x99 as f32 / 255.0, 0x63 as f32 / 255.0, 0.52),
                -3.8,
            ),
            (
                "HOT BLOODED IN MORE WAYS THAN ONE",
                397.0,
                43.0,
                glam::Vec4::new(1.0, 0xF3 as f32 / 255.0, 0x83 as f32 / 255.0, 0.55),
                6.8,
            ),
            (
                "BOYFRIEND",
                450.0,
                60.0,
                glam::Vec4::new(1.0, 0xA4 as f32 / 255.0, 0.0, 0.52),
                -3.8,
            ),
        ];
        let seconds = cursor.0.max(0) as f64 / f64::from(sample_rate.max(1));
        for (text, y, size, color, speed) in ROWS {
            let width = estimated_text_width(text, size);
            let span = width + BACKING_TEXT_GAP;
            let scroll = (seconds * f64::from(speed) * 60.0) as f32;
            let count = (PINKBACK_TARGET_HEIGHT / span).ceil() as i32 + 4;
            for i in -1..count {
                let origin = glam::vec2(i as f32 * span - scroll.rem_euclid(span), y);
                for cmd in skin.commands_with(
                    text,
                    BitmapTextDraw {
                        origin,
                        scale: size / 16.0,
                        letter_spacing: 0,
                        color,
                        layer: RenderLayer::Background,
                        z: -82,
                    },
                ) {
                    commands.push(cmd);
                }
            }
        }
    }
}

fn estimated_text_width(text: &str, size_px: f32) -> f32 {
    text.chars().count() as f32 * size_px * 0.62
}
