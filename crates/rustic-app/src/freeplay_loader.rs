use super::helpers::{clone_frames, digit_frames, load_sparrow_atlas, load_static_texture};
use super::{CapsuleKind, FreeplayAssets, FreeplayCapsule, WHITE_TEXTURE_ID};
use crate::asset_roots::baked_assets_root;
use crate::freeplay_dj::load_freeplay_dj;
use crate::preview_song::PreviewSong;
use anyhow::Result;
use rustic_asset::OverlayResolver;
use rustic_render::{FilterMode, Texture};
use std::collections::HashMap;

pub fn load_freeplay_assets(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<FreeplayAssets> {
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
            Some("rustic.freeplay.white"),
        ),
    );
    let pink_back = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/pinkBack.png",
        FilterMode::Linear,
    )?;
    let bg_image = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayBGweek1-bf.png",
        FilterMode::Linear,
    )?;
    let (capsule_atlas, capsule_atlas_data) = load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayCapsule/capsule/freeplayCapsule.xml",
    )?;
    let capsule_selected_frames = clone_frames(&capsule_atlas_data, "mp3 capsule w backing0");
    let capsule_unselected_frames =
        clone_frames(&capsule_atlas_data, "mp3 capsule w backing NOT SELECTED");
    let (selector_atlas, selector_atlas_data) = load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplaySelector/freeplaySelector.xml",
    )?;
    let selector_frames = clone_frames(&selector_atlas_data, "arrow pointer loop");
    let difficulty_easy = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayeasy.png",
        FilterMode::Linear,
    )?;
    let difficulty_normal = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplaynormal.png",
        FilterMode::Linear,
    )?;
    let difficulty_hard = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayhard.png",
        FilterMode::Linear,
    )?;
    let difficulty_erect = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayerect.png",
        FilterMode::Linear,
    )
    .unwrap_or(super::helpers::StaticTexture {
        texture_id: difficulty_hard.texture_id,
        width: difficulty_hard.width,
        height: difficulty_hard.height,
        filter: difficulty_hard.filter,
    });
    let (difficulty_nightmare, nightmare_atlas) = load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplaynightmare.xml",
    )?;
    let difficulty_nightmare_frames = clone_frames(&nightmare_atlas, "idle");

    let mut songs = vec![FreeplayCapsule {
        kind: CapsuleKind::Random,
        display_name: "RANDOM".to_string(),
    }];
    songs.extend(
        PreviewSong::CYCLABLE_WEEK1
            .iter()
            .map(|song| FreeplayCapsule {
                kind: CapsuleKind::Song(*song),
                display_name: song.display_name().to_ascii_uppercase(),
            }),
    );

    let dj = match load_freeplay_dj(device, queue) {
        Ok(mut dj) => {
            if let Some((tex_id, tex)) = dj.take_texture() {
                textures.insert(tex_id, tex);
            }
            Some(dj)
        }
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay DJ unavailable: {e:#}");
            None
        }
    };

    let (bignumbers_atlas, bignumbers_digits) = match load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayCapsule/bignumbers.xml",
    ) {
        Ok((handle, atlas)) => {
            let digits = digit_frames(&atlas);
            (Some(handle), digits)
        }
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay bignumbers unavailable: {e:#}");
            (
                None,
                [None, None, None, None, None, None, None, None, None, None],
            )
        }
    };

    let (highscore_atlas, highscore_frames) = match load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/highscore.xml",
    ) {
        Ok((handle, atlas)) => {
            let frames = clone_frames(&atlas, "highscore small instance 1");
            (Some(handle), frames)
        }
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay highscore unavailable: {e:#}");
            (None, Vec::new())
        }
    };

    let album_cover = match load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/albumRoll/volume1.png",
        FilterMode::Linear,
    ) {
        Ok(tex) => Some(tex),
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay album cover unavailable: {e:#}");
            None
        }
    };
    let (album_title_atlas, album_title_frame) = match load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/albumRoll/volume1-text.xml",
    ) {
        Ok((handle, atlas)) => {
            let frame = atlas.first_animation_frame("idle", &[]).cloned();
            (Some(handle), frame)
        }
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay album title unavailable: {e:#}");
            (None, None)
        }
    };

    let mini_arrow = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/miniArrow.png",
        FilterMode::Linear,
    )
    .ok();
    let seperator = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/seperator.png",
        FilterMode::Linear,
    )
    .ok();
    let (sparkle_atlas, sparkle_frames) = match load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/sparkle.xml",
    ) {
        Ok((handle, atlas)) => {
            let frames = clone_frames(&atlas, "sparkle Export0");
            (Some(handle), frames)
        }
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay sparkle unavailable: {e:#}");
            (None, Vec::new())
        }
    };
    let clear_box = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/clearBox.png",
        FilterMode::Linear,
    )
    .ok();

    Ok(FreeplayAssets {
        songs,
        pink_back,
        bg_image,
        capsule_atlas,
        capsule_selected_frames,
        capsule_unselected_frames,
        selector_atlas,
        selector_frames,
        difficulty_easy,
        difficulty_normal,
        difficulty_hard,
        difficulty_erect,
        difficulty_nightmare,
        difficulty_nightmare_frames,
        dj,
        bignumbers_atlas,
        bignumbers_digits,
        highscore_atlas,
        highscore_frames,
        album_cover,
        album_title_atlas,
        album_title_frame,
        mini_arrow,
        seperator,
        sparkle_atlas,
        sparkle_frames,
        clear_box,
        textures,
    })
}
