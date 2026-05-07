//! Funkin' v0.8.5 hold-note cover glow animations.
// LINT-ALLOW: long-file four-lane hold cover atlas wiring plus animation tests

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

const FNF_WIDTH: f32 = 1280.0;
const STRUMLINE_X_OFFSET: f32 = 48.0;
const STRUMLINE_Y_OFFSET: f32 = 24.0;
const STRUMLINE_SIZE: f32 = 104.0;
const NOTE_SPACING: f32 = STRUMLINE_SIZE + 8.0;
const INITIAL_OFFSET: f32 = -0.275 * STRUMLINE_SIZE;
const COVER_X_ADJUST: f32 = -12.0;
const COVER_Y_ADJUST: f32 = -96.0;
const COVER_FPS: u16 = 24;

#[derive(Debug, Clone)]
pub struct HoldCoverSkin {
    lanes: [LaneHoldCoverSkin; 4],
}

#[derive(Debug, Clone)]
struct LaneHoldCoverSkin {
    texture_id: AssetId,
    texture_width: u32,
    texture_height: u32,
    start_frames: Vec<SparrowFrame>,
    hold_frames: Vec<SparrowFrame>,
    end_frames: Vec<SparrowFrame>,
}

#[derive(Debug, Default, Clone)]
pub struct HoldCovers {
    active: [[Option<ActiveHoldCover>; 4]; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoldCoverPhase {
    Start,
    Hold,
    End,
}

#[derive(Debug, Clone, Copy)]
struct ActiveHoldCover {
    side: StrumlineSide,
    lane: Lane,
    hold_end_at: Samples,
    phase: HoldCoverPhase,
    phase_started_at: Samples,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrumlineSide {
    Opponent,
    Player,
}

impl HoldCovers {
    pub fn start(&mut self, lane: Lane, cursor: Samples, hold_end_at: Samples) {
        self.start_on(StrumlineSide::Player, lane, cursor, hold_end_at);
    }

    pub fn start_opponent(&mut self, lane: Lane, cursor: Samples, hold_end_at: Samples) {
        self.start_on(StrumlineSide::Opponent, lane, cursor, hold_end_at);
    }

    fn start_on(&mut self, side: StrumlineSide, lane: Lane, cursor: Samples, hold_end_at: Samples) {
        if hold_end_at <= cursor {
            return;
        }
        self.active[side_index(side)][lane_index(lane)] = Some(ActiveHoldCover {
            side,
            lane,
            hold_end_at,
            phase: HoldCoverPhase::Start,
            phase_started_at: cursor,
        });
    }

    pub fn end(&mut self, lane: Lane, cursor: Samples) -> Option<Samples> {
        let active = self.active[side_index(StrumlineSide::Player)][lane_index(lane)].as_mut()?;
        let hold_end_at = (active.phase != HoldCoverPhase::End && active.hold_end_at > cursor)
            .then_some(active.hold_end_at);
        if active.phase != HoldCoverPhase::End {
            active.phase = HoldCoverPhase::End;
            active.phase_started_at = cursor;
        }
        hold_end_at
    }

    pub fn commands(
        &mut self,
        skin: &HoldCoverSkin,
        cursor: Samples,
        sample_rate: u32,
    ) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        for side_index in 0..self.active.len() {
            for lane_slot in 0..self.active[side_index].len() {
                let Some(active) = self.active[side_index][lane_slot].as_mut() else {
                    continue;
                };
                if active.phase != HoldCoverPhase::End && cursor >= active.hold_end_at {
                    active.phase = HoldCoverPhase::End;
                    active.phase_started_at = active.hold_end_at;
                }

                let lane_skin = &skin.lanes[lane_index(active.lane)];
                let Some(frame) = frame_for_active(active, lane_skin, cursor, sample_rate) else {
                    self.active[side_index][lane_slot] = None;
                    continue;
                };
                commands.push(command_for_frame(
                    lane_skin,
                    active.side,
                    active.lane,
                    frame,
                ));
            }
        }
        commands
    }
}

pub fn load_hold_cover_assets(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
) -> Result<HoldCoverSkin> {
    Ok(HoldCoverSkin {
        lanes: [
            load_lane_cover(
                device,
                queue,
                resolver,
                textures,
                "holdCoverPurple",
                "Purple",
            )?,
            load_lane_cover(device, queue, resolver, textures, "holdCoverBlue", "Blue")?,
            load_lane_cover(device, queue, resolver, textures, "holdCoverGreen", "Green")?,
            load_lane_cover(device, queue, resolver, textures, "holdCoverRed", "Red")?,
        ],
    })
}

fn load_lane_cover(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resolver: &OverlayResolver,
    textures: &mut HashMap<AssetId, Texture>,
    asset_name: &str,
    color_name: &str,
) -> Result<LaneHoldCoverSkin> {
    let atlas_path = AssetPath::new(format!("images/{asset_name}.xml"))?;
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
            FilterMode::Linear,
            Some(texture_path.as_str()),
        ),
    );

    Ok(LaneHoldCoverSkin {
        texture_id,
        texture_width: image.width,
        texture_height: image.height,
        start_frames: cloned_animation_frames(&atlas, &format!("holdCoverStart{color_name}"))?,
        hold_frames: cloned_animation_frames(&atlas, &format!("holdCover{color_name}"))?,
        end_frames: cloned_animation_frames(&atlas, &format!("holdCoverEnd{color_name}"))?,
    })
}

fn frame_for_active<'a>(
    active: &mut ActiveHoldCover,
    skin: &'a LaneHoldCoverSkin,
    cursor: Samples,
    sample_rate: u32,
) -> Option<&'a SparrowFrame> {
    match active.phase {
        HoldCoverPhase::Start => {
            if let Some(index) = animation_frame_index(
                cursor,
                sample_rate,
                active.phase_started_at,
                skin.start_frames.len(),
                false,
            ) {
                Some(&skin.start_frames[index])
            } else {
                active.phase = HoldCoverPhase::Hold;
                active.phase_started_at = cursor;
                looped_frame(
                    &skin.hold_frames,
                    cursor,
                    sample_rate,
                    active.phase_started_at,
                )
            }
        }
        HoldCoverPhase::Hold => looped_frame(
            &skin.hold_frames,
            cursor,
            sample_rate,
            active.phase_started_at,
        ),
        HoldCoverPhase::End => animation_frame_index(
            cursor,
            sample_rate,
            active.phase_started_at,
            skin.end_frames.len(),
            false,
        )
        .map(|index| &skin.end_frames[index]),
    }
}

fn looped_frame(
    frames: &[SparrowFrame],
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
) -> Option<&SparrowFrame> {
    animation_frame_index(cursor, sample_rate, started_at, frames.len(), true)
        .map(|index| &frames[index])
}

fn command_for_frame(
    skin: &LaneHoldCoverSkin,
    side: StrumlineSide,
    lane: Lane,
    frame: &SparrowFrame,
) -> DrawCommand {
    let size = frame_draw_size(frame);
    let mut cmd = DrawCommand::sprite(
        skin.texture_id,
        frame_world_pos(side, lane, frame, size),
        size,
    );
    cmd.camera = CameraId(1);
    cmd.pivot = glam::Vec2::ZERO;
    cmd.layer = RenderLayer::Notes;
    cmd.z = 9;
    cmd.filter = FilterMode::Linear;
    (cmd.uv_min, cmd.uv_max) = frame_uv(frame, skin.texture_width, skin.texture_height);
    cmd.uv_rotated = frame.rotated;
    cmd
}

fn frame_draw_size(frame: &SparrowFrame) -> glam::Vec2 {
    if frame.rotated {
        glam::vec2(frame.height as f32, frame.width as f32)
    } else {
        glam::vec2(frame.width as f32, frame.height as f32)
    }
}

fn frame_world_pos(
    side: StrumlineSide,
    lane: Lane,
    frame: &SparrowFrame,
    size: glam::Vec2,
) -> glam::Vec2 {
    // ref: bdedc0aa:source/funkin/play/notes/Strumline.hx:1107-1131
    let frame_width = frame.frame_width as f32;
    let group_x = strumline_x(side) + lane_index(lane) as f32 * NOTE_SPACING + STRUMLINE_SIZE * 0.5
        - frame_width * 0.5
        + COVER_X_ADJUST;
    let group_y = STRUMLINE_Y_OFFSET + INITIAL_OFFSET + STRUMLINE_SIZE * 0.5 + COVER_Y_ADJUST;
    let trimmed = glam::vec2(frame.frame_x as f32, frame.frame_y as f32);
    glam::vec2(group_x, group_y) - trimmed + (frame_draw_size(frame) - size) * 0.5
}

fn strumline_x(side: StrumlineSide) -> f32 {
    match side {
        StrumlineSide::Opponent => STRUMLINE_X_OFFSET,
        StrumlineSide::Player => FNF_WIDTH * 0.5 + STRUMLINE_X_OFFSET,
    }
}

fn animation_frame_index(
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
    frame_count: usize,
    looped: bool,
) -> Option<usize> {
    if frame_count == 0 || cursor < started_at {
        return None;
    }
    let samples_per_frame = f64::from(sample_rate.max(1)) / f64::from(COVER_FPS);
    let frame = ((cursor.0 - started_at.0) as f64 / samples_per_frame).floor() as usize;
    if looped {
        Some(frame % frame_count)
    } else {
        (frame < frame_count).then_some(frame)
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

fn atlas_texture_path(atlas_path: &AssetPath, atlas: &SparrowAtlas) -> Result<AssetPath> {
    if atlas.image_path.contains('/') {
        Ok(AssetPath::new(atlas.image_path.clone())?)
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
        anyhow::bail!("resolve hold cover frame {prefix}");
    }
    Ok(frames)
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

fn side_index(side: StrumlineSide) -> usize {
    match side {
        StrumlineSide::Opponent => 0,
        StrumlineSide::Player => 1,
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::Path;

    fn lane_skin() -> LaneHoldCoverSkin {
        let atlas = SparrowAtlas::parse(
            br#"
            <TextureAtlas imagePath="holdCoverBlue.png" width="599" height="591">
              <SubTexture name="holdCoverStartBlue0001" x="413" y="96" width="93" height="93"
                frameX="-111" frameY="-107" frameWidth="300" frameHeight="400" />
              <SubTexture name="holdCoverBlue0001" x="407" y="242" width="108" height="138"
                rotated="true" frameX="-94" frameY="-94" frameWidth="300" frameHeight="400" />
              <SubTexture name="holdCoverEndBlue0001" x="146" y="423" width="36" height="78"
                rotated="true" frameX="-122" frameY="-133" frameWidth="300" frameHeight="400" />
            </TextureAtlas>
            "#,
        )
        .unwrap();
        LaneHoldCoverSkin {
            texture_id: AssetId::new(7),
            texture_width: 599,
            texture_height: 591,
            start_frames: cloned_animation_frames(&atlas, "holdCoverStartBlue").unwrap(),
            hold_frames: cloned_animation_frames(&atlas, "holdCoverBlue").unwrap(),
            end_frames: cloned_animation_frames(&atlas, "holdCoverEndBlue").unwrap(),
        }
    }

    fn skin() -> HoldCoverSkin {
        let lane = lane_skin();
        HoldCoverSkin {
            lanes: std::array::from_fn(|_| lane.clone()),
        }
    }

    fn source_atlas(path: &str) -> SparrowAtlas {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest_dir.parent().unwrap().parent().unwrap();
        let bytes = std::fs::read(workspace.join("assets/source").join(path)).unwrap();
        SparrowAtlas::parse(&bytes).unwrap()
    }

    fn source_lane_skin(asset_name: &str, color_name: &str) -> LaneHoldCoverSkin {
        let atlas = source_atlas(&format!("images/{asset_name}.xml"));
        LaneHoldCoverSkin {
            texture_id: AssetId::new(7),
            texture_width: 599,
            texture_height: 591,
            start_frames: cloned_animation_frames(&atlas, &format!("holdCoverStart{color_name}"))
                .unwrap(),
            hold_frames: cloned_animation_frames(&atlas, &format!("holdCover{color_name}"))
                .unwrap(),
            end_frames: cloned_animation_frames(&atlas, &format!("holdCoverEnd{color_name}"))
                .unwrap(),
        }
    }

    fn source_skin() -> HoldCoverSkin {
        HoldCoverSkin {
            lanes: [
                source_lane_skin("holdCoverPurple", "Purple"),
                source_lane_skin("holdCoverBlue", "Blue"),
                source_lane_skin("holdCoverGreen", "Green"),
                source_lane_skin("holdCoverRed", "Red"),
            ],
        }
    }

    #[test]
    fn rotated_frames_swap_draw_size_and_set_uv_flag() {
        let skin = lane_skin();
        let frame = &skin.hold_frames[0];

        let cmd = command_for_frame(&skin, StrumlineSide::Player, Lane::Down, frame);

        assert!(cmd.uv_rotated);
        assert_eq!(cmd.size, glam::vec2(138.0, 108.0));
    }

    #[test]
    fn opponent_cover_uses_left_strumline() {
        let skin = lane_skin();
        let frame = &skin.start_frames[0];

        let player = command_for_frame(&skin, StrumlineSide::Player, Lane::Left, frame);
        let opponent = command_for_frame(&skin, StrumlineSide::Opponent, Lane::Left, frame);

        assert!((player.world_pos.x - opponent.world_pos.x - FNF_WIDTH * 0.5).abs() < 1e-3);
    }

    #[test]
    fn covers_transition_from_start_to_hold_to_end() {
        let mut covers = HoldCovers::default();
        covers.start(Lane::Down, Samples(0), Samples(48_000));

        let start = covers.commands(&skin(), Samples(0), 48_000);
        assert_eq!(start.len(), 1);
        assert_eq!(start[0].texture, AssetId::new(7));
        assert!(!start[0].uv_rotated);

        let hold = covers.commands(&skin(), Samples(2_100), 48_000);
        assert_eq!(hold.len(), 1);
        assert!(hold[0].uv_rotated);

        assert_eq!(
            covers.end(Lane::Down, Samples(3_000)),
            Some(Samples(48_000))
        );
        let end = covers.commands(&skin(), Samples(3_000), 48_000);
        assert_eq!(end.len(), 1);
        assert!(end[0].uv_rotated);

        let done = covers.commands(&skin(), Samples(6_000), 48_000);
        assert!(done.is_empty());
    }

    #[test]
    fn tracked_source_hold_covers_emit_commands_for_all_lanes() {
        let skin = source_skin();
        for lane in &skin.lanes {
            assert_eq!(lane.start_frames.len(), 1);
            assert_eq!(lane.hold_frames.len(), 4);
            assert_eq!(lane.end_frames.len(), 8);
        }

        let mut covers = HoldCovers::default();
        for lane in [Lane::Left, Lane::Down, Lane::Up, Lane::Right] {
            covers.start(lane, Samples(0), Samples(48_000));
        }

        let commands = covers.commands(&skin, Samples(0), 48_000);
        assert_eq!(commands.len(), 4);
        assert!(commands.iter().all(|cmd| cmd.layer == RenderLayer::Notes));
        assert!(commands.iter().all(|cmd| cmd.z == 9));
    }
}
