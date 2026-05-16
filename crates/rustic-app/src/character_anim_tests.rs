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
fn note_kind_hit_overrides_use_scripted_character_poses() {
    let mut state = CharacterAnimState::default();

    state.opponent_note_hit_kind(Lane::Up, Some(NoteKind::Ugh), Samples(1_000), 48_000, 100.0);
    assert_eq!(state.poses().opponent.name, "ugh");

    state.opponent_note_hit_kind(
        Lane::Left,
        Some(NoteKind::Mom),
        Samples(2_000),
        48_000,
        100.0,
    );
    assert_eq!(state.poses().opponent.name, "singLEFT-alt");

    state.player_note_hit_kind(
        Lane::Right,
        Some(NoteKind::Censor),
        Samples(3_000),
        48_000,
        100.0,
    );
    assert_eq!(state.poses().player.name, "singRIGHT-censor");
}

#[test]
fn noanim_hit_preserves_current_pose() {
    let mut state = CharacterAnimState::default();

    state.player_note_hit_kind(
        Lane::Right,
        Some(NoteKind::NoAnim),
        Samples(3_000),
        48_000,
        100.0,
    );

    assert_eq!(state.poses().player.name, "idle");
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
fn beat_index_rounds_like_fnf_conductor_before_flooring() {
    assert_eq!(beat_index(Samples(1_323_000), 44_100, 74.0), 37);
    assert_eq!(beat_index(Samples(-529_200), 44_100, 65.0), -13);
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
fn stress_pico_chart_events_trigger_scripted_poses() {
    let mut state = CharacterAnimState::default();

    assert!(state.play_chart_animation("dad", "redheadsAnim", Samples(1_000), true));
    assert!(state.play_chart_animation("boyfriend", "knifeToss", Samples(2_000), true));

    assert_eq!(state.poses().opponent.name, "redheadsAnim");
    assert_eq!(state.poses().opponent.started_at, Samples(1_000));
    assert_eq!(state.poses().player.name, "knifeToss");
    assert_eq!(state.poses().player.started_at, Samples(2_000));
}

#[test]
fn stress_pico_end_cutscene_poses_are_allowed() {
    let mut state = CharacterAnimState::default();

    assert!(state.play_chart_animation("dad", "stressPicoEnding", Samples(1_000), true));
    assert!(state.play_chart_animation("boyfriend", "laughEnd", Samples(2_000), true));
    assert!(state.play_chart_animation("boyfriend", "laughEnd-loop", Samples(3_000), true));

    assert_eq!(state.poses().opponent.name, "stressPicoEnding");
    assert_eq!(state.poses().player.name, "laughEnd-loop");
    assert_eq!(state.poses().player.started_at, Samples(3_000));
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
