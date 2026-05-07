//! `rustic-app` — winit/platform glue. See `PLAN.md` Sections 4 and 11.
//!
//! `rustic-app` is the only release crate allowed to wire audio, render,
//! game, settings, and assets together. Other crates stay headless.

pub mod active_holds;
pub mod animate_character_assets;
pub mod app;
pub mod audio_fallback;
pub mod bitmap_text_assets;
pub mod boot;
pub mod camera_events;
pub mod camera_fx;
pub mod character_anim;
pub mod countdown_assets;
pub mod hold_cover_assets;
pub mod hud_assets;
pub mod hud_bop;
pub mod input_bridge;
pub mod lane_state;
pub mod note_assets;
pub mod note_splash_assets;
pub mod popup_assets;
pub mod preview_song;
pub mod scene_assets;
pub mod screen;
pub mod settings;
pub mod song_audio;

pub use app::{run, AppOptions};
