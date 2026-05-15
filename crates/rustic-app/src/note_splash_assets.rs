//! Note splash asset loading and transient draw commands.
//!
//! ref: bdedc0aa:source/funkin/play/notes/NoteSplash.hx:42-61
//! ref: bdedc0aa:source/funkin/play/notes/Strumline.hx:49,1079-1100,1228-1260
//! ref: bdedc0aa:source/funkin/play/notes/notestyle/NoteStyle.hx:853-975
// LINT-ALLOW: long-file note splash asset loading and animation tests stay together.

use crate::animation_timing::visible_flixel_frame_index;
use anyhow::{Context, Result};
use rustic_asset::{
    load_png, load_sparrow, AssetPath, OverlayResolver, SparrowAtlas, SparrowFrame,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_game::Lane;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

const PLAYER_STRUMLINE_X: f32 = 1280.0 / 2.0 + 48.0;
const STRUMLINE_Y_OFFSET: f32 = 24.0;
const STRUMLINE_SIZE: f32 = 104.0;
const NOTE_SPACING: f32 = STRUMLINE_SIZE + 8.0;
const INITIAL_OFFSET: f32 = -0.275 * STRUMLINE_SIZE;
const SPLASH_OFFSET: glam::Vec2 = glam::vec2(25.0, -5.0);
const SPLASH_SCALE: f32 = 1.0;
const SPLASH_ALPHA: f32 = 0.8;
const SPLASH_FPS: u16 = 24;
const SPLASH_FPS_VARIANCE: i16 = 2;
const NOTE_SPLASH_CAP: usize = 6;

#[derive(Debug, Clone)]
pub struct NoteSplashSkin {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    scale: f32,
    offset: glam::Vec2,
    alpha: f32,
    fps: u16,
    fps_variance: i16,
    filter: FilterMode,
    lanes: [LaneSplashFrames; 4],
}

#[derive(Debug, Clone)]
struct LaneSplashFrames {
    variants: Vec<Vec<SparrowFrame>>,
}

#[derive(Debug, Default, Clone)]
pub struct NoteSplashes {
    active: Vec<ActiveNoteSplash>,
    sequence: u32,
}

#[derive(Debug, Clone, Copy)]
struct ActiveNoteSplash {
    lane: Lane,
    sequence: u32,
    started_at: Samples,
}

impl NoteSplashes {
    pub fn push(&mut self, lane: Lane, cursor: Samples) {
        let sequence = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);

        if self.active.len() >= NOTE_SPLASH_CAP {
            self.active.remove(0);
        }
        self.active.push(ActiveNoteSplash {
            lane,
            sequence,
            started_at: cursor,
        });
    }

    pub fn commands(
        &mut self,
        skin: &NoteSplashSkin,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        self.active
            .retain(|splash| splash_frame_index(splash, skin, cursor, sample_rate).is_some());

        let mut commands = Vec::with_capacity(self.active.len());
        for splash in &self.active {
            let Some(index) = splash_frame_index(splash, skin, cursor, sample_rate) else {
                continue;
            };
            let frame = &skin.frames(splash.lane, splash_variant(splash, skin))[index];
            let mut cmd = DrawCommand::sprite(
                skin.texture_id,
                splash_world_pos(splash.lane, frame, skin.scale, skin.offset),
                glam::vec2(
                    frame.width as f32 * skin.scale,
                    frame.height as f32 * skin.scale,
                ),
            );
            cmd.camera = CameraId(1);
            cmd.pivot = glam::Vec2::ZERO;
            cmd.layer = RenderLayer::Notes;
            cmd.z = 8;
            cmd.filter = skin.filter;
            cmd.color.w = skin.alpha;
            (cmd.uv_min, cmd.uv_max) = frame_uv(frame, skin.texture_width, skin.texture_height);
            cmd.uv_rotated = frame.rotated;
            commands.push(cmd);
        }
        commands
    }
}

impl NoteSplashSkin {
    fn frames(&self, lane: Lane, variant: usize) -> &[SparrowFrame] {
        let variants = &self.lanes[lane_index(lane)].variants;
        let index = variant.min(variants.len().saturating_sub(1));
        &variants[index]
    }
}

pub fn load_note_splash_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<NoteSplashSkin> {
    load_note_splash_assets_for_style(device, queue, resolver, textures, "funkin")
}

pub fn load_note_splash_assets_for_style(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    style: &str,
) -> Result<NoteSplashSkin> {
    if style == "pixel" {
        return load_note_splash_skin(
            device,
            queue,
            resolver,
            textures,
            NoteSplashSpec {
                atlas_path: "images/pixelNoteSplash.xml",
                scale: 4.0,
                offset: glam::vec2(27.5, 18.5),
                alpha: 1.0,
                fps: 33,
                fps_variance: SPLASH_FPS_VARIANCE,
                filter: FilterMode::Nearest,
                lane_prefixes: [
                    &["purple1", "purple2", "purple3"],
                    &["blue1", "blue2", "blue3"],
                    &["green1", "green2", "green3"],
                    &["orange1", "orange2", "orange3"],
                ],
            },
        );
    }

    load_note_splash_skin(
        device,
        queue,
        resolver,
        textures,
        NoteSplashSpec {
            atlas_path: "images/noteSplashes.xml",
            scale: SPLASH_SCALE,
            offset: SPLASH_OFFSET,
            alpha: SPLASH_ALPHA,
            fps: SPLASH_FPS,
            fps_variance: SPLASH_FPS_VARIANCE,
            filter: FilterMode::Linear,
            lane_prefixes: [
                &["note impact 1 purple0", "note impact 2 purple0"],
                &["note impact 1  blue0", "note impact 2 blue0"],
                &["note impact 1 green0", "note impact 2 green0"],
                &["note impact 1 red0", "note impact 2 red0"],
            ],
        },
    )
}

struct NoteSplashSpec<'a> {
    atlas_path: &'a str,
    scale: f32,
    offset: glam::Vec2,
    alpha: f32,
    fps: u16,
    fps_variance: i16,
    filter: FilterMode,
    lane_prefixes: [&'a [&'a str]; 4],
}

fn load_note_splash_skin(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    spec: NoteSplashSpec<'_>,
) -> Result<NoteSplashSkin> {
    let atlas_path = AssetPath::new(spec.atlas_path)?;
    let atlas = load_sparrow(resolver, &atlas_path)
        .with_context(|| format!("load {}", atlas_path.as_str()))?;
    let texture_path = atlas_texture_path(&atlas_path, &atlas)?;
    let image = load_png(resolver, &texture_path)
        .with_context(|| format!("load {}", texture_path.as_str()))?;
    let texture_id = asset_id_for_path(&texture_path);

    textures.insert(
        texture_id,
        Texture::from_png_image(
            device,
            queue,
            &image,
            spec.filter,
            Some(texture_path.as_str()),
        ),
    );

    let lanes: Vec<_> = spec
        .lane_prefixes
        .iter()
        .map(|prefixes| {
            Ok(LaneSplashFrames {
                variants: prefixes
                    .iter()
                    .map(|prefix| cloned_animation_frames(&atlas, prefix))
                    .collect::<Result<Vec<_>>>()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let lanes: [LaneSplashFrames; 4] = lanes
        .try_into()
        .map_err(|_| anyhow::anyhow!("note splash skin requires four lanes"))?;

    Ok(NoteSplashSkin {
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
        scale: spec.scale,
        offset: spec.offset,
        alpha: spec.alpha,
        fps: spec.fps,
        fps_variance: spec.fps_variance,
        filter: spec.filter,
        lanes,
    })
}

fn splash_frame_index(
    splash: &ActiveNoteSplash,
    skin: &NoteSplashSkin,
    cursor: Samples,
    sample_rate: u32,
) -> Option<usize> {
    let frames = skin.frames(splash.lane, splash_variant(splash, skin));
    visible_flixel_frame_index(
        cursor,
        sample_rate,
        splash.started_at,
        splash_fps(splash, skin),
        frames.len(),
        false,
    )
}

fn splash_variant(splash: &ActiveNoteSplash, skin: &NoteSplashSkin) -> usize {
    let count = skin.lanes[lane_index(splash.lane)].variants.len().max(1);
    splash.sequence as usize % count
}

fn splash_fps(splash: &ActiveNoteSplash, skin: &NoteSplashSkin) -> u16 {
    let count = skin.lanes[lane_index(splash.lane)].variants.len().max(1) as u32;
    let variance = skin.fps_variance.max(0);
    let spread = u32::try_from(variance.saturating_mul(2).saturating_add(1)).unwrap_or(1);
    let delta = (splash.sequence / count % spread) as i16 - variance;
    (skin.fps as i16 + delta).max(1) as u16
}

fn splash_world_pos(
    lane: Lane,
    frame: &SparrowFrame,
    scale: f32,
    offset: glam::Vec2,
) -> glam::Vec2 {
    let base = glam::vec2(
        PLAYER_STRUMLINE_X + lane_index(lane) as f32 * NOTE_SPACING + INITIAL_OFFSET,
        STRUMLINE_Y_OFFSET - INITIAL_OFFSET,
    ) + offset * scale;
    let frame_offset =
        glam::vec2(frame.frame_width as f32, frame.frame_height as f32) * scale * 0.3;
    let trimmed = glam::vec2(frame.frame_x as f32, frame.frame_y as f32) * scale;
    base - frame_offset - trimmed
}

fn atlas_texture_path(atlas_path: &AssetPath, atlas: &SparrowAtlas) -> Result<AssetPath> {
    if atlas.image_path.contains('/') {
        Ok(AssetPath::new(atlas.image_path.clone())?)
    } else if atlas.image_path == "spritesheet.png" {
        let Some(stem) = atlas_path.as_str().strip_suffix(".xml") else {
            anyhow::bail!("spritesheet atlas path must end in .xml: {}", atlas_path);
        };
        Ok(AssetPath::new(format!("{stem}.png"))?)
    } else {
        Ok(atlas_path.sibling(&atlas.image_path)?)
    }
}

fn cloned_animation_frames(atlas: &SparrowAtlas, prefix: &str) -> Result<Vec<SparrowFrame>> {
    let frames: Vec<_> = atlas
        .animation_frames(prefix, &[])
        .into_iter()
        .cloned()
        .collect();
    if frames.is_empty() {
        anyhow::bail!("resolve note splash frame {prefix}");
    }
    Ok(frames)
}

fn frame_uv(
    frame: &SparrowFrame,
    texture_width: u32,
    texture_height: u32,
) -> (glam::Vec2, glam::Vec2) {
    let width = texture_width.max(1) as f32;
    let height = texture_height.max(1) as f32;
    (
        glam::vec2(frame.x as f32 / width, frame.y as f32 / height),
        glam::vec2(
            (frame.x as f32 + frame.width as f32) / width,
            (frame.y as f32 + frame.height as f32) / height,
        ),
    )
}

fn lane_index(lane: Lane) -> usize {
    match lane {
        Lane::Left => 0,
        Lane::Down => 1,
        Lane::Up => 2,
        Lane::Right => 3,
        _ => 3,
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
    use std::path::Path;

    fn frame() -> SparrowFrame {
        SparrowAtlas::parse(
            br#"
            <TextureAtlas imagePath="noteSplashes.png">
              <SubTexture name="note impact 1 purple0000" x="100" y="50"
                width="189" height="270" frameX="-32" frameY="-12"
                frameWidth="260" frameHeight="298"/>
            </TextureAtlas>
            "#,
        )
        .unwrap()
        .frames[0]
            .clone()
    }

    fn skin() -> NoteSplashSkin {
        let f = frame();
        NoteSplashSkin {
            texture_id: AssetId::new(7),
            texture_width: 2048,
            texture_height: 1088,
            scale: SPLASH_SCALE,
            offset: SPLASH_OFFSET,
            alpha: SPLASH_ALPHA,
            fps: SPLASH_FPS,
            fps_variance: SPLASH_FPS_VARIANCE,
            filter: FilterMode::Linear,
            lanes: std::array::from_fn(|_| LaneSplashFrames {
                variants: vec![vec![f.clone(), f.clone()], vec![f.clone()]],
            }),
        }
    }

    fn source_resolver() -> OverlayResolver {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest_dir.parent().unwrap().parent().unwrap();
        OverlayResolver::new().with_baked_root(workspace.join("assets/source"))
    }

    fn source_atlas(resolver: &OverlayResolver, path: &str) -> SparrowAtlas {
        load_sparrow(resolver, &AssetPath::new(path).unwrap()).unwrap()
    }

    fn source_skin() -> NoteSplashSkin {
        let resolver = source_resolver();
        let atlas = source_atlas(&resolver, "images/noteSplashes.xml");
        NoteSplashSkin {
            texture_id: AssetId::new(7),
            texture_width: 2048,
            texture_height: 1088,
            scale: SPLASH_SCALE,
            offset: SPLASH_OFFSET,
            alpha: SPLASH_ALPHA,
            fps: SPLASH_FPS,
            fps_variance: SPLASH_FPS_VARIANCE,
            filter: FilterMode::Linear,
            lanes: [
                LaneSplashFrames {
                    variants: vec![
                        cloned_animation_frames(&atlas, "note impact 1 purple0").unwrap(),
                        cloned_animation_frames(&atlas, "note impact 2 purple0").unwrap(),
                    ],
                },
                LaneSplashFrames {
                    variants: vec![
                        cloned_animation_frames(&atlas, "note impact 1  blue0").unwrap(),
                        cloned_animation_frames(&atlas, "note impact 2 blue0").unwrap(),
                    ],
                },
                LaneSplashFrames {
                    variants: vec![
                        cloned_animation_frames(&atlas, "note impact 1 green0").unwrap(),
                        cloned_animation_frames(&atlas, "note impact 2 green0").unwrap(),
                    ],
                },
                LaneSplashFrames {
                    variants: vec![
                        cloned_animation_frames(&atlas, "note impact 1 red0").unwrap(),
                        cloned_animation_frames(&atlas, "note impact 2 red0").unwrap(),
                    ],
                },
            ],
        }
    }

    #[test]
    fn splash_position_uses_strumline_offsets_trim_and_note_splash_offset() {
        let pos = splash_world_pos(Lane::Left, &frame(), SPLASH_SCALE, SPLASH_OFFSET);
        assert!((pos.x - 638.4).abs() < 1e-3);
        assert!((pos.y - -29.800003).abs() < 1e-3);
    }

    #[test]
    fn splash_commands_advance_by_variant_frame_rate() {
        let mut splashes = NoteSplashes::default();
        splashes.push(Lane::Left, Samples(0));
        let commands = splashes.commands(&skin(), Samples(0), 48_000);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].texture, AssetId::new(7));
        assert_eq!(commands[0].camera, CameraId(1));
        assert_eq!(commands[0].layer, RenderLayer::Notes);
        assert!((commands[0].color.w - SPLASH_ALPHA).abs() < 1e-6);

        let commands = splashes.commands(&skin(), Samples(48_000), 48_000);
        assert!(commands.is_empty());
    }

    #[test]
    fn splash_frame_holds_previous_frame_on_exact_boundary() {
        let splash = ActiveNoteSplash {
            lane: Lane::Left,
            sequence: 0,
            started_at: Samples(0),
        };
        let mut skin = skin();
        skin.fps_variance = 0;

        assert_eq!(
            splash_frame_index(&splash, &skin, Samples(2_000), 48_000),
            Some(0)
        );
        assert_eq!(
            splash_frame_index(&splash, &skin, Samples(2_001), 48_000),
            Some(1)
        );
        assert_eq!(
            splash_frame_index(&splash, &skin, Samples(4_000), 48_000),
            Some(1)
        );
        assert_eq!(
            splash_frame_index(&splash, &skin, Samples(4_001), 48_000),
            None
        );
    }

    #[test]
    fn splash_pool_caps_to_og_size() {
        let mut splashes = NoteSplashes::default();
        for i in 0..8 {
            splashes.push(Lane::Left, Samples(i));
        }
        assert_eq!(splashes.active.len(), NOTE_SPLASH_CAP);
        assert_eq!(splashes.active[0].started_at, Samples(2));
    }

    #[test]
    fn tracked_source_note_splashes_emit_commands_for_all_lanes() {
        let skin = source_skin();
        for lane in [Lane::Left, Lane::Down, Lane::Up, Lane::Right] {
            assert_eq!(skin.frames(lane, 0).len(), 4);
            assert_eq!(skin.frames(lane, 1).len(), 4);
        }

        let mut splashes = NoteSplashes::default();
        for lane in [Lane::Left, Lane::Down, Lane::Up, Lane::Right] {
            splashes.push(lane, Samples(0));
        }

        let commands = splashes.commands(&skin, Samples(0), 48_000);
        assert_eq!(commands.len(), 4);
        assert!(commands.iter().all(|cmd| cmd.layer == RenderLayer::Notes));
        assert!(commands.iter().all(|cmd| cmd.z == 8));
    }

    #[test]
    fn pixel_source_note_splashes_use_three_variants_and_nearest_filter() {
        let resolver = source_resolver();
        let atlas = source_atlas(&resolver, "images/pixelNoteSplash.xml");
        let skin = NoteSplashSkin {
            texture_id: AssetId::new(7),
            texture_width: 432,
            texture_height: 384,
            scale: 4.0,
            offset: glam::vec2(27.5, 18.5),
            alpha: 1.0,
            fps: 33,
            fps_variance: SPLASH_FPS_VARIANCE,
            filter: FilterMode::Nearest,
            lanes: [
                LaneSplashFrames {
                    variants: vec![
                        cloned_animation_frames(&atlas, "purple1").unwrap(),
                        cloned_animation_frames(&atlas, "purple2").unwrap(),
                        cloned_animation_frames(&atlas, "purple3").unwrap(),
                    ],
                },
                LaneSplashFrames {
                    variants: vec![
                        cloned_animation_frames(&atlas, "blue1").unwrap(),
                        cloned_animation_frames(&atlas, "blue2").unwrap(),
                        cloned_animation_frames(&atlas, "blue3").unwrap(),
                    ],
                },
                LaneSplashFrames {
                    variants: vec![
                        cloned_animation_frames(&atlas, "green1").unwrap(),
                        cloned_animation_frames(&atlas, "green2").unwrap(),
                        cloned_animation_frames(&atlas, "green3").unwrap(),
                    ],
                },
                LaneSplashFrames {
                    variants: vec![
                        cloned_animation_frames(&atlas, "orange1").unwrap(),
                        cloned_animation_frames(&atlas, "orange2").unwrap(),
                        cloned_animation_frames(&atlas, "orange3").unwrap(),
                    ],
                },
            ],
        };

        assert_eq!(skin.frames(Lane::Left, 2).len(), 8);
        let mut splashes = NoteSplashes::default();
        splashes.push(Lane::Left, Samples(0));
        let commands = splashes.commands(&skin, Samples(0), 48_000);
        assert_eq!(commands[0].filter, FilterMode::Nearest);
        assert!((commands[0].color.w - 1.0).abs() < 1e-6);
    }
}
