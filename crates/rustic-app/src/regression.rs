//! Headless regression scene helpers. See `PLAN.md` Section 14.
//!
//! Exposes minimal entry points the `xtask regression` command (and future
//! visual regression tests) can use to load deterministic gameplay scenes
//! without standing up the winit/cpal runtime.

use crate::camera_events::{apply_camera_event, focus_initial_camera};
use crate::camera_fx::CameraFx;
use crate::preview_song::{PreviewDifficulty, PreviewSelection, PreviewSong};
use crate::scene_assets::{load_preview_play_state_for, load_preview_scene_for, LoadedScene};
use anyhow::Result;
use rustic_core::time::Samples;
use rustic_game::PlayState;
use rustic_render::{CameraRegistry, DrawCommand};

/// Default sample rate used by the regression renderer. Matches
/// `scene_assets::SAMPLE_RATE` so chart hit windows quantize identically
/// to the live app.
pub const REGRESSION_SAMPLE_RATE: u32 = crate::scene_assets::SAMPLE_RATE;

/// One regression scene: which preview song + difficulty, plus a stable
/// label used as the golden filename.
#[derive(Debug, Clone, Copy)]
pub struct RegressionScenario {
    pub label: &'static str,
    pub song: PreviewSong,
    pub difficulty: PreviewDifficulty,
    pub cursor_ms: u32,
    pub frame_kind: RegressionFrameKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionFrameKind {
    Title,
    MainMenu,
    Credits,
    Options,
    StoryMenu,
    Freeplay,
    Stage,
    Gameplay,
    Pause,
}

impl RegressionScenario {
    pub const fn new(
        label: &'static str,
        song: PreviewSong,
        difficulty: PreviewDifficulty,
        cursor_ms: u32,
        frame_kind: RegressionFrameKind,
    ) -> Self {
        Self {
            label,
            song,
            difficulty,
            cursor_ms,
            frame_kind,
        }
    }

    pub fn selection(self) -> PreviewSelection {
        PreviewSelection::new(self.song, self.difficulty)
    }

    pub fn cursor(self) -> Samples {
        Samples((u64::from(self.cursor_ms) * u64::from(REGRESSION_SAMPLE_RATE) / 1000) as i64)
    }
}

/// Curated set used by the first golden batch per `PLAN.md` Section 14.
pub const FIRST_GOLDEN_SCENARIOS: &[RegressionScenario] = &[
    RegressionScenario::new(
        "title_skipped_intro",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        1200,
        RegressionFrameKind::Title,
    ),
    RegressionScenario::new(
        "main_menu_initial",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        600,
        RegressionFrameKind::MainMenu,
    ),
    RegressionScenario::new(
        "options_menu_root",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        600,
        RegressionFrameKind::Options,
    ),
    RegressionScenario::new(
        "credits_scroll_start",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        4000,
        RegressionFrameKind::Credits,
    ),
    RegressionScenario::new(
        "story_menu_week1",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        600,
        RegressionFrameKind::StoryMenu,
    ),
    RegressionScenario::new(
        "freeplay_week1",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        600,
        RegressionFrameKind::Freeplay,
    ),
    RegressionScenario::new(
        "stage_idle_bopeebo",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        0,
        RegressionFrameKind::Stage,
    ),
    RegressionScenario::new(
        "stage_idle_tutorial",
        PreviewSong::TUTORIAL,
        PreviewDifficulty::Normal,
        0,
        RegressionFrameKind::Stage,
    ),
    RegressionScenario::new(
        "bf_idle_two_beat_bopeebo",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        1200,
        RegressionFrameKind::Stage,
    ),
    RegressionScenario::new(
        "stage_camera_bump_bopeebo",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        5000,
        RegressionFrameKind::Stage,
    ),
    RegressionScenario::new(
        "gameplay_notes_crossing_bopeebo",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        5400,
        RegressionFrameKind::Gameplay,
    ),
    RegressionScenario::new(
        "pause_bopeebo",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
        5400,
        RegressionFrameKind::Pause,
    ),
];

/// Load the static scene (stage, characters, HUD skin, etc.) for a given
/// scenario. The returned `LoadedScene::commands` already contains the
/// initial frame's stage prop draws; characters/HUD/notes draws are still
/// driven per-frame by the live app.
pub fn load_scenario_scene(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scenario: RegressionScenario,
) -> Result<LoadedScene> {
    load_preview_scene_for(device, queue, scenario.selection())
}

/// Load the chart-driven `PlayState` for a scenario at the regression
/// sample rate.
pub fn load_scenario_play_state(scenario: RegressionScenario) -> Result<PlayState> {
    load_preview_play_state_for(scenario.selection(), REGRESSION_SAMPLE_RATE)
}

pub fn scenario_stage_prop_commands(
    scene: &LoadedScene,
    scenario: RegressionScenario,
) -> Vec<DrawCommand> {
    scene
        .stage_props
        .commands(scenario.cursor(), REGRESSION_SAMPLE_RATE)
}

pub fn scenario_cameras(
    scene: &LoadedScene,
    play_state: &PlayState,
    scenario: RegressionScenario,
) -> CameraRegistry {
    let mut cameras = CameraRegistry::with_default_fnf();
    let mut camera_fx = CameraFx::default();
    camera_fx.reset(&mut cameras, scene.camera_zoom);
    focus_initial_camera(&mut cameras, &mut camera_fx, scene.camera_focus);

    let mut state = play_state.clone();
    let cursor = scenario.cursor();
    let mut step = Samples(0);
    let frame_samples = i64::from(REGRESSION_SAMPLE_RATE / 60).max(1);
    loop {
        for event in state.resolve_song_events(step) {
            apply_camera_event(
                &mut cameras,
                &mut camera_fx,
                scene.camera_focus,
                &event.kind,
                step,
                REGRESSION_SAMPLE_RATE,
                state.bpm,
            );
        }
        camera_fx.update(&mut cameras, step, REGRESSION_SAMPLE_RATE, state.bpm);
        if step >= cursor {
            break;
        }
        step = Samples((step.0 + frame_samples).min(cursor.0));
    }
    cameras
}
