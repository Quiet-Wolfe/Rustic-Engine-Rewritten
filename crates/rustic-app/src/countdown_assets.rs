//! OG ready/set/go countdown sprites.

use anyhow::{Context, Result};
use rustic_asset::{load_png, AssetPath, OverlayResolver};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const FNF_WIDTH: f32 = 1280.0;
const FNF_HEIGHT: f32 = 720.0;

#[derive(Debug, Clone)]
pub struct CountdownSkin {
    ready: CountdownImage,
    set: CountdownImage,
    go: CountdownImage,
}

impl CountdownSkin {
    pub fn commands(&self, cursor: Samples, sample_rate: u32, bpm: f64) -> Vec<DrawCommand> {
        let crochet = crochet_samples(sample_rate, bpm);
        if cursor.0 < -3 * crochet || cursor.0 >= 0 {
            return Vec::new();
        }

        let (image, start) = if cursor.0 < -2 * crochet {
            (&self.ready, -3 * crochet)
        } else if cursor.0 < -crochet {
            (&self.set, -2 * crochet)
        } else {
            (&self.go, -crochet)
        };
        vec![image.command(progress(cursor.0, start, crochet))]
    }
}

#[derive(Debug, Clone)]
struct CountdownImage {
    texture_id: AssetId,
    size: glam::Vec2,
}

impl CountdownImage {
    fn command(&self, progress: f32) -> DrawCommand {
        let eased = cube_in_out(progress);
        let world_pos = glam::vec2(
            (FNF_WIDTH - self.size.x) * 0.5,
            (FNF_HEIGHT - self.size.y) * 0.5,
        );
        let mut cmd = DrawCommand::sprite(self.texture_id, world_pos, self.size);
        // ref: bdedc0aa:source/funkin/play/Countdown.hx:215-236
        cmd.camera = CameraId(1);
        cmd.pivot = glam::Vec2::ZERO;
        cmd.layer = RenderLayer::Overlay;
        cmd.filter = FilterMode::Linear;
        cmd.color.w = 1.0 - eased;
        cmd
    }
}

pub fn load_countdown_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<CountdownSkin> {
    Ok(CountdownSkin {
        ready: load_countdown_image(device, queue, resolver, textures, "images/ready.png")?,
        set: load_countdown_image(device, queue, resolver, textures, "images/set.png")?,
        go: load_countdown_image(device, queue, resolver, textures, "images/go.png")?,
    })
}

fn load_countdown_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    path: &str,
) -> Result<CountdownImage> {
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
    Ok(CountdownImage {
        texture_id,
        size: glam::vec2(image.width as f32, image.height as f32),
    })
}

pub fn countdown_start_cursor(sample_rate: u32, bpm: f64) -> Samples {
    Samples(-5 * crochet_samples(sample_rate, bpm))
}

fn crochet_samples(sample_rate: u32, bpm: f64) -> i64 {
    (f64::from(sample_rate) * 60.0 / bpm.max(1.0)).round() as i64
}

fn progress(cursor: i64, start: i64, crochet: i64) -> f32 {
    ((cursor - start) as f32 / crochet.max(1) as f32).clamp(0.0, 1.0)
}

fn cube_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) * 0.5
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

    fn skin() -> CountdownSkin {
        CountdownSkin {
            ready: CountdownImage {
                texture_id: AssetId::new(1),
                size: glam::vec2(100.0, 50.0),
            },
            set: CountdownImage {
                texture_id: AssetId::new(2),
                size: glam::vec2(100.0, 50.0),
            },
            go: CountdownImage {
                texture_id: AssetId::new(3),
                size: glam::vec2(100.0, 50.0),
            },
        }
    }

    #[test]
    fn countdown_windows_match_og_ready_set_go_beats() {
        let skin = skin();
        assert!(skin.commands(Samples(-144_000), 48_000, 100.0).is_empty());
        assert_eq!(
            skin.commands(Samples(-86_400), 48_000, 100.0)[0].texture,
            AssetId::new(1)
        );
        assert_eq!(
            skin.commands(Samples(-57_600), 48_000, 100.0)[0].texture,
            AssetId::new(2)
        );
        assert_eq!(
            skin.commands(Samples(-28_800), 48_000, 100.0)[0].texture,
            AssetId::new(3)
        );
        assert!(skin.commands(Samples(0), 48_000, 100.0).is_empty());
    }

    #[test]
    fn countdown_starts_at_minus_five_crochets() {
        assert_eq!(countdown_start_cursor(48_000, 100.0), Samples(-144_000));
    }

    #[test]
    fn countdown_uses_hud_camera_and_screen_center() {
        let skin = skin();
        let command = skin.commands(Samples(-86_400), 48_000, 100.0)[0].clone();
        let mid_fade = skin.commands(Samples(-72_000), 48_000, 100.0)[0].clone();

        assert_eq!(command.camera, CameraId(1));
        assert_eq!(command.pivot, glam::Vec2::ZERO);
        assert_eq!(command.world_pos, glam::vec2(590.0, 335.0));
        assert_eq!(mid_fade.world_pos, glam::vec2(590.0, 335.0));
        assert!(mid_fade.color.w < command.color.w);
    }
}
