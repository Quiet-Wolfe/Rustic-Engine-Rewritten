use super::backing_text::FreeplayBackingText;
use super::capsule_metadata::load_capsule_metadata_assets;
// LINT-ALLOW: long-file Freeplay asset bootstrap covers vanilla screen dependencies.
use super::difficulty_stars::load_freeplay_difficulty_stars;
use super::freeplay_icons::load_freeplay_icons;
use super::helpers::{clone_frames, load_sparrow_atlas, load_static_texture};
use super::song_metadata::{FreeplayDifficultyRatings, FreeplaySongMetadata};
use super::{CapsuleKind, FreeplayAlbum, FreeplayAssets, FreeplayCapsule, WHITE_TEXTURE_ID};
use crate::asset_roots::baked_assets_root;
use crate::bitmap_text_assets::load_bitmap_text_assets;
use crate::freeplay_dj::load_freeplay_dj_for_asset;
use crate::preview_song::{PreviewDifficulty, PreviewSong, VARIATION_BF, VARIATION_PICO};
use anyhow::{Context, Result};
use rustic_asset::{load_bytes, AssetPath, OverlayResolver};
use rustic_render::{FilterMode, Texture};
use serde::Deserialize;
use std::collections::HashMap;

#[cfg(test)]
#[path = "freeplay_loader_tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeplayStyle {
    Bf,
    Pico,
}

impl FreeplayStyle {
    fn data_path(self) -> &'static str {
        match self {
            Self::Bf => "data/ui/freeplay/styles/bf.json",
            Self::Pico => "data/ui/freeplay/styles/pico.json",
        }
    }

    fn dj_asset_path(self) -> &'static str {
        match self {
            Self::Bf => "images/freeplay/freeplay-boyfriend",
            Self::Pico => "images/freeplay/freeplay-pico",
        }
    }

    fn player_data_path(self) -> &'static str {
        match self {
            Self::Bf => "data/players/bf.json",
            Self::Pico => "data/players/pico.json",
        }
    }
}

pub fn load_freeplay_assets(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<FreeplayAssets> {
    load_freeplay_assets_for_style(device, queue, FreeplayStyle::Bf)
}

pub fn load_freeplay_assets_for_style(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    style: FreeplayStyle,
) -> Result<FreeplayAssets> {
    let resolver = OverlayResolver::new().with_baked_root(baked_assets_root());
    let style_config = load_freeplay_style_config(&resolver, style)?;
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
    load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/pinkBack.png",
        FilterMode::Linear,
    )?;
    // Custom right-triangle alpha mask used past BF's right shoulder. The
    // pinkBack alpha is too subtle to read as a triangle when stretched, so
    // we generate a proper triangle PNG (wide at top, single point at the
    // bottom-left) and composite it over the solid rectangle.
    let back_triangle = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/freeplayBackTriangle.png",
        FilterMode::Linear,
    )
    .ok();
    let bg_image_path = style_config.bg_image_path();
    let bg_image = load_static_texture(
        device,
        queue,
        &resolver,
        &mut textures,
        &bg_image_path,
        FilterMode::Linear,
    )?;
    let capsule_atlas_path = style_config.capsule_atlas_path();
    let (capsule_atlas, capsule_atlas_data) =
        load_sparrow_atlas(device, queue, &resolver, &mut textures, &capsule_atlas_path)?;
    let capsule_selected_frames = clone_frames(&capsule_atlas_data, "mp3 capsule w backing0");
    let capsule_unselected_frames =
        clone_frames(&capsule_atlas_data, "mp3 capsule w backing NOT SELECTED");
    let selector_atlas_path = style_config.selector_atlas_path();
    let (selector_atlas, selector_atlas_data) = load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        &selector_atlas_path,
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
        PreviewSong::ALL
            .iter()
            .chain(PreviewSong::FREEPLAY_EXTRA.iter())
            .map(|song| FreeplayCapsule {
                kind: CapsuleKind::Song(*song),
                display_name: song.display_name().to_ascii_uppercase(),
            }),
    );

    let dj = match load_freeplay_dj_for_asset(device, queue, style.dj_asset_path()) {
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

    let capsule_metadata = load_capsule_metadata_assets(device, queue, &resolver, &mut textures);

    let (highscore_atlas, highscore_frame) = match load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/highscore.xml",
    ) {
        Ok((handle, atlas)) => {
            let frame = atlas
                .frames
                .iter()
                .find(|frame| frame.name == "highscore small instance 10000")
                .cloned();
            (Some(handle), frame)
        }
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay highscore unavailable: {e:#}");
            (None, None)
        }
    };

    let song_albums = load_song_metadata_map(&resolver);
    let albums = load_freeplay_albums(device, queue, &resolver, &mut textures, &song_albums);
    let difficulty_stars = match load_freeplay_difficulty_stars(
        device,
        queue,
        &resolver,
        &mut textures,
    ) {
        Ok(stars) => Some(stars),
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay difficulty stars unavailable: {e:#}");
            None
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
    let (fav_heart_atlas, fav_heart_frames) = match load_sparrow_atlas(
        device,
        queue,
        &resolver,
        &mut textures,
        "images/freeplay/favHeart.xml",
    ) {
        Ok((handle, atlas)) => {
            let frames = clone_frames(&atlas, "favorite heart");
            (Some(handle), frames)
        }
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay favorite heart unavailable: {e:#}");
            (None, Vec::new())
        }
    };
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
    let icons = load_freeplay_icons(device, queue, &resolver, &mut textures);
    let backing_text_skin = match load_bitmap_text_assets(device, queue, &resolver, &mut textures) {
        Ok(skin) => Some(skin),
        Err(e) => {
            tracing::warn!(target: "rustic.asset", "freeplay backing text unavailable: {e:#}");
            None
        }
    };

    Ok(FreeplayAssets {
        songs,
        back_triangle,
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
        capsule_metadata,
        highscore_atlas,
        highscore_frame,
        albums,
        song_albums,
        difficulty_stars,
        mini_arrow,
        seperator,
        fav_heart_atlas,
        fav_heart_frames,
        sparkle_atlas,
        sparkle_frames,
        clear_box,
        icons,
        backing_text: style_config.backing_text,
        backing_text_skin,
        enter_started_at: None,
        visual_selected_index: 0.0,
        visual_selected_from: 0.0,
        visual_selected_to: 0.0,
        visual_selected_started_at: None,
        start_delay_secs: style_config.start_delay,
        capsule_text_colors: style_config.capsule_text_colors,
        textures,
    })
}

fn load_freeplay_style_config(
    resolver: &OverlayResolver,
    style: FreeplayStyle,
) -> Result<FreeplayStyleConfig> {
    let path = AssetPath::new(style.data_path())?;
    let bytes = load_bytes(resolver, &path).with_context(|| format!("load {}", path.as_str()))?;
    let raw: RawFreeplayStyle =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.as_str()))?;
    let backing_text = load_player_backing_text(resolver, style).unwrap_or_else(|error| {
        tracing::warn!(
            target: "rustic.asset",
            "freeplay player backing text unavailable for {:?}: {error:#}",
            style
        );
        FreeplayBackingText::boyfriend()
    });
    Ok(raw.into_config(backing_text))
}

fn load_player_backing_text(
    resolver: &OverlayResolver,
    style: FreeplayStyle,
) -> Result<FreeplayBackingText> {
    let path = AssetPath::new(style.player_data_path())?;
    let bytes = load_bytes(resolver, &path).with_context(|| format!("load {}", path.as_str()))?;
    let raw: RawFreeplayPlayer =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.as_str()))?;
    let dj = raw
        .freeplay_dj
        .ok_or_else(|| anyhow::anyhow!("missing freeplayDJ text block"))?;
    Ok(FreeplayBackingText::new(dj.text1, dj.text2, dj.text3))
}

fn load_song_metadata_map(resolver: &OverlayResolver) -> HashMap<u32, FreeplaySongMetadata> {
    PreviewSong::ALL
        .iter()
        .chain(PreviewSong::FREEPLAY_EXTRA.iter())
        .map(|song| {
            let base =
                load_song_metadata(resolver, song.metadata_path_for(PreviewDifficulty::Normal))
                    .unwrap_or_else(|error| {
                        tracing::warn!(
                            target: "rustic.asset",
                            "freeplay metadata unavailable for {}: {error:#}",
                            song.folder
                        );
                        FreeplayLoadedSongMetadata::fallback("volume1")
                    });
            let mut variant_albums = HashMap::new();
            let mut variant_ratings = HashMap::new();
            let mut variant_icon_ids = HashMap::new();
            if song
                .available_difficulties()
                .contains(&PreviewDifficulty::Erect)
            {
                if let Ok(metadata) =
                    load_song_metadata(resolver, song.metadata_path_for(PreviewDifficulty::Erect))
                {
                    variant_albums.insert("erect".to_string(), metadata.album);
                    variant_ratings.insert("erect".to_string(), metadata.ratings);
                    if let Some(opponent) = metadata.opponent {
                        variant_icon_ids.insert("erect".to_string(), opponent);
                    }
                }
            }
            for variation in [VARIATION_BF, VARIATION_PICO] {
                if song.has_variation(variation) {
                    if let Ok(metadata) =
                        load_song_metadata(resolver, song.metadata_path_for_suffix(Some(variation)))
                    {
                        variant_albums.insert(variation.to_string(), metadata.album);
                        variant_ratings.insert(variation.to_string(), metadata.ratings);
                        if let Some(opponent) = metadata.opponent {
                            variant_icon_ids.insert(variation.to_string(), opponent);
                        }
                    }
                }
            }
            (
                song.id,
                FreeplaySongMetadata::new(
                    base.album,
                    variant_albums,
                    base.ratings,
                    variant_ratings,
                    base.opponent,
                    variant_icon_ids,
                ),
            )
        })
        .collect()
}

#[cfg(test)]
fn load_song_album_id(resolver: &OverlayResolver, path: String) -> Result<String> {
    Ok(load_song_metadata(resolver, path)?.album)
}

fn load_song_metadata(
    resolver: &OverlayResolver,
    path: String,
) -> Result<FreeplayLoadedSongMetadata> {
    let path = AssetPath::new(path)?;
    let bytes = load_bytes(resolver, &path).with_context(|| format!("load {}", path.as_str()))?;
    let metadata: RawSongMetadata =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.as_str()))?;
    Ok(FreeplayLoadedSongMetadata {
        album: metadata.play_data.album,
        ratings: FreeplayDifficultyRatings::from_map(&metadata.play_data.ratings),
        opponent: metadata
            .play_data
            .characters
            .and_then(|characters| characters.opponent),
    })
}

fn load_freeplay_albums(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<rustic_core::ids::AssetId, Texture>,
    song_albums: &HashMap<u32, FreeplaySongMetadata>,
) -> HashMap<String, FreeplayAlbum> {
    let mut ids = vec!["volume1".to_string()];
    for metadata in song_albums.values() {
        ids.extend(metadata.album_ids().map(ToString::to_string));
    }
    ids.sort();
    ids.dedup();
    ids.into_iter()
        .filter_map(|album_id| {
            match load_freeplay_album(device, queue, resolver, textures, &album_id) {
                Ok(album) => Some((album_id, album)),
                Err(error) => {
                    tracing::warn!(
                        target: "rustic.asset",
                        "freeplay album {album_id} unavailable: {error:#}"
                    );
                    None
                }
            }
        })
        .collect()
}

fn load_freeplay_album(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<rustic_core::ids::AssetId, Texture>,
    album_id: &str,
) -> Result<FreeplayAlbum> {
    let path = AssetPath::new(format!("data/ui/freeplay/albums/{album_id}.json"))?;
    let bytes = load_bytes(resolver, &path).with_context(|| format!("load {}", path.as_str()))?;
    let data: RawFreeplayAlbum =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.as_str()))?;
    let cover = load_static_texture(
        device,
        queue,
        resolver,
        textures,
        &format!("images/{}.png", data.album_art_asset),
        FilterMode::Linear,
    )?;
    let (title_atlas, title_data) = load_sparrow_atlas(
        device,
        queue,
        resolver,
        textures,
        &format!("images/{}.xml", data.album_title_asset),
    )?;
    let title_frame = title_data
        .first_animation_frame("idle", &[])
        .or_else(|| title_data.first_animation_frame("switch", &[]))
        .or_else(|| title_data.frames.first())
        .cloned();
    let [offset_x, offset_y] = data.album_title_offsets.unwrap_or([0.0, 0.0]);
    Ok(FreeplayAlbum {
        cover,
        title_atlas,
        title_frame,
        title_offset: glam::vec2(offset_x, offset_y),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSongMetadata {
    play_data: RawSongPlayData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSongPlayData {
    album: String,
    ratings: HashMap<String, u8>,
    characters: Option<RawSongCharacters>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSongCharacters {
    opponent: Option<String>,
}

struct FreeplayLoadedSongMetadata {
    album: String,
    ratings: FreeplayDifficultyRatings,
    opponent: Option<String>,
}

impl FreeplayLoadedSongMetadata {
    fn fallback(album: &str) -> Self {
        Self {
            album: album.to_string(),
            ratings: FreeplayDifficultyRatings::default(),
            opponent: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFreeplayAlbum {
    album_art_asset: String,
    album_title_asset: String,
    album_title_offsets: Option<[f32; 2]>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFreeplayStyle {
    bg_asset: String,
    selector_asset: String,
    capsule_asset: String,
    capsule_text_colors: Option<[String; 2]>,
    #[serde(default = "default_start_delay")]
    start_delay: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFreeplayPlayer {
    #[serde(rename = "freeplayDJ")]
    freeplay_dj: Option<RawFreeplayDjText>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFreeplayDjText {
    text1: String,
    text2: String,
    text3: String,
}

#[derive(Debug, Clone, PartialEq)]
struct FreeplayStyleConfig {
    bg_asset: String,
    selector_asset: String,
    capsule_asset: String,
    capsule_text_colors: [glam::Vec4; 2],
    start_delay: f64,
    backing_text: FreeplayBackingText,
}

impl RawFreeplayStyle {
    fn into_config(self, backing_text: FreeplayBackingText) -> FreeplayStyleConfig {
        FreeplayStyleConfig {
            bg_asset: self.bg_asset,
            selector_asset: self.selector_asset,
            capsule_asset: self.capsule_asset,
            capsule_text_colors: parse_capsule_text_colors(self.capsule_text_colors),
            start_delay: self.start_delay,
            backing_text,
        }
    }
}

impl FreeplayStyleConfig {
    fn bg_image_path(&self) -> String {
        image_asset_path(&self.bg_asset, "png")
    }

    fn selector_atlas_path(&self) -> String {
        image_asset_path(&self.selector_asset, "xml")
    }

    fn capsule_atlas_path(&self) -> String {
        image_asset_path(&self.capsule_asset, "xml")
    }
}

fn image_asset_path(asset: &str, extension: &str) -> String {
    let asset = asset.strip_prefix("images/").unwrap_or(asset);
    let with_prefix = format!("images/{asset}");
    if with_prefix.ends_with(&format!(".{extension}")) {
        with_prefix
    } else {
        format!("{with_prefix}.{extension}")
    }
}

fn default_start_delay() -> f64 {
    1.0
}

fn parse_capsule_text_colors(colors: Option<[String; 2]>) -> [glam::Vec4; 2] {
    colors
        .map(|[selected, unselected]| {
            [
                parse_hex_color(&selected).unwrap_or(glam::Vec4::ONE),
                parse_hex_color(&unselected).unwrap_or(glam::Vec4::ONE),
            ]
        })
        .unwrap_or([glam::Vec4::ONE, glam::Vec4::ONE])
}

fn parse_hex_color(value: &str) -> Option<glam::Vec4> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let parse = |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).ok();
    Some(glam::vec4(
        f32::from(parse(0..2)?) / 255.0,
        f32::from(parse(2..4)?) / 255.0,
        f32::from(parse(4..6)?) / 255.0,
        1.0,
    ))
}
