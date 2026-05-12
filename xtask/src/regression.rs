//! Visual regression runner. See `PLAN.md` Section 14.
//!
//! Boots a deterministic capture path through `wgpu` (no swapchain, no
//! winit), renders each scenario into the 1280x720 reference target, and
//! either writes a fresh golden into `tests/golden/` or compares to an
//! existing one. Pixel diffs above the per-image threshold fail with a
//! non-zero exit.

use anyhow::{Context, Result};
use glam::{vec2, vec4};
use image::{ImageBuffer, RgbaImage};
use rustic_app::bitmap_text_assets::load_vcr_ttf_bytes;
use rustic_app::character_anim::CharacterAnimState;
use rustic_app::regression::{
    load_scenario_play_state, load_scenario_scene, RegressionScenario, FIRST_GOLDEN_SCENARIOS,
    REGRESSION_SAMPLE_RATE,
};
use rustic_core::ids::CameraId;
use rustic_core::time::Samples;
use rustic_render::{
    capture_reference_rgba, CameraRegistry, RenderCommandList, RenderState, SpriteBatcher,
    SpritePipeline, TextCommand, TextCommandList, TextSystem, REFERENCE_HEIGHT, REFERENCE_WIDTH,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const DEFAULT_DIFF_THRESHOLD: f64 = 0.005;

pub fn run(workspace: &Path, write: bool, scenario_filter: Option<&str>) -> Result<()> {
    let golden_dir = workspace.join("tests/golden");
    std::fs::create_dir_all(&golden_dir)
        .with_context(|| format!("create {}", golden_dir.display()))?;

    let mut harness = Harness::new()?;
    let scenarios = collect_scenarios(scenario_filter);
    if scenarios.is_empty() {
        anyhow::bail!("no regression scenarios matched filter {scenario_filter:?}");
    }

    let mut failures = Vec::new();
    for scenario in &scenarios {
        let png = harness.render_scenario(scenario)?;
        let path = golden_dir.join(format!("{}.png", scenario.label));
        match handle_golden(&path, &png, write) {
            Ok(GoldenOutcome::Wrote) => println!("regression: wrote {}", path.display()),
            Ok(GoldenOutcome::Match { diff_ratio }) => println!(
                "regression: {} matches golden (ratio={:.6})",
                scenario.label, diff_ratio
            ),
            Ok(GoldenOutcome::Stale { diff_ratio }) => {
                failures.push(format!(
                    "{} diverged (ratio={:.6} > {})",
                    scenario.label, diff_ratio, DEFAULT_DIFF_THRESHOLD
                ));
            }
            Err(e) => failures.push(format!("{}: {e:#}", scenario.label)),
        }
    }
    if !failures.is_empty() {
        anyhow::bail!(
            "regression failures:\n  - {}\nRe-run with --write to refresh goldens.",
            failures.join("\n  - ")
        );
    }
    Ok(())
}

fn collect_scenarios(filter: Option<&str>) -> Vec<RegressionScenario> {
    match filter {
        None => FIRST_GOLDEN_SCENARIOS.to_vec(),
        Some(name) => FIRST_GOLDEN_SCENARIOS
            .iter()
            .filter(|s| s.label == name)
            .copied()
            .collect(),
    }
}

struct Harness {
    rs: RenderState,
    pipeline: SpritePipeline,
    batcher: SpriteBatcher,
    text: TextSystem,
}

impl Harness {
    fn new() -> Result<Self> {
        let rs = pollster::block_on(RenderState::headless())?;
        let pipeline = SpritePipeline::new(&rs.device, wgpu::TextureFormat::Rgba8UnormSrgb);
        let mut text = TextSystem::new(&rs, wgpu::TextureFormat::Rgba8UnormSrgb);
        match load_vcr_ttf_bytes() {
            Ok(bytes) => {
                text.add_font_bytes(bytes);
                text.set_default_family("VCR OSD Mono");
            }
            Err(e) => tracing::warn!(target: "rustic.asset", "VCR font unavailable: {e:#}"),
        }
        Ok(Self {
            rs,
            pipeline,
            batcher: SpriteBatcher::new(),
            text,
        })
    }

    fn render_scenario(&mut self, scenario: &RegressionScenario) -> Result<Vec<u8>> {
        let (sprite_cmds, text_cmds, atlases) = build_scenario(self, scenario)?;
        let bytes = capture_reference_rgba(
            &self.rs,
            &self.pipeline,
            &mut self.batcher,
            Some(&mut self.text),
            &build_cameras(),
            &atlases,
            sprite_cmds.as_slice(),
            text_cmds.as_slice(),
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        )?;
        Ok(bytes)
    }
}

fn build_cameras() -> CameraRegistry {
    let mut cameras = CameraRegistry::with_default_fnf();
    for id in [CameraId(0), CameraId(1), CameraId(2)] {
        if let Some(cam) = cameras.get_mut(id) {
            cam.zoom = 1.0;
            cam.position = vec2(REFERENCE_WIDTH as f32 * 0.5, REFERENCE_HEIGHT as f32 * 0.5);
        }
    }
    cameras
}

fn build_scenario(
    harness: &Harness,
    scenario: &RegressionScenario,
) -> Result<(
    RenderCommandList,
    TextCommandList,
    HashMap<rustic_core::ids::AssetId, rustic_render::Texture>,
)> {
    let scene = load_scenario_scene(&harness.rs.device, &harness.rs.queue, *scenario)
        .context("load regression scene")?;
    let play_state = load_scenario_play_state(*scenario).context("load regression PlayState")?;

    let mut sprite_cmds = scene.commands.clone();
    if let Some(characters) = &scene.characters {
        let anim = CharacterAnimState::default();
        for cmd in characters.commands(anim.poses(), Samples(0), REGRESSION_SAMPLE_RATE) {
            sprite_cmds.push(cmd);
        }
    }
    if let Some(hud_skin) = &scene.hud_skin {
        for cmd in hud_skin.commands_with_icon_scale(play_state.health, 1.0) {
            sprite_cmds.push(cmd);
        }
    }
    if let Some(text_skin) = &scene.bitmap_text_skin {
        for cmd in text_skin.score_text_commands(play_state.score) {
            sprite_cmds.push(cmd);
        }
    }

    let mut text_cmds = TextCommandList::new();
    let mut header = TextCommand::new(
        format!(
            "{} ({})",
            scenario.song.display_name(),
            scenario.difficulty.as_str()
        ),
        vec2(24.0, 24.0),
        32.0,
    );
    header.color = vec4(1.0, 1.0, 1.0, 0.85);
    text_cmds.push(header);

    Ok((sprite_cmds, text_cmds, scene.textures))
}

enum GoldenOutcome {
    Wrote,
    Match { diff_ratio: f64 },
    Stale { diff_ratio: f64 },
}

fn handle_golden(path: &PathBuf, rgba: &[u8], write: bool) -> Result<GoldenOutcome> {
    let img: RgbaImage = ImageBuffer::from_raw(REFERENCE_WIDTH, REFERENCE_HEIGHT, rgba.to_vec())
        .context("rgba buffer did not match reference resolution")?;

    if write || !path.exists() {
        img.save(path)
            .with_context(|| format!("write {}", path.display()))?;
        return Ok(GoldenOutcome::Wrote);
    }

    let existing = image::open(path)
        .with_context(|| format!("open {}", path.display()))?
        .to_rgba8();
    if existing.dimensions() != img.dimensions() {
        anyhow::bail!(
            "golden dimensions {}x{} differ from captured {}x{}",
            existing.width(),
            existing.height(),
            img.width(),
            img.height()
        );
    }
    let mut diff = 0u64;
    for (a, b) in existing.as_raw().iter().zip(img.as_raw().iter()) {
        diff += a.abs_diff(*b) as u64;
    }
    let total = u64::from(img.width()) * u64::from(img.height()) * 4 * 255;
    let ratio = diff as f64 / total as f64;
    if ratio > DEFAULT_DIFF_THRESHOLD {
        Ok(GoldenOutcome::Stale { diff_ratio: ratio })
    } else {
        Ok(GoldenOutcome::Match { diff_ratio: ratio })
    }
}
