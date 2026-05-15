//! Sserafim character mouth-overlay animation.

use crate::animate_character_assets::{
    load_animate_atlas, AnimateCharacterPose, AnimateCharacterSprite, LoadedAnimateAtlas,
};
use anyhow::Result;
use rustic_asset::{AnimateDrawPart, AssetPath, CharacterDefinition, OverlayResolver};
use rustic_core::ids::AssetId;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct SserafimLipSyncOverlay {
    asset: LoadedAnimateAtlas,
    mouth_keyword: &'static str,
    offset: glam::Vec2,
    angle_degrees: f32,
    alpha: f32,
}

impl SserafimLipSyncOverlay {
    pub(crate) fn commands(
        &self,
        owner: &AnimateCharacterSprite,
        pose: &AnimateCharacterPose,
        parts: &[AnimateDrawPart],
        request_name: &str,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        if !request_name.starts_with("sing") {
            return Vec::new();
        }
        let Some(mouth) = parts.iter().find(|part| {
            part.symbol_stack
                .iter()
                .any(|symbol| symbol == self.mouth_keyword)
        }) else {
            return Vec::new();
        };
        let frame_index = lip_sync_frame_index(cursor, sample_rate);
        let lip_parts = self
            .asset
            .animation
            .flatten_symbol_frame(&self.asset.animation.symbol_name, frame_index)
            .unwrap_or_default();
        lip_parts
            .iter()
            .filter_map(|part| {
                let matrix = compose_affine(
                    compose_affine(
                        mouth.matrix,
                        lip_sync_offset(self.offset, self.angle_degrees),
                    ),
                    part.matrix,
                );
                owner.command_for_overlay_part(pose, &self.asset, part, matrix, self.alpha)
            })
            .collect()
    }
}

pub(crate) fn load_sserafim_lip_sync(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    character: &CharacterDefinition,
    filter: FilterMode,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<Option<SserafimLipSyncOverlay>> {
    let Some(spec) = sserafim_lip_sync_spec(character) else {
        return Ok(None);
    };
    let asset_path = AssetPath::new(spec.asset_path)?;
    let asset = load_animate_atlas(device, queue, resolver, &asset_path, filter, textures)?;
    Ok(Some(SserafimLipSyncOverlay {
        asset,
        mouth_keyword: spec.mouth_keyword,
        offset: spec.offset,
        angle_degrees: spec.angle_degrees,
        alpha: spec.alpha,
    }))
}

#[derive(Debug, Clone, Copy)]
struct SserafimLipSyncSpec {
    asset_path: &'static str,
    mouth_keyword: &'static str,
    offset: glam::Vec2,
    angle_degrees: f32,
    alpha: f32,
}

fn sserafim_lip_sync_spec(character: &CharacterDefinition) -> Option<SserafimLipSyncSpec> {
    let path = character.asset_path.as_ref()?.as_str();
    let spec = if path.ends_with("sserafim/yunjin") {
        (
            "sserafim/sserafim-lipsync-yunjin",
            "mouth yunjin",
            glam::vec2(8.0, 6.0),
            23.0,
            1.0,
        )
    } else if path.ends_with("sserafim/sakura") {
        (
            "sserafim/sserafim-lipsync",
            "mouth edit",
            glam::vec2(7.0, 2.0),
            -14.0,
            1.0,
        )
    } else if path.ends_with("sserafim/chaewon") {
        (
            "sserafim/sserafim-lipsync",
            "mouth default",
            glam::vec2(41.0, 3.0),
            -166.0,
            0.5,
        )
    } else if path.ends_with("sserafim/eunchae") {
        (
            "sserafim/sserafim-lipsync",
            "mouth default",
            glam::vec2(43.0, 6.0),
            -168.0,
            1.0,
        )
    } else if path.ends_with("sserafim/kazuha") {
        (
            "sserafim/sserafim-lipsync",
            "mouth default",
            glam::vec2(5.0, 4.0),
            -13.0,
            1.0,
        )
    } else {
        return None;
    };
    Some(SserafimLipSyncSpec {
        asset_path: spec.0,
        mouth_keyword: spec.1,
        offset: spec.2,
        angle_degrees: spec.3,
        alpha: spec.4,
    })
}

fn lip_sync_frame_index(cursor: Samples, sample_rate: u32) -> u32 {
    let seconds = cursor.0.max(0) as f32 / sample_rate.max(1) as f32;
    (seconds * 24.0).floor().max(1.0) as u32 - 1
}

fn lip_sync_offset(offset: glam::Vec2, angle_degrees: f32) -> [f32; 6] {
    let radians = angle_degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    [cos, sin, -sin, cos, offset.x, offset.y]
}

fn compose_affine(parent: [f32; 6], child: [f32; 6]) -> [f32; 6] {
    [
        parent[0] * child[0] + parent[2] * child[1],
        parent[1] * child[0] + parent[3] * child[1],
        parent[0] * child[2] + parent[2] * child[3],
        parent[1] * child[2] + parent[3] * child[3],
        parent[0] * child[4] + parent[2] * child[5] + parent[4],
        parent[1] * child[4] + parent[3] * child[5] + parent[5],
    ]
}
