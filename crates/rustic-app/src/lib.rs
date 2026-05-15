//! `rustic-app` — winit/platform glue. See `PLAN.md` Sections 4 and 11.
//!
//! `rustic-app` is the only release crate allowed to wire audio, render,
//! game, settings, and assets together. Other crates stay headless.

pub mod active_holds;
pub mod animate_character_assets;
mod animation_timing;
pub mod app;
pub mod app_runtime;
pub mod app_text;
pub mod app_types;
pub mod asset_roots;
pub mod audio_clock;
pub mod audio_fallback;
pub mod bitmap_text_assets;
pub mod boot;
pub mod camera_events;
pub mod camera_fx;
pub mod character_anim;
pub mod countdown_assets;
pub mod countdown_audio;
pub mod credits_assets;
pub(crate) mod dialogue_state;
pub mod freeplay_assets;
pub mod freeplay_dj;
pub mod freeplay_preview_audio;
pub mod game_over;
pub mod game_over_audio;
mod gameplay_note_sfx;
pub mod hold_cover_assets;
pub mod hud_assets;
pub mod hud_bop;
pub mod input_bridge;
pub mod lane_state;
pub mod main_menu_assets;
pub mod menu_audio;
pub mod menu_music;
pub mod miss_note_audio;
pub mod note_assets;
mod note_kind_anim;
pub(crate) mod note_layout_preferences;
pub mod note_splash_assets;
pub mod options_menu_assets;
pub(crate) mod options_preferences;
pub mod pause_audio;
pub mod pause_menu;
pub mod popup_assets;
pub mod preview_song;
pub mod regression;
pub mod scene_assets;
pub mod screen;
pub(crate) mod scripted_stage_objects;
pub mod settings;
pub mod song_audio;
pub(crate) mod sparrow_character_assets;
pub(crate) mod sserafim_stage;
pub(crate) mod stage_object_asset_helpers;
pub mod stage_object_assets;
pub(crate) mod stage_scripted_motion;
pub(crate) mod stage_sfx;
pub(crate) mod stage_static_prop;
pub mod story_menu_assets;
pub mod title_assets;

pub use app::run;
pub use app_types::AppOptions;
