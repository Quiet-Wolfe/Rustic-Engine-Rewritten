//! Headless regression scene helpers. See `PLAN.md` Section 14.
//!
//! Exposes minimal entry points the `xtask regression` command (and future
//! visual regression tests) can use to load deterministic gameplay scenes
//! without standing up the winit/cpal runtime.

use crate::preview_song::{PreviewDifficulty, PreviewSelection, PreviewSong};
use crate::scene_assets::{load_preview_play_state_for, load_preview_scene_for, LoadedScene};
use anyhow::Result;
use rustic_game::PlayState;

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
}

impl RegressionScenario {
    pub const fn new(
        label: &'static str,
        song: PreviewSong,
        difficulty: PreviewDifficulty,
    ) -> Self {
        Self {
            label,
            song,
            difficulty,
        }
    }

    pub fn selection(self) -> PreviewSelection {
        PreviewSelection::new(self.song, self.difficulty)
    }
}

/// Curated set used by the first golden batch per `PLAN.md` Section 14.
pub const FIRST_GOLDEN_SCENARIOS: &[RegressionScenario] = &[
    RegressionScenario::new(
        "stage_idle_bopeebo",
        PreviewSong::BOPEEBO,
        PreviewDifficulty::Normal,
    ),
    RegressionScenario::new(
        "stage_idle_tutorial",
        PreviewSong::TUTORIAL,
        PreviewDifficulty::Normal,
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
