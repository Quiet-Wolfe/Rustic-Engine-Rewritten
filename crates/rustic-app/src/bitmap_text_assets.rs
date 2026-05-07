//! Bitmap VCR text drawing backed by the OG AngelCode font assets.
//!
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:815,2015-2024,2702-2713

use anyhow::{Context, Result};
use rustic_asset::{
    load_bitmap_font, load_png, AssetPath, BitmapFont, BitmapGlyph, OverlayResolver,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const HEALTH_BAR_X: f32 = (1280.0 - 601.0) * 0.5;
const HEALTH_BAR_Y: f32 = 720.0 * 0.9;
const SCORE_TEXT_RIGHT_X: f32 = HEALTH_BAR_X + 601.0;
const SCORE_TEXT_Y: f32 = HEALTH_BAR_Y + 30.0;
const SCORE_TEXT_Z: i32 = 10;
const SCORE_TEXT_LETTER_SPACING: i32 = -1;
const SCORE_TEXT_SCALE: f32 = 1.0;
const SCORE_TEXT_COLOR: glam::Vec4 = glam::vec4(1.0, 1.0, 1.0, 1.0);
const SCORE_TEXT_OUTLINE: glam::Vec4 = glam::vec4(0.0, 0.0, 0.0, 1.0);
const OUTLINE_OFFSETS: [glam::Vec2; 4] = [
    glam::vec2(-1.0, 0.0),
    glam::vec2(1.0, 0.0),
    glam::vec2(0.0, -1.0),
    glam::vec2(0.0, 1.0),
];

#[derive(Debug, Clone)]
pub struct BitmapTextSkin {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    font: BitmapFont,
}

impl BitmapTextSkin {
    pub fn score_text_commands(&self, score: i64) -> Vec<DrawCommand> {
        self.outlined_commands_right_aligned(
            &format!("Score: {}", format_money(score)),
            SCORE_TEXT_RIGHT_X,
            SCORE_TEXT_Y,
            SCORE_TEXT_SCALE,
            SCORE_TEXT_LETTER_SPACING,
            SCORE_TEXT_Z,
        )
    }

    fn outlined_commands_right_aligned(
        &self,
        text: &str,
        right_x: f32,
        y: f32,
        scale: f32,
        letter_spacing: i32,
        z: i32,
    ) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        for offset in OUTLINE_OFFSETS {
            commands.extend(self.commands_right_aligned(
                text,
                glam::vec2(right_x + offset.x, y + offset.y),
                scale,
                letter_spacing,
                SCORE_TEXT_OUTLINE,
                z,
            ));
        }
        commands.extend(self.commands_right_aligned(
            text,
            glam::vec2(right_x, y),
            scale,
            letter_spacing,
            SCORE_TEXT_COLOR,
            z + 1,
        ));
        commands
    }

    fn commands_right_aligned(
        &self,
        text: &str,
        right_pos: glam::Vec2,
        scale: f32,
        letter_spacing: i32,
        color: glam::Vec4,
        z: i32,
    ) -> Vec<DrawCommand> {
        let line_width = self.measure_line(text, letter_spacing) * scale;
        self.commands(
            text,
            glam::vec2(right_pos.x - line_width, right_pos.y),
            scale,
            letter_spacing,
            color,
            z,
        )
    }

    fn commands(
        &self,
        text: &str,
        origin: glam::Vec2,
        scale: f32,
        letter_spacing: i32,
        color: glam::Vec4,
        z: i32,
    ) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        let mut cursor = glam::Vec2::ZERO;
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\n' {
                cursor.x = 0.0;
                cursor.y += self.font.line_height as f32 + letter_spacing as f32;
                continue;
            }

            let Some(glyph) = self.glyph(ch) else {
                cursor.x += self.missing_advance();
                if chars.peek().is_some() {
                    cursor.x += letter_spacing as f32;
                }
                continue;
            };

            if glyph.width > 0 && glyph.height > 0 && glyph.page == 0 {
                commands.push(self.glyph_command(glyph, origin, cursor, scale, color, z));
            }

            cursor.x += glyph.xadvance as f32;
            if chars.peek().is_some() {
                cursor.x += letter_spacing as f32;
            }
        }

        commands
    }

    fn glyph_command(
        &self,
        glyph: &BitmapGlyph,
        origin: glam::Vec2,
        cursor: glam::Vec2,
        scale: f32,
        color: glam::Vec4,
        z: i32,
    ) -> DrawCommand {
        let world_pos = origin
            + glam::vec2(
                cursor.x + glyph.xoffset as f32,
                cursor.y + glyph.yoffset as f32,
            ) * scale;
        let mut cmd = DrawCommand::sprite(
            self.texture_id,
            world_pos,
            glam::vec2(glyph.width as f32, glyph.height as f32) * scale,
        );
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Hud;
        cmd.z = z;
        cmd.filter = FilterMode::Nearest;
        cmd.color = color;
        (cmd.uv_min, cmd.uv_max) = glyph_uv(glyph, self.texture_width, self.texture_height);
        cmd
    }

    fn measure_line(&self, text: &str, letter_spacing: i32) -> f32 {
        let chars: Vec<_> = text.chars().take_while(|ch| *ch != '\n').collect();
        let mut width = 0.0;
        for (index, ch) in chars.iter().enumerate() {
            width += self
                .glyph(*ch)
                .map(|glyph| glyph.xadvance as f32)
                .unwrap_or_else(|| self.missing_advance());
            if index + 1 < chars.len() {
                width += letter_spacing as f32;
            }
        }
        width.max(0.0)
    }

    fn glyph(&self, ch: char) -> Option<&BitmapGlyph> {
        self.font.glyph(ch as u32)
    }

    fn missing_advance(&self) -> f32 {
        self.font
            .glyph(' ' as u32)
            .map(|glyph| glyph.xadvance as f32)
            .unwrap_or((self.font.size / 2).max(1) as f32)
    }
}

pub fn load_bitmap_text_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<BitmapTextSkin> {
    let font_path = AssetPath::new("fonts/vcr-bmp.fnt")?;
    let font =
        load_bitmap_font(resolver, &font_path).with_context(|| format!("load {}", font_path))?;
    let page = font
        .pages
        .iter()
        .find(|page| page.id == 0)
        .context("bitmap font missing page 0")?;
    let texture_path = font_path.sibling(&page.file)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);

    textures.insert(
        texture_id,
        Texture::from_png_image(
            device,
            queue,
            &image,
            FilterMode::Nearest,
            Some(texture_path.as_str()),
        ),
    );

    Ok(BitmapTextSkin {
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
        font,
    })
}

fn glyph_uv(
    glyph: &BitmapGlyph,
    texture_width: u32,
    texture_height: u32,
) -> (glam::Vec2, glam::Vec2) {
    let width = texture_width.max(1) as f32;
    let height = texture_height.max(1) as f32;
    (
        glam::vec2(glyph.x as f32 / width, glyph.y as f32 / height),
        glam::vec2(
            (glyph.x as f32 + glyph.width as f32) / width,
            (glyph.y as f32 + glyph.height as f32) / height,
        ),
    )
}

fn format_money(score: i64) -> String {
    let sign = if score < 0 { "-" } else { "" };
    let digits = score.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!("{sign}{}", grouped.chars().rev().collect::<String>())
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

    fn test_font() -> BitmapFont {
        BitmapFont::parse(
            br#"
            <font>
              <info face="test" size="16"/>
              <common lineHeight="18" base="14" scaleW="100" scaleH="50"/>
              <pages><page id="0" file="test.png"/></pages>
              <chars count="3">
                <char id="32" x="0" y="0" width="0" height="8"
                  xoffset="0" yoffset="2" xadvance="3" page="0" chnl="15"/>
                <char id="65" x="10" y="0" width="4" height="8"
                  xoffset="1" yoffset="2" xadvance="6" page="0" chnl="15"/>
                <char id="66" x="20" y="0" width="5" height="8"
                  xoffset="0" yoffset="2" xadvance="7" page="0" chnl="15"/>
              </chars>
            </font>
            "#,
        )
        .unwrap()
    }

    fn skin() -> BitmapTextSkin {
        BitmapTextSkin {
            texture_id: AssetId::new(42),
            texture_width: 100,
            texture_height: 50,
            font: test_font(),
        }
    }

    #[test]
    fn score_text_formats_with_commas() {
        assert_eq!(format_money(0), "0");
        assert_eq!(format_money(12_345), "12,345");
        assert_eq!(format_money(-9_876_543), "-9,876,543");
    }

    #[test]
    fn right_aligned_text_uses_xadvance_spacing_and_glyph_offsets() {
        let commands = skin().commands_right_aligned(
            "A B",
            glam::vec2(100.0, 20.0),
            2.0,
            -1,
            glam::Vec4::ONE,
            5,
        );

        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].world_pos, glam::vec2(74.0, 24.0));
        assert_eq!(commands[0].size, glam::vec2(8.0, 16.0));
        assert_eq!(commands[0].uv_min, glam::vec2(0.1, 0.0));
        assert_eq!(commands[0].uv_max, glam::vec2(0.14, 0.16));
        assert_eq!(commands[1].world_pos, glam::vec2(86.0, 24.0));
        assert_eq!(commands[1].z, 5);
        assert_eq!(commands[1].camera, CameraId(1));
        assert_eq!(commands[1].layer, RenderLayer::Hud);
    }

    #[test]
    fn outlined_score_draws_outline_before_fill() {
        let commands = skin().outlined_commands_right_aligned(
            "A",
            SCORE_TEXT_RIGHT_X,
            SCORE_TEXT_Y,
            SCORE_TEXT_SCALE,
            SCORE_TEXT_LETTER_SPACING,
            SCORE_TEXT_Z,
        );
        assert!(!commands.is_empty());
        assert_eq!(commands[0].color, SCORE_TEXT_OUTLINE);
        assert_eq!(commands[0].z, SCORE_TEXT_Z);
        assert_eq!(commands.last().unwrap().color, SCORE_TEXT_COLOR);
        assert_eq!(commands.last().unwrap().z, SCORE_TEXT_Z + 1);
    }
}
