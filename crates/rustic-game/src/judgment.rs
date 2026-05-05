//! Judgment windows, ratings, and scoring math.
//!
//! Ported from base FNF. The window boundaries are derived from
//! `Conductor.safeZoneOffset` so changes to the offset propagate the way
//! they do upstream.
//!
//! ref: 50fccded:source/Conductor.hx:26-27       // safeFrames=10, safeZoneOffset=(10/60)*1000
//! ref: 50fccded:source/PlayState.hx:1684-1714   // popUpScore thresholds (sick/good/bad/shit)
//! ref: 50fccded:source/PlayState.hx:2028-2042   // noteMiss: -10 score, -0.04 health, combo=0
//! ref: 50fccded:source/PlayState.hx:2096-2109   // goodNoteHit: +1 combo, +0.023 health (taps)
//! ref: 50fccded:source/PlayState.hx:78          // health starts at 1.0
//! ref: 50fccded:source/PlayState.hx:1297-1298   // health clamped to 2.0
//! ref: 50fccded:source/PlayState.hx:1462        // game over at health <= 0

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
    /// Conductor.safeZoneOffset (ms). Base FNF: (safeFrames=10) / 60 * 1000.
    /// Hit ratings are percentages of this offset.
    pub safe_zone_ms: Milliseconds,
}

impl JudgmentWindows {
    pub const DEFAULT_SAFE_ZONE_MS: f64 = (10.0 / 60.0) * 1000.0;

    pub const fn base_fnf() -> Self {
        Self {
            safe_zone_ms: Milliseconds(Self::DEFAULT_SAFE_ZONE_MS),
        }
    }

    /// Rating for an absolute timing delta in milliseconds. Outside the
    /// safe zone the caller should record a miss instead of calling this;
    /// values past `safe_zone` still classify as `Shit` to mirror the
    /// `> 0.9 * safeZone → shit` branch.
    /// ref: 50fccded:source/PlayState.hx:1700-1714
    pub fn judge(&self, abs_diff_ms: f64) -> Judgment {
        let z = self.safe_zone_ms.0;
        if abs_diff_ms > z * 0.9 {
            Judgment::Shit
        } else if abs_diff_ms > z * 0.75 {
            Judgment::Bad
        } else if abs_diff_ms > z * 0.2 {
            Judgment::Good
        } else {
            Judgment::Sick
        }
    }
}

impl Default for JudgmentWindows {
    fn default() -> Self {
        Self::base_fnf()
    }
}

/// Score delta for a judgment.
/// ref: 50fccded:source/PlayState.hx:1696,1703,1708,1713 (350/50/100/200)
/// ref: 50fccded:source/PlayState.hx:2039 (miss -10)
pub fn score_value(j: Judgment) -> i64 {
    match j {
        Judgment::Sick => 350,
        Judgment::Good => 200,
        Judgment::Bad => 100,
        Judgment::Shit => 50,
        Judgment::Miss => -10,
    }
}

/// Health delta for a judgment. Upstream applies the same +0.023 hit
/// health for normal note data, including sustain child notes.
/// ref: 50fccded:source/PlayState.hx:2107 (+0.023 on hit)
/// ref: 50fccded:source/PlayState.hx:2032 (-0.04 on miss)
pub fn health_delta(j: Judgment) -> f32 {
    match j {
        Judgment::Sick | Judgment::Good | Judgment::Bad | Judgment::Shit => 0.023,
        Judgment::Miss => -0.04,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn safe_zone_matches_fnf_constant() {
        let w = JudgmentWindows::base_fnf();
        // safeFrames=10 / 60 * 1000 ≈ 166.6667ms
        assert!((w.safe_zone_ms.0 - 166.666_666_666_666_67).abs() < 1e-9);
    }

    #[test]
    fn judge_thresholds_match_fnf() {
        let w = JudgmentWindows::base_fnf();
        // Per PlayState.hx:1700-1714, boundaries are at:
        //   0.9   ≈ 150.000 ms
        //   0.75  ≈ 125.000 ms
        //   0.2   ≈ 33.333 ms
        assert_eq!(w.judge(0.0), Judgment::Sick);
        assert_eq!(w.judge(33.0), Judgment::Sick);
        assert_eq!(w.judge(34.0), Judgment::Good);
        assert_eq!(w.judge(120.0), Judgment::Good);
        assert_eq!(w.judge(126.0), Judgment::Bad);
        assert_eq!(w.judge(149.0), Judgment::Bad);
        assert_eq!(w.judge(151.0), Judgment::Shit);
    }

    #[test]
    fn score_values_match_fnf() {
        assert_eq!(score_value(Judgment::Sick), 350);
        assert_eq!(score_value(Judgment::Good), 200);
        assert_eq!(score_value(Judgment::Bad), 100);
        assert_eq!(score_value(Judgment::Shit), 50);
        assert_eq!(score_value(Judgment::Miss), -10);
    }

    #[test]
    fn health_deltas_match_fnf() {
        assert!((health_delta(Judgment::Sick) - 0.023).abs() < 1e-6);
        assert!((health_delta(Judgment::Miss) - -0.04).abs() < 1e-6);
    }
}
