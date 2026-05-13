//! Freeplay capsule BPM and difficulty-number sprites.
//!
//! ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:101-133,318-382

use super::helpers::{
    digit_frames, load_sparrow_atlas, load_static_texture, sparrow_scaled_command,
    SparrowAtlasHandle, StaticTexture,
};
use crate::preview_song::{PreviewDifficulty, PreviewSong};
use anyhow::Result;
use rustic_asset::{OverlayResolver, SparrowFrame};
use rustic_render::{FilterMode, RenderCommandList, Texture};
use std::collections::HashMap;

const CAPSULE_META_SCALE: f32 = 0.9;
const CAPSULE_BPM_TEXT_POS: glam::Vec2 = glam::vec2(144.0, 87.0);
const CAPSULE_DIFFICULTY_TEXT_POS: glam::Vec2 = glam::vec2(414.0, 87.0);
const CAPSULE_DIFFICULTY_NUMBER_POS: glam::Vec2 = glam::vec2(466.0, 32.0);
const CAPSULE_DIFFICULTY_NUMBER_SPACING: f32 = 30.0;
const CAPSULE_BPM_NUMBER_Y: f32 = 88.5;
const CAPSULE_BPM_NUMBER_SPACING: f32 = 11.0;
// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:335 (fpScoreDisplay anchor)
const SCORE_X: f32 = 1280.0 - 353.0;
const SCORE_Y: f32 = 60.0;
const SCORE_DIGIT_COUNT: usize = 7;
const SCORE_DIGIT_SPACING: f32 = 36.0;

#[derive(Debug)]
pub(super) struct CapsuleMetadataAssets {
    bignumbers_atlas: Option<SparrowAtlasHandle>,
    bignumbers_digits: [Option<SparrowFrame>; 10],
    smallnumbers_atlas: Option<SparrowAtlasHandle>,
    smallnumbers_digits: [Option<SparrowFrame>; 10],
    bpm_text: Option<StaticTexture>,
    difficulty_text: Option<StaticTexture>,
}

impl CapsuleMetadataAssets {
    pub(super) fn push_score(&self, commands: &mut RenderCommandList) {
        let Some(atlas) = self.bignumbers_atlas.as_ref() else {
            return;
        };
        for digit_index in 0..SCORE_DIGIT_COUNT {
            let Some(frame) = self.bignumbers_digits[0].as_ref() else {
                return;
            };
            let x = SCORE_X + digit_index as f32 * SCORE_DIGIT_SPACING;
            commands.push(sparrow_scaled_command(
                atlas.texture_id,
                atlas.width,
                atlas.height,
                frame,
                glam::vec2(x, SCORE_Y),
                glam::Vec2::ONE,
                glam::Vec4::ONE,
                310,
            ));
        }
    }

    pub(super) fn push_capsule(
        &self,
        commands: &mut RenderCommandList,
        song: PreviewSong,
        difficulty: PreviewDifficulty,
        pos: glam::Vec2,
        alpha: f32,
        z: i32,
    ) {
        let color = glam::Vec4::new(1.0, 1.0, 1.0, alpha);
        if let Some(bpm) = self.bpm_text.as_ref() {
            commands.push(bpm.command(
                pos + CAPSULE_BPM_TEXT_POS,
                color,
                z,
                glam::vec2(
                    bpm.width as f32 * CAPSULE_META_SCALE,
                    bpm.height as f32 * CAPSULE_META_SCALE,
                ),
            ));
        }
        if let Some(difficulty_text) = self.difficulty_text.as_ref() {
            commands.push(difficulty_text.command(
                pos + CAPSULE_DIFFICULTY_TEXT_POS,
                color,
                z,
                glam::vec2(
                    difficulty_text.width as f32 * CAPSULE_META_SCALE,
                    difficulty_text.height as f32 * CAPSULE_META_SCALE,
                ),
            ));
        }
        self.push_bpm_digits(
            commands,
            pos,
            song.starting_bpm_for(difficulty),
            color,
            z + 1,
        );
        self.push_difficulty_rating_digits(
            commands,
            pos,
            song.difficulty_rating_for(difficulty),
            color,
            z + 1,
        );
    }

    fn push_bpm_digits(
        &self,
        commands: &mut RenderCommandList,
        pos: glam::Vec2,
        bpm: u16,
        color: glam::Vec4,
        z: i32,
    ) {
        let Some(atlas) = self.smallnumbers_atlas.as_ref() else {
            return;
        };
        let hundreds = (bpm / 100) % 10;
        let tens = (bpm / 10) % 10;
        let ones = bpm % 10;
        let mut temp_shift = 0.0;
        let shift_x = if hundreds == 1 { 186.0 } else { 191.0 };
        for (index, digit) in [hundreds, tens, ones].into_iter().enumerate() {
            if index == 1 && tens == 1 {
                temp_shift = -4.0;
            }
            if index == 2 && ones == 1 {
                temp_shift -= 4.0;
            }
            let x = shift_x + index as f32 * CAPSULE_BPM_NUMBER_SPACING + temp_shift;
            push_digit(
                commands,
                DigitDraw {
                    atlas,
                    digits: &self.smallnumbers_digits,
                    digit: digit as usize,
                    pos: pos + glam::vec2(x + digit_offset_x(digit as usize), CAPSULE_BPM_NUMBER_Y),
                    scale: CAPSULE_META_SCALE,
                    color,
                    z,
                },
            );
        }
    }

    fn push_difficulty_rating_digits(
        &self,
        commands: &mut RenderCommandList,
        pos: glam::Vec2,
        rating: u8,
        color: glam::Vec4,
        z: i32,
    ) {
        let Some(atlas) = self.bignumbers_atlas.as_ref() else {
            return;
        };
        let clamped = rating.min(99);
        let tens = if clamped < 10 { 0 } else { clamped / 10 };
        let ones = clamped % 10;
        for (index, digit) in [tens, ones].into_iter().enumerate() {
            let x = CAPSULE_DIFFICULTY_NUMBER_POS.x
                + index as f32 * CAPSULE_DIFFICULTY_NUMBER_SPACING
                + digit_offset_x(digit as usize);
            push_digit(
                commands,
                DigitDraw {
                    atlas,
                    digits: &self.bignumbers_digits,
                    digit: digit as usize,
                    pos: pos + glam::vec2(x, CAPSULE_DIFFICULTY_NUMBER_POS.y),
                    scale: CAPSULE_META_SCALE,
                    color,
                    z,
                },
            );
        }
    }
}

pub(super) fn load_capsule_metadata_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<rustic_core::ids::AssetId, Texture>,
) -> CapsuleMetadataAssets {
    let (bignumbers_atlas, bignumbers_digits) = load_digits_or_warn(
        device,
        queue,
        resolver,
        textures,
        "images/freeplay/freeplayCapsule/bignumbers.xml",
        "bignumbers",
    );
    let (smallnumbers_atlas, smallnumbers_digits) = load_digits_or_warn(
        device,
        queue,
        resolver,
        textures,
        "images/freeplay/freeplayCapsule/smallnumbers.xml",
        "smallnumbers",
    );
    let bpm_text = load_static_texture(
        device,
        queue,
        resolver,
        textures,
        "images/freeplay/freeplayCapsule/bpmtext.png",
        FilterMode::Linear,
    )
    .ok();
    let difficulty_text = load_static_texture(
        device,
        queue,
        resolver,
        textures,
        "images/freeplay/freeplayCapsule/difficultytext.png",
        FilterMode::Linear,
    )
    .ok();

    CapsuleMetadataAssets {
        bignumbers_atlas,
        bignumbers_digits,
        smallnumbers_atlas,
        smallnumbers_digits,
        bpm_text,
        difficulty_text,
    }
}

fn load_digits_or_warn(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<rustic_core::ids::AssetId, Texture>,
    path: &str,
    label: &str,
) -> (Option<SparrowAtlasHandle>, [Option<SparrowFrame>; 10]) {
    match load_digits(device, queue, resolver, textures, path) {
        Ok((handle, digits)) => (Some(handle), digits),
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay {label} unavailable: {e:#}");
            (
                None,
                [None, None, None, None, None, None, None, None, None, None],
            )
        }
    }
}

fn load_digits(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<rustic_core::ids::AssetId, Texture>,
    path: &str,
) -> Result<(SparrowAtlasHandle, [Option<SparrowFrame>; 10])> {
    let (handle, atlas) = load_sparrow_atlas(device, queue, resolver, textures, path)?;
    Ok((handle, digit_frames(&atlas)))
}

struct DigitDraw<'a> {
    atlas: &'a SparrowAtlasHandle,
    digits: &'a [Option<SparrowFrame>; 10],
    digit: usize,
    pos: glam::Vec2,
    scale: f32,
    color: glam::Vec4,
    z: i32,
}

fn push_digit(commands: &mut RenderCommandList, draw: DigitDraw<'_>) {
    let Some(frame) = draw.digits.get(draw.digit).and_then(Option::as_ref) else {
        return;
    };
    commands.push(sparrow_scaled_command(
        draw.atlas.texture_id,
        draw.atlas.width,
        draw.atlas.height,
        frame,
        draw.pos,
        glam::Vec2::splat(draw.scale),
        draw.color,
        draw.z,
    ));
}

fn digit_offset_x(digit: usize) -> f32 {
    // ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:884-896
    match digit {
        1 => 4.0,
        3 => 1.0,
        _ => 0.0,
    }
}
