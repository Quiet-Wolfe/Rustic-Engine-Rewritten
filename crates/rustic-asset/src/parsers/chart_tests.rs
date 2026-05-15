use super::*;

const SAMPLE: &str = r#"{
    "song": {
        "song": "Test",
        "bpm": 120.0,
        "speed": 2.0,
        "needsVoices": true,
        "player1": "bf",
        "player2": "dad",
        "stage": "stage",
        "notes": [
            {
                "mustHitSection": true,
                "lengthInSteps": 16,
                "sectionNotes": [[1000.0, 0, 0], [1500.0, 5, 200]],
                "altAnim": false,
                "bpm": 120, "changeBPM": false
            },
            {
                "mustHitSection": false,
                "lengthInSteps": 16,
                "sectionNotes": [[2000.0, 2, 0], [2500.0, 6, 0]],
                "altAnim": false,
                "bpm": 140, "changeBPM": true
            }
        ]
    }
}"#;

const VSLICE_CHART: &str = r#"{
    "version": "2.0.0",
    "scrollSpeed": { "default": 1.1, "normal": 1.3, "hard": 1.6 },
    "events": [
        { "t": 1200.0, "e": "PlayAnimation",
          "v": { "target": "bf", "anim": "hey", "force": true } },
        { "t": 0.0, "e": "FocusCamera", "v": { "char": "1" } },
        { "t": 2500.0, "e": "ZoomCamera", "v": { "zoom": "1.05" } },
        { "t": 3000.0, "e": "ScrollSpeed",
          "v": { "scroll": 1.2, "duration": 4, "strumline": "both", "absolute": false } },
        { "t": 3250.0, "e": "SetHealthIcon",
          "v": { "char": 1, "id": "tankman-bloody", "scale": 1.1,
                 "flipX": true, "isPixel": false, "offsetX": 2, "offsetY": -3 } },
        { "t": 3500.0, "e": "SetCameraBop",
          "v": { "rate": 1, "offset": 0.25, "intensity": 0.6 } }
    ],
    "notes": {
        "normal": [
            { "t": 1000.0, "d": 1 },
            { "t": 500.0, "d": 6, "l": 250.0, "k": "weekend-1-lightcan" }
        ],
        "hard": [
            { "t": 750.0, "d": 3 }
        ]
    }
}"#;

const VSLICE_METADATA: &str = r#"{
    "version": "2.2.4",
    "songName": "Bopeebo",
    "playData": {
        "characters": {
            "player": "bf",
            "opponent": "dad"
        },
        "stage": "mainStage"
    },
    "timeChanges": [{ "t": 0, "b": 0, "bpm": 100 }]
}"#;

#[test]
fn parses_full_chart_and_sorts_notes() {
    let p = ParsedSong::parse(SAMPLE.as_bytes()).unwrap();
    assert_eq!(p.name, "Test");
    assert_eq!(p.chart.bpm, 120.0);
    assert_eq!(p.chart.speed, 2.0);
    assert_eq!(p.chart.player1, "bf");
    assert_eq!(p.chart.girlfriend, "gf");
    assert_eq!(p.chart.stage, "stage");
    assert_eq!(p.chart.sections.len(), 2);
    assert_eq!(p.chart.notes.len(), 4);

    let times: Vec<_> = p.chart.notes.iter().map(|n| n.time_ms).collect();
    assert_eq!(times, vec![1000.0, 1500.0, 2000.0, 2500.0]);
}

#[test]
fn must_hit_section_resolves_owner() {
    let p = ParsedSong::parse(SAMPLE.as_bytes()).unwrap();
    let n0 = p.chart.notes.iter().find(|n| n.time_ms == 1000.0).unwrap();
    let n1 = p.chart.notes.iter().find(|n| n.time_ms == 1500.0).unwrap();
    assert!(n0.is_player);
    assert!(!n1.is_player);

    let n2 = p.chart.notes.iter().find(|n| n.time_ms == 2000.0).unwrap();
    let n3 = p.chart.notes.iter().find(|n| n.time_ms == 2500.0).unwrap();
    assert!(!n2.is_player);
    assert!(n3.is_player);
}

#[test]
fn sustain_preserved() {
    let p = ParsedSong::parse(SAMPLE.as_bytes()).unwrap();
    let n = p.chart.notes.iter().find(|n| n.time_ms == 1500.0).unwrap();
    assert_eq!(n.sustain_ms, 200.0);
}

#[test]
fn section_change_bpm_recorded() {
    let p = ParsedSong::parse(SAMPLE.as_bytes()).unwrap();
    assert_eq!(p.chart.sections[0].bpm_change, None);
    assert_eq!(p.chart.sections[1].bpm_change, Some(140.0));
}

#[test]
fn valid_score_set_after_parse() {
    // ref: 50fccded:source/Song.hx:73 — parseJSONshit unconditionally
    // sets validScore=true on load. Mirror that.
    let p = ParsedSong::parse(SAMPLE.as_bytes()).unwrap();
    assert!(p.chart.valid_score);
}

#[test]
fn accepts_reference_chart_nul_padding() {
    let mut bytes = SAMPLE.as_bytes().to_vec();
    bytes.extend_from_slice(&[0, 0, 0, b'\n']);

    let p = ParsedSong::parse(&bytes).unwrap();
    assert_eq!(p.name, "Test");
}

#[test]
fn missing_optional_song_fields_get_fnf_defaults() {
    // Only song name + bpm + notes; the rest should fall back to
    // FNF defaults from Song.hx (needsVoices=true, bf/dad, speed=1).
    let minimal = r#"{"song":{"song":"x","bpm":100,"notes":[]}}"#;
    let p = ParsedSong::parse(minimal.as_bytes()).unwrap();
    assert!(p.chart.needs_voices);
    assert_eq!(p.chart.player1, "bf");
    assert_eq!(p.chart.player2, "dad");
    assert_eq!(p.chart.girlfriend, "gf");
    assert_eq!(p.chart.stage, "stage");
    assert_eq!(p.chart.speed, 1.0);
}

#[test]
fn rejects_lane_out_of_range() {
    let bad = r#"{"song":{"song":"x","bpm":100,"notes":[
        {"mustHitSection":true,"lengthInSteps":16,
         "sectionNotes":[[100.0,9,0]]}
    ]}}"#;
    assert!(ParsedSong::parse(bad.as_bytes()).is_err());
}

#[test]
fn rejects_short_note_tuple() {
    let bad = r#"{"song":{"song":"x","bpm":100,"notes":[
        {"mustHitSection":true,"lengthInSteps":16,
         "sectionNotes":[[100.0,0]]}
    ]}}"#;
    assert!(ParsedSong::parse(bad.as_bytes()).is_err());
}

#[test]
fn parses_vslice_chart_and_metadata() {
    let p = ParsedSong::parse_vslice(
        VSLICE_CHART.as_bytes(),
        VSLICE_METADATA.as_bytes(),
        "normal",
    )
    .unwrap();

    assert_eq!(p.name, "Bopeebo");
    assert_eq!(p.chart.bpm, 100.0);
    assert_eq!(p.chart.speed, 1.3);
    assert!(p.chart.needs_voices);
    assert_eq!(p.chart.player1, "bf");
    assert_eq!(p.chart.player2, "dad");
    assert_eq!(p.chart.girlfriend, "");
    assert_eq!(p.chart.stage, "mainStage");
    assert_eq!(p.chart.note_style, "funkin");
    assert!(p.chart.valid_score);
    assert!(p.chart.sections.is_empty());
    assert_eq!(p.chart.events.len(), 6);
    assert_eq!(p.chart.events[0].time_ms, 0.0);
    assert_eq!(
        p.chart.events[0].kind,
        ChartEventKind::FocusCamera {
            target: Some(1),
            x: 0.0,
            y: 0.0,
            duration_steps: 4.0,
            ease: "CLASSIC".to_string()
        }
    );
    assert_eq!(
        p.chart.events[1].kind,
        ChartEventKind::PlayAnimation {
            target: "bf".to_string(),
            animation: "hey".to_string(),
            force: true
        }
    );
    assert_eq!(
        p.chart.events[2].kind,
        ChartEventKind::ZoomCamera {
            zoom: 1.05,
            duration_steps: 4.0,
            direct: true,
            ease: "linear".to_string()
        }
    );
    assert_eq!(
        p.chart.events[3].kind,
        ChartEventKind::ScrollSpeed {
            scroll: 1.2,
            duration_steps: 4.0,
            absolute: false,
            strumline: "both".to_string(),
            ease: "linear".to_string()
        }
    );
    assert_eq!(
        p.chart.events[4].kind,
        ChartEventKind::SetHealthIcon {
            target: 1,
            id: "tankman-bloody".to_string(),
            scale: 1.1,
            flip_x: true,
            is_pixel: false,
            offset_x: 2.0,
            offset_y: -3.0
        }
    );
    assert_eq!(
        p.chart.events[5].kind,
        ChartEventKind::SetCameraBop {
            rate: 1.0,
            offset: 0.25,
            intensity: 0.6
        }
    );

    let times: Vec<_> = p.chart.notes.iter().map(|n| n.time_ms).collect();
    assert_eq!(times, vec![500.0, 1000.0]);
    assert_eq!(p.chart.notes[0].raw_lane, 6);
    assert!(!p.chart.notes[0].is_player);
    assert_eq!(p.chart.notes[0].sustain_ms, 250.0);
    assert_eq!(p.chart.notes[0].kind.as_deref(), Some("weekend-1-lightcan"));
    assert_eq!(p.chart.notes[1].raw_lane, 1);
    assert!(p.chart.notes[1].is_player);
    assert_eq!(p.chart.notes[1].kind, None);
}

#[test]
fn vslice_metadata_preserves_note_style() {
    let metadata = VSLICE_METADATA.replace(
        r#""stage": "mainStage""#,
        r#""stage": "school", "noteStyle": "pixel""#,
    );
    let p =
        ParsedSong::parse_vslice(VSLICE_CHART.as_bytes(), metadata.as_bytes(), "normal").unwrap();

    assert_eq!(p.chart.stage, "school");
    assert_eq!(p.chart.note_style, "pixel");
}

#[test]
fn vslice_notes_fall_back_to_normal() {
    let p = ParsedSong::parse_vslice(VSLICE_CHART.as_bytes(), VSLICE_METADATA.as_bytes(), "easy")
        .unwrap();

    assert_eq!(p.chart.speed, 1.1);
    assert_eq!(p.chart.notes.len(), 2);
}

#[test]
fn vslice_rejects_lane_out_of_range() {
    let bad = r#"{
        "scrollSpeed": { "normal": 1.0 },
        "notes": { "normal": [{ "t": 100.0, "d": 8 }] }
    }"#;

    assert!(
        ParsedSong::parse_vslice(bad.as_bytes(), VSLICE_METADATA.as_bytes(), "normal").is_err()
    );
}
