//! Judgment windows, ratings, and scoring math.
//!
//! ref: bdedc0aa:source/funkin/play/scoring/Scoring.hx:89-193
//! ref: bdedc0aa:source/funkin/util/Constants.hx:352,436-529,531-535

use rustic_core::time::Milliseconds;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Judgment {
    Sick,
    Good,
    Bad,
    Shit,
    Miss,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct JudgmentWindows {
    /// `Constants.HIT_WINDOW_MS` in v0.8.5.
    pub hit_window_ms: Milliseconds,
}

impl JudgmentWindows {
    pub const DEFAULT_HIT_WINDOW_MS: f64 = PBOT1_MISS_THRESHOLD;

    pub const fn base_fnf() -> Self {
        Self {
            hit_window_ms: Milliseconds(Self::DEFAULT_HIT_WINDOW_MS),
        }
    }

    /// PBOT1 rating for an absolute timing delta in milliseconds.
    pub fn judge(&self, abs_diff_ms: f64) -> Judgment {
        let abs = abs_diff_ms.abs();
        if abs <= PBOT1_SICK_THRESHOLD {
            Judgment::Sick
        } else if abs <= PBOT1_GOOD_THRESHOLD {
            Judgment::Good
        } else if abs <= PBOT1_BAD_THRESHOLD {
            Judgment::Bad
        } else if abs <= self.hit_window_ms.0 {
            Judgment::Shit
        } else {
            Judgment::Miss
        }
    }
}

impl Default for JudgmentWindows {
    fn default() -> Self {
        Self::base_fnf()
    }
}

pub const PBOT1_MAX_SCORE: i64 = 500;
pub const PBOT1_SCORING_OFFSET: f64 = 54.99;
pub const PBOT1_SCORING_SLOPE: f64 = 0.080;
pub const PBOT1_MIN_SCORE: f64 = 9.0;
pub const PBOT1_MISS_SCORE: i64 = -100;
pub const PBOT1_PERFECT_THRESHOLD: f64 = 5.0;
pub const PBOT1_MISS_THRESHOLD: f64 = 160.0;
pub const PBOT1_SICK_THRESHOLD: f64 = 45.0;
pub const PBOT1_GOOD_THRESHOLD: f64 = 90.0;
pub const PBOT1_BAD_THRESHOLD: f64 = 135.0;
pub const GHOST_MISS_SCORE: i64 = -10;
pub const HEALTH_HOLD_BONUS_PER_SECOND: f32 = 0.12;
pub const SCORE_HOLD_BONUS_PER_SECOND: f64 = 250.0;
pub const SCORE_HOLD_DROP_PENALTY_PER_SECOND: f64 = -125.0;
pub const HOLD_DROP_PENALTY_THRESHOLD_MS: f64 = 160.0;

/// PBOT1 score delta for a note hit at `abs_diff_ms`.
pub fn score_for_timing(abs_diff_ms: f64) -> i64 {
    let abs = abs_diff_ms.abs();
    if abs > PBOT1_MISS_THRESHOLD {
        PBOT1_MISS_SCORE
    } else if abs < PBOT1_PERFECT_THRESHOLD {
        PBOT1_MAX_SCORE
    } else {
        let exponent = -PBOT1_SCORING_SLOPE * (abs - PBOT1_SCORING_OFFSET);
        let factor = 1.0 - (1.0 / (1.0 + exponent.exp()));
        (PBOT1_MAX_SCORE as f64 * factor + PBOT1_MIN_SCORE) as i64
    }
}

pub fn note_miss_score_delta() -> i64 {
    PBOT1_MISS_SCORE
}

pub fn ghost_miss_score_delta() -> i64 {
    GHOST_MISS_SCORE
}

pub fn hold_drop_score_delta(remaining_ms: f64) -> Option<i64> {
    (remaining_ms > HOLD_DROP_PENALTY_THRESHOLD_MS)
        .then(|| (remaining_ms / 1000.0 * SCORE_HOLD_DROP_PENALTY_PER_SECOND).round() as i64)
}

/// Health delta for PBOT1 judgments.
pub fn health_delta(j: Judgment) -> f32 {
    match j {
        Judgment::Sick => 0.03,
        Judgment::Good => 0.015,
        Judgment::Bad => 0.0,
        Judgment::Shit => -0.02,
        Judgment::Miss => -0.08,
    }
}

pub fn combo_breaks(j: Judgment) -> bool {
    matches!(j, Judgment::Bad | Judgment::Shit | Judgment::Miss)
}

pub fn ghost_miss_health_delta() -> f32 {
    -0.08
}

/// Health delta for an unhit player note that has gone late/offscreen.
pub fn late_note_health_delta() -> f32 {
    health_delta(Judgment::Miss)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn hit_window_matches_fnf_constant() {
        let w = JudgmentWindows::base_fnf();
        assert_eq!(w.hit_window_ms.0, 160.0);
    }

    #[test]
    fn judge_thresholds_match_pbot1() {
        let w = JudgmentWindows::base_fnf();
        assert_eq!(w.judge(0.0), Judgment::Sick);
        assert_eq!(w.judge(45.0), Judgment::Sick);
        assert_eq!(w.judge(46.0), Judgment::Good);
        assert_eq!(w.judge(90.0), Judgment::Good);
        assert_eq!(w.judge(91.0), Judgment::Bad);
        assert_eq!(w.judge(135.0), Judgment::Bad);
        assert_eq!(w.judge(136.0), Judgment::Shit);
        assert_eq!(w.judge(160.0), Judgment::Shit);
        assert_eq!(w.judge(161.0), Judgment::Miss);
    }

    #[test]
    fn timing_scores_match_pbot1_shape() {
        assert_eq!(score_for_timing(0.0), 500);
        assert_eq!(score_for_timing(4.999), 500);
        assert_eq!(score_for_timing(200.0), -100);
        assert!(score_for_timing(45.0) > score_for_timing(90.0));
        assert!(score_for_timing(90.0) > score_for_timing(135.0));
    }

    #[test]
    fn health_deltas_match_pbot1_constants() {
        assert!((health_delta(Judgment::Sick) - 0.03).abs() < 1e-6);
        assert!((health_delta(Judgment::Good) - 0.015).abs() < 1e-6);
        assert!((health_delta(Judgment::Bad) - 0.0).abs() < 1e-6);
        assert!((health_delta(Judgment::Shit) - -0.02).abs() < 1e-6);
        assert!((health_delta(Judgment::Miss) - -0.08).abs() < 1e-6);
        assert!((late_note_health_delta() - -0.08).abs() < 1e-6);
        assert!((HEALTH_HOLD_BONUS_PER_SECOND - 0.12).abs() < 1e-6);
        assert_eq!(note_miss_score_delta(), -100);
        assert_eq!(ghost_miss_score_delta(), -10);
        assert_eq!(SCORE_HOLD_BONUS_PER_SECOND, 250.0);
        assert_eq!(hold_drop_score_delta(160.0), None);
        assert_eq!(hold_drop_score_delta(1000.0), Some(-125));
    }
}
