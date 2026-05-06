//! `rustic-app` — winit/platform glue. See `PLAN.md` Sections 4 and 11.
//!
//! `rustic-app` is the only release crate allowed to wire audio, render,
//! game, settings, and assets together. Other crates stay headless.

pub mod app;
pub mod boot;
pub mod camera_fx;
pub mod character_anim;
pub mod countdown_assets;
pub mod hud_assets;
pub mod input_bridge;
pub mod lane_state;
pub mod popup_assets;
pub mod scene_assets;
pub mod screen;
pub mod settings;
pub mod song_audio;

pub use app::{run, AppOptions};
