//! V-slice chart event parsing.

use super::{ChartEvent, ChartEventKind, SserafimEvent, VSliceChart, VSliceEvent};
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
            duration_steps: event_float(&event.value, "duration", 4.0),
            ease: focus_camera_ease_name(&event.value),
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
        "ScrollSpeed" => ChartEventKind::ScrollSpeed {
            scroll: event_float_or_scalar(&event.value, "scroll", 1.0),
            duration_steps: event_float(&event.value, "duration", 4.0),
            absolute: event
                .value
                .get("absolute")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            strumline: event
                .value
                .get("strumline")
                .and_then(Value::as_str)
                .unwrap_or("both")
                .to_string(),
            ease: event_ease_name(&event.value),
        },
        "SetCameraBop" => ChartEventKind::SetCameraBop {
            rate: event_float(&event.value, "rate", 4.0),
            offset: event_float(&event.value, "offset", 0.0),
            intensity: event_float(&event.value, "intensity", 1.0),
        },
        "SetHealthIcon" => ChartEventKind::SetHealthIcon {
            target: event
                .value
                .get("char")
                .and_then(value_i64)
                .and_then(|target| i8::try_from(target).ok())
                .unwrap_or(0),
            id: event
                .value
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("face")
                .to_string(),
            scale: event_float(&event.value, "scale", 1.0),
            flip_x: event_bool(&event.value, "flipX", false),
            is_pixel: event_bool(&event.value, "isPixel", false),
            offset_x: event_float(&event.value, "offsetX", 0.0),
            offset_y: event_float(&event.value, "offsetY", 0.0),
        },
        "sserafimShow" => ChartEventKind::Sserafim(SserafimEvent::Show {
            visible: event_bool_array(&event.value, "visible"),
        }),
        "sserafimSing" => ChartEventKind::Sserafim(SserafimEvent::Sing {
            singing: event_bool_array(&event.value, "singing"),
        }),
        "sserafimDark" => ChartEventKind::Sserafim(SserafimEvent::Dark {
            amount: event_float(&event.value, "amount", 0.0),
            duration: event_float(&event.value, "duration", 0.0),
        }),
        "sserafimLights" => ChartEventKind::Sserafim(SserafimEvent::Lights {
            amount: event_float(&event.value, "amount", 0.0),
            duration: event_float(&event.value, "duration", 0.0),
        }),
        "sserafimPulseLights" => ChartEventKind::Sserafim(SserafimEvent::PulseLights {
            enabled: event_bool(&event.value, "enabled", false),
            colors: event_string_array(&event.value, "colors"),
            durations: event_float_array(&event.value, "durations"),
            intensities: event_float_array(&event.value, "intensities"),
        }),
        "sserafimCover" => ChartEventKind::Sserafim(SserafimEvent::Cover {
            visible: event_bool(&event.value, "visible", false),
        }),
        "sserafimFlash" => ChartEventKind::Sserafim(SserafimEvent::Flash {
            duration: event_float(&event.value, "duration", 0.0),
        }),
        "sserafimKick" => ChartEventKind::Sserafim(SserafimEvent::Kick {
            final_kick: event_bool(&event.value, "final", false),
        }),
        "sserafimBeautiful" => ChartEventKind::Sserafim(SserafimEvent::Beautiful {
            beautiful: event_bool(&event.value, "beautiful", false),
        }),
        "sserafimGuitarVibration" => ChartEventKind::Sserafim(SserafimEvent::GuitarVibration {
            duration: event_float(&event.value, "duration", 0.0),
        }),
        "sserafimEnd" => ChartEventKind::Sserafim(SserafimEvent::End),
        _ => ChartEventKind::Unknown {
            name: event.name.clone(),
        },
    }
}

fn focus_camera_target(value: &Value) -> Option<i8> {
    value_i64(value)
        .or_else(|| value.get("char").and_then(value_i64))
        .and_then(|target| i8::try_from(target).ok())
}

fn event_float(value: &Value, key: &str, default: f32) -> f32 {
    value
        .get(key)
        .and_then(value_f64)
        .map(|value| value as f32)
        .unwrap_or(default)
}

fn event_float_or_scalar(value: &Value, key: &str, default: f32) -> f32 {
    value_f64(value)
        .map(|value| value as f32)
        .unwrap_or_else(|| event_float(value, key, default))
}

fn event_bool(value: &Value, key: &str, default: bool) -> bool {
    value.get(key).and_then(value_bool).unwrap_or(default)
}

fn value_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn value_bool(value: &Value) -> Option<bool> {
    value
        .as_bool()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn value_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn event_bool_array(value: &Value, key: &str) -> Vec<bool> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(value_bool).collect())
        .unwrap_or_default()
}

fn event_float_array(value: &Value, key: &str) -> Vec<f32> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(value_f64)
                .map(|value| value as f32)
                .collect()
        })
        .unwrap_or_default()
}

fn event_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
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

fn focus_camera_ease_name(value: &Value) -> String {
    let ease = value
        .get("ease")
        .and_then(Value::as_str)
        .unwrap_or("CLASSIC");
    if ease.eq_ignore_ascii_case("CLASSIC") || ease.eq_ignore_ascii_case("INSTANT") {
        return ease.to_string();
    }
    event_ease_name(value)
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
