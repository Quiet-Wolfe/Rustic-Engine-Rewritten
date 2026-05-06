//! `rustic-app` — winit/platform glue. See `PLAN.md` Sections 4 and 11.
//!
//! `rustic-app` is the only release crate allowed to wire audio, render,
//! game, settings, and assets together. Other crates stay headless.

pub mod app;
pub mod boot;
pub mod hud_assets;
pub mod input_bridge;
pub mod lane_state;
pub mod scene_assets;
pub mod screen;
pub mod settings;

pub use app::{run, AppOptions};
