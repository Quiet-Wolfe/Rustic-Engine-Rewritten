//! Headless gameplay view models for notes.
//!
//! ref: 50fccded:source/Note.hx:27,104-118       // swagWidth and lane x
//! ref: 50fccded:source/PlayState.hx:629         // strumLine y = 50
//! ref: 50fccded:source/PlayState.hx:1487-1494   // spawn notes within 1500ms
//! ref: 50fccded:source/PlayState.hx:1512        // note y scroll formula
//! ref: 50fccded:source/PlayState.hx:1175-1176   // x += 50 + half screen for player

use crate::note::Lane;
use crate::state::PlayState;
use rustic_core::ids::NoteId;
use rustic_core::time::Samples;

pub const FNF_WIDTH: f32 = 1280.0;
pub const FNF_HEIGHT: f32 = 720.0;
pub const STRUM_LINE_Y: f32 = 50.0;
pub const NOTE_SWAG_WIDTH: f32 = 160.0 * 0.7;
pub const NOTE_BASE_X: f32 = 50.0;
pub const NOTE_SPAWN_LEAD_MS: f32 = 1500.0;
pub const NOTE_SCROLL_FACTOR: f32 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct NoteView {
    pub id: NoteId,
    pub lane: Lane,
    pub opponent: bool,
    pub is_sustain: bool,
    pub x: f32,
    pub y: f32,
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
                x: note_x(note.lane, !note.opponent),
                y,
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
            opponent,
        }
    }

    #[test]
    fn note_x_matches_og_lane_offsets() {
        assert_eq!(note_x(Lane::Left, false), 50.0);
        assert_eq!(note_x(Lane::Down, false), 162.0);
        assert_eq!(note_x(Lane::Up, false), 274.0);
        assert_eq!(note_x(Lane::Right, false), 386.0);
        assert_eq!(note_x(Lane::Left, true), 690.0);
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
        assert_eq!(views[0].x, 50.0);
        assert!((views[0].y - 500.0).abs() < 1e-6);
    }

    #[test]
    fn scroll_speed_is_rounded_like_flx_round_decimal() {
        let mut state = PlayState::new();
        state.scroll_speed = 1.234;
        state.notes.push(note(0, Lane::Left, 48_000, true));

        let views = state.note_views(Samples(0), 48_000);

        assert!((views[0].y - 603.5).abs() < 1e-4);
    }
}
