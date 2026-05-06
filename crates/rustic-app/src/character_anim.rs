//! Runtime character pose state driven by OG note-hit rules.

use rustic_core::time::Samples;
use rustic_game::Lane;

// ref: 50fccded:source/PlayState.hx:1551          // dad.holdTimer = 0 on note
// ref: 50fccded:source/Character.hx:531-540       // dad holdTimer threshold
const DAD_HOLD_STEPS: f64 = 6.1;
// ref: 50fccded:source/PlayState.hx:1983          // BF returns after 4 steps
const BF_HOLD_STEPS: f64 = 4.0;

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
    last_beat: i64,
    gf_danced: bool,
}

impl Default for CharacterAnimState {
    fn default() -> Self {
        Self {
            girlfriend_pose: "danceRight",
            opponent_pose: "idle",
            player_pose: "idle",
            girlfriend_started: Samples(0),
            opponent_started: Samples(0),
            player_started: Samples(0),
            opponent_until: Samples(0),
            player_until: Samples(0),
            last_beat: -1,
            gf_danced: false,
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

    pub fn update(
        &mut self,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        player_holding: bool,
        bopeebo_hey: bool,
    ) {
        self.update_beat_dances(cursor, sample_rate, bpm, bopeebo_hey);
        if self.opponent_pose.starts_with("sing") && cursor >= self.opponent_until {
            self.opponent_pose = "idle";
            self.opponent_started = cursor;
        }
        if self.player_pose.starts_with("sing")
            && !self.player_pose.ends_with("miss")
            && !player_holding
            && cursor >= self.player_until
        {
            self.player_pose = "idle";
            self.player_started = cursor;
        }
    }

    pub fn opponent_note_hit(&mut self, lane: Lane, cursor: Samples, sample_rate: u32, bpm: f64) {
        // ref: 50fccded:source/PlayState.hx:1538-1549
        self.opponent_pose = sing_pose(lane);
        self.opponent_started = cursor;
        self.opponent_until = Samples(cursor.0 + hold_samples(sample_rate, bpm, DAD_HOLD_STEPS));
    }

    pub fn player_note_hit(&mut self, lane: Lane, cursor: Samples, sample_rate: u32, bpm: f64) {
        // ref: 50fccded:source/PlayState.hx:2111-2122
        self.player_pose = sing_pose(lane);
        self.player_started = cursor;
        self.player_until = Samples(cursor.0 + hold_samples(sample_rate, bpm, BF_HOLD_STEPS));
    }

    pub fn player_note_miss(&mut self, lane: Lane, cursor: Samples) {
        // ref: 50fccded:source/PlayState.hx:2056-2066
        self.player_pose = miss_pose(lane);
        self.player_started = cursor;
    }

    pub fn player_first_death(&mut self, cursor: Samples) {
        // ref: 50fccded:source/GameOverSubstate.hx:51
        self.player_pose = "firstDeath";
        self.player_started = cursor;
    }

    pub fn player_death_loop(&mut self, cursor: Samples) {
        // ref: 50fccded:source/Boyfriend.hx:36-38
        self.player_pose = "deathLoop";
        self.player_started = cursor;
    }

    fn update_beat_dances(
        &mut self,
        cursor: Samples,
        sample_rate: u32,
        bpm: f64,
        bopeebo_hey: bool,
    ) {
        let beat = beat_index(cursor, sample_rate, bpm);
        if beat <= self.last_beat {
            return;
        }
        self.last_beat = beat;
        // ref: 50fccded:source/PlayState.hx:2296-2298
        if self.girlfriend_pose.starts_with("dance") {
            self.gf_danced = !self.gf_danced;
            self.girlfriend_pose = if self.gf_danced {
                "danceRight"
            } else {
                "danceLeft"
            };
            self.girlfriend_started = cursor;
        }
        // ref: 50fccded:source/PlayState.hx:2300-2308
        if !self.player_pose.starts_with("sing") && !self.player_pose.starts_with("death") {
            self.player_pose = "idle";
            self.player_started = cursor;
        }
        if bopeebo_hey && beat.rem_euclid(8) == 7 && !self.player_pose.starts_with("sing") {
            self.player_pose = "hey";
            self.player_started = cursor;
        }
    }
}

fn hold_samples(sample_rate: u32, bpm: f64, steps: f64) -> i64 {
    let bpm = bpm.max(1.0);
    ((15.0 * f64::from(sample_rate) / bpm) * steps).round() as i64
}

fn beat_index(cursor: Samples, sample_rate: u32, bpm: f64) -> i64 {
    let samples_per_beat = f64::from(sample_rate) * 60.0 / bpm.max(1.0);
    (cursor.0.max(0) as f64 / samples_per_beat).floor() as i64
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opponent_sings_then_returns_to_idle_after_dad_hold_time() {
        let mut state = CharacterAnimState::default();
        state.opponent_note_hit(Lane::Left, Samples(1_000), 48_000, 100.0);

        assert_eq!(state.poses().opponent.name, "singLEFT");
        assert_eq!(state.poses().opponent.started_at, Samples(1_000));
        state.update(Samples(44_919), 48_000, 100.0, false, false);
        assert_eq!(state.poses().opponent.name, "singLEFT");
        state.update(Samples(44_920), 48_000, 100.0, false, false);
        assert_eq!(state.poses().opponent.name, "idle");
        assert_eq!(state.poses().opponent.started_at, Samples(44_920));
    }

    #[test]
    fn player_hold_keeps_sing_pose_until_released() {
        let mut state = CharacterAnimState::default();
        state.player_note_hit(Lane::Down, Samples(0), 48_000, 100.0);

        state.update(Samples(30_000), 48_000, 100.0, true, false);
        assert_eq!(state.poses().player.name, "singDOWN");
        state.update(Samples(30_000), 48_000, 100.0, false, false);
        assert_eq!(state.poses().player.name, "idle");
    }

    #[test]
    fn girlfriend_toggles_dance_on_new_beat() {
        let mut state = CharacterAnimState::default();

        state.update(Samples(0), 48_000, 100.0, false, false);
        assert_eq!(state.poses().girlfriend.name, "danceRight");
        assert_eq!(state.poses().girlfriend.started_at, Samples(0));
        state.update(Samples(28_800), 48_000, 100.0, false, false);
        assert_eq!(state.poses().girlfriend.name, "danceLeft");
        assert_eq!(state.poses().girlfriend.started_at, Samples(28_800));
    }

    #[test]
    fn player_idle_restarts_on_each_beat_when_not_singing() {
        let mut state = CharacterAnimState::default();

        state.update(Samples(0), 48_000, 100.0, false, false);
        state.update(Samples(28_800), 48_000, 100.0, false, false);

        assert_eq!(state.poses().player.name, "idle");
        assert_eq!(state.poses().player.started_at, Samples(28_800));
    }

    #[test]
    fn bopeebo_triggers_bf_hey_on_beat_seven() {
        let mut state = CharacterAnimState::default();

        state.update(Samples(201_600), 48_000, 100.0, false, true);
        assert_eq!(state.poses().player.name, "hey");
        assert_eq!(state.poses().player.started_at, Samples(201_600));

        state.update(Samples(230_400), 48_000, 100.0, false, true);
        assert_eq!(state.poses().player.name, "idle");
        assert_eq!(state.poses().player.started_at, Samples(230_400));
    }

    #[test]
    fn miss_pose_restarts_from_the_miss_cursor() {
        let mut state = CharacterAnimState::default();
        state.player_note_miss(Lane::Right, Samples(1_234));

        assert_eq!(state.poses().player.name, "singRIGHTmiss");
        assert_eq!(state.poses().player.started_at, Samples(1_234));
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
    }
}
