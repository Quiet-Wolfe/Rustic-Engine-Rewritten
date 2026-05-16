//! Sserafim character mouth-overlay animation.

use crate::animate_character_assets::{
    load_animate_atlas, AnimateCharacterPose, AnimateCharacterSprite, LoadedAnimateAtlas,
};
use anyhow::Result;
use rustic_asset::{
    AnimateAnimation, AnimateDrawPart, AssetPath, CharacterDefinition, OverlayResolver,
};
use rustic_core::ids::AssetId;
use rustic_core::time::Samples;
use rustic_render::{DrawCommand, FilterMode, Texture};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct SserafimLipSyncOverlay {
    asset: LoadedAnimateAtlas,
    symbol_name: String,
    mouth_keyword: &'static str,
    offsets: &'static [SserafimLipSyncPose],
    alpha: f32,
    flip_x: bool,
}

impl SserafimLipSyncOverlay {
    pub(crate) fn commands(
        &self,
        owner: &AnimateCharacterSprite,
        pose: &AnimateCharacterPose,
        parts: &[AnimateDrawPart],
        request_name: &str,
        should_sing: bool,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        let lip_pose = self.pose_for_request(request_name);
        let Some(mouth) = parts.iter().find(|part| {
            part.symbol_stack
                .iter()
                .any(|symbol| symbol == self.mouth_keyword)
        }) else {
            return Vec::new();
        };
        let frame_index = if should_sing {
            lip_sync_frame_index(cursor, sample_rate)
        } else {
            0
        };
        let lip_parts = self
            .asset
            .animation
            .flatten_symbol_frame(&self.symbol_name, frame_index)
            .unwrap_or_default();
        lip_parts
            .iter()
            .filter_map(|part| {
                let matrix = compose_affine(
                    compose_affine(
                        mouth.matrix,
                        lip_sync_offset(lip_pose.offset, lip_pose.angle_degrees),
                    ),
                    part.matrix,
                );
                owner.command_for_overlay_part(
                    pose,
                    &self.asset,
                    part,
                    matrix,
                    self.alpha,
                    self.flip_x,
                )
            })
            .collect()
    }

    fn pose_for_request(&self, request_name: &str) -> SserafimLipSyncPose {
        lip_sync_pose_for_request(self.offsets, request_name)
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
    let symbol_name = sserafim_lip_sync_root_symbol(&asset.animation).to_owned();
    Ok(Some(SserafimLipSyncOverlay {
        asset,
        symbol_name,
        mouth_keyword: spec.mouth_keyword,
        offsets: spec.offsets,
        alpha: spec.alpha,
        flip_x: spec.flip_x,
    }))
}

fn sserafim_lip_sync_root_symbol(animation: &AnimateAnimation) -> &str {
    if animation.has_symbol(&animation.symbol_name) {
        &animation.symbol_name
    } else if animation.has_symbol("lip sync all") {
        "lip sync all"
    } else {
        &animation.symbol_name
    }
}

#[derive(Debug, Clone, Copy)]
struct SserafimLipSyncSpec {
    asset_path: &'static str,
    mouth_keyword: &'static str,
    offsets: &'static [SserafimLipSyncPose],
    alpha: f32,
    flip_x: bool,
}

#[derive(Debug, Clone, Copy)]
struct SserafimLipSyncPose {
    name: &'static str,
    offset: glam::Vec2,
    angle_degrees: f32,
}

fn sserafim_lip_sync_spec(character: &CharacterDefinition) -> Option<SserafimLipSyncSpec> {
    let path = character.asset_path.as_ref()?.as_str();
    let spec = if path.ends_with("sserafim/yunjin") {
        (
            "sserafim/sserafim-lipsync-yunjin",
            "mouth yunjin",
            YUNJIN_LIP_SYNC,
            1.0,
            false,
        )
    } else if path.ends_with("sserafim/sakura") {
        (
            "sserafim/sserafim-lipsync",
            "mouth edit",
            SAKURA_LIP_SYNC,
            1.0,
            true,
        )
    } else if path.ends_with("sserafim/chaewon") {
        (
            "sserafim/sserafim-lipsync",
            "mouth default",
            CHAEWON_LIP_SYNC,
            0.5,
            false,
        )
    } else if path.ends_with("sserafim/eunchae") {
        (
            "sserafim/sserafim-lipsync",
            "mouth default",
            EUNCHAE_LIP_SYNC,
            1.0,
            false,
        )
    } else if path.ends_with("sserafim/kazuha") {
        (
            "sserafim/sserafim-lipsync",
            "mouth default",
            KAZUHA_LIP_SYNC,
            1.0,
            true,
        )
    } else {
        return None;
    };
    Some(SserafimLipSyncSpec {
        asset_path: spec.0,
        mouth_keyword: spec.1,
        offsets: spec.2,
        alpha: spec.3,
        flip_x: spec.4,
    })
}

fn lip_sync_pose_for_request(
    offsets: &'static [SserafimLipSyncPose],
    request_name: &str,
) -> SserafimLipSyncPose {
    if let Some(pose) = offsets.iter().find(|pose| pose.name == request_name) {
        return *pose;
    }
    let base = request_name.strip_suffix("miss").unwrap_or(request_name);
    offsets
        .iter()
        .find(|pose| pose.name == base)
        .copied()
        .unwrap_or(offsets[0])
}

const YUNJIN_LIP_SYNC: &[SserafimLipSyncPose] = &[
    lip_pose("idle", 8.0, 6.0, 23.0),
    lip_pose("singUP", 6.0, 8.0, 22.0),
    lip_pose("singRIGHT", 6.0, 8.0, 23.0),
    lip_pose("singDOWN", 8.0, 6.0, 23.0),
    lip_pose("singLEFT", 6.0, 8.0, 23.0),
];

const KAZUHA_LIP_SYNC: &[SserafimLipSyncPose] = &[
    lip_pose("idle", 5.0, 4.0, -13.0),
    lip_pose("singUP", 7.0, 2.0, -14.0),
    lip_pose("singRIGHT", 7.0, 2.0, -13.0),
    lip_pose("singDOWN", 4.0, 6.0, -12.0),
    lip_pose("singLEFT", 5.0, 4.0, -14.0),
];

const CHAEWON_LIP_SYNC: &[SserafimLipSyncPose] = &[
    lip_pose("idle", 41.0, 3.0, -166.0),
    lip_pose("singUP", 38.0, 0.0, -168.0),
    lip_pose("singRIGHT", 39.0, 1.0, -165.0),
    lip_pose("singDOWN", 41.0, 3.0, -167.0),
    lip_pose("singLEFT", 40.0, 2.0, -165.0),
];

const EUNCHAE_LIP_SYNC: &[SserafimLipSyncPose] = &[
    lip_pose("idle", 43.0, 6.0, -168.0),
    lip_pose("singUP", 45.0, 10.0, -166.0),
    lip_pose("singRIGHT", 42.0, 5.0, -166.0),
    lip_pose("singDOWN", 41.0, 3.0, -168.0),
    lip_pose("singLEFT", 43.0, 6.0, -169.0),
];

const SAKURA_LIP_SYNC: &[SserafimLipSyncPose] = &[
    lip_pose("idle", 7.0, 2.0, -14.0),
    lip_pose("singUP", 8.0, 1.0, -15.0),
    lip_pose("singRIGHT", 7.0, 2.0, -15.0),
    lip_pose("singDOWN", 6.0, 3.0, -15.0),
    lip_pose("singLEFT", 7.0, 2.0, -14.0),
    lip_pose("singUP-joint", 10.0, -1.0, -14.0),
    lip_pose("singRIGHT-joint", 6.0, 3.0, -15.0),
    lip_pose("singDOWN-joint", 5.0, 5.0, -15.0),
    lip_pose("singLEFT-joint", 7.0, 2.0, -16.0),
];

const fn lip_pose(
    name: &'static str,
    offset_x: f32,
    offset_y: f32,
    angle_degrees: f32,
) -> SserafimLipSyncPose {
    SserafimLipSyncPose {
        name,
        offset: glam::Vec2::new(offset_x, offset_y),
        angle_degrees,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_roots::baked_assets_root;
    use rustic_asset::{load_animate_animation, load_character};

    fn source_resolver() -> OverlayResolver {
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("crate should live under workspace/crates");
        OverlayResolver::new().with_baked_root(workspace.join("assets/source"))
    }

    fn baked_resolver() -> OverlayResolver {
        OverlayResolver::new().with_baked_root(baked_assets_root())
    }

    fn character_from_source(resolver: &OverlayResolver, id: &str) -> CharacterDefinition {
        load_character(
            resolver,
            &AssetPath::new(format!("data/characters/{id}.json")).unwrap(),
        )
        .unwrap()
    }

    fn animate_animation(resolver: &OverlayResolver, raw_path: &str) -> AnimateAnimation {
        let stripped = raw_path
            .split_once(':')
            .map(|(_, path)| path)
            .unwrap_or(raw_path);
        load_animate_animation(
            resolver,
            &AssetPath::new(format!("images/{stripped}/Animation.json")).unwrap(),
        )
        .unwrap()
    }

    fn assert_mouth_symbol_in_pose(
        resolver: &OverlayResolver,
        character_id: &str,
        animation_name: &str,
    ) {
        let character = character_from_source(resolver, character_id);
        let spec = sserafim_lip_sync_spec(&character).unwrap();
        let animation =
            animate_animation(resolver, character.asset_path.as_ref().unwrap().as_str());
        let character_animation = character
            .animations
            .iter()
            .find(|animation| animation.name == animation_name)
            .unwrap();
        let parts = animation
            .flatten_label_frame(&character_animation.prefix, 0)
            .unwrap();

        assert!(
            parts.iter().any(|part| part
                .symbol_stack
                .iter()
                .any(|symbol| symbol == spec.mouth_keyword)),
            "{character_id}:{animation_name} should contain {}",
            spec.mouth_keyword
        );
    }

    fn assert_lip_sync_asset_has_draw_parts(resolver: &OverlayResolver, asset_path: &str) {
        let animation = animate_animation(resolver, asset_path);
        let symbol_name = sserafim_lip_sync_root_symbol(&animation);
        let parts = animation.flatten_symbol_frame(symbol_name, 3).unwrap();

        assert!(
            !parts.is_empty(),
            "{asset_path} should flatten from its root symbol"
        );
    }

    #[test]
    fn sakura_lip_sync_uses_script_joint_offsets() {
        let pose = lip_sync_pose_for_request(SAKURA_LIP_SYNC, "singDOWN-joint");

        assert_eq!(pose.offset, glam::vec2(5.0, 5.0));
        assert_eq!(pose.angle_degrees, -15.0);
    }

    #[test]
    fn lip_sync_miss_poses_reuse_base_pose_offsets() {
        let pose = lip_sync_pose_for_request(KAZUHA_LIP_SYNC, "singRIGHTmiss");

        assert_eq!(pose.offset, glam::vec2(7.0, 2.0));
        assert_eq!(pose.angle_degrees, -13.0);
    }

    #[test]
    fn character_asset_path_selects_matching_lip_sync_asset() {
        let character = match CharacterDefinition::parse(
            br#"{
              "name": "sserafim-yunjin",
              "renderType": "animateatlas",
              "assetPath": "shared:characters/sserafim/yunjin",
              "animations": [{ "name": "idle", "prefix": "idle" }]
            }"#,
        ) {
            Ok(character) => character,
            Err(error) => panic!("test fixture should parse: {error}"),
        };
        let spec = match sserafim_lip_sync_spec(&character) {
            Some(spec) => spec,
            None => panic!("expected yunjin lip sync spec"),
        };

        assert_eq!(spec.asset_path, "sserafim/sserafim-lipsync-yunjin");
        assert_eq!(spec.mouth_keyword, "mouth yunjin");
        assert!(!spec.flip_x);
    }

    #[test]
    fn kazuha_lip_sync_matches_script_local_flip() {
        let character = match CharacterDefinition::parse(
            br#"{
              "name": "sserafim-kazuha",
              "renderType": "animateatlas",
              "assetPath": "shared:characters/sserafim/kazuha",
              "animations": [{ "name": "idle", "prefix": "idle" }]
            }"#,
        ) {
            Ok(character) => character,
            Err(error) => panic!("test fixture should parse: {error}"),
        };
        let spec = match sserafim_lip_sync_spec(&character) {
            Some(spec) => spec,
            None => panic!("expected kazuha lip sync spec"),
        };

        assert!(spec.flip_x);
    }

    #[test]
    fn sserafim_character_assets_expose_scripted_mouth_symbols() {
        let resolver = source_resolver();

        assert_mouth_symbol_in_pose(&resolver, "sserafim-yunjin", "idle");
        assert_mouth_symbol_in_pose(&resolver, "sserafim-kazuha", "idle");
        assert_mouth_symbol_in_pose(&resolver, "sserafim-chaewon", "idle");
        assert_mouth_symbol_in_pose(&resolver, "sserafim-eunchae", "idle");
        assert_mouth_symbol_in_pose(&resolver, "sserafim-sakura", "singDOWN-joint");
    }

    #[test]
    fn sserafim_lip_sync_assets_flatten_despite_exported_root_typo() {
        let resolver = baked_resolver();
        let default_lips = animate_animation(&resolver, "sserafim/sserafim-lipsync");

        assert_eq!(default_lips.symbol_name, "ssearafim-lipsync");
        assert_eq!(sserafim_lip_sync_root_symbol(&default_lips), "lip sync all");
        assert_lip_sync_asset_has_draw_parts(&resolver, "sserafim/sserafim-lipsync");
        assert_lip_sync_asset_has_draw_parts(&resolver, "sserafim/sserafim-lipsync-yunjin");
    }
}
