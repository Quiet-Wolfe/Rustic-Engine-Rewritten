//! HUD beat-bop helpers.

use rustic_core::time::Samples;

pub(crate) fn health_icon_scale(cursor: Samples, sample_rate: u32, bpm: f64) -> f32 {
    // ref: bdedc0aa:source/funkin/play/components/HealthIcon.hx:227-242,296-315
    if cursor.0 < 0 {
        return 1.0;
    }
    const BOP_SCALE: f32 = 0.2;
    let bpm = bpm.max(1.0);
    let beat_samples = (f64::from(sample_rate.max(1)) * 60.0 / bpm).round() as i64;
    let step_length_ms = 15_000.0 / bpm;
    let tween_secs = (step_length_ms * 0.002).min(0.175);
    let tween_samples = (f64::from(sample_rate.max(1)) * tween_secs).round() as i64;
    let phase_samples = cursor.0.rem_euclid(beat_samples.max(1));
    if tween_samples <= 0 || phase_samples >= tween_samples {
        1.0
    } else {
        let t = phase_samples as f32 / tween_samples as f32;
        1.0 + BOP_SCALE * (1.0 - t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_icon_bop_uses_v085_step_tween_duration() {
        assert_eq!(health_icon_scale(Samples(-1), 48_000, 100.0), 1.0);
        assert!((health_icon_scale(Samples(0), 48_000, 100.0) - 1.2).abs() < 1e-6);
        assert!((health_icon_scale(Samples(4_200), 48_000, 100.0) - 1.1).abs() < 1e-6);
        assert_eq!(health_icon_scale(Samples(8_400), 48_000, 100.0), 1.0);
        assert!((health_icon_scale(Samples(28_800), 48_000, 100.0) - 1.2).abs() < 1e-6);
    }
}
