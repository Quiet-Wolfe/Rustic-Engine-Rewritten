use super::*;

#[test]
fn health_icon_bop_uses_v085_step_tween_duration() {
    assert_eq!(health_icon_scale(Samples(-1), 48_000, 100.0), 1.0);
    assert!((health_icon_scale(Samples(0), 48_000, 100.0) - 1.2).abs() < 1e-6);
    assert!((health_icon_scale(Samples(4_200), 48_000, 100.0) - 1.1).abs() < 1e-6);
    assert_eq!(health_icon_scale(Samples(8_400), 48_000, 100.0), 1.0);
    assert!((health_icon_scale(Samples(28_800), 48_000, 100.0) - 1.2).abs() < 1e-6);
}
