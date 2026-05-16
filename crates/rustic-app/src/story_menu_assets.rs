//! Story-menu asset wiring from Funkin' v0.8.5.
//!
//! ref: bdedc0aa:source/funkin/ui/story/StoryMenuState.hx:129-579
// LINT-ALLOW: long-file story menu asset loading and layout stay co-located for fidelity.

use crate::asset_roots::baked_assets_root;
use crate::preview_song::{PreviewDifficulty, PreviewSelection, PreviewSong};
use anyhow::{bail, Context, Result};
use rustic_asset::{
    load_level, load_png, load_sparrow, AssetPath, CharacterAnimation, LevelDefinition,
    LevelPropDefinition, OverlayResolver, SparrowAtlas, SparrowFrame,
};
use rustic_core::ids::{AssetId, CameraId};
use rustic_core::render::RenderLayer;
use rustic_core::time::Samples;
use rustic_render::Texture;
use rustic_render::{DrawCommand, FilterMode, RenderCommandList, TextCommand, TextCommandList};
use std::collections::HashMap;

#[path = "story_menu_assets_helpers.rs"]
mod helpers;
use helpers::*;

const STORY_LEVEL_IDS: [&str; 10] = [
    "tutorial", "week1", "week2", "week3", "week4", "week5", "week6", "week7", "weekend1",
    "sserafim",
];
const STORY_DIFFICULTIES: [PreviewDifficulty; 3] = [
    PreviewDifficulty::Easy,
    PreviewDifficulty::Normal,
    PreviewDifficulty::Hard,
];
const MENU_BPM: f64 = 102.0;
const LEVEL_BG_Y: f32 = 56.0;
const LEVEL_BG_HEIGHT: f32 = 400.0;
const TITLE_SELECTED_Y: f32 = 480.0;
const MIN_TITLE_SPACING: f32 = 125.0;
const WHITE_TEXTURE_ID: AssetId = AssetId::new(0x7374_6f72_795f_0001);

#[derive(Debug)]
pub struct StoryMenuAssets {
    levels: Vec<StoryLevel>,
    arrows: ArrowSkin,
    difficulties: HashMap<PreviewDifficulty, StaticTexture>,
    pub textures: HashMap<AssetId, Texture>,
}

impl StoryMenuAssets {
    pub fn commands(
        &self,
        selected_index: usize,
        difficulty: PreviewDifficulty,
        cursor: Samples,
        sample_rate: u32,
    ) -> RenderCommandList {
        let mut commands = RenderCommandList::new();
        let Some(level) = self.level(selected_index) else {
            return commands;
        };
        commands.push(solid_command(
            glam::vec2(0.0, 0.0),
            glam::vec2(1280.0, LEVEL_BG_Y + LEVEL_BG_HEIGHT),
            glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
            0,
        ));
        commands.push(solid_command(
            glam::vec2(0.0, LEVEL_BG_Y),
            glam::vec2(1280.0, LEVEL_BG_HEIGHT),
            color_from_story_hex(&level.data.background),
            1,
        ));
        self.push_level_titles(&mut commands, selected_index);
        for (index, prop) in level.props.iter().enumerate() {
            if let Some(command) = prop.command(index, cursor, sample_rate) {
                commands.push(command);
            }
        }
        self.push_difficulty_selector(&mut commands, difficulty, selected_index);
        commands
    }

    pub fn text_commands(
        &self,
        selected_index: usize,
        difficulty: PreviewDifficulty,
    ) -> TextCommandList {
        let mut commands = TextCommandList::new();
        let Some(level) = self.level(selected_index) else {
            return commands;
        };
        push_text(
            &mut commands,
            "LEVEL SCORE: 0",
            glam::vec2(10.0, 10.0),
            32.0,
            glam::Vec4::ONE,
            1000,
        );
        push_text(
            &mut commands,
            &level.data.name,
            glam::vec2(815.0, 10.0),
            32.0,
            glam::Vec4::new(1.0, 1.0, 1.0, 0.7),
            1000,
        );
        push_text(
            &mut commands,
            &tracklist_text(level, difficulty),
            glam::vec2(235.0, 500.0),
            32.0,
            glam::Vec4::new(0.90, 0.34, 0.47, 1.0),
            1000,
        );
        commands
    }

    pub fn item_count(&self) -> usize {
        self.levels.len()
    }

    pub fn difficulty_for_level(
        &self,
        level_index: usize,
        desired: PreviewDifficulty,
    ) -> PreviewDifficulty {
        self.level(level_index)
            .and_then(|level| {
                level
                    .difficulties
                    .iter()
                    .copied()
                    .find(|diff| *diff == desired)
            })
            .unwrap_or(PreviewDifficulty::Normal)
    }

    pub fn next_difficulty(
        &self,
        level_index: usize,
        current: PreviewDifficulty,
    ) -> PreviewDifficulty {
        self.cycle_difficulty(level_index, current, 1)
    }

    pub fn previous_difficulty(
        &self,
        level_index: usize,
        current: PreviewDifficulty,
    ) -> PreviewDifficulty {
        self.cycle_difficulty(level_index, current, -1)
    }

    pub fn preview_selection(
        &self,
        level_index: usize,
        difficulty: PreviewDifficulty,
    ) -> Option<PreviewSelection> {
        let song = self.preview_playlist(level_index)?.first().copied()?;
        Some(PreviewSelection::new(
            song,
            self.difficulty_for_level(level_index, difficulty),
        ))
    }

    pub fn preview_playlist(&self, level_index: usize) -> Option<Vec<PreviewSong>> {
        let level = self.level(level_index)?;
        let songs = level
            .data
            .songs
            .iter()
            .map(|song_id| PreviewSong::from_key(song_id))
            .collect::<Option<Vec<_>>>()?;
        (!songs.is_empty()).then_some(songs)
    }

    fn level(&self, selected_index: usize) -> Option<&StoryLevel> {
        self.levels
            .get(selected_index.min(self.levels.len().saturating_sub(1)))
    }

    fn cycle_difficulty(
        &self,
        level_index: usize,
        current: PreviewDifficulty,
        delta: isize,
    ) -> PreviewDifficulty {
        let Some(level) = self.level(level_index) else {
            return PreviewDifficulty::Normal;
        };
        let Some(index) = level
            .difficulties
            .iter()
            .position(|difficulty| *difficulty == current)
        else {
            return level
                .difficulties
                .first()
                .copied()
                .unwrap_or(PreviewDifficulty::Normal);
        };
        let len = level.difficulties.len() as isize;
        let next = (index as isize + delta).rem_euclid(len.max(1)) as usize;
        level.difficulties[next]
    }

    fn push_level_titles(&self, commands: &mut RenderCommandList, selected_index: usize) {
        let title_y = title_y_positions(&self.levels, selected_index);
        for (index, level) in self.levels.iter().enumerate() {
            let alpha = if index == selected_index { 1.0 } else { 0.6 };
            commands.push(level.title.command(
                glam::vec2((1280.0 - level.title.width as f32) * 0.5, title_y[index]),
                glam::Vec4::new(1.0, 1.0, 1.0, alpha),
                15,
            ));
        }
    }

    fn push_difficulty_selector(
        &self,
        commands: &mut RenderCommandList,
        difficulty: PreviewDifficulty,
        selected_index: usize,
    ) {
        let diff = self.difficulty_for_level(selected_index, difficulty);
        let show_arrows = self
            .level(selected_index)
            .map(|level| level.difficulties.len() > 1)
            .unwrap_or(false);
        if show_arrows {
            commands.push(
                self.arrows
                    .left_idle
                    .command(glam::vec2(870.0, 480.0), 1200),
            );
            commands.push(
                self.arrows
                    .right_idle
                    .command(glam::vec2(1245.0, 480.0), 1200),
            );
        }
        if let Some(sprite) = self.difficulties.get(&diff) {
            let normal_height = self
                .difficulties
                .get(&PreviewDifficulty::Normal)
                .map(|normal| normal.height as f32)
                .unwrap_or(sprite.height as f32);
            let normal_width = self
                .difficulties
                .get(&PreviewDifficulty::Normal)
                .map(|normal| normal.width as f32)
                .unwrap_or(sprite.width as f32);
            let x = 928.0 + (normal_width - sprite.width as f32) * 0.5;
            let y = 490.0 - (sprite.height as f32 - normal_height) * 0.5;
            commands.push(sprite.command(glam::vec2(x, y), glam::Vec4::ONE, 1200));
        }
    }
}

#[derive(Debug)]
struct StoryLevel {
    data: LevelDefinition,
    title: StaticTexture,
    props: Vec<StoryPropClip>,
    difficulties: Vec<PreviewDifficulty>,
}

#[derive(Debug)]
struct StaticTexture {
    texture_id: AssetId,
    width: u32,
    height: u32,
    filter: FilterMode,
}

impl StaticTexture {
    fn command(&self, pos: glam::Vec2, color: glam::Vec4, z: i32) -> DrawCommand {
        let mut cmd = DrawCommand::sprite(
            self.texture_id,
            pos,
            glam::vec2(self.width as f32, self.height as f32),
        );
        cmd.camera = CameraId(1);
        cmd.layer = RenderLayer::Overlay;
        cmd.z = z;
        cmd.pivot = glam::Vec2::ZERO;
        cmd.filter = self.filter;
        cmd.color = color;
        cmd
    }
}

#[derive(Debug)]
struct ArrowSkin {
    left_idle: SparrowStill,
    right_idle: SparrowStill,
}

#[derive(Debug)]
struct SparrowStill {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frame: SparrowFrame,
}

impl SparrowStill {
    fn command(&self, pos: glam::Vec2, z: i32) -> DrawCommand {
        sparrow_command(
            self.texture_id,
            self.texture_width,
            self.texture_height,
            &self.frame,
            pos,
            glam::Vec2::ONE,
            glam::Vec4::ONE,
            z,
        )
    }
}

#[derive(Debug)]
struct StoryPropClip {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    position: glam::Vec2,
    scale: glam::Vec2,
    alpha: f32,
    starting_animation: Option<String>,
    animations: Vec<StoryAnimationClip>,
}

impl StoryPropClip {
    fn command(&self, index: usize, cursor: Samples, sample_rate: u32) -> Option<DrawCommand> {
        let animation = self.animation_for_cursor(cursor, sample_rate)?;
        let frame_index = animation_frame_index(
            cursor,
            sample_rate,
            animation.started_at(cursor, sample_rate),
            animation.fps,
            animation.frames.len(),
            animation.looped,
        );
        let frame = animation.frames.get(frame_index)?;
        let pos = self.position
            - glam::vec2(
                frame.frame_x as f32 * self.scale.x + animation.offset.x,
                frame.frame_y as f32 * self.scale.y + animation.offset.y,
            );
        Some(sparrow_command(
            self.texture_id,
            self.texture_width,
            self.texture_height,
            frame,
            pos,
            self.scale,
            glam::Vec4::new(1.0, 1.0, 1.0, self.alpha),
            1000 + index as i32,
        ))
    }

    fn animation_for_cursor(
        &self,
        cursor: Samples,
        sample_rate: u32,
    ) -> Option<&StoryAnimationClip> {
        let left = self.animations.iter().find(|anim| anim.name == "danceLeft");
        let right = self
            .animations
            .iter()
            .find(|anim| anim.name == "danceRight");
        match (left, right) {
            (Some(left), Some(right)) => {
                if story_beat(cursor, sample_rate) % 2 == 0 {
                    Some(left)
                } else {
                    Some(right)
                }
            }
            _ => self
                .starting_animation
                .as_deref()
                .and_then(|name| self.animations.iter().find(|anim| anim.name == name))
                .or_else(|| self.animations.first()),
        }
    }
}

#[derive(Debug)]
struct StoryAnimationClip {
    name: String,
    fps: u16,
    looped: bool,
    offset: glam::Vec2,
    frames: Vec<SparrowFrame>,
}

impl StoryAnimationClip {
    fn started_at(&self, cursor: Samples, sample_rate: u32) -> Samples {
        if self.name == "danceLeft" || self.name == "danceRight" {
            return current_beat_start(cursor, sample_rate);
        }
        Samples(0)
    }
}

pub fn load_story_menu_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<StoryMenuAssets> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
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
            Some("rustic.storymenu.white"),
        ),
    );
    let levels = STORY_LEVEL_IDS
        .iter()
        .map(|level_id| load_story_level(device, queue, &resolver, &mut textures, level_id))
        .collect::<Result<Vec<_>>>()?;
    let arrows = load_arrows(device, queue, &resolver, &mut textures)?;
    let difficulties = load_difficulties(device, queue, &resolver, &mut textures)?;
    Ok(StoryMenuAssets {
        levels,
        arrows,
        difficulties,
        textures,
    })
}

fn load_story_level(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    level_id: &str,
) -> Result<StoryLevel> {
    let level_path = AssetPath::new(format!("data/levels/{level_id}.json"))?;
    let data = load_level(resolver, &level_path).with_context(|| format!("load {level_path}"))?;
    let title = load_static_texture(
        device,
        queue,
        resolver,
        textures,
        &format!("images/{}.png", data.title_asset),
        FilterMode::Linear,
    )?;
    let props = data
        .props
        .iter()
        .enumerate()
        .map(|(index, prop)| load_story_prop(device, queue, resolver, textures, index, prop))
        .collect::<Result<Vec<_>>>()?;
    let difficulties = story_difficulties(&data);
    Ok(StoryLevel {
        data,
        title,
        props,
        difficulties,
    })
}

fn load_static_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    path: &str,
    filter: FilterMode,
) -> Result<StaticTexture> {
    let path = AssetPath::new(path)?;
    let image = load_png(resolver, &path).with_context(|| format!("load {path}"))?;
    let texture_id = asset_id_for_path(&path);
    let (width, height) = (image.width, image.height);
    textures.insert(
        texture_id,
        Texture::from_png_image(device, queue, &image, filter, Some(path.as_str())),
    );
    Ok(StaticTexture {
        texture_id,
        width,
        height,
        filter,
    })
}

fn load_story_prop(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    index: usize,
    prop: &LevelPropDefinition,
) -> Result<StoryPropClip> {
    let atlas_path = AssetPath::new(format!("images/{}.xml", prop.asset_path))?;
    let atlas =
        load_sparrow(resolver, &atlas_path).with_context(|| format!("load {atlas_path}"))?;
    let (texture_id, texture_width, texture_height) =
        load_sparrow_texture(device, queue, resolver, textures, &atlas_path, &atlas)?;
    let animations = prop
        .animations
        .iter()
        .map(|animation| story_animation(&atlas, animation))
        .collect::<Result<Vec<_>>>()?;
    Ok(StoryPropClip {
        texture_id,
        texture_width,
        texture_height,
        position: glam::vec2(prop.offset.x + 320.0 * index as f32, prop.offset.y),
        scale: glam::Vec2::splat(prop.scale * if prop.is_pixel { 6.0 } else { 1.0 }),
        alpha: prop.alpha,
        starting_animation: (!prop.starting_animation.is_empty())
            .then(|| prop.starting_animation.clone()),
        animations,
    })
}

fn story_animation(
    atlas: &SparrowAtlas,
    animation: &CharacterAnimation,
) -> Result<StoryAnimationClip> {
    let frames: Vec<_> = atlas
        .animation_frames(&animation.prefix, &animation.indices)
        .into_iter()
        .cloned()
        .collect();
    if frames.is_empty() {
        bail!("story prop animation {} has no frames", animation.name);
    }
    Ok(StoryAnimationClip {
        name: animation.name.clone(),
        fps: animation.fps,
        looped: animation.looped || animation.name == "idle",
        offset: glam::vec2(animation.offset.x, animation.offset.y),
        frames,
    })
}

fn load_arrows(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<ArrowSkin> {
    let atlas_path = AssetPath::new("images/storymenu/ui/arrows.xml")?;
    let atlas = load_sparrow(resolver, &atlas_path)?;
    let (texture_id, texture_width, texture_height) =
        load_sparrow_texture(device, queue, resolver, textures, &atlas_path, &atlas)?;
    Ok(ArrowSkin {
        left_idle: first_frame(
            &atlas,
            texture_id,
            texture_width,
            texture_height,
            "leftIdle0",
        )?,
        right_idle: first_frame(
            &atlas,
            texture_id,
            texture_width,
            texture_height,
            "rightIdle0",
        )?,
    })
}

fn first_frame(
    atlas: &SparrowAtlas,
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    prefix: &str,
) -> Result<SparrowStill> {
    let frame = atlas
        .first_animation_frame(prefix, &[])
        .with_context(|| format!("resolve {prefix}"))?
        .clone();
    Ok(SparrowStill {
        texture_id,
        texture_width,
        texture_height,
        frame,
    })
}

fn load_difficulties(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<HashMap<PreviewDifficulty, StaticTexture>> {
    let mut out = HashMap::new();
    for difficulty in STORY_DIFFICULTIES {
        out.insert(
            difficulty,
            load_static_texture(
                device,
                queue,
                resolver,
                textures,
                &format!("images/storymenu/difficulties/{}.png", difficulty.as_str()),
                FilterMode::Linear,
            )?,
        );
    }
    Ok(out)
}

fn load_sparrow_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    atlas_path: &AssetPath,
    atlas: &SparrowAtlas,
) -> Result<(AssetId, u32, u32)> {
    let texture_path = atlas_path.sibling(&atlas.image_path)?;
    let image =
        load_png(resolver, &texture_path).with_context(|| format!("load {texture_path}"))?;
    let texture_id = asset_id_for_path(&texture_path);
    let (width, height) = (image.width, image.height);
    textures.insert(
        texture_id,
        Texture::from_png_image(
            device,
            queue,
            &image,
            FilterMode::Linear,
            Some(texture_path.as_str()),
        ),
    );
    Ok((texture_id, width, height))
}

#[cfg(test)]
#[path = "story_menu_assets_tests.rs"]
mod tests;
