use super::*;
use crate::animation_timing::flixel_frame_index;

pub(super) fn story_difficulties(level: &LevelDefinition) -> Vec<PreviewDifficulty> {
    let mut difficulties = STORY_DIFFICULTIES.to_vec();
    for song_id in &level.songs {
        if let Some(song) = PreviewSong::from_key(song_id) {
            difficulties.retain(|difficulty| song.available_difficulties().contains(difficulty));
        }
    }
    if difficulties.is_empty() {
        difficulties.push(PreviewDifficulty::Normal);
    }
    difficulties
}

pub(super) fn title_y_positions(levels: &[StoryLevel], selected_index: usize) -> Vec<f32> {
    let mut out = vec![TITLE_SELECTED_Y; levels.len()];
    if levels.is_empty() {
        return out;
    }
    let selected = selected_index.min(levels.len() - 1);
    out[selected] = TITLE_SELECTED_Y;
    for index in (0..selected).rev() {
        let next = index + 1;
        out[index] = out[next] - (levels[index].title.height as f32 + 20.0).max(MIN_TITLE_SPACING);
    }
    for index in selected + 1..levels.len() {
        let previous = index - 1;
        out[index] = out[previous] + levels[previous].title.height as f32 + 20.0;
    }
    out
}

pub(super) fn tracklist_text(level: &StoryLevel, difficulty: PreviewDifficulty) -> String {
    let mut text = String::from("TRACKS\n\n");
    let names = level
        .data
        .songs
        .iter()
        .map(|song| {
            PreviewSong::from_key(song)
                .map(PreviewSong::display_name)
                .unwrap_or("Unknown")
        })
        .collect::<Vec<_>>();
    text.push_str(&names.join("\n"));
    text.push_str(&format!("\n\n{}", difficulty.as_str().to_ascii_uppercase()));
    text
}

pub(super) fn push_text(
    commands: &mut TextCommandList,
    text: &str,
    position: glam::Vec2,
    size: f32,
    color: glam::Vec4,
    z: i32,
) {
    let mut command = TextCommand::new(text, position, size);
    command.color = color;
    command.z = z;
    commands.push(command);
}

pub(super) fn solid_command(
    pos: glam::Vec2,
    size: glam::Vec2,
    color: glam::Vec4,
    z: i32,
) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(WHITE_TEXTURE_ID, pos, size);
    cmd.camera = CameraId(1);
    cmd.layer = RenderLayer::Background;
    cmd.z = z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Nearest;
    cmd.color = color;
    cmd
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sparrow_command(
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    frame: &SparrowFrame,
    position: glam::Vec2,
    scale: glam::Vec2,
    color: glam::Vec4,
    z: i32,
) -> DrawCommand {
    let mut cmd = DrawCommand::sprite(texture_id, position, frame_draw_size(frame) * scale);
    cmd.camera = CameraId(1);
    cmd.layer = RenderLayer::Overlay;
    cmd.z = z;
    cmd.pivot = glam::Vec2::ZERO;
    cmd.filter = FilterMode::Linear;
    cmd.color = color;
    (cmd.uv_min, cmd.uv_max) = frame_uv(frame, texture_width, texture_height);
    cmd.uv_rotated = frame.rotated;
    cmd
}

pub(super) fn animation_frame_index(
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
    fps: u16,
    frame_count: usize,
    looped: bool,
) -> usize {
    flixel_frame_index(cursor, sample_rate, started_at, fps, frame_count, looped)
}

pub(super) fn story_beat(cursor: Samples, sample_rate: u32) -> i64 {
    let beat_samples = f64::from(sample_rate.max(1)) * 60.0 / MENU_BPM;
    (cursor.0.max(0) as f64 / beat_samples).floor() as i64
}

pub(super) fn current_beat_start(cursor: Samples, sample_rate: u32) -> Samples {
    let beat_samples = f64::from(sample_rate.max(1)) * 60.0 / MENU_BPM;
    Samples((story_beat(cursor, sample_rate) as f64 * beat_samples).round() as i64)
}

pub(super) fn title_confirm_flash_color(
    cursor: Samples,
    sample_rate: u32,
    confirm_started_at: Option<Samples>,
    alpha: f32,
) -> glam::Vec4 {
    let Some(started_at) = confirm_started_at else {
        return glam::Vec4::new(1.0, 1.0, 1.0, alpha);
    };
    let elapsed = cursor.0.saturating_sub(started_at.0).max(0);
    let tick_samples = i64::from(sample_rate.max(1)) / 20;
    if tick_samples > 0 && elapsed / tick_samples % 2 == 1 {
        glam::Vec4::new(0x33 as f32 / 255.0, 1.0, 1.0, alpha)
    } else {
        glam::Vec4::new(1.0, 1.0, 1.0, alpha)
    }
}

fn frame_draw_size(frame: &SparrowFrame) -> glam::Vec2 {
    if frame.rotated {
        glam::vec2(frame.height as f32, frame.width as f32)
    } else {
        glam::vec2(frame.width as f32, frame.height as f32)
    }
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

pub(super) fn color_from_story_hex(value: &str) -> glam::Vec4 {
    let raw = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if raw.len() != 6 {
        return glam::Vec4::new(0.98, 0.81, 0.32, 1.0);
    }
    match u32::from_str_radix(raw, 16) {
        Ok(color) => glam::Vec4::new(
            ((color >> 16) & 0xff) as f32 / 255.0,
            ((color >> 8) & 0xff) as f32 / 255.0,
            (color & 0xff) as f32 / 255.0,
            1.0,
        ),
        Err(_) => glam::Vec4::new(0.98, 0.81, 0.32, 1.0),
    }
}

pub(super) fn asset_id_for_path(path: &AssetPath) -> AssetId {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in path.as_str().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    AssetId::new(hash)
}
