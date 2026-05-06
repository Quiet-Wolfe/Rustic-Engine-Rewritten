//! Score popup asset loading and transient draw commands.

use anyhow::{Context, Result};
use rustic_asset::{load_png, AssetPath, OverlayResolver};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_game::Judgment;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const FNF_WIDTH: f32 = 1280.0;
const FNF_HEIGHT: f32 = 720.0;
const COOL_TEXT_X: f32 = FNF_WIDTH * 0.55;
const RATING_SCALE: f32 = 0.7;
const DIGIT_SCALE: f32 = 0.5;
const DIGIT_SPACING: f32 = 43.0;
const POPUP_LIFETIME_SECS: f32 = 0.8;
const POPUP_FADE_SECS: f32 = 0.2;
const POPUP_RISE_PIXELS: f32 = 70.0;

#[derive(Debug, Clone)]
pub struct PopupSkin {
    ratings: [PopupImage; 4],
    digits: [PopupImage; 10],
}

#[derive(Debug, Clone, Copy)]
struct PopupImage {
    texture_id: AssetId,
    size: glam::Vec2,
}

#[derive(Debug, Default, Clone)]
pub struct ScorePopups {
    active: Vec<ScorePopup>,
}

#[derive(Debug, Clone, Copy)]
struct ScorePopup {
    judgment: Judgment,
    combo: u32,
    started_at: Samples,
}

impl ScorePopups {
    pub fn push(&mut self, judgment: Judgment, combo: u32, cursor: Samples) {
        if judgment == Judgment::Miss {
            return;
        }
        self.active.push(ScorePopup {
            judgment,
            combo,
            started_at: cursor,
        });
    }

    pub fn commands(
        &mut self,
        skin: &PopupSkin,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        let lifetime = (POPUP_LIFETIME_SECS * sample_rate as f32).round() as i64;
        self.active
            .retain(|popup| cursor.0.saturating_sub(popup.started_at.0) <= lifetime);

        let mut commands = Vec::new();
        for popup in &self.active {
            if cursor < popup.started_at {
                continue;
            }
            let age_samples = cursor.0 - popup.started_at.0;
            let age_secs = age_samples as f32 / sample_rate.max(1) as f32;
            let alpha = popup_alpha(age_secs);
            let rise = (age_secs / POPUP_LIFETIME_SECS).clamp(0.0, 1.0) * POPUP_RISE_PIXELS;

            if let Some(rating) = skin.rating(popup.judgment) {
                commands.push(sprite_command(
                    rating,
                    glam::vec2(COOL_TEXT_X - 40.0, centered_y(rating) - 60.0 - rise),
                    RATING_SCALE,
                    alpha,
                    20,
                ));
            }

            if popup.combo >= 10 || popup.combo == 0 {
                for (index, digit) in combo_digits(popup.combo).into_iter().enumerate() {
                    let image = skin.digits[digit as usize];
                    commands.push(sprite_command(
                        image,
                        glam::vec2(
                            COOL_TEXT_X + DIGIT_SPACING * index as f32 - 90.0,
                            centered_y(image) + 80.0 - rise,
                        ),
                        DIGIT_SCALE,
                        alpha,
                        21 + index as i32,
                    ));
                }
            }
        }
        commands
    }
}

impl PopupSkin {
    fn rating(&self, judgment: Judgment) -> Option<PopupImage> {
        match judgment {
            Judgment::Sick => Some(self.ratings[0]),
            Judgment::Good => Some(self.ratings[1]),
            Judgment::Bad => Some(self.ratings[2]),
            Judgment::Shit => Some(self.ratings[3]),
            Judgment::Miss => None,
            _ => None,
        }
    }
}

pub fn load_popup_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<PopupSkin> {
    Ok(PopupSkin {
        ratings: [
            load_popup_image(device, queue, resolver, textures, "images/sick.png")?,
            load_popup_image(device, queue, resolver, textures, "images/good.png")?,
            load_popup_image(device, queue, resolver, textures, "images/bad.png")?,
            load_popup_image(device, queue, resolver, textures, "images/shit.png")?,
        ],
        digits: [
            load_popup_image(device, queue, resolver, textures, "images/num0.png")?,
            load_popup_image(device, queue, resolver, textures, "images/num1.png")?,
            load_popup_image(device, queue, resolver, textures, "images/num2.png")?,
            load_popup_image(device, queue, resolver, textures, "images/num3.png")?,
            load_popup_image(device, queue, resolver, textures, "images/num4.png")?,
            load_popup_image(device, queue, resolver, textures, "images/num5.png")?,
            load_popup_image(device, queue, resolver, textures, "images/num6.png")?,
            load_popup_image(device, queue, resolver, textures, "images/num7.png")?,
            load_popup_image(device, queue, resolver, textures, "images/num8.png")?,
            load_popup_image(device, queue, resolver, textures, "images/num9.png")?,
        ],
    })
}

fn load_popup_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    path: &str,
) -> Result<PopupImage> {
    let path = AssetPath::new(path)?;
    let image = load_png(resolver, &path).with_context(|| format!("load {}", path.as_str()))?;
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
    Ok(PopupImage {
        texture_id,
        size: glam::vec2(image.width as f32, image.height as f32),
    })
}

fn sprite_command(
    image: PopupImage,
    world_pos: glam::Vec2,
    scale: f32,
    alpha: f32,
    z: i32,
) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(image.texture_id, world_pos, image.size * scale);
    cmd.camera = CameraId(0);
    cmd.pivot = glam::Vec2::ZERO;
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.filter = FilterMode::Linear;
    cmd.color.w = alpha;
    cmd
}

fn centered_y(image: PopupImage) -> f32 {
    (FNF_HEIGHT - image.size.y) * 0.5
}

fn combo_digits(combo: u32) -> [u32; 3] {
    [combo / 100, (combo % 100) / 10, combo % 10]
}

fn popup_alpha(age_secs: f32) -> f32 {
    let fade_start = POPUP_LIFETIME_SECS - POPUP_FADE_SECS;
    if age_secs <= fade_start {
        1.0
    } else {
        1.0 - ((age_secs - fade_start) / POPUP_FADE_SECS).clamp(0.0, 1.0)
    }
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

    fn image(id: u64, width: f32, height: f32) -> PopupImage {
        PopupImage {
            texture_id: AssetId::new(id),
            size: glam::vec2(width, height),
        }
    }

    fn skin() -> PopupSkin {
        PopupSkin {
            ratings: [
                image(1, 403.0, 152.0),
                image(2, 317.0, 126.0),
                image(3, 261.0, 131.0),
                image(4, 285.0, 163.0),
            ],
            digits: [
                image(10, 94.0, 119.0),
                image(11, 98.0, 120.0),
                image(12, 105.0, 129.0),
                image(13, 102.0, 134.0),
                image(14, 98.0, 130.0),
                image(15, 111.0, 135.0),
                image(16, 108.0, 134.0),
                image(17, 91.0, 111.0),
                image(18, 90.0, 115.0),
                image(19, 91.0, 124.0),
            ],
        }
    }

    #[test]
    fn popup_renders_rating_and_three_digits_for_zero_combo() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Sick, 0, Samples(100));

        let commands = popups.commands(&skin(), Samples(100), 48_000);

        assert_eq!(commands.len(), 4);
        assert_eq!(commands[0].texture, AssetId::new(1));
        assert_eq!(commands[1].texture, AssetId::new(10));
        assert_eq!(commands[2].texture, AssetId::new(10));
        assert_eq!(commands[3].texture, AssetId::new(10));
    }

    #[test]
    fn single_digit_combo_only_renders_rating_like_og() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Good, 1, Samples(0));

        let commands = popups.commands(&skin(), Samples(0), 48_000);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].texture, AssetId::new(2));
    }

    #[test]
    fn expired_popups_are_removed() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Bad, 10, Samples(0));

        let commands = popups.commands(&skin(), Samples(48_000), 48_000);

        assert!(commands.is_empty());
    }
}
