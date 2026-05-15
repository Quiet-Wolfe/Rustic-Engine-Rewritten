use super::*;
use rustic_core::ids::NoteId;

fn test_note_skin() -> NoteSkin {
    let strumline = SparrowAtlas::parse(
        br#"
        <TextureAtlas imagePath="noteStrumline.png">
            <SubTexture name="staticLeft0001" x="0" y="0" width="154" height="157"/>
            <SubTexture name="pressLeft0001" x="154" y="0" width="139" height="141"
                frameX="-4" frameY="-3" frameWidth="146" frameHeight="148"/>
            <SubTexture name="pressLeft0002" x="293" y="0" width="146" height="148"/>
            <SubTexture name="confirmLeft0001" x="439" y="0" width="226" height="228"/>
            <SubTexture name="confirmLeft0002" x="665" y="0" width="216" height="218"/>
            <SubTexture name="confirmLeft0003" x="881" y="0" width="217" height="217"/>
            <SubTexture name="confirmLeft0004" x="881" y="0" width="217" height="217"/>
            <SubTexture name="confirmHoldLeft0001" x="1098" y="0" width="217" height="217"/>
            <SubTexture name="confirmHoldLeft0002" x="1315" y="0" width="223" height="224"/>
        </TextureAtlas>
        "#,
    )
    .unwrap();
    let tap = SparrowAtlas::parse(
        br#"
        <TextureAtlas imagePath="notes.png">
            <SubTexture name="noteLeft0001" x="0" y="0" width="154" height="157"/>
        </TextureAtlas>
        "#,
    )
    .unwrap();
    let hold = SparrowAtlas::parse(
        br#"
        <TextureAtlas imagePath="NOTE_assets.png">
            <SubTexture name="purple hold piece0000" x="0" y="0" width="50" height="44"/>
            <SubTexture name="pruple end hold0000" x="50" y="0" width="50" height="64"/>
        </TextureAtlas>
        "#,
    )
    .unwrap();
    let static_frame = strumline
        .first_animation_frame("staticLeft0", &[])
        .unwrap()
        .clone();
    let press_frames: Vec<_> = strumline
        .animation_frames("pressLeft0", &[])
        .into_iter()
        .cloned()
        .collect();
    let confirm_frames: Vec<_> = strumline
        .animation_frames("confirmLeft0", &[])
        .into_iter()
        .cloned()
        .collect();
    let confirm_hold_frames: Vec<_> = strumline
        .animation_frames("confirmHoldLeft0", &[])
        .into_iter()
        .cloned()
        .collect();
    let tap_frame = tap.first_animation_frame("noteLeft0", &[]).unwrap().clone();
    let hold_frame = hold
        .first_animation_frame("purple hold piece", &[])
        .unwrap()
        .clone();
    let hold_end_frame = hold
        .first_animation_frame("pruple end hold", &[])
        .unwrap()
        .clone();

    NoteSkin {
        scale: NOTE_ASSET_SCALE,
        filter: FilterMode::Linear,
        strumline_offset: glam::Vec2::ZERO,
        tap_texture_id: AssetId::new(1),
        tap_texture_width: 311,
        tap_texture_height: 311,
        strumline_texture_id: AssetId::new(2),
        strumline_texture_width: 2019,
        strumline_texture_height: 810,
        hold_texture_id: AssetId::new(3),
        hold_texture_width: 2048,
        hold_texture_height: 1024,
        hold_trail_texture_id: AssetId::new(4),
        hold_trail_texture_width: 416,
        hold_trail_texture_height: 87,
        static_frames: std::array::from_fn(|_| static_frame.clone()),
        press_frames: std::array::from_fn(|_| press_frames.clone()),
        confirm_frames: std::array::from_fn(|_| confirm_frames.clone()),
        confirm_hold_frames: std::array::from_fn(|_| confirm_hold_frames.clone()),
        tap_frames: std::array::from_fn(|_| tap_frame.clone()),
        hold_frames: std::array::from_fn(|_| hold_frame.clone()),
        hold_end_frames: std::array::from_fn(|_| hold_end_frame.clone()),
    }
}

fn hold_view(id: u32, height: f32) -> HoldTrailView {
    HoldTrailView::new(
        NoteId::new(id),
        Lane::Down,
        false,
        false,
        800.0,
        100.0,
        height,
    )
}

#[test]
fn note_sprite_x_centers_tap_assets_on_v085_slot() {
    assert!((note_sprite_x(688.0, 154.0 * NOTE_ASSET_SCALE) - 684.1).abs() < 1e-4);
}

#[test]
fn receptor_frames_center_the_source_frame_on_the_lane_slot() {
    let skin = test_note_skin();
    let static_cmd =
        skin.receptor_command(1, Lane::Left, ReceptorState::Static, Samples(0), 48_000);
    let press_cmd = skin.receptor_command(
        1,
        Lane::Left,
        ReceptorState::Pressed {
            started_at: Samples(0),
        },
        Samples(0),
        48_000,
    );

    let static_center = command_source_center(&static_cmd, &skin.static_frames[0]);
    let press_center = command_source_center(&press_cmd, &skin.press_frames[0][0]);
    let confirm_cmd = skin.receptor_command(
        1,
        Lane::Left,
        ReceptorState::Confirm {
            started_at: Samples(0),
            hold: false,
        },
        Samples(0),
        48_000,
    );
    let confirm_center = command_source_center(&confirm_cmd, &skin.confirm_frames[0][0]);

    assert_eq!(static_cmd.texture, AssetId::new(2));
    assert_eq!(press_cmd.texture, AssetId::new(2));
    assert!((press_center.x - static_center.x).abs() < 1e-5);
    assert!((press_center.y - static_center.y).abs() < 1e-5);
    assert!((confirm_center.x - static_center.x).abs() < 1e-5);
    assert!((confirm_center.y - static_center.y).abs() < 1e-5);
}

#[test]
fn receptor_layout_can_hide_opponent_and_shift_player() {
    let skin = test_note_skin();

    let commands = skin.receptor_commands_with_layout(Samples(0), 48_000, false, -272.0, |_, _| {
        ReceptorState::Static
    });

    assert_eq!(commands.len(), 4);
    let shifted = skin.receptor_command(1, Lane::Left, ReceptorState::Static, Samples(0), 48_000);
    assert_eq!(commands[0].world_pos.x, shifted.world_pos.x - 272.0);
}

fn command_source_center(cmd: &DrawCommand, frame: &SparrowFrame) -> glam::Vec2 {
    cmd.world_pos + (frame_trim_offset(frame) + frame_source_size(frame) * 0.5) * NOTE_ASSET_SCALE
}

#[test]
fn confirm_duration_includes_og_hold_timer() {
    assert_eq!(test_note_skin().confirm_duration(48_000), Samples(15_200));
}

#[test]
fn hold_confirm_switches_to_confirm_hold_after_confirm_animation() {
    let skin = test_note_skin();
    let state = ReceptorState::Confirm {
        started_at: Samples(0),
        hold: true,
    };

    assert_eq!(
        skin.receptor_frame(Lane::Left, state, Samples(7_999), 48_000)
            .name,
        "confirmLeft0004"
    );
    assert_eq!(
        skin.receptor_frame(Lane::Left, state, Samples(8_000), 48_000)
            .name,
        "confirmHoldLeft0001"
    );
}

#[test]
fn hold_trail_commands_use_wrapped_tail_and_cap_from_note_hold_assets() {
    let skin = test_note_skin();
    let commands = skin.hold_trail_commands(&hold_view(0, 225.0));

    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].texture, AssetId::new(4));
    assert!((commands[0].world_pos.x - 833.8).abs() < 1e-3);
    assert_eq!(commands[0].world_pos.y, 100.0);
    assert!((commands[0].size.y - 194.55).abs() < 1e-3);
    assert_eq!((commands[0].uv_min.x, commands[0].uv_max.x), (0.25, 0.375));
    assert!((commands[0].uv_min.y + 3.194_581).abs() < 1e-3);
    assert_eq!(commands[0].uv_max.y, 0.0);
    assert!(commands[0].uv_wrap_y);
    assert_eq!((commands[1].uv_min.x, commands[1].uv_max.x), (0.375, 0.5));
    assert_eq!(commands[1].uv_max.y, HOLD_TRAIL_BOTTOM_CLIP);

    let short = skin.hold_trail_commands(&hold_view(1, 15.0));
    assert_eq!(short.len(), 1);
    assert!(!short[0].uv_wrap_y);
    assert!((short[0].uv_min.y - 0.2537).abs() < 1e-3);
    assert!((short[0].size.y - 39.36).abs() < 1e-3);
}

#[test]
fn animated_note_frame_uses_started_cursor_and_clamps() {
    let atlas = SparrowAtlas::parse(
        br#"
        <TextureAtlas imagePath="noteStrumline.png">
            <SubTexture name="confirm0000" x="0" y="0" width="1" height="1"/>
            <SubTexture name="confirm0001" x="1" y="0" width="1" height="1"/>
            <SubTexture name="confirm0002" x="2" y="0" width="1" height="1"/>
        </TextureAtlas>
        "#,
    )
    .unwrap();
    let frames: Vec<_> = atlas
        .animation_frames("confirm", &[])
        .into_iter()
        .cloned()
        .collect();

    assert_eq!(
        animated_note_frame(&frames, Samples(12_000), 48_000, Samples(10_000), 24, false).name,
        "confirm0000"
    );
    assert_eq!(
        animated_note_frame(&frames, Samples(12_001), 48_000, Samples(10_000), 24, false).name,
        "confirm0001"
    );
    assert_eq!(
        animated_note_frame(&frames, Samples(96_000), 48_000, Samples(10_000), 24, false).name,
        "confirm0002"
    );
}
