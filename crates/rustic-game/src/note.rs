//! Note and lane types, plus chart → gameplay conversion.
//!
//! Lanes use the four base-FNF directions. Sustain length is in mixer
//! samples so judgment math stays in audio-domain time.
//!
//! ref: bdedc0aa:source/funkin/play/notes/Strumline.hx:575-604
//! ref: bdedc0aa:source/funkin/play/notes/Strumline.hx:1192-1225

use rustic_asset::ChartNote;
use rustic_core::ids::NoteId;
use rustic_core::time::Samples;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Lane {
    Left = 0,
    Down = 1,
    Up = 2,
    Right = 3,
}

impl Lane {
    /// Lane is the chart note's `data % strumlineSize`.
    /// ref: bdedc0aa:source/funkin/data/song/SongData.hx:1181-1185
    pub fn from_raw(raw: u8) -> Lane {
        match raw % 4 {
            0 => Lane::Left,
            1 => Lane::Down,
            2 => Lane::Up,
            _ => Lane::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NoteKind {
    NoAnim,
    Mom,
    Ugh,
    HehPrettyGood,
    Censor,
    Weekend1LightCan,
    Weekend1KneeCan,
    Weekend1KickCan,
    Weekend1CockGun,
    Weekend1FireGun,
    Weekend1PunchHigh,
    Weekend1PunchHighDodged,
    Weekend1PunchHighBlocked,
    Weekend1PunchHighSpin,
    Weekend1PunchLow,
    Weekend1PunchLowDodged,
    Weekend1PunchLowBlocked,
    Weekend1PunchLowSpin,
    Weekend1PicoUppercutPrep,
    Weekend1PicoUppercut,
    Weekend1BlockHigh,
    Weekend1BlockLow,
    Weekend1BlockSpin,
    Weekend1DodgeHigh,
    Weekend1DodgeLow,
    Weekend1DodgeSpin,
    Weekend1HitHigh,
    Weekend1HitLow,
    Weekend1HitSpin,
    Weekend1DarnellUppercutPrep,
    Weekend1DarnellUppercut,
    Weekend1Idle,
    Weekend1Fakeout,
    Weekend1Taunt,
    Weekend1TauntForce,
    Weekend1ReverseFakeout,
    SakuraJoint,
    SakuraBf1,
    SakuraBf2,
    NonScoreable,
}

impl NoteKind {
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "noanim" => Self::NoAnim,
            "mom" => Self::Mom,
            "ugh" => Self::Ugh,
            "hehPrettyGood" => Self::HehPrettyGood,
            "censor" => Self::Censor,
            "weekend-1-lightcan" => Self::Weekend1LightCan,
            "weekend-1-kneecan" => Self::Weekend1KneeCan,
            "weekend-1-kickcan" => Self::Weekend1KickCan,
            "weekend-1-cockgun" => Self::Weekend1CockGun,
            "weekend-1-firegun" => Self::Weekend1FireGun,
            "weekend-1-punchhigh" => Self::Weekend1PunchHigh,
            "weekend-1-punchhighdodged" => Self::Weekend1PunchHighDodged,
            "weekend-1-punchhighblocked" => Self::Weekend1PunchHighBlocked,
            "weekend-1-punchhighspin" => Self::Weekend1PunchHighSpin,
            "weekend-1-punchlow" => Self::Weekend1PunchLow,
            "weekend-1-punchlowdodged" => Self::Weekend1PunchLowDodged,
            "weekend-1-punchlowblocked" => Self::Weekend1PunchLowBlocked,
            "weekend-1-punchlowspin" => Self::Weekend1PunchLowSpin,
            "weekend-1-picouppercutprep" => Self::Weekend1PicoUppercutPrep,
            "weekend-1-picouppercut" => Self::Weekend1PicoUppercut,
            "weekend-1-blockhigh" => Self::Weekend1BlockHigh,
            "weekend-1-blocklow" => Self::Weekend1BlockLow,
            "weekend-1-blockspin" => Self::Weekend1BlockSpin,
            "weekend-1-dodgehigh" => Self::Weekend1DodgeHigh,
            "weekend-1-dodgelow" => Self::Weekend1DodgeLow,
            "weekend-1-dodgespin" => Self::Weekend1DodgeSpin,
            "weekend-1-hithigh" => Self::Weekend1HitHigh,
            "weekend-1-hitlow" => Self::Weekend1HitLow,
            "weekend-1-hitspin" => Self::Weekend1HitSpin,
            "weekend-1-darnelluppercutprep" => Self::Weekend1DarnellUppercutPrep,
            "weekend-1-darnelluppercut" => Self::Weekend1DarnellUppercut,
            "weekend-1-idle" => Self::Weekend1Idle,
            "weekend-1-fakeout" => Self::Weekend1Fakeout,
            "weekend-1-taunt" => Self::Weekend1Taunt,
            "weekend-1-tauntforce" => Self::Weekend1TauntForce,
            "weekend-1-reversefakeout" => Self::Weekend1ReverseFakeout,
            "sakura-joint" => Self::SakuraJoint,
            "sakura-bf1" => Self::SakuraBf1,
            "sakura-bf2" => Self::SakuraBf2,
            "non_scoreable" => Self::NonScoreable,
            _ => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::NoAnim => "noanim",
            Self::Mom => "mom",
            Self::Ugh => "ugh",
            Self::HehPrettyGood => "hehPrettyGood",
            Self::Censor => "censor",
            Self::Weekend1LightCan => "weekend-1-lightcan",
            Self::Weekend1KneeCan => "weekend-1-kneecan",
            Self::Weekend1KickCan => "weekend-1-kickcan",
            Self::Weekend1CockGun => "weekend-1-cockgun",
            Self::Weekend1FireGun => "weekend-1-firegun",
            Self::Weekend1PunchHigh => "weekend-1-punchhigh",
            Self::Weekend1PunchHighDodged => "weekend-1-punchhighdodged",
            Self::Weekend1PunchHighBlocked => "weekend-1-punchhighblocked",
            Self::Weekend1PunchHighSpin => "weekend-1-punchhighspin",
            Self::Weekend1PunchLow => "weekend-1-punchlow",
            Self::Weekend1PunchLowDodged => "weekend-1-punchlowdodged",
            Self::Weekend1PunchLowBlocked => "weekend-1-punchlowblocked",
            Self::Weekend1PunchLowSpin => "weekend-1-punchlowspin",
            Self::Weekend1PicoUppercutPrep => "weekend-1-picouppercutprep",
            Self::Weekend1PicoUppercut => "weekend-1-picouppercut",
            Self::Weekend1BlockHigh => "weekend-1-blockhigh",
            Self::Weekend1BlockLow => "weekend-1-blocklow",
            Self::Weekend1BlockSpin => "weekend-1-blockspin",
            Self::Weekend1DodgeHigh => "weekend-1-dodgehigh",
            Self::Weekend1DodgeLow => "weekend-1-dodgelow",
            Self::Weekend1DodgeSpin => "weekend-1-dodgespin",
            Self::Weekend1HitHigh => "weekend-1-hithigh",
            Self::Weekend1HitLow => "weekend-1-hitlow",
            Self::Weekend1HitSpin => "weekend-1-hitspin",
            Self::Weekend1DarnellUppercutPrep => "weekend-1-darnelluppercutprep",
            Self::Weekend1DarnellUppercut => "weekend-1-darnelluppercut",
            Self::Weekend1Idle => "weekend-1-idle",
            Self::Weekend1Fakeout => "weekend-1-fakeout",
            Self::Weekend1Taunt => "weekend-1-taunt",
            Self::Weekend1TauntForce => "weekend-1-tauntforce",
            Self::Weekend1ReverseFakeout => "weekend-1-reversefakeout",
            Self::SakuraJoint => "sakura-joint",
            Self::SakuraBf1 => "sakura-bf1",
            Self::SakuraBf2 => "sakura-bf2",
            Self::NonScoreable => "non_scoreable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Note {
    pub id: NoteId,
    pub lane: Lane,
    /// Time at which the note should be hit, in mixer samples relative to
    /// the song origin.
    pub hit_at: Samples,
    /// Sustain duration in samples. Zero for tap notes.
    pub sustain_samples: i64,
    /// True for legacy/bookkeeping sustain child notes.
    pub is_sustain: bool,
    /// True for the final legacy/bookkeeping child in a sustain chain.
    pub is_sustain_end: bool,
    /// Whose side the note belongs to. `false` = player, `true` = opponent.
    pub opponent: bool,
    /// Base v-slice note kind behavior used by scripted Week 5/7/Weekend 1
    /// charts. Unknown mod note kinds are ignored by v1 core gameplay.
    #[serde(default)]
    pub kind: Option<NoteKind>,
}

/// Convert a chart's notes into gameplay `Note`s, materialising sample
/// positions from the chart's millisecond domain. v0.8.5 keeps holds as
/// note heads with a sustain length and renders the trail separately.
pub fn notes_from_chart<'a>(
    chart_notes: impl IntoIterator<Item = &'a ChartNote>,
    sample_rate: u32,
    _bpm: f64,
) -> Vec<Note> {
    let scale = sample_rate as f64 / 1000.0;
    let mut notes: Vec<_> = chart_notes
        .into_iter()
        .map(|n| Note {
            id: NoteId::new(0),
            lane: Lane::from_raw(n.raw_lane),
            hit_at: Samples((n.time_ms * scale).round() as i64),
            sustain_samples: (n.sustain_ms * scale).round() as i64,
            is_sustain: false,
            is_sustain_end: false,
            opponent: !n.is_player,
            kind: n.kind.as_deref().and_then(NoteKind::from_id),
        })
        .collect();

    notes.sort_by_key(|note| note.hit_at);
    for (i, note) in notes.iter_mut().enumerate() {
        note.id = NoteId::new(i as u32);
    }
    notes
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rustic_asset::ParsedSong;

    #[test]
    fn lane_is_raw_modulo_four() {
        assert_eq!(Lane::from_raw(0), Lane::Left);
        assert_eq!(Lane::from_raw(1), Lane::Down);
        assert_eq!(Lane::from_raw(2), Lane::Up);
        assert_eq!(Lane::from_raw(3), Lane::Right);
        assert_eq!(Lane::from_raw(4), Lane::Left);
        assert_eq!(Lane::from_raw(7), Lane::Right);
    }

    #[test]
    fn chart_to_notes_converts_ms_to_samples() {
        // 1000ms @ 48kHz → 48000 samples.
        const CHART: &str = r#"{
            "song": {
                "song": "X",
                "bpm": 100.0,
                "notes": [
                    {"mustHitSection": true, "lengthInSteps": 16,
                     "sectionNotes": [[1000.0, 0, 0], [2000.0, 5, 250]]}
                ]
            }
        }"#;
        let parsed = ParsedSong::parse(CHART.as_bytes()).unwrap();
        let notes = notes_from_chart(parsed.chart.notes.iter(), 48_000, parsed.chart.bpm);
        assert_eq!(notes.len(), 2);

        // First note: 1000ms, lane 0 (player owner because mustHit=true).
        assert_eq!(notes[0].lane, Lane::Left);
        assert_eq!(notes[0].hit_at, Samples(48_000));
        assert_eq!(notes[0].sustain_samples, 0);
        assert!(!notes[0].is_sustain);
        assert!(!notes[0].is_sustain_end);
        assert!(!notes[0].opponent);

        // Second note: 2000ms, raw lane 5 → mod 4 = Down,
        // is_player=false (mustHit=true & lane>3 → opponent).
        assert_eq!(notes[1].lane, Lane::Down);
        assert_eq!(notes[1].hit_at, Samples(96_000));
        assert_eq!(notes[1].sustain_samples, 12_000);
        assert!(!notes[1].is_sustain);
        assert!(!notes[1].is_sustain_end);
        assert!(notes[1].opponent);
        assert_eq!(notes[1].kind, None);

        // v0.8.5 keeps the hold as head + length; it does not
        // materialize per-step sustain children.
        assert_eq!(notes.iter().filter(|note| note.is_sustain).count(), 0);
    }

    #[test]
    fn chart_to_notes_preserves_known_note_kinds() {
        const CHART: &str = r#"{
            "scrollSpeed": { "normal": 1.0 },
            "notes": { "normal": [
                { "t": 1000.0, "d": 6, "k": "ugh" },
                { "t": 2000.0, "d": 0, "k": "made-up" }
            ] }
        }"#;
        const METADATA: &str = r#"{
            "songName": "Ugh",
            "timeChanges": [{ "bpm": 100 }]
        }"#;
        let parsed =
            ParsedSong::parse_vslice(CHART.as_bytes(), METADATA.as_bytes(), "normal").unwrap();
        let notes = notes_from_chart(parsed.chart.notes.iter(), 48_000, parsed.chart.bpm);

        assert_eq!(notes[0].kind, Some(NoteKind::Ugh));
        assert_eq!(notes[1].kind, None);
    }
}
