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
use crate::parsers::stage::stage_id_for_song_name;
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
    #[serde(default, rename = "k")]
    kind: Option<String>,
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
#[serde(rename_all = "camelCase")]
struct VSlicePlayData {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:457-465
    #[serde(default)]
    characters: VSliceCharacters,
    #[serde(default = "default_vslice_stage")]
    stage: String,
    #[serde(default = "default_note_style")]
    note_style: String,
}

#[derive(Debug, Clone, Deserialize)]
struct VSliceCharacters {
    // ref: bdedc0aa:source/funkin/data/song/SongData.hx:548-560
    #[serde(default = "default_player1")]
    player: String,
    #[serde(default = "default_player2")]
    opponent: String,
    #[serde(default)]
    girlfriend: String,
}

impl Default for VSliceCharacters {
    fn default() -> Self {
        Self {
            player: default_player1(),
            opponent: default_player2(),
            girlfriend: String::new(),
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

fn default_vslice_stage() -> String {
    "mainStage".to_string()
}

fn default_note_style() -> String {
    "funkin".to_string()
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
    /// v-slice note kind id, such as Weekend 1 combat notes.
    pub kind: Option<String>,
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
        duration_steps: f32,
        ease: String,
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
    SetHealthIcon {
        target: i8,
        id: String,
        scale: f32,
        flip_x: bool,
        is_pixel: bool,
        offset_x: f32,
        offset_y: f32,
    },
    Sserafim(SserafimEvent),
    Unknown {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SserafimEvent {
    Show {
        visible: Vec<bool>,
    },
    Sing {
        singing: Vec<bool>,
    },
    Dark {
        amount: f32,
        duration: f32,
    },
    Lights {
        amount: f32,
        duration: f32,
    },
    PulseLights {
        enabled: bool,
        colors: Vec<String>,
        durations: Vec<f32>,
        intensities: Vec<f32>,
    },
    Cover {
        visible: bool,
    },
    Flash {
        duration: f32,
    },
    Kick {
        final_kick: bool,
    },
    Beautiful {
        beautiful: bool,
    },
    GuitarVibration {
        duration: f32,
    },
    End,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Chart {
    pub bpm: f64,
    pub speed: f64,
    pub needs_voices: bool,
    pub player1: String,
    pub player2: String,
    pub girlfriend: String,
    pub stage: String,
    pub note_style: String,
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
            girlfriend: "gf".to_string(),
            stage: stage_id_for_song_name(&raw.song).to_string(),
            note_style: default_note_style(),
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
            girlfriend: metadata.play_data.characters.girlfriend,
            stage: metadata.play_data.stage,
            note_style: metadata.play_data.note_style,
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
        kind: None,
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
        kind: note.kind.clone(),
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
#[path = "chart_tests.rs"]
mod tests;
