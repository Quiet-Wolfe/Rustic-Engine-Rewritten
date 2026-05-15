//! Shared Flixel-style frame timing helpers for app-owned sprite animations.

use rustic_core::time::Samples;

pub(crate) fn flixel_frame_index(
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
    fps: u16,
    frame_count: usize,
    looped: bool,
) -> usize {
    if frame_count <= 1 {
        return 0;
    }
    let frame = raw_flixel_frame(cursor, sample_rate, started_at, fps);
    if looped {
        frame % frame_count
    } else {
        frame.min(frame_count - 1)
    }
}

pub(crate) fn visible_flixel_frame_index(
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
    fps: u16,
    frame_count: usize,
    looped: bool,
) -> Option<usize> {
    if frame_count == 0 || cursor < started_at {
        return None;
    }
    let frame = raw_flixel_frame(cursor, sample_rate, started_at, fps);
    if looped {
        Some(frame % frame_count)
    } else {
        (frame < frame_count).then_some(frame)
    }
}

fn raw_flixel_frame(cursor: Samples, sample_rate: u32, started_at: Samples, fps: u16) -> usize {
    // ref: bdedc0aa:references/flxanimate/flxanimate/animate/FlxAnim.hx:407-416
    // Flixel advances while accumulated time is strictly greater than the
    // frame delay. At an exact frame boundary, the previous frame is still the
    // one rendered for that tick.
    let elapsed = cursor.0.saturating_sub(started_at.0).max(0) as u128;
    if elapsed == 0 {
        return 0;
    }
    let numerator = elapsed.saturating_mul(u128::from(fps.max(1)));
    let denominator = u128::from(sample_rate.max(1));
    ((numerator.saturating_sub(1)) / denominator).min(usize::MAX as u128) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flixel_frame_index_holds_previous_frame_on_exact_boundary() {
        assert_eq!(
            flixel_frame_index(Samples(0), 48_000, Samples(0), 24, 3, false),
            0
        );
        assert_eq!(
            flixel_frame_index(Samples(1_999), 48_000, Samples(0), 24, 3, false),
            0
        );
        assert_eq!(
            flixel_frame_index(Samples(2_000), 48_000, Samples(0), 24, 3, false),
            0
        );
        assert_eq!(
            flixel_frame_index(Samples(2_001), 48_000, Samples(0), 24, 3, false),
            1
        );
    }

    #[test]
    fn flixel_frame_index_wraps_after_loop_boundary() {
        assert_eq!(
            flixel_frame_index(Samples(6_000), 48_000, Samples(0), 24, 3, true),
            2
        );
        assert_eq!(
            flixel_frame_index(Samples(6_001), 48_000, Samples(0), 24, 3, true),
            0
        );
    }

    #[test]
    fn visible_frame_keeps_final_frame_through_exact_duration() {
        assert_eq!(
            visible_flixel_frame_index(Samples(6_000), 48_000, Samples(0), 24, 3, false),
            Some(2)
        );
        assert_eq!(
            visible_flixel_frame_index(Samples(6_001), 48_000, Samples(0), 24, 3, false),
            None
        );
    }
}
