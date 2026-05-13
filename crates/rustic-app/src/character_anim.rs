//! Runtime character pose state driven by OG note-hit rules.
// LINT-ALLOW: long-file character pose state plus v0.8.5 timing tests

use rustic_core::time::Samples;
use rustic_game::{Judgment, Lane};

// ref: bdedc0aa:source/funkin/play/character/BaseCharacter.hx:389-399,457-473
const CHART_ANIM_HOLD_STEPS: f64 = 4.0;
const MISS_SING_TIME_MULTIPLIER: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterAnimTimings {
    pub player_sing_steps: f64,
    pub opponent_sing_steps: f64,
    pub girlfriend_combo_timings: CountAnimationTimings,
    pub girlfriend_drop_timings: CountAnimationTimings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CountAnimationTiming {
    pub count: u32,
    pub duration_seconds: f64,
}

impl Default for CharacterAnimTimings {
    fn default() -> Self {
        // ref: bdedc0aa:source/funkin/data/character/CharacterData.hx:452
        Self {
            player_sing_steps: 8.0,
            opponent_sing_steps: 8.0,
            girlfriend_combo_timings: [None; COUNT_ANIM_SLOTS],
            girlfriend_drop_timings: [None; COUNT_ANIM_SLOTS],
        }
    }
}

pub const COUNT_ANIM_SLOTS: usize = 4;
pub type CountAnimationTimings = [Option<CountAnimationTiming>; COUNT_ANIM_SLOTS];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterPoseRequest {
    pub name: &'static str,
    pub started_at: Samples,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterPoseNames {
    pub girlfriend: CharacterPoseRequest,
    pub opponent: CharacterPoseRequest,
    pub player: CharacterPoseRequest,
}

#[derive(Debug, Clone)]
pub struct CharacterAnimState {
    girlfriend_pose: &'static str,
    opponent_pose: &'static str,
    player_pose: &'static str,
    girlfriend_started: Samples,
    opponent_started: Samples,
    player_started: Samples,
    opponent_until: Samples,
    player_until: Samples,
    girlfriend_count_duration_seconds: Option<f64>,
    timings: CharacterAnimTimings,
    last_beat: i64,
    gf_danced: bool,
}

impl Default for CharacterAnimState {
    fn default() -> Self {
        Self {
            // ref: bdedc0aa:source/funkin/play/stage/Bopper.hx:179-202
            // `resetCharacter` forces one dance immediately; with `hasDanced`
            // initially false, that first dance is `danceLeft`.
            girlfriend_pose: "danceLeft",
            opponent_pose: "idle",
            player_pose: "idle",
            girlfriend_started: Samples(0),
            opponent_started: Samples(0),
            player_started: Samples(0),
            opponent_until: Samples(0),
            player_until: Samples(0),
            girlfriend_count_duration_seconds: None,
            timings: CharacterAnimTimings::default(),
            last_beat: i64::MIN,
            gf_danced: true,
        }
    }
}

impl CharacterAnimState {
    pub fn poses(&self) -> CharacterPoseNames {
        CharacterPoseNames {
            girlfriend: CharacterPoseRequest {
                name: self.girlfriend_pose,
                started_at: self.girlfriend_started,
            },
            opponent: CharacterPoseRequest {
                name: self.opponent_pose,
                started_at: self.opponent_started,
            },
            player: CharacterPoseRequest {
                name: self.player_pose,
                started_at: self.player_started,
            },
        }
    }

    pub fn set_timings(&mut self, timings: CharacterAnimTimings) {
        self.timings = timings;
    }

    pub fn reset_song(&mut self) {
        let timings = self.timings;
        *self = Self::default();
        self.timings = timings;
    }

    pub fn update(&mut self, cursor: Samples, sample_rate: u32, bpm: f64, player_holding: bool) {
        self.update_beat_dances(cursor, sample_rate, bpm);
        self.update_chart_animation_resets(cursor, sample_rate, bpm);
        if self.opponent_pose.starts_with("sing") && cursor >= self.opponent_until {
            self.opponent_pose = "idle";
            self.opponent_started = cursor;
        }
        if self.player_pose.starts_with("sing")
            && (self.player_pose.ends_with("miss") || !player_holding)
            && cursor >= self.player_until
        {
            self.player_pose = "idle";
            self.player_started = cursor;
        }
    }

    fn update_chart_animation_resets(&mut self, cursor: Samples, sample_rate: u32, bpm: f64) {
        let hold = hold_samples(sample_rate, bpm, CHART_ANIM_HOLD_STEPS);
        if is_girlfriend_count_pose(self.girlfriend_pose) {
            if self.girlfriend_count_finished(cursor, sample_rate) {
                self.dance_girlfriend(cursor);
            }
        } else if is_chart_special_pose(self.girlfriend_pose)
            && cursor.0.saturating_sub(self.girlfriend_started.0) >= hold
        {
            self.dance_girlfriend(cursor);
        }
        if is_chart_special_pose(self.opponent_pose)
            && cursor.0.saturating_sub(self.opponent_started.0) >= hold
        {
            self.opponent_pose = "idle";
            self.opponent_started = cursor;
        }
        if is_chart_special_pose(self.player_pose)
            && cursor.0.saturating_sub(self.player_started.0) >= hold
        {
            self.player_pose = "idle";
            self.player_started = cursor;
        }
    }

    pub fn play_chart_animation(
        &mut self,
        target: &str,
        animation: &str,
        cursor: Samples,
        force: bool,
    ) -> bool {
        let Some(pose) = chart_animation_pose(animation) else {
            return false;
        };
        let target = target.to_ascii_lowercase();
        match target.as_str() {
            "bf" | "boyfriend" | "player" => {
                if force || self.player_pose != pose {
                    self.player_pose = pose;
                    self.player_started = cursor;
                }
                true
            }
            "dad" | "opponent" => {
                if force || self.opponent_pose != pose {
                    self.opponent_pose = pose;
                    self.opponent_started = cursor;
                }
                true
            }
            "gf" | "girlfriend" => {
                if force || self.girlfriend_pose != pose {
                    self.girlfriend_pose = pose;
                    self.girlfriend_started = cursor;
                    self.girlfriend_count_duration_seconds = None;
                }
                true
            }
            _ => false,
        }
    }

    pub fn opponent_note_hit(&mut self, lane: Lane, cursor: Samples, sample_rate: u32, bpm: f64) {
        // ref: bdedc0aa:source/funkin/play/character/BaseCharacter.hx:551-565
        self.opponent_pose = sing_pose(lane);
        self.opponent_started = cursor;
        self.opponent_until =
            Samples(cursor.0 + hold_samples(sample_rate, bpm, self.timings.opponent_sing_steps));
    }

    pub fn player_note_hit(&mut self, lane: Lane, cursor: Samples, sample_rate: u32, bpm: f64) {
        // ref: bdedc0aa:source/funkin/play/character/BaseCharacter.hx:535-549
        self.player_pose = sing_pose(lane);
        self.player_started = cursor;
        self.player_until =
            Samples(cursor.0 + hold_samples(sample_rate, bpm, self.timings.player_sing_steps));
    }

    pub fn player_note_miss(&mut self, lane: Lane, cursor: Samples, sample_rate: u32, bpm: f64) {
        // ref: bdedc0aa:source/funkin/play/character/BaseCharacter.hx:590-601
        self.player_pose = miss_pose(lane);
        self.player_started = cursor;
        self.player_until = Samples(
            cursor.0
                + hold_samples(
                    sample_rate,
                    bpm,
                    self.timings.player_sing_steps * MISS_SING_TIME_MULTIPLIER,
                ),
        );
    }

    pub fn girlfriend_note_hit(&mut self, judgment: Judgment, combo_count: u32, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/character/BaseCharacter.hx:528-576
        match judgment {
            Judgment::Sick | Judgment::Good => {
                if let Some(timing) =
                    exact_count_timing(&self.timings.girlfriend_combo_timings, combo_count)
                {
                    self.play_girlfriend_count_animation(combo_pose(timing.count), timing, cursor);
                }
            }
            _ => self.girlfriend_combo_drop(combo_count, cursor),
        }
    }

    pub fn girlfriend_combo_drop(&mut self, combo_count: u32, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/character/BaseCharacter.hx:639-654
        let Some(drop) = highest_drop_timing(&self.timings.girlfriend_drop_timings, combo_count)
        else {
            return;
        };
        self.play_girlfriend_count_animation(drop_pose(drop.count), drop, cursor);
    }

    pub fn player_first_death(&mut self, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/GameOverSubState.hx:259
        self.player_pose = "firstDeath";
        self.player_started = cursor;
    }

    pub fn player_death_loop(&mut self, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/GameOverSubState.hx:312-315
        self.player_pose = "deathLoop";
        self.player_started = cursor;
    }

    pub fn player_death_confirm(&mut self, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/GameOverSubState.hx:379
        self.player_pose = "deathConfirm";
        self.player_started = cursor;
    }

    fn update_beat_dances(&mut self, cursor: Samples, sample_rate: u32, bpm: f64) {
        let beat = beat_index(cursor, sample_rate, bpm);
        if beat <= self.last_beat {
            return;
        }
        self.last_beat = beat;
        // ref: bdedc0aa:source/funkin/play/stage/Bopper.hx:172-180
        if self.girlfriend_pose.starts_with("dance") {
            self.dance_girlfriend(cursor);
        }
        // ref: bdedc0aa:source/funkin/play/character/BaseCharacter.hx:457-473
        if self.opponent_pose.starts_with("idle") || self.opponent_pose.starts_with("dance") {
            self.opponent_pose = "idle";
            self.opponent_started = cursor;
        }
        if self.player_pose.starts_with("idle") || self.player_pose.starts_with("dance") {
            self.player_pose = "idle";
            self.player_started = cursor;
        }
    }

    fn dance_girlfriend(&mut self, cursor: Samples) {
        // ref: bdedc0aa:source/funkin/play/stage/Bopper.hx:179-202
        self.girlfriend_pose = if self.gf_danced {
            "danceRight"
        } else {
            "danceLeft"
        };
        self.gf_danced = !self.gf_danced;
        self.girlfriend_started = cursor;
        self.girlfriend_count_duration_seconds = None;
    }

    fn play_girlfriend_count_animation(
        &mut self,
        pose: Option<&'static str>,
        timing: CountAnimationTiming,
        cursor: Samples,
    ) {
        if let Some(pose) = pose {
            self.girlfriend_pose = pose;
            self.girlfriend_started = cursor;
            self.girlfriend_count_duration_seconds = Some(timing.duration_seconds);
        }
    }

    fn girlfriend_count_finished(&self, cursor: Samples, sample_rate: u32) -> bool {
        let Some(seconds) = self.girlfriend_count_duration_seconds else {
            return false;
        };
        let duration = (seconds.max(0.0) * f64::from(sample_rate.max(1))).ceil() as i64;
        cursor.0.saturating_sub(self.girlfriend_started.0) >= duration.max(1)
    }
}

fn chart_animation_pose(animation: &str) -> Option<&'static str> {
    match animation {
        "hey" => Some("hey"),
        "cheer" => Some("cheer"),
        _ => None,
    }
}

fn hold_samples(sample_rate: u32, bpm: f64, steps: f64) -> i64 {
    let bpm = bpm.max(1.0);
    ((15.0 * f64::from(sample_rate) / bpm) * steps).round() as i64
}

fn beat_index(cursor: Samples, sample_rate: u32, bpm: f64) -> i64 {
    // ref: bdedc0aa:source/funkin/Conductor.hx:481-486
    let samples_per_beat = f64::from(sample_rate) * 60.0 / bpm.max(1.0);
    (cursor.0 as f64 / samples_per_beat).floor() as i64
}

fn is_chart_special_pose(pose: &str) -> bool {
    !pose.starts_with("dance")
        && !pose.starts_with("idle")
        && !pose.starts_with("sing")
        && !pose.starts_with("death")
}

fn is_girlfriend_count_pose(pose: &str) -> bool {
    pose.starts_with("combo") || pose.starts_with("drop")
}

fn exact_count_timing(
    timings: &CountAnimationTimings,
    combo_count: u32,
) -> Option<CountAnimationTiming> {
    timings
        .iter()
        .flatten()
        .copied()
        .find(|timing| timing.count == combo_count)
}

fn highest_drop_timing(
    timings: &CountAnimationTimings,
    combo_count: u32,
) -> Option<CountAnimationTiming> {
    timings
        .iter()
        .flatten()
        .copied()
        .filter(|timing| combo_count >= timing.count)
        .max_by_key(|timing| timing.count)
}

fn sing_pose(lane: Lane) -> &'static str {
    match lane {
        Lane::Left => "singLEFT",
        Lane::Down => "singDOWN",
        Lane::Up => "singUP",
        Lane::Right => "singRIGHT",
        _ => "singRIGHT",
    }
}

fn miss_pose(lane: Lane) -> &'static str {
    match lane {
        Lane::Left => "singLEFTmiss",
        Lane::Down => "singDOWNmiss",
        Lane::Up => "singUPmiss",
        Lane::Right => "singRIGHTmiss",
        _ => "singRIGHTmiss",
    }
}

fn combo_pose(count: u32) -> Option<&'static str> {
    match count {
        50 => Some("combo50"),
        200 => Some("combo200"),
        _ => None,
    }
}

fn drop_pose(count: u32) -> Option<&'static str> {
    match count {
        70 => Some("drop70"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_timing(count: u32, duration_seconds: f64) -> CountAnimationTiming {
        CountAnimationTiming {
            count,
            duration_seconds,
        }
    }

    #[test]
    fn opponent_sings_then_returns_to_idle_after_dad_hold_time() {
        let mut state = CharacterAnimState::default();
        state.opponent_note_hit(Lane::Left, Samples(1_000), 48_000, 100.0);

        assert_eq!(state.poses().opponent.name, "singLEFT");
        assert_eq!(state.poses().opponent.started_at, Samples(1_000));
        state.update(Samples(58_599), 48_000, 100.0, false);
        assert_eq!(state.poses().opponent.name, "singLEFT");
        state.update(Samples(58_600), 48_000, 100.0, false);
        assert_eq!(state.poses().opponent.name, "idle");
        assert_eq!(state.poses().opponent.started_at, Samples(58_600));
    }

    #[test]
    fn player_hold_keeps_sing_pose_until_sing_time_after_release() {
        let mut state = CharacterAnimState::default();
        state.player_note_hit(Lane::Down, Samples(0), 48_000, 100.0);

        state.update(Samples(30_000), 48_000, 100.0, true);
        assert_eq!(state.poses().player.name, "singDOWN");
        state.update(Samples(30_000), 48_000, 100.0, false);
        assert_eq!(state.poses().player.name, "singDOWN");
        state.update(Samples(57_600), 48_000, 100.0, false);
        assert_eq!(state.poses().player.name, "idle");
    }

    #[test]
    fn character_sing_timing_uses_loaded_vslice_steps() {
        let mut state = CharacterAnimState::default();
        state.set_timings(CharacterAnimTimings {
            player_sing_steps: 5.0,
            opponent_sing_steps: 6.5,
            ..CharacterAnimTimings::default()
        });

        state.player_note_hit(Lane::Down, Samples(0), 48_000, 100.0);
        state.update(Samples(35_999), 48_000, 100.0, false);
        assert_eq!(state.poses().player.name, "singDOWN");
        state.update(Samples(36_000), 48_000, 100.0, false);
        assert_eq!(state.poses().player.name, "idle");

        state.opponent_note_hit(Lane::Up, Samples(0), 48_000, 100.0);
        state.update(Samples(46_799), 48_000, 100.0, false);
        assert_eq!(state.poses().opponent.name, "singUP");
        state.update(Samples(46_800), 48_000, 100.0, false);
        assert_eq!(state.poses().opponent.name, "idle");
    }

    #[test]
    fn girlfriend_toggles_dance_on_new_beat() {
        let mut state = CharacterAnimState::default();

        assert_eq!(state.poses().girlfriend.name, "danceLeft");
        state.update(Samples(0), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceRight");
        assert_eq!(state.poses().girlfriend.started_at, Samples(0));
        state.update(Samples(28_800), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceLeft");
        assert_eq!(state.poses().girlfriend.started_at, Samples(28_800));
    }

    #[test]
    fn girlfriend_dances_through_negative_countdown_beats() {
        let mut state = CharacterAnimState::default();

        state.update(Samples(-144_000), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceRight");
        state.update(Samples(-115_200), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceLeft");
        state.update(Samples(-86_400), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceRight");
        state.update(Samples(-57_600), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceLeft");
        state.update(Samples(-28_800), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceRight");
        state.update(Samples(0), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceLeft");
    }

    #[test]
    fn player_idle_restarts_on_each_beat_when_not_singing() {
        let mut state = CharacterAnimState::default();

        state.update(Samples(0), 48_000, 100.0, false);
        state.update(Samples(28_800), 48_000, 100.0, false);

        assert_eq!(state.poses().player.name, "idle");
        assert_eq!(state.poses().player.started_at, Samples(28_800));
    }

    #[test]
    fn opponent_idle_restarts_on_each_beat_when_not_singing() {
        let mut state = CharacterAnimState::default();

        state.update(Samples(0), 48_000, 100.0, false);
        state.update(Samples(28_800), 48_000, 100.0, false);

        assert_eq!(state.poses().opponent.name, "idle");
        assert_eq!(state.poses().opponent.started_at, Samples(28_800));
    }

    #[test]
    fn chart_event_triggers_bf_hey_until_next_beat() {
        let mut state = CharacterAnimState::default();

        assert!(state.play_chart_animation("bf", "hey", Samples(201_600), true));
        assert_eq!(state.poses().player.name, "hey");
        assert_eq!(state.poses().player.started_at, Samples(201_600));

        state.update(Samples(230_400), 48_000, 100.0, false);
        assert_eq!(state.poses().player.name, "idle");
        assert_eq!(state.poses().player.started_at, Samples(230_400));
    }

    #[test]
    fn chart_event_triggers_known_girlfriend_animation() {
        let mut state = CharacterAnimState::default();

        assert!(state.play_chart_animation("girlfriend", "cheer", Samples(500), true));

        assert_eq!(state.poses().girlfriend.name, "cheer");
        assert_eq!(state.poses().girlfriend.started_at, Samples(500));
    }

    #[test]
    fn girlfriend_chart_animation_returns_to_dance_after_hold_time() {
        let mut state = CharacterAnimState::default();
        state.play_chart_animation("girlfriend", "cheer", Samples(0), true);

        state.update(Samples(28_799), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "cheer");
        state.update(Samples(28_800), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceRight");
        assert_eq!(state.poses().girlfriend.started_at, Samples(28_800));
    }

    #[test]
    fn girlfriend_plays_combo_animation_only_on_matching_good_hit_count() {
        let mut state = CharacterAnimState::default();
        state.set_timings(CharacterAnimTimings {
            girlfriend_combo_timings: [Some(count_timing(50, 0.5)), None, None, None],
            ..CharacterAnimTimings::default()
        });

        state.girlfriend_note_hit(Judgment::Sick, 49, Samples(100));
        assert_eq!(state.poses().girlfriend.name, "danceLeft");

        state.girlfriend_note_hit(Judgment::Good, 50, Samples(200));
        assert_eq!(state.poses().girlfriend.name, "combo50");
        assert_eq!(state.poses().girlfriend.started_at, Samples(200));
    }

    #[test]
    fn girlfriend_plays_highest_matching_drop_animation() {
        let mut state = CharacterAnimState::default();
        state.set_timings(CharacterAnimTimings {
            girlfriend_drop_timings: [
                Some(count_timing(10, 0.1)),
                Some(count_timing(70, 0.5)),
                None,
                None,
            ],
            ..CharacterAnimTimings::default()
        });

        state.girlfriend_note_hit(Judgment::Bad, 69, Samples(100));
        assert_eq!(state.poses().girlfriend.name, "danceLeft");

        state.girlfriend_combo_drop(70, Samples(200));
        assert_eq!(state.poses().girlfriend.name, "drop70");
        assert_eq!(state.poses().girlfriend.started_at, Samples(200));
    }

    #[test]
    fn girlfriend_count_animation_returns_to_dance_after_source_duration() {
        let mut state = CharacterAnimState::default();
        state.set_timings(CharacterAnimTimings {
            girlfriend_combo_timings: [Some(count_timing(50, 0.25)), None, None, None],
            ..CharacterAnimTimings::default()
        });

        state.girlfriend_note_hit(Judgment::Sick, 50, Samples(1_000));
        state.update(Samples(12_999), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "combo50");
        state.update(Samples(13_000), 48_000, 100.0, false);
        assert_eq!(state.poses().girlfriend.name, "danceRight");
        assert_eq!(state.poses().girlfriend.started_at, Samples(13_000));
    }

    #[test]
    fn opponent_chart_animation_returns_to_idle_after_hold_time() {
        let mut state = CharacterAnimState::default();
        state.play_chart_animation("dad", "cheer", Samples(0), true);

        state.update(Samples(28_799), 48_000, 100.0, false);
        assert_eq!(state.poses().opponent.name, "cheer");
        state.update(Samples(28_800), 48_000, 100.0, false);
        assert_eq!(state.poses().opponent.name, "idle");
        assert_eq!(state.poses().opponent.started_at, Samples(28_800));
    }

    #[test]
    fn miss_pose_restarts_from_the_miss_cursor() {
        let mut state = CharacterAnimState::default();
        state.player_note_miss(Lane::Right, Samples(1_234), 48_000, 100.0);

        assert_eq!(state.poses().player.name, "singRIGHTmiss");
        assert_eq!(state.poses().player.started_at, Samples(1_234));
    }

    #[test]
    fn miss_pose_returns_to_idle_after_extended_sing_time() {
        let mut state = CharacterAnimState::default();
        state.player_note_miss(Lane::Right, Samples(0), 48_000, 100.0);

        state.update(Samples(115_199), 48_000, 100.0, true);
        assert_eq!(state.poses().player.name, "singRIGHTmiss");
        state.update(Samples(115_200), 48_000, 100.0, true);
        assert_eq!(state.poses().player.name, "idle");
        assert_eq!(state.poses().player.started_at, Samples(115_200));
    }

    #[test]
    fn death_poses_restart_from_death_cursors() {
        let mut state = CharacterAnimState::default();
        state.player_first_death(Samples(4_800));

        assert_eq!(state.poses().player.name, "firstDeath");
        assert_eq!(state.poses().player.started_at, Samples(4_800));

        state.player_death_loop(Samples(9_600));
        assert_eq!(state.poses().player.name, "deathLoop");
        assert_eq!(state.poses().player.started_at, Samples(9_600));

        state.player_death_confirm(Samples(14_400));
        assert_eq!(state.poses().player.name, "deathConfirm");
        assert_eq!(state.poses().player.started_at, Samples(14_400));
    }

    #[test]
    fn reset_song_restores_starting_poses_and_keeps_loaded_timings() {
        let mut state = CharacterAnimState::default();
        let timings = CharacterAnimTimings {
            player_sing_steps: 6.1,
            opponent_sing_steps: 4.2,
            girlfriend_combo_timings: [Some(count_timing(50, 0.5)), None, None, None],
            girlfriend_drop_timings: [Some(count_timing(70, 0.5)), None, None, None],
        };
        state.set_timings(timings);
        state.player_note_hit(Lane::Left, Samples(1_000), 48_000, 100.0);
        state.opponent_note_hit(Lane::Right, Samples(1_000), 48_000, 100.0);

        state.reset_song();

        assert_eq!(state.poses().girlfriend.name, "danceLeft");
        assert_eq!(state.poses().opponent.name, "idle");
        assert_eq!(state.poses().player.name, "idle");
        assert_eq!(state.timings, timings);
    }
}
