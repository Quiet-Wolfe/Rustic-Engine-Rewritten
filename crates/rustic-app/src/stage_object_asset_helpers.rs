use crate::animation_timing::flixel_frame_index;
use rustic_asset::AssetPath;
use rustic_core::ids::AssetId;
use rustic_core::time::Samples;
use rustic_render::FilterMode;

pub(crate) fn stage_frame_index(
    cursor: Samples,
    sample_rate: u32,
    started_at: Samples,
    frame_rate: u16,
    frame_count: usize,
    looped: bool,
) -> usize {
    flixel_frame_index(
        cursor,
        sample_rate,
        started_at,
        frame_rate,
        frame_count,
        looped,
    )
}

pub(crate) fn stage_beat(cursor: Samples, sample_rate: u32, bpm: f64) -> i64 {
    let beat_samples = (f64::from(sample_rate.max(1)) * 60.0 / bpm.max(1.0)).round() as i64;
    cursor.0.max(0).div_euclid(beat_samples.max(1))
}

pub(crate) fn stage_beat_start(cursor: Samples, sample_rate: u32, bpm: f64) -> Samples {
    let beat_samples = (f64::from(sample_rate.max(1)) * 60.0 / bpm.max(1.0)).round() as i64;
    Samples(stage_beat(cursor, sample_rate, bpm) * beat_samples.max(1))
}

pub(crate) fn halloween_lightning_start(
    cursor: Samples,
    sample_rate: u32,
    bpm: f64,
) -> Option<Samples> {
    let beat = stage_beat(cursor, sample_rate, bpm);
    let strike_beat = if (4..=5).contains(&beat) {
        4
    } else if beat > 5 {
        let candidate = 4 + (beat - 4).div_euclid(16) * 16;
        if (candidate..=candidate + 1).contains(&beat) {
            candidate
        } else {
            return None;
        }
    } else {
        return None;
    };
    let beat_samples = (f64::from(sample_rate.max(1)) * 60.0 / bpm.max(1.0)).round() as i64;
    Some(Samples(strike_beat * beat_samples.max(1)))
}

pub(crate) fn filter_for_antialiasing(antialiasing: bool) -> FilterMode {
    if antialiasing {
        FilterMode::Linear
    } else {
        FilterMode::Nearest
    }
}

pub(crate) fn asset_id_for_path(path: &AssetPath) -> AssetId {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in path.as_str().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    AssetId::new(hash)
}
