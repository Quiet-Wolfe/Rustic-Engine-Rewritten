//! V-slice chart event parsing.

use super::{ChartEvent, ChartEventKind, VSliceChart, VSliceEvent};
use serde_json::Value;

pub(super) fn parse_vslice_events(chart: &VSliceChart) -> Vec<ChartEvent> {
    let mut events: Vec<_> = chart
        .events
        .iter()
        .map(|event| ChartEvent {
            time_ms: event.time_ms,
            kind: parse_vslice_event_kind(event),
        })
        .collect();
    events.sort_by(|a, b| {
        a.time_ms
            .partial_cmp(&b.time_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    events
}

fn parse_vslice_event_kind(event: &VSliceEvent) -> ChartEventKind {
    match event.name.as_str() {
        "FocusCamera" => ChartEventKind::FocusCamera {
            target: focus_camera_target(&event.value),
            x: event_float(&event.value, "x", 0.0),
            y: event_float(&event.value, "y", 0.0),
        },
        "PlayAnimation" => parse_play_animation_event(event),
        "ZoomCamera" => ChartEventKind::ZoomCamera {
            zoom: event_float_or_scalar(&event.value, "zoom", 1.0),
            duration_steps: event_float(&event.value, "duration", 4.0),
            direct: event
                .value
                .get("mode")
                .and_then(Value::as_str)
                .map(|mode| mode == "direct")
                .unwrap_or(true),
            ease: event_ease_name(&event.value),
        },
        _ => ChartEventKind::Unknown {
            name: event.name.clone(),
        },
    }
}

fn focus_camera_target(value: &Value) -> Option<i8> {
    value
        .as_i64()
        .or_else(|| value.get("char").and_then(Value::as_i64))
        .and_then(|target| i8::try_from(target).ok())
}

fn event_float(value: &Value, key: &str, default: f32) -> f32 {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(default)
}

fn event_float_or_scalar(value: &Value, key: &str, default: f32) -> f32 {
    value
        .as_f64()
        .map(|value| value as f32)
        .unwrap_or_else(|| event_float(value, key, default))
}

fn event_ease_name(value: &Value) -> String {
    let ease = value
        .get("ease")
        .and_then(Value::as_str)
        .unwrap_or("linear");
    if ease.eq_ignore_ascii_case("linear")
        || ease.eq_ignore_ascii_case("INSTANT")
        || ease.ends_with("In")
        || ease.ends_with("Out")
        || ease.ends_with("InOut")
    {
        return ease.to_string();
    }
    let dir = value.get("easeDir").and_then(Value::as_str).unwrap_or("In");
    format!("{ease}{dir}")
}

fn parse_play_animation_event(event: &VSliceEvent) -> ChartEventKind {
    let Some(target) = event.value.get("target").and_then(Value::as_str) else {
        return ChartEventKind::Unknown {
            name: event.name.clone(),
        };
    };
    let Some(animation) = event.value.get("anim").and_then(Value::as_str) else {
        return ChartEventKind::Unknown {
            name: event.name.clone(),
        };
    };
    ChartEventKind::PlayAnimation {
        target: target.to_string(),
        animation: animation.to_string(),
        force: event
            .value
            .get("force")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}
