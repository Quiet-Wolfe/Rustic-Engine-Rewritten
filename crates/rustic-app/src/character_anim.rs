//! Runtime character pose state driven by OG note-hit rules.
// LINT-ALLOW: long-file character pose state plus v0.8.5 timing tests

use crate::note_kind_anim::{NoteKindAction, NoteKindAnimState};
use rustic_core::time::Samples;
use rustic_game::{Judgment, Lane, NoteKind};

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
    note_kind_anim: NoteKindAnimState,
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
            note_kind_anim: NoteKindAnimState::default(),
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

    pub fn opponent_note_hit_kind(
        &mut self,
        lane: Lane,
        kind: Option<NoteKind>,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        if let Some(kind) = kind {
            let action = self.note_kind_anim.opponent_hit(kind, self.opponent_pose);
            if self.apply_opponent_note_kind_action(action, lane, cursor, sample_rate, bpm) {
                return;
            }
        }
        self.opponent_note_hit(lane, cursor, sample_rate, bpm);
    }

    pub fn player_note_hit(&mut self, lane: Lane, cursor: Samples, sample_rate: u32, bpm: f64) {
        // ref: bdedc0aa:source/funkin/play/character/BaseCharacter.hx:535-549
        self.player_pose = sing_pose(lane);
        self.player_started = cursor;
        self.player_until =
            Samples(cursor.0 + hold_samples(sample_rate, bpm, self.timings.player_sing_steps));
    }

    pub fn player_note_hit_kind(
        &mut self,
        lane: Lane,
        kind: Option<NoteKind>,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        if let Some(kind) = kind {
            let action = self.note_kind_anim.player_hit(kind, self.player_pose);
            if self.apply_player_note_kind_action(action, lane, cursor, sample_rate, bpm) {
                return;
            }
        }
        self.player_note_hit(lane, cursor, sample_rate, bpm);
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

    pub fn player_note_miss_kind(
        &mut self,
        lane: Lane,
        kind: Option<NoteKind>,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) {
        if let Some(kind) = kind {
            let action = self.note_kind_anim.player_miss(kind, self.player_pose);
            if self.apply_player_note_kind_action(action, lane, cursor, sample_rate, bpm) {
                return;
            }
        }
        self.player_note_miss(lane, cursor, sample_rate, bpm);
    }

    fn apply_opponent_note_kind_action(
        &mut self,
        action: NoteKindAction,
        lane: Lane,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) -> bool {
        match action {
            NoteKindAction::Fallthrough => false,
            NoteKindAction::Skip => true,
            NoteKindAction::SingSuffix(suffix) => {
                self.opponent_pose = sing_pose_with_suffix(lane, suffix);
                self.opponent_started = cursor;
                self.opponent_until = Samples(
                    cursor.0 + hold_samples(sample_rate, bpm, self.timings.opponent_sing_steps),
                );
                true
            }
            NoteKindAction::MissSuffix(_) => false,
            NoteKindAction::Pose(pose) => {
                self.opponent_pose = pose;
                self.opponent_started = cursor;
                true
            }
        }
    }

    fn apply_player_note_kind_action(
        &mut self,
        action: NoteKindAction,
        lane: Lane,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
    ) -> bool {
        match action {
            NoteKindAction::Fallthrough => false,
            NoteKindAction::Skip => true,
            NoteKindAction::SingSuffix(suffix) => {
                self.player_pose = sing_pose_with_suffix(lane, suffix);
                self.player_started = cursor;
                self.player_until = Samples(
                    cursor.0 + hold_samples(sample_rate, bpm, self.timings.player_sing_steps),
                );
                true
            }
            NoteKindAction::MissSuffix(suffix) => {
                self.player_pose = miss_pose_with_suffix(lane, suffix);
                self.player_started = cursor;
                self.player_until = Samples(
                    cursor.0
                        + hold_samples(
                            sample_rate,
                            bpm,
                            self.timings.player_sing_steps * MISS_SING_TIME_MULTIPLIER,
                        ),
                );
                true
            }
            NoteKindAction::Pose(pose) => {
                self.player_pose = pose;
                self.player_started = cursor;
                true
            }
        }
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
    // ref: bdedc0aa:source/funkin/Conductor.hx:469-486
    let samples_per_step = f64::from(sample_rate.max(1)) * 15.0 / bpm.max(1.0);
    let step_time = round_decimal(cursor.0 as f64 / samples_per_step, 6);
    (step_time / 4.0).floor() as i64
}

fn round_decimal(value: f64, decimals: i32) -> f64 {
    let scale = 10_f64.powi(decimals.max(0));
    (value * scale).round() / scale
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

fn sing_pose_with_suffix(lane: Lane, suffix: &str) -> &'static str {
    match suffix {
        "alt" => match lane {
            Lane::Left => "singLEFT-alt",
            Lane::Down => "singDOWN-alt",
            Lane::Up => "singUP-alt",
            Lane::Right => "singRIGHT-alt",
            _ => "singRIGHT-alt",
        },
        "censor" => match lane {
            Lane::Left => "singLEFT-censor",
            Lane::Down => "singDOWN-censor",
            Lane::Up => "singUP-censor",
            Lane::Right => "singRIGHT-censor",
            _ => "singRIGHT-censor",
        },
        "joint" => match lane {
            Lane::Left => "singLEFT-joint",
            Lane::Down => "singDOWN-joint",
            Lane::Up => "singUP-joint",
            Lane::Right => "singRIGHT-joint",
            _ => "singRIGHT-joint",
        },
        "bf1" => match lane {
            Lane::Left => "singLEFT-bf1",
            Lane::Down => "singDOWN-bf1",
            Lane::Up => "singUP-bf1",
            Lane::Right => "singRIGHT-bf1",
            _ => "singRIGHT-bf1",
        },
        "bf2" => match lane {
            Lane::Left => "singLEFT-bf2",
            Lane::Down => "singDOWN-bf2",
            Lane::Up => "singUP-bf2",
            Lane::Right => "singRIGHT-bf2",
            _ => "singRIGHT-bf2",
        },
        _ => sing_pose(lane),
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

fn miss_pose_with_suffix(lane: Lane, suffix: &str) -> &'static str {
    match suffix {
        "joint" => match lane {
            Lane::Left => "singLEFTmiss-joint",
            Lane::Down => "singDOWNmiss-joint",
            Lane::Up => "singUPmiss-joint",
            Lane::Right => "singRIGHTmiss-joint",
            _ => "singRIGHTmiss-joint",
        },
        "bf2" => match lane {
            Lane::Left => "singLEFTmiss-bf2",
            Lane::Down => "singDOWNmiss-bf2",
            Lane::Up => "singUPmiss-bf2",
            Lane::Right => "singRIGHTmiss-bf2",
            _ => "singRIGHTmiss-bf2",
        },
        _ => miss_pose(lane),
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
#[path = "character_anim_tests.rs"]
mod tests;
