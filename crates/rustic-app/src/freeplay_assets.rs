// LINT-ALLOW: long-file freeplay asset loading and layout stay co-located for fidelity.

use crate::bitmap_text_assets::BitmapTextSkin;
use crate::freeplay_dj::FreeplayDJ;
use crate::preview_song::{PreviewDifficulty, PreviewSelection, PreviewSong};
use rustic_asset::SparrowFrame;
use rustic_core::ids::AssetId;
use rustic_core::time::Samples;
use rustic_render::{RenderCommandList, TextCommand, TextCommandList, Texture};
use std::collections::HashMap;

#[path = "freeplay_assets_helpers.rs"]
mod helpers;
pub use helpers::REQUIRED_FREEPLAY_ASSETS;
use helpers::*;

#[path = "freeplay_capsule_metadata.rs"]
mod capsule_metadata;
use capsule_metadata::CapsuleMetadataAssets;

#[path = "freeplay_loader.rs"]
mod freeplay_loader;
pub use freeplay_loader::{load_freeplay_assets, load_freeplay_assets_for_style, FreeplayStyle};

#[path = "freeplay_backing_text.rs"]
mod backing_text;

#[path = "freeplay_difficulty_stars.rs"]
mod difficulty_stars;
use difficulty_stars::FreeplayDifficultyStars;
#[path = "freeplay_difficulty_cycle.rs"]
mod difficulty_cycle;

#[path = "freeplay_song_metadata.rs"]
mod song_metadata;
use song_metadata::FreeplaySongMetadata;

const CAPSULE_REAL_SCALED: f32 = 0.8;
const CAPSULE_BASE_X: f32 = 270.0;
const CAPSULE_BASE_Y: f32 = 320.0;
const CAPSULE_SIN_AMPLITUDE: f32 = 60.0;
const CAPSULE_SPACING_PAD: f32 = 10.0;
const CAPSULE_FRAME_HEIGHT: f32 = 132.0;
const CAPSULE_FRAME_WIDTH: f32 = 612.0;
const CAPSULE_ANIM_FPS: u16 = 24;
// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:354-356
const SELECTOR_LEFT_X: f32 = 20.0;
const SELECTOR_RIGHT_X: f32 = 325.0;
const SELECTOR_Y: f32 = 70.0;
const SELECTOR_ANIM_FPS: u16 = 24;
// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:339
const DIFFICULTY_GROUP_X: f32 = 90.0;
const DIFFICULTY_GROUP_Y: f32 = 80.0;
const ORANGE_BAR_X: f32 = 84.0;
const ORANGE_BAR_Y: f32 = 645.0;
const ORANGE_BAR_HEIGHT: f32 = 75.0;
const FREEPLAY_TITLE_X: f32 = 8.0;
const FREEPLAY_TITLE_Y: f32 = 8.0;
const PINKBACK_TARGET_HEIGHT: f32 = 720.0;
// ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:603 (xFrames)
const CAPSULE_BEAT_XFRAMES: [f32; 7] = [1.7, 1.8, 0.85, 0.85, 0.97, 0.97, 1.0];
const CAPSULE_BEAT_FPS: u16 = 24;
// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:596-602
// fnfHighscoreSpr draws at native size; OG x = FlxG.width - 420, y = 70.
const HIGHSCORE_X: f32 = 1280.0 - 420.0;
const HIGHSCORE_Y: f32 = 70.0;
const HIGHSCORE_SCALE: f32 = 1.0;
// ref: bdedc0aa:source/funkin/ui/freeplay/AlbumRoll.hx:50-53
const ALBUM_ART_X: f32 = 1280.0 - 360.0;
const ALBUM_ART_Y: f32 = 220.0;
const ALBUM_ART_SCALE: f32 = 0.5;
const ALBUM_TITLE_X: f32 = 1280.0 - 360.0;
const ALBUM_TITLE_Y: f32 = 480.0;
const BACKING_TEXT_GAP: f32 = 20.0;

const PINKBACK_COLOR: glam::Vec4 = glam::Vec4::new(
    0xFF as f32 / 255.0,
    0xD8 as f32 / 255.0,
    0x63 as f32 / 255.0,
    1.0,
);
const ORANGE_BAR_COLOR: glam::Vec4 = glam::Vec4::new(
    0xFE as f32 / 255.0,
    0xDA as f32 / 255.0,
    0x00 as f32 / 255.0,
    1.0,
);
// ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx (songText color)
const WHITE_TEXTURE_ID: AssetId = AssetId::new(0x4672_6565_706c_6179);

const ENTER_DURATION_SECS: f64 = 0.6;
const CAPSULE_ENTER_OFFSET_X: f32 = 600.0;

#[derive(Debug)]
pub struct FreeplayAssets {
    songs: Vec<FreeplayCapsule>,
    back_triangle: Option<StaticTexture>,
    bg_image: StaticTexture,
    capsule_atlas: SparrowAtlasHandle,
    capsule_selected_frames: Vec<SparrowFrame>,
    capsule_unselected_frames: Vec<SparrowFrame>,
    selector_atlas: SparrowAtlasHandle,
    selector_frames: Vec<SparrowFrame>,
    difficulty_easy: StaticTexture,
    difficulty_normal: StaticTexture,
    difficulty_hard: StaticTexture,
    difficulty_erect: StaticTexture,
    difficulty_nightmare: SparrowAtlasHandle,
    difficulty_nightmare_frames: Vec<SparrowFrame>,
    dj: Option<FreeplayDJ>,
    capsule_metadata: CapsuleMetadataAssets,
    highscore_atlas: Option<SparrowAtlasHandle>,
    highscore_frame: Option<SparrowFrame>,
    albums: HashMap<String, FreeplayAlbum>,
    song_albums: HashMap<u32, FreeplaySongMetadata>,
    difficulty_stars: Option<FreeplayDifficultyStars>,
    mini_arrow: Option<StaticTexture>,
    seperator: Option<StaticTexture>,
    fav_heart_atlas: Option<SparrowAtlasHandle>,
    fav_heart_frames: Vec<SparrowFrame>,
    sparkle_atlas: Option<SparrowAtlasHandle>,
    sparkle_frames: Vec<SparrowFrame>,
    clear_box: Option<StaticTexture>,
    backing_text_skin: Option<BitmapTextSkin>,
    enter_started_at: Option<Samples>,
    pub start_delay_secs: f64,
    capsule_text_colors: [glam::Vec4; 2],
    pub textures: HashMap<AssetId, Texture>,
}

impl FreeplayAssets {
    pub fn commands(
        &self,
        selection: PreviewSelection,
        selected_index: usize,
        cursor: Samples,
        sample_rate: u32,
    ) -> RenderCommandList {
        let mut commands = RenderCommandList::new();
        commands.push(solid_command(
            glam::vec2(0.0, 0.0),
            glam::vec2(1280.0, 720.0),
            glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
            -100,
        ));

        let enter_t = self.enter_progress(cursor, sample_rate);
        let back_x = -(1.0 - enter_t) * 1280.0;
        let overhang_y = -100.0 - (1.0 - enter_t) * 164.0;

        commands.push(solid_command(
            glam::vec2(0.0, overhang_y),
            glam::vec2(1280.0, 164.0),
            glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
            295,
        ));

        let solid_rect_w = helpers::PINKBACK_RECT_WIDTH;
        let triangle_w = helpers::PINKBACK_TRIANGLE_WIDTH;
        let back_h = helpers::PINKBACK_RECT_HEIGHT;
        commands.push(solid_command(
            glam::vec2(back_x, 0.0),
            glam::vec2(solid_rect_w, back_h),
            PINKBACK_COLOR,
            -90,
        ));
        if let Some(tri) = self.back_triangle.as_ref() {
            commands.push(tri.background_command(
                glam::vec2(back_x + solid_rect_w, 0.0),
                PINKBACK_COLOR,
                -90,
                glam::vec2(triangle_w, back_h),
            ));
        }
        let logical_width = self.pink_back_logical_width();
        commands.push(solid_command(
            glam::vec2(back_x + ORANGE_BAR_X, ORANGE_BAR_Y),
            glam::vec2(logical_width, ORANGE_BAR_HEIGHT),
            ORANGE_BAR_COLOR,
            -85,
        ));
        // alsoOrangeLOL: small 100-wide hot-yellow strip at the orange bar's left edge.
        // ref: bdedc0aa:source/funkin/ui/freeplay/backcards/BackingCard.hx:58
        commands.push(solid_command(
            glam::vec2(back_x, ORANGE_BAR_Y),
            glam::vec2(100.0, ORANGE_BAR_HEIGHT),
            glam::Vec4::new(1.0, 0xD4 as f32 / 255.0, 0.0, 1.0),
            -84,
        ));
        self.push_backing_text(&mut commands, cursor, sample_rate);

        let bg_scale = bg_image_scale(&self.bg_image);
        let bg_pos = glam::vec2(logical_width * 0.74, 0.0);
        commands.push(self.bg_image.background_command(
            bg_pos,
            glam::Vec4::ONE,
            -80,
            glam::vec2(
                self.bg_image.width as f32 * bg_scale,
                self.bg_image.height as f32 * bg_scale,
            ),
        ));

        if let Some(dj) = self.dj.as_ref() {
            for cmd in dj.commands(cursor, sample_rate) {
                commands.push(cmd);
            }
        }

        let selected_index = self.clamped_index(selected_index);
        let capsule_enter_offset = (1.0 - enter_t) * CAPSULE_ENTER_OFFSET_X;
        self.push_capsules(
            &mut commands,
            selected_index,
            selection.difficulty,
            cursor,
            sample_rate,
            capsule_enter_offset,
        );
        self.push_difficulty(&mut commands, selection.difficulty, cursor, sample_rate);
        self.push_highscore(&mut commands);
        self.push_score(&mut commands);
        self.push_clear_box(&mut commands);
        self.push_album(&mut commands, selection, cursor, sample_rate);
        self.push_letter_sort(&mut commands);
        self.push_difficulty_dots(&mut commands, selection.difficulty);
        self.push_sparkles(&mut commands, selected_index, cursor, sample_rate);
        commands
    }

    /// ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:165-176,237-241
    fn push_sparkles(
        &self,
        commands: &mut RenderCommandList,
        selected_index: usize,
        cursor: Samples,
        sample_rate: u32,
    ) {
        let (Some(atlas), Some(frame)) = (
            self.sparkle_atlas.as_ref(),
            frame_for_cursor(&self.sparkle_frames, cursor, sample_rate, 24, true),
        ) else {
            return;
        };
        let offset = 0.0f32; // selected capsule sits at offset 0
        let capsule_pos = capsule_position(offset);
        let _ = selected_index;
        let ranking_pos =
            capsule_pos + glam::vec2(420.0 * CAPSULE_REAL_SCALED, 41.0 * CAPSULE_REAL_SCALED);
        commands.push(sparrow_scaled_command(
            atlas.texture_id,
            atlas.width,
            atlas.height,
            frame,
            ranking_pos,
            glam::Vec2::splat(0.8),
            glam::Vec4::new(1.0, 1.0, 1.0, 0.7),
            340,
        ));
    }

    /// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:341,510-525 (difficultyDots)
    fn push_difficulty_dots(
        &self,
        commands: &mut RenderCommandList,
        difficulty: PreviewDifficulty,
    ) {
        let Some(sep) = self.seperator.as_ref() else {
            return;
        };
        let dots: [PreviewDifficulty; 5] = [
            PreviewDifficulty::Easy,
            PreviewDifficulty::Normal,
            PreviewDifficulty::Hard,
            PreviewDifficulty::Erect,
            PreviewDifficulty::Nightmare,
        ];
        let base_x = 260.0;
        let base_y = 170.0;
        let spacing = (sep.width as f32 + 8.0).max(18.0);
        let scale = 1.0;
        for (idx, kind) in dots.iter().enumerate() {
            let selected = *kind == difficulty;
            let mut color = if selected {
                glam::Vec4::new(
                    0xFA as f32 / 255.0,
                    0xFA as f32 / 255.0,
                    0xFA as f32 / 255.0,
                    1.0,
                )
            } else {
                glam::Vec4::new(
                    0x91 as f32 / 255.0,
                    0x91 as f32 / 255.0,
                    0x91 as f32 / 255.0,
                    0.9,
                )
            };
            if matches!(
                kind,
                PreviewDifficulty::Nightmare | PreviewDifficulty::Erect
            ) {
                color = if selected {
                    glam::Vec4::new(0xC2 as f32 / 255.0, 0x8A as f32 / 255.0, 1.0, 1.0)
                } else {
                    glam::Vec4::new(
                        0x34 as f32 / 255.0,
                        0x29 as f32 / 255.0,
                        0x6A as f32 / 255.0,
                        0.9,
                    )
                };
            }
            commands.push(sep.command(
                glam::vec2(base_x + idx as f32 * spacing, base_y),
                color,
                325,
                glam::vec2(sep.width as f32 * scale, sep.height as f32 * scale),
            ));
        }
    }

    /// ref: bdedc0aa:source/funkin/ui/freeplay/LetterSort.hx:38-86
    fn push_letter_sort(&self, commands: &mut RenderCommandList) {
        let group_x = 400.0;
        let group_y = 75.0;
        if let Some(arrow) = self.mini_arrow.as_ref() {
            commands.push(arrow.command(
                glam::vec2(group_x + -20.0 + arrow.width as f32, group_y + 15.0),
                glam::Vec4::ONE,
                330,
                glam::vec2(-(arrow.width as f32), arrow.height as f32),
            ));
            commands.push(arrow.command(
                glam::vec2(group_x + 380.0, group_y + 15.0),
                glam::Vec4::ONE,
                330,
                glam::vec2(arrow.width as f32, arrow.height as f32),
            ));
        }
        if let Some(sep) = self.seperator.as_ref() {
            for i in 0..4 {
                commands.push(sep.command(
                    glam::vec2(group_x + (i as f32 * 80.0) + 60.0, group_y + 20.0),
                    glam::Vec4::ONE,
                    329,
                    glam::vec2(sep.width as f32, sep.height as f32),
                ));
            }
        }
        if let (Some(atlas), Some(frame)) =
            (self.fav_heart_atlas.as_ref(), self.fav_heart_frames.get(1))
        {
            commands.push(sparrow_scaled_command(
                atlas.texture_id,
                atlas.width,
                atlas.height,
                frame,
                glam::vec2(group_x + 80.0 + 52.0, group_y + 44.0),
                glam::Vec2::ONE,
                glam::Vec4::ONE,
                335,
            ));
        }
    }

    pub fn text_commands(
        &self,
        selected_index: usize,
        cursor: Samples,
        sample_rate: u32,
    ) -> TextCommandList {
        let mut commands = TextCommandList::new();
        let selected_index = self.clamped_index(selected_index);

        let enter_t = self.enter_progress(cursor, sample_rate);
        let top_y_offset = -(1.0 - enter_t) * 164.0;
        let capsule_enter_offset_x = (1.0 - enter_t) * CAPSULE_ENTER_OFFSET_X;

        let mut title = TextCommand::new(
            "FREEPLAY",
            glam::vec2(FREEPLAY_TITLE_X, FREEPLAY_TITLE_Y + top_y_offset),
            48.0,
        );
        title.color = glam::Vec4::new(1.0, 0.84, 0.26, 1.0);
        title.z = 300;
        commands.push(title);

        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:349 (ostName)
        // ref: bdedc0aa:source/funkin/util/Constants.hx:292 (DEFAULT_OST_NAME)
        // OG renders ostName at size 48 right-aligned; we approximate with a
        // 48px left-aligned x near the right edge.
        let mut ost = TextCommand::new("OFFICIAL OST", glam::vec2(900.0, 8.0 + top_y_offset), 48.0);
        ost.color = glam::Vec4::new(1.0, 1.0, 1.0, 0.9);
        ost.z = 300;
        commands.push(ost);

        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:347 (txtCompletion)
        // We have no persisted score data yet, so render the empty state (0%)
        // instead of the placeholder 100%.
        let mut completion = TextCommand::new("0%", glam::vec2(1280.0 - 95.0, 87.0), 32.0);
        completion.color = glam::Vec4::new(1.0, 1.0, 1.0, 1.0);
        completion.z = 312;
        commands.push(completion);

        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:350 (charSelectHint)
        let mut hint = TextCommand::new(
            "Press [ TAB ] to change characters",
            glam::vec2(420.0, 26.0 + top_y_offset),
            24.0,
        );
        hint.color = glam::Vec4::new(1.0, 1.0, 1.0, 0.7);
        hint.z = 305;
        commands.push(hint);

        for (index, capsule) in self.songs.iter().enumerate() {
            let offset = index as f32 - selected_index as f32;
            let pos = capsule_position(offset) + glam::vec2(capsule_enter_offset_x, 0.0);
            let is_selected = index == selected_index;
            let mut text = TextCommand::new(
                capsule.display_name.clone(),
                pos + capsule_text_offset(),
                36.0 * CAPSULE_REAL_SCALED,
            );
            let mut color = if is_selected {
                self.capsule_text_colors[0]
            } else {
                self.capsule_text_colors[1]
            };
            color.w = if is_selected { 1.0 } else { 0.6 };
            text.color = color;
            text.z = 320 + index as i32;
            commands.push(text);
        }

        // ref: bdedc0aa:source/funkin/ui/freeplay/LetterSort.hx:275-279 regexLetters
        // The slot at index 2 is the active category (center). Neighbors are
        // the previous/next entries in the alphabet array; ALL is centered by
        // default. "A-B" and "C-D" are letter ranges, rendered with a hyphen.
        const LETTERS: [&str; 5] = ["#", "", "ALL", "A-B", "C-D"];
        for (i, glyph) in LETTERS.iter().enumerate() {
            let is_center = i == 2;
            let scale = if is_center { 1.0 } else { 0.8 };
            let darkness = ((i as f32 - 2.0).abs() / 6.0).max(0.01);
            let alpha = 1.0 - darkness;
            let mut text = TextCommand::new(
                (*glyph).to_string(),
                glam::vec2(400.0 + (i as f32 * 80.0) + 50.0, 75.0 + 50.0),
                36.0 * scale,
            );
            text.color = glam::Vec4::new(1.0, 1.0, 1.0, alpha);
            text.z = 335;
            commands.push(text);
        }

        commands
    }

    /// Advance per-frame freeplay UI state (currently just the DJ state machine).
    /// Call before `commands()` each frame so Intro→Idle transitions land.
    pub fn tick(&mut self, cursor: Samples, sample_rate: u32) {
        if let Some(dj) = self.dj.as_mut() {
            dj.tick(cursor, sample_rate);
        }
    }

    /// Reset the DJ into the Intro animation AND restart the menu slide-in
    /// tween. Called on entering Freeplay so BF "ejects in" while the
    /// pinkBack/overhang/capsules slide to their resting positions.
    /// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:848-857
    /// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:725-744 (applyEnter)
    pub fn reset_dj_intro(&mut self, cursor: Samples) {
        self.enter_started_at = Some(cursor);
        if let Some(dj) = self.dj.as_mut() {
            dj.reset_intro(cursor);
        }
    }

    /// Eased 0..1 progress of the freeplay enter tween. Returns 1.0 (settled)
    /// when no enter has been started or the tween has elapsed.
    /// quartOut: t' = 1 - (1 - t)^4 — matches FlxEase.quartOut on the OG.
    fn enter_progress(&self, cursor: Samples, sample_rate: u32) -> f32 {
        let Some(start) = self.enter_started_at else {
            return 1.0;
        };
        let elapsed = cursor.0.saturating_sub(start.0).max(0) as f64;
        let secs = elapsed / f64::from(sample_rate.max(1));
        let t = (secs / ENTER_DURATION_SECS).clamp(0.0, 1.0) as f32;
        1.0 - (1.0 - t).powi(4)
    }

    /// Trigger the DJ Confirm animation when a song is selected.
    /// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:2744-2846
    pub fn dj_enter_confirm(&mut self, cursor: Samples) {
        if let Some(dj) = self.dj.as_mut() {
            dj.enter_confirm(cursor);
        }
    }

    pub fn dj_on_player_action(&mut self, cursor: Samples) {
        if let Some(dj) = self.dj.as_mut() {
            dj.on_player_action(cursor);
        }
    }

    pub fn item_count(&self) -> usize {
        self.songs.len()
    }

    pub fn song_at(&self, index: usize) -> Option<PreviewSong> {
        self.songs
            .get(index)
            .and_then(|capsule| match capsule.kind {
                CapsuleKind::Song(song) => Some(song),
                CapsuleKind::Random => None,
            })
    }

    pub fn is_random_at(&self, index: usize) -> bool {
        matches!(
            self.songs.get(index).map(|c| c.kind),
            Some(CapsuleKind::Random)
        )
    }

    pub fn index_of(&self, song: PreviewSong) -> Option<usize> {
        self.songs.iter().position(|capsule| match capsule.kind {
            CapsuleKind::Song(s) => s.id == song.id,
            CapsuleKind::Random => false,
        })
    }

    fn clamped_index(&self, index: usize) -> usize {
        index.min(self.songs.len().saturating_sub(1))
    }

    /// Logical width of the back footprint, used by elements that should
    /// settle with the back rather than extend with the diagonal (orange bar,
    /// BG cartoon position).
    fn pink_back_logical_width(&self) -> f32 {
        helpers::PINKBACK_LOGICAL_WIDTH
    }

    fn push_capsules(
        &self,
        commands: &mut RenderCommandList,
        selected_index: usize,
        difficulty: PreviewDifficulty,
        cursor: Samples,
        sample_rate: u32,
        enter_offset_x: f32,
    ) {
        // ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:663-691 (doJumpIn)
        // OG runs the xFrames sequence ONCE on initJumpIn and locks at (1,1).
        // We anchor it at `enter_started_at` so every capsule plays the
        // pop-in once on screen entry and then sits still — no per-beat loop.
        let enter_anchor = self.enter_started_at.unwrap_or(Samples(0));
        let (beat_scale_x, beat_scale_y) = capsule_beat_scale(cursor, enter_anchor, sample_rate);
        for index in 0..self.songs.len() {
            let offset = index as f32 - selected_index as f32;
            let pos = capsule_position(offset) + glam::vec2(enter_offset_x, 0.0);
            let is_selected = index == selected_index;
            let frames = if is_selected {
                &self.capsule_selected_frames
            } else {
                &self.capsule_unselected_frames
            };
            // ref: bdedc0aa:source/funkin/ui/freeplay/SongMenuItem.hx:96-98 (one-shot prefix)
            let Some(frame) =
                frame_for_cursor(frames, cursor, sample_rate, CAPSULE_ANIM_FPS, false)
            else {
                continue;
            };
            // Random capsule fades out below the selection until BF "does his hand";
            // for now keep it half-alpha to match the OG's intro state.
            let alpha = match self.songs[index].kind {
                CapsuleKind::Random => {
                    if is_selected {
                        1.0
                    } else {
                        0.6
                    }
                }
                CapsuleKind::Song(_) => 1.0,
            };
            commands.push(sparrow_scaled_command(
                self.capsule_atlas.texture_id,
                self.capsule_atlas.width,
                self.capsule_atlas.height,
                frame,
                pos,
                glam::vec2(
                    CAPSULE_REAL_SCALED * beat_scale_x,
                    CAPSULE_REAL_SCALED * beat_scale_y,
                ),
                glam::Vec4::new(1.0, 1.0, 1.0, alpha),
                200 + index as i32,
            ));
            if let CapsuleKind::Song(song) = self.songs[index].kind {
                self.capsule_metadata.push_capsule(
                    commands,
                    song,
                    difficulty,
                    pos,
                    alpha,
                    220 + index as i32,
                );
            }
        }
    }

    fn push_score(&self, commands: &mut RenderCommandList) {
        self.capsule_metadata.push_score(commands);
    }

    fn push_album(
        &self,
        commands: &mut RenderCommandList,
        selection: PreviewSelection,
        cursor: Samples,
        sample_rate: u32,
    ) {
        let album_id = self.album_id_for_selection(selection);
        let Some(album) = self
            .albums
            .get(album_id)
            .or_else(|| self.albums.get("volume1"))
        else {
            return;
        };
        commands.push(album.cover.command(
            glam::vec2(ALBUM_ART_X, ALBUM_ART_Y),
            glam::Vec4::ONE,
            315,
            glam::vec2(
                album.cover.width as f32 * ALBUM_ART_SCALE,
                album.cover.height as f32 * ALBUM_ART_SCALE,
            ),
        ));
        if let Some(frame) = album.title_frame.as_ref() {
            commands.push(sparrow_scaled_command(
                album.title_atlas.texture_id,
                album.title_atlas.width,
                album.title_atlas.height,
                frame,
                glam::vec2(ALBUM_TITLE_X, ALBUM_TITLE_Y) + album.title_offset,
                glam::Vec2::ONE,
                glam::Vec4::ONE,
                316,
            ));
        }
        if let Some(stars) = self.difficulty_stars.as_ref() {
            let rating = self.rating_for_selection(selection);
            for command in stars.commands(rating, cursor, sample_rate) {
                commands.push(command);
            }
        }
    }

    fn album_id_for_selection(&self, selection: PreviewSelection) -> &str {
        self.song_albums
            .get(&selection.song.id)
            .map(|metadata| metadata.album_id_for_selection(selection))
            .unwrap_or("volume1")
    }

    fn rating_for_selection(&self, selection: PreviewSelection) -> u8 {
        self.song_albums
            .get(&selection.song.id)
            .and_then(|metadata| metadata.rating_for_selection(selection))
            .unwrap_or_else(|| selection.song.difficulty_rating_for(selection.difficulty))
    }

    fn push_clear_box(&self, commands: &mut RenderCommandList) {
        // ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:613 (clearBoxSprite)
        let Some(tex) = self.clear_box.as_ref() else {
            return;
        };
        commands.push(tex.command(
            glam::vec2(1280.0 - 115.0, 65.0),
            glam::Vec4::ONE,
            308,
            glam::vec2(tex.width as f32, tex.height as f32),
        ));
    }

    fn push_highscore(&self, commands: &mut RenderCommandList) {
        let (Some(atlas), Some(frame)) =
            (self.highscore_atlas.as_ref(), self.highscore_frame.as_ref())
        else {
            return;
        };
        commands.push(sparrow_scaled_command(
            atlas.texture_id,
            atlas.width,
            atlas.height,
            frame,
            glam::vec2(HIGHSCORE_X, HIGHSCORE_Y),
            glam::Vec2::splat(HIGHSCORE_SCALE),
            glam::Vec4::ONE,
            305,
        ));
    }

    fn push_difficulty(
        &self,
        commands: &mut RenderCommandList,
        difficulty: PreviewDifficulty,
        cursor: Samples,
        sample_rate: u32,
    ) {
        if let Some(frame) = frame_for_cursor(
            &self.selector_frames,
            cursor,
            sample_rate,
            SELECTOR_ANIM_FPS,
            true,
        ) {
            commands.push(sparrow_scaled_command(
                self.selector_atlas.texture_id,
                self.selector_atlas.width,
                self.selector_atlas.height,
                frame,
                glam::vec2(SELECTOR_LEFT_X, SELECTOR_Y),
                glam::Vec2::ONE,
                glam::Vec4::ONE,
                280,
            ));
            commands.push(sparrow_scaled_command(
                self.selector_atlas.texture_id,
                self.selector_atlas.width,
                self.selector_atlas.height,
                frame,
                glam::vec2(SELECTOR_RIGHT_X + frame.frame_width as f32, SELECTOR_Y),
                glam::vec2(-1.0, 1.0),
                glam::Vec4::ONE,
                280,
            ));
        }
        match difficulty {
            PreviewDifficulty::Easy => self.push_static_difficulty(commands, &self.difficulty_easy),
            PreviewDifficulty::Normal => {
                self.push_static_difficulty(commands, &self.difficulty_normal)
            }
            PreviewDifficulty::Hard => self.push_static_difficulty(commands, &self.difficulty_hard),
            PreviewDifficulty::Erect => {
                self.push_static_difficulty(commands, &self.difficulty_erect)
            }
            PreviewDifficulty::Nightmare => {
                if let Some(frame) = frame_for_cursor(
                    &self.difficulty_nightmare_frames,
                    cursor,
                    sample_rate,
                    CAPSULE_ANIM_FPS,
                    true,
                ) {
                    commands.push(sparrow_scaled_command(
                        self.difficulty_nightmare.texture_id,
                        self.difficulty_nightmare.width,
                        self.difficulty_nightmare.height,
                        frame,
                        glam::vec2(DIFFICULTY_GROUP_X, DIFFICULTY_GROUP_Y),
                        glam::Vec2::ONE,
                        glam::Vec4::ONE,
                        290,
                    ));
                }
            }
        }
    }

    fn push_static_difficulty(&self, commands: &mut RenderCommandList, texture: &StaticTexture) {
        commands.push(texture.command(
            glam::vec2(DIFFICULTY_GROUP_X, DIFFICULTY_GROUP_Y),
            glam::Vec4::ONE,
            290,
            glam::vec2(texture.width as f32, texture.height as f32),
        ));
    }
}

#[derive(Debug)]
struct FreeplayCapsule {
    kind: CapsuleKind,
    display_name: String,
}

#[derive(Debug)]
pub(super) struct FreeplayAlbum {
    cover: StaticTexture,
    title_atlas: SparrowAtlasHandle,
    title_frame: Option<SparrowFrame>,
    title_offset: glam::Vec2,
}

#[derive(Debug, Clone, Copy)]
enum CapsuleKind {
    /// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:971-981
    Random,
    Song(PreviewSong),
}
