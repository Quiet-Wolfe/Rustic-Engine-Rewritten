//! FNF chart JSON parser.
// LINT-ALLOW: long-file source-referenced chart tests stay beside lane parsing.
//!
//! Runtime chart loading now targets Funkin' v0.8.5 v-slice chart/metadata
//! pairs through `ParsedSong::parse_vslice`. The older SwagSong parser remains
//! here as transitional compatibility for original prototype assets and tests;
//! do not treat its `50fccded` citations as current fidelity proof.
//!
//! ref: 50fccded:source/Song.hx:10-21      // SwagSong typedef
//! ref: 50fccded:source/Section.hx:3-12    // SwagSection typedef
//! ref: 50fccded:source/Song.hx:70-75      // parseJSONshit / validScore=true
//! ref: 50fccded:source/PlayState.hx:1014-1018  // mustHit lane-flip rule
//!
//! Legacy SwagSong wire format (from `parseJSONshit`):
//!
//! ```json
//! {
//!   "song": {
//!     "song": "Bopeebo",
//!     "bpm": 100,
//!     "speed": 1.0,
//!     "needsVoices": true,
//!     "player1": "bf",
//!     "player2": "dad",
//!     "notes": [
//!       { "mustHitSection": true,
//!         "lengthInSteps": 16,
//!         "typeOfSection": 0,
//!         "sectionNotes": [[2400.0, 0, 0]],
//!         "altAnim": false,
//!         "bpm": 100, "changeBPM": false }
//!     ]
//!   }
//! }
//! ```
//!
//! Lanes:
//!   0..3  -> "owner" lanes for the section.
//!   4..7  -> "non-owner" lanes for the section.
//!
//! Lane-ownership rule from PlayState.hx:1014-1018 — start with
//! `gottaHitNote = mustHitSection`, and flip when `songNotes[1] > 3`.
//! The parser preserves the raw lane index and exposes a derived
//! `is_player` boolean.
//!
//! Stage is NOT in `SwagSong` at this baseline. PlayState picks the
//! stage from the song name (`PlayState.hx:227`), so chart parsing
//! does not produce one. The owning gameplay/screen layer should
//! resolve stage from song name.

use crate::error::{AssetError, AssetResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[path = "chart_events.rs"]
mod chart_events;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OuterSong {
    song: RawSong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSong {
    song: String,
    bpm: f64,
    #[serde(default = "default_speed")]
    speed: f64,
    #[serde(default = "default_needs_voices")]
    needs_voices: bool,
    #[serde(default = "default_player1")]
    player1: String,
    #[serde(default = "default_player2")]
    player2: String,
    #[serde(default)]
    valid_score: bool,
    notes: Vec<RawSection>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VSliceChart {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:615-619
    #[serde(default)]
    scroll_speed: HashMap<String, f64>,
    #[serde(default)]
    events: Vec<VSliceEvent>,
    #[serde(default)]
    notes: HashMap<String, Vec<VSliceNote>>,
}

#[derive(Debug, Clone, Deserialize)]
struct VSliceEvent {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:981-1013
    #[serde(rename = "t")]
    time_ms: f64,
    #[serde(rename = "e")]
    name: String,
    #[serde(default, rename = "v")]
    value: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct VSliceNote {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:1093-1123
    #[serde(rename = "t")]
    time_ms: f64,
    #[serde(rename = "d")]
    data: i32,
    #[serde(default, rename = "l")]
    length_ms: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VSliceMetadata {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:19-64
    #[serde(default = "default_unknown")]
    song_name: String,
    #[serde(default)]
    play_data: VSlicePlayData,
    #[serde(default)]
    time_changes: Vec<VSliceTimeChange>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct VSlicePlayData {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:457-465
    #[serde(default)]
    characters: VSliceCharacters,
}

#[derive(Debug, Clone, Deserialize)]
struct VSliceCharacters {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:548-560
    #[serde(default = "default_player1")]
    player: String,
    #[serde(default = "default_player2")]
    opponent: String,
}

impl Default for VSliceCharacters {
    fn default() -> Self {
        Self {
            player: default_player1(),
            opponent: default_player2(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct VSliceTimeChange {
    bpm: f64,
}

fn default_speed() -> f64 {
    1.0
}

// ref: 50fccded:source/Song.hx:28 — `needsVoices` defaults to true.
fn default_needs_voices() -> bool {
    true
}

// ref: 50fccded:source/Song.hx:31-32 — player defaults `bf` / `dad`.
fn default_player1() -> String {
    "bf".to_string()
}

fn default_player2() -> String {
    "dad".to_string()
}

fn default_unknown() -> String {
    "Unknown".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSection {
    // ref: 50fccded:source/Section.hx:20 — defaults to true.
    #[serde(default = "default_must_hit_section")]
    must_hit_section: bool,
    // ref: 50fccded:source/Section.hx:18 — defaults to 16.
    #[serde(default = "default_length_in_steps")]
    length_in_steps: u32,
    // ref: 50fccded:source/Section.hx:19 — defaults to 0.
    #[serde(default)]
    type_of_section: u32,
    #[serde(default)]
    section_notes: Vec<RawNote>,
    #[serde(default)]
    alt_anim: bool,
    #[serde(default)]
    bpm: Option<f64>,
    /// FNF spells this `changeBPM` (full caps), which `rename_all =
    /// "camelCase"` would map to `changeBpm`. Override explicitly.
    #[serde(default, rename = "changeBPM")]
    change_bpm: bool,
}

fn default_must_hit_section() -> bool {
    true
}

fn default_length_in_steps() -> u32 {
    16
}

/// `[time_ms, lane, sustain_ms]` plus optional trailing fields some
/// charts include but we don't use yet (e.g. note type). `serde_json`
/// drops the extras when reading into a tuple, so we read into a
/// `Vec<f64>` and validate length.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawNote(Vec<serde_json::Value>);

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ChartNote {
    /// Absolute time in milliseconds from song start.
    pub time_ms: f64,
    /// Sustain length in milliseconds (0 for tap).
    pub sustain_ms: f64,
    /// Raw lane index 0..7 as it appeared in the chart.
    pub raw_lane: u8,
    /// Resolved owner: `true` = player (BF), `false` = opponent.
    pub is_player: bool,
    /// Section index for diagnostics and rewind.
    pub section_index: u32,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ChartSection {
    pub index: u32,
    pub must_hit_section: bool,
    pub length_in_steps: u32,
    pub type_of_section: u32,
    pub bpm_change: Option<f64>,
    pub alt_anim: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChartEvent {
    pub time_ms: f64,
    pub kind: ChartEventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChartEventKind {
    FocusCamera {
        target: Option<i8>,
        x: f32,
        y: f32,
    },
    PlayAnimation {
        target: String,
        animation: String,
        force: bool,
    },
    ZoomCamera {
        zoom: f32,
        duration_steps: f32,
        direct: bool,
        ease: String,
    },
    ScrollSpeed {
        scroll: f32,
        duration_steps: f32,
        absolute: bool,
        strumline: String,
        ease: String,
    },
    SetCameraBop {
        rate: f32,
        offset: f32,
        intensity: f32,
    },
    Unknown {
        name: String,
    },
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Chart {
    pub bpm: f64,
    pub speed: f64,
    pub needs_voices: bool,
    pub player1: String,
    pub player2: String,
    /// Mirrors `parseJSONshit` setting `validScore = true` after
    /// load.
    /// ref: 50fccded:source/Song.hx:70-75
    pub valid_score: bool,
    pub sections: Vec<ChartSection>,
    pub events: Vec<ChartEvent>,
    pub notes: Vec<ChartNote>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ParsedSong {
    pub name: String,
    pub chart: Chart,
}

impl ParsedSong {
    pub fn parse(bytes: &[u8]) -> AssetResult<Self> {
        let outer: OuterSong = serde_json::from_slice(trim_chart_bytes(bytes))
            .map_err(|e| AssetError::InvalidPath(format!("chart json: {e}")))?;
        let raw = outer.song;
        let mut chart = Chart {
            bpm: raw.bpm,
            speed: raw.speed,
            needs_voices: raw.needs_voices,
            player1: raw.player1,
            player2: raw.player2,
            // ref: 50fccded:source/Song.hx:73 — parseJSONshit sets
            // validScore=true unconditionally after load.
            valid_score: true,
            sections: Vec::with_capacity(raw.notes.len()),
            events: Vec::new(),
            notes: Vec::new(),
        };

        for (i, sec) in raw.notes.into_iter().enumerate() {
            let bpm_change = if sec.change_bpm { sec.bpm } else { None };
            chart.sections.push(ChartSection {
                index: i as u32,
                must_hit_section: sec.must_hit_section,
                length_in_steps: sec.length_in_steps,
                type_of_section: sec.type_of_section,
                bpm_change,
                alt_anim: sec.alt_anim,
            });
            for n in sec.section_notes {
                let parsed = parse_note(i as u32, sec.must_hit_section, &n)?;
                chart.notes.push(parsed);
            }
        }

        // Stable sort by time so gameplay can step through linearly.
        chart.notes.sort_by(|a, b| {
            a.time_ms
                .partial_cmp(&b.time_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(Self {
            name: raw.song,
            chart,
        })
    }

    pub fn parse_vslice(
        chart_bytes: &[u8],
        metadata_bytes: &[u8],
        difficulty: &str,
    ) -> AssetResult<Self> {
        let raw_chart: VSliceChart = serde_json::from_slice(trim_chart_bytes(chart_bytes))
            .map_err(|e| AssetError::InvalidData(format!("v-slice chart json: {e}")))?;
        let metadata: VSliceMetadata = serde_json::from_slice(trim_chart_bytes(metadata_bytes))
            .map_err(|e| AssetError::InvalidData(format!("v-slice metadata json: {e}")))?;

        let mut chart = Chart {
            bpm: v_slice_bpm(&metadata),
            speed: v_slice_scroll_speed(&raw_chart, difficulty),
            needs_voices: true,
            player1: metadata.play_data.characters.player,
            player2: metadata.play_data.characters.opponent,
            valid_score: true,
            sections: Vec::new(),
            events: chart_events::parse_vslice_events(&raw_chart),
            notes: Vec::new(),
        };

        // ref: bdedc0aa:source/funkin/data/song/SongData.hx:655-662
        if let Some(notes) = v_slice_notes_for(&raw_chart, difficulty) {
            chart.notes.reserve(notes.len());
            for note in notes {
                chart.notes.push(parse_vslice_note(note)?);
            }
        }

        chart.notes.sort_by(|a, b| {
            a.time_ms
                .partial_cmp(&b.time_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(Self {
            name: metadata.song_name,
            chart,
        })
    }
}

fn trim_chart_bytes(mut bytes: &[u8]) -> &[u8] {
    while let Some((&last, rest)) = bytes.split_last() {
        if last == 0 || last.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
}

fn parse_note(section_index: u32, must_hit: bool, n: &RawNote) -> AssetResult<ChartNote> {
    if n.0.len() < 3 {
        return Err(AssetError::InvalidPath(format!(
            "section {section_index} note has {} fields (need at least 3)",
            n.0.len()
        )));
    }
    let time_ms = as_f64(&n.0[0]).ok_or_else(|| invalid("note[0] not number"))?;
    let lane_f = as_f64(&n.0[1]).ok_or_else(|| invalid("note[1] not number"))?;
    let sustain_ms = as_f64(&n.0[2]).unwrap_or(0.0).max(0.0);

    if !(0.0..=7.0).contains(&lane_f) {
        return Err(invalid(&format!("lane {lane_f} out of 0..=7")));
    }
    let raw_lane = lane_f as u8;
    // FNF flip rule: notes with raw lane 0..3 belong to whoever owns
    // the section ("must_hit_section"); 4..7 belong to the other side.
    let is_owner_lane = raw_lane < 4;
    let is_player = if must_hit {
        is_owner_lane
    } else {
        !is_owner_lane
    };

    Ok(ChartNote {
        time_ms,
        sustain_ms,
        raw_lane,
        is_player,
        section_index,
    })
}

fn parse_vslice_note(note: &VSliceNote) -> AssetResult<ChartNote> {
    if !(0..=7).contains(&note.data) {
        return Err(invalid(&format!("v-slice lane {} out of 0..=7", note.data)));
    }
    let raw_lane = note.data as u8;

    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:1107-1112
    // ref: bdedc0aa:source/funkin/play/PlayState.hx:2499-2512
    Ok(ChartNote {
        time_ms: note.time_ms,
        sustain_ms: note.length_ms.max(0.0),
        raw_lane,
        is_player: raw_lane < 4,
        section_index: 0,
    })
}

fn v_slice_notes_for<'a>(chart: &'a VSliceChart, difficulty: &str) -> Option<&'a Vec<VSliceNote>> {
    chart.notes.get(difficulty).or_else(|| {
        if difficulty != "normal" {
            chart.notes.get("normal")
        } else {
            None
        }
    })
}

fn v_slice_scroll_speed(chart: &VSliceChart, difficulty: &str) -> f64 {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:640-647
    let result = chart
        .scroll_speed
        .get(difficulty)
        .copied()
        .unwrap_or_default();
    if result == 0.0 && difficulty != "default" {
        return chart
            .scroll_speed
            .get("default")
            .copied()
            .filter(|speed| *speed != 0.0)
            .unwrap_or(1.0);
    }
    if result == 0.0 {
        1.0
    } else {
        result
    }
}

fn v_slice_bpm(metadata: &VSliceMetadata) -> f64 {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:72-88
    metadata
        .time_changes
        .first()
        .map(|change| change.bpm)
        .unwrap_or(100.0)
}

fn as_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
}

fn invalid(msg: &str) -> AssetError {
    AssetError::InvalidPath(format!("chart: {msg}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
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
            { "t": 0.0, "e": "FocusCamera", "v": { "char": 1 } },
            { "t": 2500.0, "e": "ZoomCamera", "v": { "zoom": 1.05 } },
            { "t": 3000.0, "e": "ScrollSpeed",
              "v": { "scroll": 1.2, "duration": 4, "strumline": "both", "absolute": false } },
            { "t": 3500.0, "e": "SetCameraBop",
              "v": { "rate": 1, "offset": 0.25, "intensity": 0.6 } }
        ],
        "notes": {
            "normal": [
                { "t": 1000.0, "d": 1 },
                { "t": 500.0, "d": 6, "l": 250.0 }
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
            }
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
        assert_eq!(p.chart.sections.len(), 2);
        assert_eq!(p.chart.notes.len(), 4);

        // Sorted by time:
        let times: Vec<_> = p.chart.notes.iter().map(|n| n.time_ms).collect();
        assert_eq!(times, vec![1000.0, 1500.0, 2000.0, 2500.0]);
    }

    #[test]
    fn must_hit_section_resolves_owner() {
        let p = ParsedSong::parse(SAMPLE.as_bytes()).unwrap();
        // Section 0 is mustHit=true.
        // Note (1000.0, lane=0)  -> owner lane (0..3) under mustHit -> player.
        // Note (1500.0, lane=5)  -> non-owner lane (4..7) under mustHit -> opponent.
        let n0 = p.chart.notes.iter().find(|n| n.time_ms == 1000.0).unwrap();
        let n1 = p.chart.notes.iter().find(|n| n.time_ms == 1500.0).unwrap();
        assert!(n0.is_player);
        assert!(!n1.is_player);

        // Section 1 is mustHit=false (flipped).
        // (2000.0, lane=2) -> 0..3 lane under mustHit=false -> opponent.
        // (2500.0, lane=6) -> 4..7 lane under mustHit=false -> player.
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
        assert!(p.chart.valid_score);
        assert!(p.chart.sections.is_empty());
        assert_eq!(p.chart.events.len(), 5);
        assert_eq!(p.chart.events[0].time_ms, 0.0);
        assert_eq!(
            p.chart.events[0].kind,
            ChartEventKind::FocusCamera {
                target: Some(1),
                x: 0.0,
                y: 0.0
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
        assert_eq!(p.chart.notes[1].raw_lane, 1);
        assert!(p.chart.notes[1].is_player);
    }

    #[test]
    fn vslice_notes_fall_back_to_normal() {
        let p =
            ParsedSong::parse_vslice(VSLICE_CHART.as_bytes(), VSLICE_METADATA.as_bytes(), "easy")
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
}
