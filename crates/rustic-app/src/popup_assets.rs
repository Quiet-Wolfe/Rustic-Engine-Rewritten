//! Score popup asset loading and transient draw commands.
// LINT-ALLOW: long-file popup motion, atlas loading, and source-aligned tests stay together.

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
const RATING_SCALE: f32 = 0.65;
const DIGIT_SCALE: f32 = 0.45;
const COMBO_DIGIT_SPACING: f32 = 36.0;
const POPUP_FADE_SECS: f32 = 0.2;

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
    combo: Option<u32>,
    started_at: Samples,
}

impl ScorePopups {
    pub fn push(&mut self, judgment: Judgment, combo: Option<u32>, cursor: Samples) {
        if judgment == Judgment::Miss && combo.is_none() {
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
        bpm: f64,
    ) -> Vec<DrawCommand> {
        let beat_secs = 60.0 / bpm.max(1.0) as f32;
        self.active.retain(|popup| {
            let max_delay = if popup.combo.is_some() {
                beat_secs * 2.0
            } else {
                beat_secs
            };
            let lifetime = ((max_delay + POPUP_FADE_SECS) * sample_rate as f32).round() as i64;
            cursor.0.saturating_sub(popup.started_at.0) <= lifetime
        });

        let mut commands = Vec::new();
        for popup in &self.active {
            if cursor < popup.started_at {
                continue;
            }
            let age_samples = cursor.0 - popup.started_at.0;
            let age_secs = age_samples as f32 / sample_rate.max(1) as f32;

            if let Some(rating) = skin.rating(popup.judgment) {
                let alpha = popup_alpha(age_secs, beat_secs);
                let seed = popup_seed(popup, 0);
                let motion = PopupMotion {
                    velocity: glam::vec2(
                        -random_int(seed, 0, 10),
                        -random_int(seed >> 8, 140, 175),
                    ),
                    acceleration_y: 550.0,
                };
                commands.push(sprite_command(
                    rating,
                    popup_position(rating_base(rating), motion, age_secs),
                    RATING_SCALE,
                    alpha,
                    20,
                ));
            }

            if let Some(combo) = popup.combo {
                for (index, digit) in combo_digits(combo).into_iter().enumerate() {
                    let image = skin.digits[digit as usize];
                    let seed = popup_seed(popup, 1 + index as u64);
                    let motion = PopupMotion {
                        velocity: glam::vec2(
                            random_float(seed, -5.0, 5.0),
                            -random_int(seed >> 8, 130, 150),
                        ),
                        acceleration_y: random_int(seed >> 16, 250, 300),
                    };
                    commands.push(sprite_command(
                        image,
                        popup_position(combo_base(index), motion, age_secs),
                        DIGIT_SCALE,
                        popup_alpha(age_secs, beat_secs * 2.0),
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
    // ref: bdedc0aa:source/funkin/play/PlayState.hx:2348-2350
    cmd.camera = CameraId(1);
    cmd.pivot = glam::Vec2::ZERO;
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.filter = FilterMode::Linear;
    cmd.color.w = alpha;
    cmd
}

#[derive(Debug, Clone, Copy)]
struct PopupMotion {
    velocity: glam::Vec2,
    acceleration_y: f32,
}

fn rating_base(image: PopupImage) -> glam::Vec2 {
    // ref: bdedc0aa:source/funkin/play/components/PopUpStuff.hx:40-43
    glam::vec2(
        FNF_WIDTH * 0.474 - image.size.x * RATING_SCALE * 0.5,
        FNF_HEIGHT * 0.45 - 60.0 - image.size.y * RATING_SCALE * 0.5,
    )
}

fn combo_base(index: usize) -> glam::Vec2 {
    // ref: bdedc0aa:source/funkin/play/components/PopUpStuff.hx:93-94
    glam::vec2(
        FNF_WIDTH * 0.507 - COMBO_DIGIT_SPACING * (index as f32 + 1.0) - 65.0,
        FNF_HEIGHT * 0.44,
    )
}

fn popup_position(base: glam::Vec2, motion: PopupMotion, age_secs: f32) -> glam::Vec2 {
    base + glam::vec2(
        motion.velocity.x * age_secs,
        motion.velocity.y * age_secs + 0.5 * motion.acceleration_y * age_secs * age_secs,
    )
}

fn combo_digits(combo: u32) -> Vec<u32> {
    // ref: bdedc0aa:source/funkin/play/components/PopUpStuff.hx:74-80
    let mut digits = Vec::new();
    let mut value = combo;
    while value != 0 {
        digits.push(value % 10);
        value /= 10;
    }
    while digits.len() < 3 {
        digits.push(0);
    }
    digits
}

fn popup_alpha(age_secs: f32, fade_start: f32) -> f32 {
    if age_secs <= fade_start {
        1.0
    } else {
        1.0 - ((age_secs - fade_start) / POPUP_FADE_SECS).clamp(0.0, 1.0)
    }
}

fn random_int(seed: u64, min: u32, max: u32) -> f32 {
    let span = max.saturating_sub(min).saturating_add(1);
    min as f32
        + (random_unit(seed) * span as f32)
            .floor()
            .min(span as f32 - 1.0)
}

fn random_float(seed: u64, min: f32, max: f32) -> f32 {
    min + (max - min) * random_unit(seed)
}

fn random_unit(mut seed: u64) -> f32 {
    seed ^= seed >> 12;
    seed ^= seed << 25;
    seed ^= seed >> 27;
    let value = seed.wrapping_mul(0x2545_f491_4f6c_dd1d);
    ((value >> 40) as f32) / ((1u64 << 24) as f32)
}

fn popup_seed(popup: &ScorePopup, salt: u64) -> u64 {
    (popup.started_at.0 as u64)
        ^ ((popup.combo.unwrap_or(0) as u64) << 17)
        ^ (judgment_seed(popup.judgment) << 29)
        ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

fn judgment_seed(judgment: Judgment) -> u64 {
    match judgment {
        Judgment::Sick => 1,
        Judgment::Good => 2,
        Judgment::Bad => 3,
        Judgment::Shit => 4,
        Judgment::Miss => 5,
        _ => 6,
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
        popups.push(Judgment::Sick, Some(0), Samples(100));

        let commands = popups.commands(&skin(), Samples(100), 48_000, 100.0);

        assert_eq!(commands.len(), 4);
        assert_eq!(commands[0].texture, AssetId::new(1));
        assert_eq!(commands[1].texture, AssetId::new(10));
        assert_eq!(commands[2].texture, AssetId::new(10));
        assert_eq!(commands[3].texture, AssetId::new(10));
        assert!(commands.iter().all(|command| command.camera == CameraId(1)));
    }

    #[test]
    fn single_digit_combo_only_renders_rating_like_og() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Good, None, Samples(0));

        let commands = popups.commands(&skin(), Samples(0), 48_000, 100.0);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].texture, AssetId::new(2));
    }

    #[test]
    fn expired_popups_are_removed() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Bad, Some(10), Samples(0));

        let commands = popups.commands(&skin(), Samples(96_000), 48_000, 100.0);

        assert!(commands.is_empty());
    }

    #[test]
    fn combo_digits_render_least_significant_first_like_og() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Sick, Some(123), Samples(0));

        let commands = popups.commands(&skin(), Samples(0), 48_000, 100.0);

        assert_eq!(
            commands
                .iter()
                .map(|command| command.texture)
                .collect::<Vec<_>>(),
            vec![
                AssetId::new(1),
                AssetId::new(13),
                AssetId::new(12),
                AssetId::new(11)
            ]
        );
        assert!(commands[1].world_pos.x > commands[2].world_pos.x);
        assert!(commands[2].world_pos.x > commands[3].world_pos.x);
    }

    #[test]
    fn combo_digits_expand_beyond_three_digits_like_og() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Sick, Some(1000), Samples(0));

        let commands = popups.commands(&skin(), Samples(0), 48_000, 100.0);

        assert_eq!(
            commands
                .iter()
                .map(|command| command.texture)
                .collect::<Vec<_>>(),
            vec![
                AssetId::new(1),
                AssetId::new(10),
                AssetId::new(10),
                AssetId::new(10),
                AssetId::new(11),
            ]
        );
    }

    #[test]
    fn popup_motion_uses_velocity_then_gravity() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Sick, Some(123), Samples(0));

        let start = popups.commands(&skin(), Samples(0), 48_000, 100.0);
        let rising = popups.commands(&skin(), Samples(12_000), 48_000, 100.0);
        let falling = popups.commands(&skin(), Samples(38_400), 48_000, 100.0);

        assert!(rising[0].world_pos.y < start[0].world_pos.y);
        assert!(falling[0].world_pos.y > start[0].world_pos.y);
        assert_ne!(rising[1].world_pos.y, rising[2].world_pos.y);
    }

    #[test]
    fn rating_fades_before_combo_digits() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Sick, Some(10), Samples(0));

        let commands = popups.commands(&skin(), Samples(38_400), 48_000, 100.0);

        assert!(commands[0].color.w < 1.0);
        assert_eq!(commands[1].color.w, 1.0);
    }

    #[test]
    fn combo_break_can_render_digits_without_rating() {
        let mut popups = ScorePopups::default();
        popups.push(Judgment::Miss, Some(0), Samples(0));

        let commands = popups.commands(&skin(), Samples(0), 48_000, 100.0);

        assert_eq!(commands.len(), 3);
        assert_eq!(
            commands
                .iter()
                .map(|command| command.texture)
                .collect::<Vec<_>>(),
            vec![AssetId::new(10), AssetId::new(10), AssetId::new(10)]
        );
    }
}
