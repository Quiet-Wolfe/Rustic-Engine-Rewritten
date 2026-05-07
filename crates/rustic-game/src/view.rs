//! Headless gameplay view models for notes.
//!
//! ref: bdedc0aa:source/funkin/play/notes/Strumline.hx:35-45,741-745,1172-1218
//! ref: bdedc0aa:source/funkin/play/PlayState.hx:2233-2252
//! ref: bdedc0aa:source/funkin/util/Constants.hx:347,632-637

use crate::note::Lane;
use crate::state::PlayState;
use rustic_core::ids::NoteId;
use rustic_core::time::Samples;

pub const FNF_WIDTH: f32 = 1280.0;
pub const FNF_HEIGHT: f32 = 720.0;
pub const STRUM_LINE_Y: f32 = 24.0;
pub const STRUMLINE_SIZE: f32 = 104.0;
pub const NOTE_SWAG_WIDTH: f32 = 104.0 + 8.0;
pub const NOTE_BASE_X: f32 = 48.0;
pub const NOTE_SPAWN_LEAD_MS: f32 = 1500.0;
pub const NOTE_SCROLL_FACTOR: f32 = 0.45;
const INITIAL_OFFSET: f32 = -0.275 * STRUMLINE_SIZE;

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct NoteView {
    pub id: NoteId,
    pub lane: Lane,
    pub opponent: bool,
    pub is_sustain: bool,
    pub is_sustain_end: bool,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct HoldTrailView {
    pub id: NoteId,
    pub lane: Lane,
    pub opponent: bool,
    pub head_resolved: bool,
    pub x: f32,
    pub y: f32,
    pub height: f32,
}

impl HoldTrailView {
    pub fn new(
        id: NoteId,
        lane: Lane,
        opponent: bool,
        head_resolved: bool,
        x: f32,
        y: f32,
        height: f32,
    ) -> Self {
        Self {
            id,
            lane,
            opponent,
            head_resolved,
            x,
            y,
            height,
        }
    }
}

impl PlayState {
    /// Visible note sprites at the current audio cursor. This mirrors the
    /// OG spawn window and y-position math while staying renderer-agnostic.
    pub fn note_views(&self, cursor: Samples, sample_rate: u32) -> Vec<NoteView> {
        let sample_rate = sample_rate.max(1) as f32;
        let song_ms = cursor.0 as f32 * 1000.0 / sample_rate;
        let rounded_speed = round_decimal(self.scroll_speed, 2);
        let mut out = Vec::new();

        for note in &self.notes {
            if self.resolved_notes.contains(&note.id) {
                continue;
            }

            let note_ms = note.hit_at.0 as f32 * 1000.0 / sample_rate;
            if note_ms - song_ms >= NOTE_SPAWN_LEAD_MS {
                continue;
            }

            let y = STRUM_LINE_Y - (song_ms - note_ms) * (NOTE_SCROLL_FACTOR * rounded_speed);
            if y > FNF_HEIGHT {
                continue;
            }

            out.push(NoteView {
                id: note.id,
                lane: note.lane,
                opponent: note.opponent,
                is_sustain: note.is_sustain,
                is_sustain_end: note.is_sustain_end,
                x: note_x(note.lane, !note.opponent),
                y,
            });
        }

        out
    }

    /// Visible v0.8.5 hold-trail strips, derived from hold heads rather
    /// than the legacy per-step sustain children.
    pub fn hold_trail_views(&self, cursor: Samples, sample_rate: u32) -> Vec<HoldTrailView> {
        let sample_rate = sample_rate.max(1) as f32;
        let song_ms = cursor.0 as f32 * 1000.0 / sample_rate;
        let rounded_speed = round_decimal(self.scroll_speed, 2);
        let scroll = NOTE_SCROLL_FACTOR * rounded_speed;
        let mut out = Vec::new();

        for note in &self.notes {
            if note.is_sustain || note.sustain_samples <= 0 {
                continue;
            }

            let note_ms = note.hit_at.0 as f32 * 1000.0 / sample_rate;
            let sustain_ms = note.sustain_samples as f32 * 1000.0 / sample_rate;
            let end_ms = note_ms + sustain_ms;
            if end_ms <= song_ms || note_ms - song_ms >= NOTE_SPAWN_LEAD_MS {
                continue;
            }

            let remaining_ms = if song_ms > note_ms {
                end_ms - song_ms
            } else {
                sustain_ms
            };
            let height = remaining_ms * scroll;
            if height <= 0.1 {
                continue;
            }

            let approach_offset = if song_ms > note_ms {
                0.0
            } else {
                (note_ms - song_ms) * scroll
            };
            out.push(HoldTrailView {
                id: note.id,
                lane: note.lane,
                opponent: note.opponent,
                head_resolved: self.resolved_notes.contains(&note.id),
                x: note_x(note.lane, !note.opponent),
                y: STRUM_LINE_Y - INITIAL_OFFSET + approach_offset + STRUMLINE_SIZE * 0.5,
                height,
            });
        }

        out
    }
}

pub fn note_x(lane: Lane, player: bool) -> f32 {
    NOTE_BASE_X + lane as u8 as f32 * NOTE_SWAG_WIDTH + if player { FNF_WIDTH / 2.0 } else { 0.0 }
}

fn round_decimal(value: f32, decimals: u32) -> f32 {
    let factor = 10_f32.powi(decimals as i32);
    (value * factor).round() / factor
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::note::Note;

    fn note(id: u32, lane: Lane, hit_at: i64, opponent: bool) -> Note {
        Note {
            id: NoteId::new(id),
            lane,
            hit_at: Samples(hit_at),
            sustain_samples: 0,
            is_sustain: false,
            is_sustain_end: false,
            opponent,
        }
    }

    #[test]
    fn note_x_matches_og_lane_offsets() {
        assert_eq!(note_x(Lane::Left, false), 48.0);
        assert_eq!(note_x(Lane::Down, false), 160.0);
        assert_eq!(note_x(Lane::Up, false), 272.0);
        assert_eq!(note_x(Lane::Right, false), 384.0);
        assert_eq!(note_x(Lane::Left, true), 688.0);
    }

    #[test]
    fn note_views_use_spawn_window_lane_offsets_and_scroll_formula() {
        let mut state = PlayState::new();
        state.notes.push(note(0, Lane::Left, 48_000, true));
        state.notes.push(note(1, Lane::Right, 48_000, false));
        state.notes.push(note(2, Lane::Down, 96_000, false));
        state.resolved_notes.push(NoteId::new(1));

        let views = state.note_views(Samples(0), 48_000);

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].id, NoteId::new(0));
        assert_eq!(views[0].x, 48.0);
        assert!((views[0].y - 474.0).abs() < 1e-6);
    }

    #[test]
    fn note_views_preserve_sustain_end_flag() {
        let mut state = PlayState::new();
        let mut sustain = note(0, Lane::Down, 48_000, false);
        sustain.is_sustain = true;
        sustain.is_sustain_end = true;
        state.notes.push(sustain);

        let views = state.note_views(Samples(48_000), 48_000);

        assert_eq!(views.len(), 1);
        assert!(views[0].is_sustain);
        assert!(views[0].is_sustain_end);
    }

    #[test]
    fn hold_trail_views_use_hold_heads_and_clip_after_strum() {
        let mut state = PlayState::new();
        let mut hold = note(0, Lane::Left, 48_000, false);
        hold.sustain_samples = 24_000;
        state.notes.push(hold);
        let mut child = note(1, Lane::Left, 60_000, false);
        child.is_sustain = true;
        state.notes.push(child);

        let incoming = state.hold_trail_views(Samples(0), 48_000);
        assert_eq!(incoming.len(), 1);
        assert!(!incoming[0].head_resolved);
        assert_eq!(incoming[0].x, 688.0);
        assert!((incoming[0].height - 225.0).abs() < 1e-4);
        assert!((incoming[0].y - 554.6).abs() < 1e-3);

        state.resolved_notes.push(rustic_core::ids::NoteId::new(0));
        let clipped = state.hold_trail_views(Samples(60_000), 48_000);
        assert_eq!(clipped.len(), 1);
        assert!(clipped[0].head_resolved);
        assert!((clipped[0].height - 112.5).abs() < 1e-4);
        assert!((clipped[0].y - 104.6).abs() < 1e-3);
    }

    #[test]
    fn scroll_speed_is_rounded_like_flx_round_decimal() {
        let mut state = PlayState::new();
        state.scroll_speed = 1.234;
        state.notes.push(note(0, Lane::Left, 48_000, true));

        let views = state.note_views(Samples(0), 48_000);

        assert!((views[0].y - 577.5).abs() < 1e-4);
    }
}
