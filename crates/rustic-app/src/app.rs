//! Top-level winit application. See `PLAN.md` Sections 7 and 11.
//!
//! Uses winit 0.30's `ApplicationHandler`: window + surface are created
//! lazily on `resumed` so the same code path works on Android (which
//! also pauses/resumes the surface). Surface is `'static` because the
//! window is held in an `Arc`.
// LINT-ALLOW: long-file app event loop plus temporary gameplay preview wiring

use crate::audio_fallback::open_audio_output_or_fallback;
use crate::boot::{init_logging, install_panic_hook};
use crate::camera_fx::CameraFx;
use crate::character_anim::CharacterAnimState;
use crate::countdown_assets::{countdown_start_cursor, CountdownSkin};
use crate::hold_cover_assets::{HoldCoverSkin, HoldCovers};
use crate::hud_assets::HudSkin;
use crate::input_bridge::{build_event, map_key};
use crate::lane_state::{lane_for_action, HeldLanes};
use crate::note_assets::{confirm_duration_or_default, NoteSkin};
use crate::note_splash_assets::{NoteSplashSkin, NoteSplashes};
use crate::popup_assets::{PopupSkin, ScorePopups};
use crate::scene_assets::{load_default_scene, load_preview_play_state, CharacterSet};
use crate::screen::ScreenStack;
use crate::song_audio::load_bopeebo_stems;
use anyhow::Result;
use rustic_audio::{AudioOutput, SharedMixer, Stem};
use rustic_core::ids::AssetId;
use rustic_core::input::{InputAction, InputState, NormalizedInputEvent};
use rustic_core::time::Samples;
use rustic_game::{Judgment, PlayState};
use rustic_render::{
    CameraRegistry, Composite, RenderCommandList, RenderState, SpriteBatcher, SpritePipeline,
    SurfaceConfig, Texture,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AppOptions {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            title: "RusticV3",
            width: 1280,
            height: 720,
        }
    }
}

/// Runtime state held inside the event loop.
struct App {
    options: AppOptions,
    boot_instant: Instant,
    mixer: SharedMixer,
    audio_output: Option<AudioOutput>,
    cameras: CameraRegistry,
    camera_fx: CameraFx,
    static_cmds: RenderCommandList,
    cmds: RenderCommandList,
    atlases: HashMap<AssetId, Texture>,
    characters: Option<CharacterSet>,
    character_anim: CharacterAnimState,
    note_skin: Option<NoteSkin>,
    note_splash_skin: Option<NoteSplashSkin>,
    hold_cover_skin: Option<HoldCoverSkin>,
    hud_skin: Option<HudSkin>,
    popup_skin: Option<PopupSkin>,
    countdown_skin: Option<CountdownSkin>,
    score_popups: ScorePopups,
    note_splashes: NoteSplashes,
    hold_covers: HoldCovers,
    held_lanes: HeldLanes,
    play_state: Option<PlayState>,
    song_start: Instant,
    song_start_cursor: Samples,
    song_started: bool,
    game_over: Option<GameOverState>,
    batcher: SpriteBatcher,
    screens: ScreenStack,
    runtime: Option<Runtime>,
}

#[derive(Debug, Clone, Copy)]
struct GameOverState {
    song_cursor: Samples,
    animation_started: Instant,
    loop_at: Samples,
    loop_started: bool,
}

struct Runtime {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_cfg: SurfaceConfig,
    rs: RenderState,
    pipeline: SpritePipeline,
    composite: Composite,
}

impl App {
    fn new(options: AppOptions) -> Self {
        let now = Instant::now();
        let (audio_output, mixer) = open_audio_output_or_fallback();
        Self {
            options,
            boot_instant: now,
            mixer,
            audio_output,
            cameras: CameraRegistry::with_default_fnf(),
            camera_fx: CameraFx::default(),
            static_cmds: RenderCommandList::new(),
            cmds: RenderCommandList::new(),
            atlases: HashMap::new(),
            characters: None,
            character_anim: CharacterAnimState::default(),
            note_skin: None,
            note_splash_skin: None,
            hold_cover_skin: None,
            hud_skin: None,
            popup_skin: None,
            countdown_skin: None,
            score_popups: ScorePopups::default(),
            note_splashes: NoteSplashes::default(),
            hold_covers: HoldCovers::default(),
            held_lanes: HeldLanes::default(),
            play_state: None,
            song_start: now,
            song_start_cursor: Samples(0),
            song_started: false,
            game_over: None,
            batcher: SpriteBatcher::new(),
            screens: ScreenStack::new(),
            runtime: None,
        }
    }

    fn create_runtime(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let attrs = WindowAttributes::default()
            .with_title(self.options.title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.options.width,
                self.options.height,
            ));
        let window = Arc::new(event_loop.create_window(attrs)?);

        // wgpu Instance must be built before we make the surface so we
        // can pass `compatible_surface` to the adapter request.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| anyhow::anyhow!("create_surface: {e}"))?;

        let rs = pollster::block_on(RenderState::new_async(instance, Some(&surface)))?;
        let inner = window.inner_size();
        let surface_cfg = rs.configure_surface(
            &surface,
            inner.width,
            inner.height,
            wgpu::PresentMode::AutoVsync,
        )?;
        let pipeline = SpritePipeline::new(&rs.device, wgpu::TextureFormat::Rgba8UnormSrgb);
        let composite = Composite::new(&rs, surface_cfg.format);

        let runtime = Runtime {
            window,
            surface,
            surface_cfg,
            rs,
            pipeline,
            composite,
        };

        match load_default_scene(&runtime.rs.device, &runtime.rs.queue) {
            Ok(scene) => {
                self.cmds = scene.commands;
                self.static_cmds = self.cmds.clone();
                self.atlases = scene.textures;
                self.characters = scene.characters;
                self.note_skin = scene.note_skin;
                self.note_splash_skin = scene.note_splash_skin;
                self.hold_cover_skin = scene.hold_cover_skin;
                self.hud_skin = scene.hud_skin;
                self.popup_skin = scene.popup_skin;
                self.countdown_skin = scene.countdown_skin;
                self.camera_fx.reset(&mut self.cameras, scene.camera_zoom);
                let sample_rate = self.play_sample_rate();
                match load_preview_play_state(sample_rate) {
                    Ok(play_state) => {
                        self.song_start_cursor =
                            countdown_start_cursor(sample_rate, play_state.bpm);
                        self.song_start = Instant::now();
                        self.song_started = false;
                        self.game_over = None;
                        if self.audio_output.is_some() {
                            if let Err(e) = load_bopeebo_stems(&self.mixer, Samples(0)) {
                                tracing::warn!(target: "rustic.audio", "preview stems unavailable: {e:#}");
                            } else if let Err(e) = self.mixer.edit(|mixer| {
                                mixer.pause();
                                Ok(())
                            }) {
                                tracing::warn!(target: "rustic.audio", "pause countdown audio: {e:#}");
                            }
                        } else {
                            tracing::warn!(
                                target: "rustic.audio",
                                "preview stems skipped because audio output is unavailable"
                            );
                        }
                        self.play_state = Some(play_state);
                        self.rebuild_frame_commands();
                    }
                    Err(e) => {
                        tracing::warn!(target: "rustic.asset", "preview chart unavailable: {e:#}");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(target: "rustic.asset", "default scene assets unavailable: {e:#}");
            }
        }

        self.runtime = Some(runtime);
        Ok(())
    }

    fn redraw(&mut self) {
        self.rebuild_frame_commands();
        let Some(rt) = self.runtime.as_mut() else {
            return;
        };
        let frame = match rt.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                rt.surface.configure(
                    &rt.rs.device,
                    &wgpu::SurfaceConfiguration {
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        format: rt.surface_cfg.format,
                        width: rt.surface_cfg.width.max(1),
                        height: rt.surface_cfg.height.max(1),
                        present_mode: rt.surface_cfg.present_mode,
                        alpha_mode: wgpu::CompositeAlphaMode::Auto,
                        view_formats: vec![],
                        desired_maximum_frame_latency: 2,
                    },
                );
                return;
            }
            Err(e) => {
                tracing::warn!(target: "rustic.render", "surface error: {e:?}");
                return;
            }
        };

        let target = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = rt
            .rs
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rustic.frame.encoder"),
            });
        let bg = wgpu::Color {
            r: 0.07,
            g: 0.07,
            b: 0.10,
            a: 1.0,
        };

        // 1. Sprite pass into the 1280x720 reference target.
        self.batcher.draw_to_reference(
            &rt.rs,
            &mut encoder,
            &rt.pipeline,
            &self.cameras,
            &self.atlases,
            self.cmds.as_slice(),
            bg,
        );
        // 2. Composite reference -> swapchain with letterbox.
        rt.composite.encode(
            &mut encoder,
            &target,
            rt.surface_cfg.width,
            rt.surface_cfg.height,
            wgpu::Color::BLACK,
        );
        rt.rs.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    fn handle_resize(&mut self, w: u32, h: u32) {
        let Some(rt) = self.runtime.as_mut() else {
            return;
        };
        rt.surface_cfg.width = w;
        rt.surface_cfg.height = h;
        rt.surface.configure(
            &rt.rs.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: rt.surface_cfg.format,
                width: w.max(1),
                height: h.max(1),
                present_mode: rt.surface_cfg.present_mode,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
    }

    fn rebuild_frame_commands(&mut self) {
        let sample_rate = self.play_sample_rate();
        let cursor = if let Some(game_over) = self.game_over {
            game_over_cursor(game_over, sample_rate)
        } else {
            self.advance_song_clock()
        };
        if self.game_over.is_some() {
            self.rebuild_game_over_commands(cursor, sample_rate);
            return;
        }
        let mut opponent_hits = Vec::new();
        let mut bpm = None;
        let mut late_misses = 0;
        let mut dead = false;
        let confirm_duration = confirm_duration_or_default(self.note_skin.as_ref(), sample_rate);
        if let Some(play_state) = self.play_state.as_mut() {
            opponent_hits = play_state.resolve_opponent_notes(cursor);
            let held_lanes: Vec<_> = self.held_lanes.active_lanes().collect();
            for lane in held_lanes {
                if play_state.resolve_held_sustains_in_lane(cursor, lane, sample_rate) > 0 {
                    self.held_lanes.hold_confirm(lane, cursor, confirm_duration);
                }
            }
            late_misses = play_state.expire_late_notes(cursor, sample_rate);
            dead = play_state.is_dead();
            bpm = Some(play_state.bpm);
        }
        if dead {
            self.enter_game_over(cursor);
            self.rebuild_game_over_commands(cursor, sample_rate);
            return;
        }
        if late_misses > 0 {
            self.set_vocals_gain(0.0);
        }
        if let Some(bpm) = bpm {
            let had_opponent_hits = !opponent_hits.is_empty();
            for hit in opponent_hits {
                self.character_anim
                    .opponent_note_hit(hit.lane, cursor, sample_rate, bpm);
            }
            if had_opponent_hits {
                self.camera_fx.enable_zooming();
                self.set_vocals_gain(1.0);
            }
            self.character_anim.update(
                cursor,
                sample_rate,
                bpm,
                self.held_lanes.active_lanes().next().is_some(),
                true,
            );
            self.camera_fx
                .update(&mut self.cameras, cursor, sample_rate, bpm);
        }

        self.cmds = self.static_cmds.clone();
        if let Some(characters) = &self.characters {
            for cmd in characters.commands(self.character_anim.poses(), cursor, sample_rate) {
                self.cmds.push(cmd);
            }
        }
        let (Some(play_state), Some(note_skin)) = (&self.play_state, &self.note_skin) else {
            return;
        };

        for view in play_state.hold_trail_views(cursor, sample_rate) {
            if view.head_resolved && !self.held_lanes.is_held(view.lane) {
                continue;
            }
            for cmd in note_skin.hold_trail_commands(&view) {
                if cmd.world_pos.y + cmd.size.y >= -200.0 {
                    self.cmds.push(cmd);
                }
            }
        }
        for cmd in note_skin.receptor_commands(cursor, sample_rate, |lane| {
            self.held_lanes.receptor_state(lane, cursor)
        }) {
            self.cmds.push(cmd);
        }

        for view in play_state.note_views(cursor, sample_rate) {
            if view.is_sustain {
                continue;
            }
            let cmd = note_skin.command_for_view(&view);
            if cmd.world_pos.y + cmd.size.y >= -200.0 {
                self.cmds.push(cmd);
            }
        }
        if let Some(note_splash_skin) = &self.note_splash_skin {
            for cmd in self
                .note_splashes
                .commands(note_splash_skin, cursor, sample_rate)
            {
                self.cmds.push(cmd);
            }
        }
        if let Some(hold_cover_skin) = &self.hold_cover_skin {
            for cmd in self
                .hold_covers
                .commands(hold_cover_skin, cursor, sample_rate)
            {
                self.cmds.push(cmd);
            }
        }
        if let Some(hud_skin) = &self.hud_skin {
            for cmd in hud_skin.commands_with_icon_scale(
                play_state.health,
                health_icon_scale(cursor, sample_rate, play_state.bpm),
            ) {
                self.cmds.push(cmd);
            }
        }
        if let Some(popup_skin) = &self.popup_skin {
            for cmd in self.score_popups.commands(popup_skin, cursor, sample_rate) {
                self.cmds.push(cmd);
            }
        }
        if let Some(countdown_skin) = &self.countdown_skin {
            for cmd in countdown_skin.commands(cursor, sample_rate, play_state.bpm) {
                self.cmds.push(cmd);
            }
        }
    }

    fn handle_gameplay_input(&mut self, event: &NormalizedInputEvent, already_held: bool) {
        if event.state != InputState::Pressed {
            return;
        }
        let cursor = event.audio_sample_cursor_at_receive;
        if self.game_over.is_some() {
            return;
        }
        let sample_rate = self.play_sample_rate();
        let confirm_duration = confirm_duration_or_default(self.note_skin.as_ref(), sample_rate);
        let gameplay_event =
            NormalizedInputEvent::new(event.action, event.state, event.wall_clock_ns, cursor);
        let mut restore_vocals = false;
        let should_enter_game_over;
        {
            let Some(play_state) = self.play_state.as_mut() else {
                return;
            };
            if event.action == InputAction::Reset {
                // ref: bdedc0aa:source/funkin/play/PlayState.hx:1243-1258
                play_state.health = 0.0;
                should_enter_game_over = true;
            } else {
                let Some(lane) = lane_for_action(event.action) else {
                    return;
                };
                if already_held {
                    return;
                }
                if let Some(outcome) =
                    play_state.try_hit_in_lane(&gameplay_event, lane, sample_rate)
                {
                    self.held_lanes.confirm(lane, cursor, confirm_duration);
                    self.character_anim
                        .player_note_hit(lane, cursor, sample_rate, play_state.bpm);
                    restore_vocals = true;
                    if !outcome.is_sustain {
                        self.score_popups
                            .push(outcome.judgment, outcome.combo_popup, cursor);
                        if outcome.judgment == Judgment::Sick {
                            self.note_splashes.push(lane, cursor);
                        }
                        if let Some(hold_end_at) = outcome.hold_end_at {
                            self.hold_covers.start(lane, cursor, hold_end_at);
                        }
                    }
                } else {
                    play_state.register_ghost_miss();
                    self.character_anim
                        .player_note_miss(lane, cursor, sample_rate, play_state.bpm);
                }
                should_enter_game_over = play_state.is_dead();
            }
        }
        if restore_vocals {
            self.set_vocals_gain(1.0);
        }
        if should_enter_game_over {
            self.enter_game_over(cursor);
        }
    }

    fn register_hold_drop(&mut self, cursor: Samples, hold_end_at: Samples) {
        let sample_rate = self.play_sample_rate();
        let remaining_samples = hold_end_at.0.saturating_sub(cursor.0);
        let dropped = self
            .play_state
            .as_mut()
            .and_then(|play_state| play_state.register_hold_drop(remaining_samples, sample_rate))
            .is_some();
        if dropped {
            self.set_vocals_gain(0.0);
        }
    }

    fn advance_song_clock(&mut self) -> Samples {
        if !self.song_started {
            let elapsed = self.song_start.elapsed().as_secs_f64();
            let elapsed_samples = (elapsed * f64::from(self.play_sample_rate())).round() as i64;
            let cursor = Samples(self.song_start_cursor.0 + elapsed_samples);
            if cursor.0 < 0 {
                return cursor;
            }
            self.song_started = true;
            self.song_start = Instant::now();
            if let Err(e) = self.mixer.edit(|mixer| {
                mixer.seek(Samples(0))?;
                mixer.resume();
                Ok(())
            }) {
                tracing::warn!(target: "rustic.audio", "start countdown audio: {e:#}");
            }
            return Samples(0);
        }

        if self.audio_output.is_some() {
            return self.mixer.sample_cursor();
        }
        let elapsed = self.song_start.elapsed().as_secs_f64();
        let elapsed_samples = (elapsed * f64::from(self.play_sample_rate())).round() as i64;
        Samples(elapsed_samples)
    }

    fn play_sample_rate(&self) -> u32 {
        self.mixer.sample_rate().max(1)
    }

    fn set_vocals_gain(&self, gain: f32) {
        if let Err(e) = self.mixer.edit(|mixer| {
            mixer.set_stem_gain(Stem::Vocals, gain);
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "set vocals gain: {e:#}");
        }
    }

    fn enter_game_over(&mut self, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/PlayState.hx:1441-1472
        if self.game_over.is_some() {
            return;
        }
        let sample_rate = self.play_sample_rate();
        let loop_after = self
            .characters
            .as_ref()
            .and_then(|characters| characters.player_animation_duration("firstDeath", sample_rate))
            .unwrap_or(Samples(i64::from(sample_rate)));
        self.character_anim.player_first_death(cursor);
        self.held_lanes = HeldLanes::default();
        self.hold_covers = HoldCovers::default();
        self.set_vocals_gain(0.0);
        if let Err(e) = self.mixer.edit(|mixer| {
            mixer.pause();
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "pause game over audio: {e:#}");
        }
        self.game_over = Some(GameOverState {
            song_cursor: cursor,
            animation_started: Instant::now(),
            loop_at: Samples(cursor.0 + loop_after.0),
            loop_started: false,
        });
    }

    fn rebuild_game_over_commands(&mut self, cursor: Samples, sample_rate: u32) {
        let Some(game_over) = self.game_over.as_mut() else {
            return;
        };
        if !game_over.loop_started && cursor >= game_over.loop_at {
            game_over.loop_started = true;
            self.character_anim.player_death_loop(game_over.loop_at);
        }

        self.cmds.clear();
        if let Some(characters) = &self.characters {
            self.cmds.push(characters.player_command(
                self.character_anim.poses().player,
                cursor,
                sample_rate,
            ));
        }
    }
}

fn game_over_cursor(game_over: GameOverState, sample_rate: u32) -> Samples {
    let elapsed = game_over.animation_started.elapsed().as_secs_f64();
    let elapsed_samples = (elapsed * f64::from(sample_rate.max(1))).round() as i64;
    Samples(game_over.song_cursor.0 + elapsed_samples)
}

fn health_icon_scale(cursor: Samples, sample_rate: u32, bpm: f64) -> f32 {
    // Legacy prototype pulse. v0.8.5 moves this into HealthIcon's own bop tween.
    // ref: bdedc0aa:source/funkin/play/components/HealthIcon.hx:227-242,296-315
    if cursor.0 < 0 {
        return 1.0;
    }
    let beat_samples = (f64::from(sample_rate) * 60.0 / bpm.max(1.0)).round() as i64;
    let phase = cursor.0.rem_euclid(beat_samples.max(1)) as f32 / beat_samples.max(1) as f32;
    if phase >= 0.5 {
        1.0
    } else {
        1.0 + 0.2 * (1.0 - phase / 0.5)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.runtime.is_some() {
            return;
        }
        if let Err(e) = self.create_runtime(event_loop) {
            tracing::error!(target: "rustic", "failed to bring up renderer: {e:#}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.handle_resize(size.width, size.height),
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(action) = map_key(event.physical_key) {
                    let song_cursor = self.advance_song_clock();
                    let evt = build_event(action, event.state, self.boot_instant, song_cursor);
                    let already_held = event.repeat
                        || lane_for_action(action)
                            .map(|lane| self.held_lanes.is_held(lane))
                            .unwrap_or(false);
                    self.held_lanes.apply(&evt);
                    if event.state == ElementState::Released {
                        if let Some(lane) = lane_for_action(action) {
                            if let Some(hold_end_at) = self.hold_covers.end(lane, song_cursor) {
                                self.register_hold_drop(song_cursor, hold_end_at);
                            }
                        }
                    }
                    self.screens.input(&evt);
                    self.handle_gameplay_input(&evt, already_held);
                    if event.state == ElementState::Pressed
                        && action == rustic_core::InputAction::Back
                    {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                if let Some(rt) = self.runtime.as_ref() {
                    rt.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

/// Public entry point. Initializes logging + panic hook, builds the app
/// state, runs the winit event loop. Returns when the user closes the
/// window or the loop exits.
pub fn run(options: AppOptions) -> Result<()> {
    init_logging();
    install_panic_hook();
    tracing::info!(target: "rustic", "starting RusticV3 ({}x{})", options.width, options.height);

    let event_loop = EventLoop::new()?;
    let mut app = App::new(options);
    event_loop.run_app(&mut app)?;
    Ok(())
}
