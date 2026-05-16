use crate::active_holds::ActiveHolds;
// LINT-ALLOW: long-file app owns prototype runtime wiring until screen modules split.
use crate::app_runtime::create_runtime as create_app_runtime;
use crate::app_text::preview_text_commands;
use crate::app_types::{AppOptions, Runtime};
use crate::audio_clock::AudioClockFallback;
use crate::audio_fallback::open_audio_output_or_fallback;
use crate::bitmap_text_assets::BitmapTextSkin;
use crate::boot::{init_logging, install_panic_hook};
use crate::camera_events::focus_initial_camera as focus_initial;
use crate::camera_fx::CameraFx;
use crate::character_anim::CharacterAnimState;
use crate::countdown_assets::{countdown_start_cursor, CountdownSkin};
use crate::countdown_audio::CountdownAudio;
use crate::credits_assets::{CreditsAssets, CreditsScrollState};
use crate::dialogue_state::DialogueState;
use crate::freeplay_assets::FreeplayAssets;
use crate::freeplay_preview_audio::FreeplayPreviewMusic;
use crate::game_over::{GameOverRestart, GameOverState};
use crate::game_over_audio::GameOverAudio;
use crate::gameplay_note_sfx::play_note_kind_miss_sfx_or_warn;
use crate::hold_cover_assets::{HoldCoverSkin, HoldCovers};
use crate::hud_assets::HudSkin;
use crate::hud_bop::health_icon_scale;
use crate::input_bridge::{build_event, map_key};
use crate::lane_state::{lane_for_action, AutoReceptors, HeldLanes};
use crate::main_menu_assets::MainMenuAssets;
use crate::menu_audio::{play_menu_sound_or_warn, MenuSound};
use crate::menu_music::MenuMusic;
use crate::miss_note_audio::{play_miss_note_or_warn as play_miss_sfx, MissNoteKind};
use crate::note_assets::{confirm_duration_or_default, NoteSkin};
use crate::note_layout_preferences::{apply_downscroll, strumline_background_commands};
use crate::note_splash_assets::{NoteSplashSkin, NoteSplashes};
use crate::options_menu_assets::{OptionsMenuAssets, OptionsMenuPage};
use crate::options_preferences::OptionsPreferences;
use crate::pause_audio::PauseMusic;
use crate::pause_menu::{ensure_pause_overlay_texture, PauseMenuState};
use crate::philly_train_stage::philly_train_pose_overrides;
use crate::popup_assets::{PopupSkin, ScorePopups};
use crate::preview_song::{PreviewDifficulty, PreviewSelection, PreviewSong};
use crate::scene_assets::{
    load_preview_play_state_for, load_preview_scene_for, CameraFocusPoints, CharacterSet,
    LoadedScene,
};
use crate::screen::ScreenStack;
use crate::settings::{load_settings_or_default, Settings};
use crate::song_audio::{load_preview_stems_for, play_sample_rate, set_vocals_gain};
use crate::spooky_mansion_stage::spooky_lightning_pose_overrides;
use crate::sserafim_stage::sserafim_intro_start_cursor;
use crate::stage_object_assets::StagePropSet;
use crate::stage_sfx::StageSfx;
use crate::story_menu_assets::StoryMenuAssets;
use crate::stress_pico_cutscene::StressPicoEndCutsceneState;
use crate::subtitle_track::SubtitleTrack;
use crate::title_assets::TitleScreenAssets;
use crate::winter_horrorland_cutscene::{
    should_play_winter_horrorland_cutscene, WinterHorrorlandCutsceneState,
};
use anyhow::Result;
use rustic_audio::{AudioOutput, SharedMixer};
use rustic_core::ids::AssetId;
use rustic_core::input::InputAction;
use rustic_core::time::Samples;
use rustic_game::PlayState;
use rustic_render::{CameraRegistry, RenderCommandList, SpriteBatcher, TextCommandList, Texture};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};

const BLAZIN_MIDDLESCROLL_PLAYER_X_OFFSET: f32 = -272.0;

mod credits_flow;
mod debug_overlay;
mod dialogue_flow;
mod game_over_flow;
mod gameplay_flow;
mod options_flow;
mod pause_flow;
mod redraw;
mod song_flow;
mod title_flow;

use title_flow::AppMode;

struct App {
    options: AppOptions,
    settings: Settings,
    settings_path: Option<PathBuf>,
    boot_instant: Instant,
    mixer: SharedMixer,
    audio_output: Option<AudioOutput>,
    cameras: CameraRegistry,
    camera_fx: CameraFx,
    camera_focus: CameraFocusPoints,
    base_camera_zoom: f32,
    static_cmds: RenderCommandList,
    stage_props: StagePropSet,
    stage_sfx: StageSfx,
    sserafim_stage: crate::sserafim_stage::SserafimStageState,
    stress_pico_end_cutscene: Option<StressPicoEndCutsceneState>,
    winter_horrorland_cutscene: Option<WinterHorrorlandCutsceneState>,
    cmds: RenderCommandList,
    text_cmds: TextCommandList,
    atlases: HashMap<AssetId, Texture>,
    characters: Option<CharacterSet>,
    bitmap_text_skin: Option<BitmapTextSkin>,
    character_anim: CharacterAnimState,
    note_skin: Option<NoteSkin>,
    note_splash_skin: Option<NoteSplashSkin>,
    hold_cover_skin: Option<HoldCoverSkin>,
    hud_skin: Option<HudSkin>,
    popup_skin: Option<PopupSkin>,
    countdown_skin: Option<CountdownSkin>,
    countdown_audio: CountdownAudio,
    dialogue: Option<DialogueState>,
    subtitle_track: Option<SubtitleTrack>,
    score_popups: ScorePopups,
    note_splashes: NoteSplashes,
    hold_covers: HoldCovers,
    active_holds: ActiveHolds,
    held_lanes: HeldLanes,
    opponent_receptors: AutoReceptors,
    preview_selection: PreviewSelection,
    mode: AppMode,
    title_assets: Option<TitleScreenAssets>,
    main_menu_assets: Option<MainMenuAssets>,
    main_menu_index: usize,
    main_menu_confirm_started_at: Option<Samples>,
    credits_assets: Option<CreditsAssets>,
    credits_scroll: CreditsScrollState,
    options_menu_assets: Option<OptionsMenuAssets>,
    options_menu_page: OptionsMenuPage,
    options_menu_index: usize,
    options_preferences: OptionsPreferences,
    freeplay_assets: Option<FreeplayAssets>,
    freeplay_selected_index: usize,
    freeplay_variation: &'static str,
    /// When Some, the freeplay Confirm animation is playing and we'll enter
    /// gameplay once the title cursor passes this sample anchor.
    /// ref: bdedc0aa:source/funkin/ui/freeplay/FreeplayState.hx:2744-2846 (start delay)
    freeplay_confirm_at: Option<Samples>,
    story_menu_assets: Option<StoryMenuAssets>,
    story_menu_index: usize,
    story_menu_difficulty: PreviewDifficulty,
    story_menu_confirm_started_at: Option<Samples>,
    story_playlist: Vec<PreviewSong>,
    story_playlist_index: usize,
    story_playlist_difficulty: PreviewDifficulty,
    menu_music: MenuMusic,
    freeplay_preview: FreeplayPreviewMusic,
    pause_menu: Option<PauseMenuState>,
    pause_music: PauseMusic,
    game_over_audio: GameOverAudio,
    game_over_restart: Option<GameOverRestart>,
    death_counter: u32,
    practice_mode: bool,
    title_start: Instant,
    play_state: Option<PlayState>,
    song_start: Instant,
    song_start_cursor: Samples,
    song_started: bool,
    audio_clock: AudioClockFallback,
    game_over: Option<GameOverState>,
    debug_overlay: bool,
    last_frame_at: Instant,
    debug_fps: f32,
    batcher: SpriteBatcher,
    screens: ScreenStack,
    runtime: Option<Runtime>,
}

impl App {
    fn new(options: AppOptions) -> Self {
        let now = Instant::now();
        let (audio_output, mixer) = open_audio_output_or_fallback();
        let (settings, settings_path) = load_settings_or_default();
        let options_preferences = OptionsPreferences::from_settings(&settings.preferences);
        Self {
            options,
            settings,
            settings_path,
            boot_instant: now,
            mixer,
            audio_output,
            cameras: CameraRegistry::with_default_fnf(),
            camera_fx: CameraFx::default(),
            camera_focus: CameraFocusPoints::default(),
            base_camera_zoom: 1.0,
            static_cmds: RenderCommandList::new(),
            stage_props: StagePropSet::default(),
            stage_sfx: StageSfx::default(),
            sserafim_stage: crate::sserafim_stage::SserafimStageState::default(),
            stress_pico_end_cutscene: None,
            winter_horrorland_cutscene: None,
            cmds: RenderCommandList::new(),
            text_cmds: TextCommandList::new(),
            atlases: HashMap::new(),
            characters: None,
            bitmap_text_skin: None,
            character_anim: CharacterAnimState::default(),
            note_skin: None,
            note_splash_skin: None,
            hold_cover_skin: None,
            hud_skin: None,
            popup_skin: None,
            countdown_skin: None,
            countdown_audio: CountdownAudio::default(),
            dialogue: None,
            subtitle_track: None,
            score_popups: ScorePopups::default(),
            note_splashes: NoteSplashes::default(),
            hold_covers: HoldCovers::default(),
            active_holds: ActiveHolds::default(),
            held_lanes: HeldLanes::default(),
            opponent_receptors: AutoReceptors::default(),
            preview_selection: PreviewSelection::from_env(),
            mode: AppMode::Title,
            title_assets: None,
            main_menu_assets: None,
            main_menu_index: 0,
            main_menu_confirm_started_at: None,
            credits_assets: None,
            credits_scroll: CreditsScrollState::default(),
            options_menu_assets: None,
            options_menu_page: OptionsMenuPage::Root,
            options_menu_index: 0,
            options_preferences,
            freeplay_assets: None,
            freeplay_selected_index: 0,
            freeplay_variation: crate::preview_song::VARIATION_BF,
            freeplay_confirm_at: None,
            story_menu_assets: None,
            story_menu_index: 1,
            story_menu_difficulty: PreviewDifficulty::Normal,
            story_menu_confirm_started_at: None,
            story_playlist: Vec::new(),
            story_playlist_index: 0,
            story_playlist_difficulty: PreviewDifficulty::Normal,
            menu_music: MenuMusic::default(),
            freeplay_preview: FreeplayPreviewMusic::default(),
            pause_menu: None,
            pause_music: PauseMusic::default(),
            game_over_audio: GameOverAudio::default(),
            game_over_restart: None,
            death_counter: 0,
            practice_mode: false,
            title_start: now,
            play_state: None,
            song_start: now,
            song_start_cursor: Samples(0),
            song_started: false,
            audio_clock: AudioClockFallback::new(now),
            game_over: None,
            debug_overlay: false,
            last_frame_at: now,
            debug_fps: 0.0,
            batcher: SpriteBatcher::new(),
            screens: ScreenStack::new(),
            runtime: None,
        }
    }
    fn create_runtime(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        self.runtime = Some(create_app_runtime(
            &self.options,
            event_loop,
            self.options_preferences,
        )?);
        self.load_title_screen();
        Ok(())
    }
    fn apply_scene(&mut self, scene: LoadedScene) {
        let anim_timings = scene
            .characters
            .as_ref()
            .map(CharacterSet::anim_timings)
            .unwrap_or_default();
        self.cmds = scene.commands;
        self.static_cmds = self.cmds.clone();
        self.stage_props = scene.stage_props;
        self.stage_sfx.reset();
        self.sserafim_stage
            .reset_for_song(self.preview_selection.song);
        self.atlases = scene.textures;
        if let Some(runtime) = self.runtime.as_ref() {
            ensure_pause_overlay_texture(&runtime.rs.device, &runtime.rs.queue, &mut self.atlases);
        }
        self.characters = scene.characters;
        self.character_anim.set_timings(anim_timings);
        self.bitmap_text_skin = scene.bitmap_text_skin;
        self.note_skin = scene.note_skin;
        self.note_splash_skin = scene.note_splash_skin;
        self.hold_cover_skin = scene.hold_cover_skin;
        self.hud_skin = scene.hud_skin;
        self.popup_skin = scene.popup_skin;
        self.countdown_skin = scene.countdown_skin;
        self.countdown_audio =
            CountdownAudio::load_for_style_or_warn(&scene.note_style, self.audio_output.is_some());
        self.camera_focus = scene.camera_focus;
        self.base_camera_zoom = scene.camera_zoom;
        self.camera_fx.reset(&mut self.cameras, scene.camera_zoom);
        self.camera_fx
            .set_zooming_enabled(&mut self.cameras, self.options_preferences.camera_zooms);
        focus_initial(&mut self.cameras, &mut self.camera_fx, self.camera_focus);
    }
    fn load_selected_scene(&mut self) {
        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };
        let scene = load_preview_scene_for(
            &runtime.rs.device,
            &runtime.rs.queue,
            self.preview_selection,
        );
        match scene {
            Ok(scene) => self.apply_scene(scene),
            Err(e) => tracing::warn!(target: "rustic.asset", "preview scene unavailable: {e:#}"),
        }
    }
    fn load_selected_song(&mut self) {
        self.update_window_title();
        self.load_selected_scene();
        let sample_rate = play_sample_rate(&self.mixer);
        match load_preview_play_state_for(self.preview_selection, sample_rate) {
            Ok(play_state) => {
                tracing::info!(
                    target: "rustic",
                    "loading {} ({})",
                    self.preview_selection.song.display_name(),
                    self.preview_selection.difficulty.as_str()
                );
                let bpm = play_state.bpm;
                self.play_state = Some(play_state);
                self.reset_song_runtime(bpm);
                self.dialogue = match DialogueState::load_for_selection(self.preview_selection) {
                    Ok(dialogue) => dialogue,
                    Err(e) => {
                        tracing::warn!(target: "rustic.asset", "dialogue unavailable: {e:#}");
                        None
                    }
                };
                self.subtitle_track =
                    match SubtitleTrack::load_for_selection(self.preview_selection) {
                        Ok(track) => track,
                        Err(e) => {
                            tracing::warn!(target: "rustic.asset", "subtitles unavailable: {e:#}");
                            None
                        }
                    };
                self.load_selected_stems();
                self.rebuild_frame_commands();
            }
            Err(e) => tracing::warn!(target: "rustic.asset", "preview chart unavailable: {e:#}"),
        }
    }
    fn update_window_title(&self) {
        if let Some(runtime) = self.runtime.as_ref() {
            runtime.window.set_title(&format!(
                "{} - {} [{}]",
                self.options.title,
                self.preview_selection.song.display_name(),
                self.preview_selection.difficulty.as_str()
            ));
        }
    }
    fn play_menu_sound(&self, sound: MenuSound) {
        if self.audio_output.is_some() {
            play_menu_sound_or_warn(&self.mixer, sound);
        }
    }
    fn load_selected_stems(&mut self) {
        if self.audio_output.is_none() {
            tracing::warn!(target: "rustic.audio", "preview stems skipped: no audio output");
            return;
        }
        if let Err(e) = load_preview_stems_for(&self.mixer, self.preview_selection, Samples(0)) {
            tracing::warn!(target: "rustic.audio", "preview stems unavailable: {e:#}");
        } else if let Err(e) = self.mixer.edit(|mixer| {
            mixer.pause();
            Ok(())
        }) {
            tracing::warn!(target: "rustic.audio", "pause countdown audio: {e:#}");
        }
    }
    fn reset_song_runtime(&mut self, bpm: f64) {
        let sample_rate = play_sample_rate(&self.mixer);
        let countdown_cursor = countdown_start_cursor(sample_rate, bpm);
        self.winter_horrorland_cutscene = None;
        self.song_start_cursor = if self.preview_selection.song == PreviewSong::SPAGHETTI {
            sserafim_intro_start_cursor(sample_rate, bpm)
        } else if should_play_winter_horrorland_cutscene(self.preview_selection) {
            let cutscene = WinterHorrorlandCutsceneState::new(countdown_cursor, sample_rate);
            let start_cursor = cutscene.song_start_cursor();
            self.winter_horrorland_cutscene = Some(cutscene);
            start_cursor
        } else {
            countdown_cursor
        };
        self.song_start = Instant::now();
        self.song_started = false;
        self.audio_clock.reset(self.song_start);
        self.game_over = None;
        self.dialogue = None;
        self.stress_pico_end_cutscene = None;
        self.countdown_audio.reset();
        self.character_anim.reset_song();
        (
            self.score_popups,
            self.note_splashes,
            self.hold_covers,
            self.active_holds,
            self.held_lanes,
            self.opponent_receptors,
        ) = Default::default();
        self.camera_fx
            .reset(&mut self.cameras, self.base_camera_zoom);
        focus_initial(&mut self.cameras, &mut self.camera_fx, self.camera_focus);
        set_vocals_gain(&self.mixer, 1.0);
    }
    fn refresh_camera_focus(&mut self) {
        // ref: bdedc0aa:source/funkin/play/event/FocusCameraSongEvent.hx:97-145
        if let Some(characters) = &self.characters {
            self.camera_focus = characters.camera_focus_points();
        }
    }
    fn handle_preview_selection_input(&mut self, action: InputAction) -> bool {
        self.preview_selection = match action {
            InputAction::UiLeft => self.preview_selection.next_song(),
            InputAction::UiRight => self.preview_selection.next_difficulty(),
            _ => return false,
        };
        if self.mode == AppMode::Title {
            self.update_window_title();
        } else {
            self.load_selected_song();
        }
        true
    }
    fn rebuild_frame_commands(&mut self) {
        if self.mode == AppMode::Title {
            self.rebuild_title_commands();
            return;
        }
        if self.mode == AppMode::SongSelect {
            self.rebuild_song_select_commands();
            return;
        }
        if self.mode == AppMode::StoryMenu {
            self.rebuild_story_menu_commands();
            return;
        }
        if self.mode == AppMode::Options {
            self.rebuild_options_menu_commands();
            return;
        }
        if self.mode == AppMode::Credits {
            self.rebuild_credits_commands();
            return;
        }
        if self.mode == AppMode::MainMenu {
            self.rebuild_main_menu_commands();
            return;
        }
        if self.pause_menu.is_some() {
            self.rebuild_pause_commands();
            return;
        }
        if self.dialogue.is_some() {
            self.rebuild_dialogue_commands();
            return;
        }
        self.text_cmds = preview_text_commands(self.preview_selection);
        let sample_rate = play_sample_rate(&self.mixer);
        let cursor = if let Some(game_over) = self.game_over {
            game_over.cursor(sample_rate)
        } else {
            self.advance_song_clock()
        };
        if self.audio_output.is_some() {
            if let Some(cutscene) = self.winter_horrorland_cutscene.as_mut() {
                cutscene.tick_audio_or_warn(&self.mixer, cursor, sample_rate);
            }
        }
        self.append_debug_overlay_commands(cursor, sample_rate);
        if self.game_over.is_some() {
            self.rebuild_game_over_commands(cursor, sample_rate);
            return;
        }
        let mut opponent_hits = Vec::new();
        let mut song_events = Vec::new();
        let mut bpm = None;
        let mut late_misses = Vec::new();
        let mut dead = false;
        let confirm_duration = confirm_duration_or_default(self.note_skin.as_ref(), sample_rate);
        let hold_ticks = self.active_holds.score_ticks(cursor);
        self.opponent_receptors.update(cursor, confirm_duration);
        for lane in self.active_holds.complete_elapsed(cursor) {
            if self.held_lanes.is_held(lane) {
                self.held_lanes.complete_hold(lane, cursor);
            } else {
                self.held_lanes.play_static(lane);
            }
        }
        let active_hold_lanes: Vec<_> = self.active_holds.active_lanes(cursor).collect();
        for lane in active_hold_lanes {
            self.held_lanes.hold_confirm(lane, cursor, confirm_duration);
        }
        if let Some(play_state) = self.play_state.as_mut() {
            for tick in hold_ticks {
                play_state.register_hold_tick(tick.elapsed_samples, sample_rate);
            }
            song_events = play_state.resolve_song_events(cursor);
            opponent_hits = play_state.resolve_opponent_notes(cursor);
            late_misses = play_state.expire_late_notes(cursor, sample_rate);
            dead = play_state.is_dead() && !self.practice_mode;
            bpm = Some(play_state.bpm);
        }
        if dead {
            self.enter_game_over(cursor);
            self.rebuild_game_over_commands(cursor, sample_rate);
            return;
        }
        if self.finish_song_if_due(cursor, sample_rate) {
            return;
        }
        if let Some(bpm) = bpm {
            let anim = &mut self.character_anim;
            let had_late_misses = !late_misses.is_empty();
            for miss in late_misses {
                anim.player_note_miss_kind(miss.lane, miss.kind, cursor, sample_rate, bpm);
                if let Some(kind) = miss.kind {
                    play_note_kind_miss_sfx_or_warn(&self.mixer, kind);
                }
                anim.girlfriend_combo_drop(miss.combo_count, cursor);
                play_miss_sfx(&self.mixer, cursor, MissNoteKind::Scoreable);
            }
            if had_late_misses {
                set_vocals_gain(&self.mixer, 0.0);
            }
            let had_opponent_hits = !opponent_hits.is_empty();
            let opponent_receptors = &mut self.opponent_receptors;
            for hit in opponent_hits {
                opponent_receptors.confirm(hit.lane, cursor, confirm_duration);
                anim.opponent_note_hit_kind(hit.lane, hit.kind, cursor, sample_rate, bpm);
                if let Some(hold_end_at) = hit.hold_end_at {
                    opponent_receptors.start_hold(hit.lane, hold_end_at, cursor, hit.note_id);
                    self.hold_covers
                        .start_opponent(hit.lane, cursor, hold_end_at);
                }
            }
            if had_opponent_hits {
                if self.options_preferences.camera_zooms {
                    self.camera_fx.enable_zooming();
                }
                set_vocals_gain(&self.mixer, 1.0);
            }
            self.refresh_camera_focus();
            for event in song_events {
                self.apply_song_event(&event.kind, cursor, sample_rate, bpm);
            }
            self.character_anim.update(
                cursor,
                sample_rate,
                bpm,
                self.held_lanes.active_lanes().next().is_some(),
            );
            if let Some(cutscene) = self.stress_pico_end_cutscene.as_ref() {
                cutscene.apply_character_poses(&mut self.character_anim, cursor, sample_rate);
            }
            if !self.song_started {
                self.countdown_audio
                    .tick_or_warn(&self.mixer, cursor, sample_rate, bpm);
            }
            self.camera_fx
                .update(&mut self.cameras, cursor, sample_rate, bpm);
            self.sserafim_stage.apply_camera(
                &mut self.cameras,
                &mut self.camera_fx,
                cursor,
                sample_rate,
                bpm,
            );
            if let Some(cutscene) = self.winter_horrorland_cutscene.as_ref() {
                cutscene.apply_camera(&mut self.cameras, &mut self.camera_fx, cursor, sample_rate);
            }
        }
        self.cmds = self.static_cmds.clone();
        let stage_bpm = bpm.unwrap_or(100.0);
        self.stage_sfx.tick_or_warn(
            self.preview_selection.song,
            &self.mixer,
            cursor,
            sample_rate,
            stage_bpm,
            &self.sserafim_stage,
        );
        for cmd in self.stage_props.commands(
            cursor,
            sample_rate,
            stage_bpm,
            Some(self.preview_selection.song),
        ) {
            self.cmds.push(cmd);
        }
        if let Some(characters) = &self.characters {
            let poses = spooky_lightning_pose_overrides(
                self.preview_selection.song,
                self.character_anim.poses(),
                cursor,
                sample_rate,
                stage_bpm,
            );
            let poses = philly_train_pose_overrides(
                self.preview_selection.song,
                poses,
                cursor,
                sample_rate,
                stage_bpm,
            );
            for cmd in
                characters.commands_with_sserafim(&self.sserafim_stage, poses, cursor, sample_rate)
            {
                self.cmds.push(cmd);
            }
        }
        let (Some(play_state), Some(note_skin)) = (&self.play_state, &self.note_skin) else {
            return;
        };
        let blazin_middlescroll = self.preview_selection.song == PreviewSong::BLAZIN;
        let show_opponent_strumline = self.sserafim_stage.opponent_strumline_visible();
        let player_x_offset = if blazin_middlescroll {
            BLAZIN_MIDDLESCROLL_PLAYER_X_OFFSET
        } else {
            0.0
        };
        let downscroll = self.options_preferences.downscroll;
        for cmd in strumline_background_commands(
            self.options_preferences.strumline_background,
            downscroll,
            !blazin_middlescroll && show_opponent_strumline,
            player_x_offset,
        ) {
            self.cmds.push(cmd);
        }
        for view in play_state.hold_trail_views(cursor, sample_rate, |lane, opp| {
            if opp {
                show_opponent_strumline
            } else {
                self.held_lanes.is_held(lane)
            }
        }) {
            if (blazin_middlescroll || !show_opponent_strumline) && view.opponent {
                continue;
            }
            let mut view = view;
            if blazin_middlescroll {
                view.x += BLAZIN_MIDDLESCROLL_PLAYER_X_OFFSET;
            }
            if view.head_resolved && !view.opponent && !self.held_lanes.is_held(view.lane) {
                continue;
            }
            for cmd in note_skin.hold_trail_commands(&view) {
                let mut cmd = cmd;
                apply_downscroll(&mut cmd, downscroll);
                if cmd.world_pos.y + cmd.size.y >= -200.0 {
                    self.cmds.push(cmd);
                }
            }
        }
        let receptor_commands = note_skin.receptor_commands_with_layout(
            cursor,
            sample_rate,
            !blazin_middlescroll && show_opponent_strumline,
            player_x_offset,
            |player, lane| match player {
                1 => self.held_lanes.receptor_state(lane, cursor),
                _ => self.opponent_receptors.receptor_state(lane, cursor),
            },
        );
        for cmd in receptor_commands {
            let mut cmd = cmd;
            apply_downscroll(&mut cmd, downscroll);
            self.cmds.push(cmd);
        }
        for view in play_state.note_views(cursor, sample_rate) {
            if (blazin_middlescroll || !show_opponent_strumline) && view.opponent {
                continue;
            }
            let mut view = view;
            if blazin_middlescroll {
                view.x += BLAZIN_MIDDLESCROLL_PLAYER_X_OFFSET;
            }
            if view.is_sustain {
                continue;
            }
            let mut cmd = note_skin.command_for_view(&view);
            apply_downscroll(&mut cmd, downscroll);
            if cmd.world_pos.y + cmd.size.y >= -200.0 {
                self.cmds.push(cmd);
            }
        }
        if let Some(note_splash_skin) = &self.note_splash_skin {
            for cmd in self
                .note_splashes
                .commands(note_splash_skin, cursor, sample_rate)
            {
                let mut cmd = cmd;
                apply_downscroll(&mut cmd, downscroll);
                self.cmds.push(cmd);
            }
        }
        if let Some(hold_cover_skin) = &self.hold_cover_skin {
            for cmd in self.hold_covers.commands_with_opponent_visibility(
                hold_cover_skin,
                cursor,
                sample_rate,
                show_opponent_strumline,
            ) {
                let mut cmd = cmd;
                apply_downscroll(&mut cmd, downscroll);
                self.cmds.push(cmd);
            }
        }
        if let Some(hud_skin) = &self.hud_skin {
            for cmd in hud_skin.commands_with_icon_scale_and_visibility(
                play_state.health,
                health_icon_scale(cursor, sample_rate, play_state.bpm),
                self.sserafim_stage.opponent_health_icon_visible(),
            ) {
                self.cmds.push(cmd);
            }
        }
        if let Some(bitmap_text_skin) = &self.bitmap_text_skin {
            for cmd in bitmap_text_skin.score_text_commands(play_state.score) {
                self.cmds.push(cmd);
            }
        }
        if let Some(popup_skin) = &self.popup_skin {
            for cmd in self
                .score_popups
                .commands(popup_skin, cursor, sample_rate, play_state.bpm)
            {
                self.cmds.push(cmd);
            }
        }
        if let Some(countdown_skin) = &self.countdown_skin {
            for cmd in countdown_skin.commands(cursor, sample_rate, play_state.bpm) {
                self.cmds.push(cmd);
            }
        }
        self.sserafim_stage
            .apply_commands(self.cmds.iter_mut(), cursor, sample_rate, stage_bpm);
        if let Some(cutscene) = self.winter_horrorland_cutscene.as_ref() {
            cutscene.apply_commands(self.cmds.iter_mut(), cursor, sample_rate);
            cutscene.append_commands(&mut self.cmds, cursor, sample_rate);
        }
        if let Some(cutscene) = self.stress_pico_end_cutscene.as_ref() {
            cutscene.apply_commands(self.cmds.iter_mut());
            cutscene.append_commands(
                &mut self.cmds,
                &mut self.text_cmds,
                cursor,
                sample_rate,
                self.options_preferences.subtitles,
            );
        } else if self.options_preferences.subtitles {
            if let Some(track) = self.subtitle_track.as_ref() {
                track.append_commands(&mut self.text_cmds, cursor, sample_rate);
            }
        }
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
            WindowEvent::Focused(false) => {
                let cursor = self.input_cursor();
                self.pause_on_unfocus(cursor);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(action) = map_key(event.physical_key) {
                    let song_cursor = self.input_cursor();
                    let evt = build_event(action, event.state, self.boot_instant, song_cursor);
                    let already_held = event.repeat
                        || lane_for_action(action)
                            .map(|lane| self.held_lanes.is_held(lane))
                            .unwrap_or(false);
                    if event.state == ElementState::Pressed && action == InputAction::Debug {
                        self.toggle_debug_overlay();
                        return;
                    }
                    if self.handle_dialogue_input(action, event.state) {
                        return;
                    }
                    if self.handle_pause_input(action, event.state, song_cursor) {
                        return;
                    }
                    if self.handle_mode_input(action, event.state, event_loop) {
                        return;
                    }
                    if self.game_over.is_some() {
                        if event.state == ElementState::Pressed
                            && self.game_over_restart.is_none()
                            && self.game_over.is_some_and(|state| state.can_accept_input())
                        {
                            match action {
                                InputAction::Confirm => {
                                    self.restart_song_after_game_over(song_cursor);
                                }
                                InputAction::Back => {
                                    self.return_to_play_menu();
                                }
                                _ => {}
                            }
                        }
                        return;
                    }
                    if event.state == ElementState::Pressed
                        && self.handle_preview_selection_input(action)
                    {
                        return;
                    }
                    self.held_lanes.apply(&evt);
                    if event.state == ElementState::Released {
                        if let Some(lane) = lane_for_action(action) {
                            if let Some(release) = self.active_holds.release(lane, song_cursor) {
                                self.register_hold_tick(release.elapsed_samples);
                                if release.hold_end_at > song_cursor {
                                    self.register_hold_drop(
                                        lane,
                                        song_cursor,
                                        release.hold_end_at,
                                        release.note_id,
                                    );
                                }
                            }
                            self.hold_covers.end(lane, song_cursor);
                            self.held_lanes.play_static(lane);
                        }
                    }
                    self.screens.input(&evt);
                    self.handle_gameplay_input(&evt, already_held);
                    if event.state == ElementState::Pressed && action == InputAction::Back {
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

pub fn run(options: AppOptions) -> Result<()> {
    init_logging();
    install_panic_hook();
    tracing::info!(target: "rustic", "starting RusticV3 ({}x{})", options.width, options.height);

    let event_loop = EventLoop::new()?;
    let mut app = App::new(options);
    event_loop.run_app(&mut app)?;
    Ok(())
}
