//! Per-frame chart progression that is not player scoring.
//!
//! ref: 50fccded:source/Note.hx:186-192          // opponent note wasGoodHit at songPosition
//! ref: 50fccded:source/PlayState.hx:1528-1552   // opponent note animation/removal

use crate::note::Lane;
use crate::state::PlayState;
use rustic_core::time::Samples;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResolvedOpponentNote {
    pub lane: Lane,
    pub is_sustain: bool,
}

impl PlayState {
    /// Resolve opponent-side notes whose strum time has reached the current
    /// conductor cursor. Base FNF uses this to drive dad animations and
    /// remove the note; it does not change player score/combo/health.
    pub fn resolve_opponent_notes(&mut self, cursor: Samples) -> Vec<ResolvedOpponentNote> {
        let mut hits = Vec::new();
        for note in &self.notes {
            if !note.opponent || self.resolved_notes.contains(&note.id) {
                continue;
            }
            if note.hit_at.0 <= cursor.0 {
                hits.push((
                    note.id,
                    ResolvedOpponentNote {
                        lane: note.lane,
                        is_sustain: note.is_sustain,
                    },
                ));
            }
        }

        self.resolved_notes.extend(hits.iter().map(|(id, _)| *id));
        hits.into_iter().map(|(_, hit)| hit).collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::note::{Lane, Note};
    use crate::state::INITIAL_HEALTH;
    use rustic_core::ids::NoteId;

    fn note(id: u32, hit_at: i64, opponent: bool) -> Note {
        Note {
            id: NoteId::new(id),
            lane: Lane::Left,
            hit_at: Samples(hit_at),
            sustain_samples: 0,
            is_sustain: false,
            is_sustain_end: false,
            opponent,
        }
    }

    #[test]
    fn opponent_notes_resolve_at_their_strum_time_without_player_scoring() {
        let mut state = PlayState::new();
        state.notes.push(note(0, 1_000, true));
        state.notes.push(note(1, 2_000, true));
        state.notes.push(note(2, 1_000, false));
        state.score = 900;
        state.combo = 4;

        let hits = state.resolve_opponent_notes(Samples(1_000));

        assert_eq!(
            hits,
            vec![ResolvedOpponentNote {
                lane: Lane::Left,
                is_sustain: false
            }]
        );
        assert!(state.resolved_notes.contains(&NoteId::new(0)));
        assert!(!state.resolved_notes.contains(&NoteId::new(1)));
        assert!(!state.resolved_notes.contains(&NoteId::new(2)));
        assert_eq!(state.score, 900);
        assert_eq!(state.combo, 4);
        assert_eq!(state.misses, 0);
        assert!((state.health - INITIAL_HEALTH).abs() < 1e-6);
    }
}
