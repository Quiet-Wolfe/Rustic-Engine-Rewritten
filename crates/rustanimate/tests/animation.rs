#![allow(clippy::unwrap_used)]

use glam::Vec2;
use rustanimate::{AnimateError, Animation, ElementKind};

const ANIMATION: &[u8] = br#"{
  "AN": {
    "N": "BoyFriend Assets_TA-Export",
    "SN": "BF ALL ANIMS",
    "TL": {
      "L": [{
        "LN": "Labels",
        "FR": [
          { "N": "Idle", "I": 0, "DU": 14, "E": [] },
          { "N": "Left", "I": 14, "DU": 8, "E": [] },
          { "I": 22, "DU": 2, "E": [] }
        ]
      }]
    },
    "SD": {
      "S": [{
        "SN": "BF idle dance",
        "TL": {
          "L": [{
            "LN": "Layer 1",
            "FR": [{
              "I": 0,
              "DU": 2,
              "E": [
                {
                  "SI": {
                    "SN": "BF Head default",
                    "FF": 4,
                    "TRP": { "x": 532.6, "y": -82 },
                    "LP": "LP",
                    "MX": [0.994, -0.105, 0.105, 0.994, 354.55, -165.35]
                  }
                },
                { "ASI": { "N": "0", "MX": [1, 0, 0, 1, 401.15, -123] } }
              ]
            }]
          }]
        }
      }, {
        "SN": "BF NOTE LEFT",
        "TL": { "L": [] }
      }]
    }
  }
}"#;

const FLATTEN_ANIMATION: &[u8] = br#"{
  "AN": {
    "N": "flatten-test",
    "SN": "root",
    "TL": { "L": [
      {
        "LN": "Labels",
        "FR": [{ "N": "Idle", "I": 0, "DU": 4, "E": [] }]
      },
      {
        "LN": "Art",
        "FR": [{
          "I": 0,
          "DU": 4,
          "E": [{
            "SI": {
              "SN": "body",
              "FF": 0,
              "LP": "LP",
              "MX": [1, 0, 0, 1, 10, 20]
            }
          }]
        }]
      }
    ] },
    "SD": { "S": [{
      "SN": "body",
      "TL": {
        "L": [
          {
            "LN": "front",
            "FR": [
              {
                "I": 0,
                "DU": 1,
                "E": [{ "ASI": { "N": "front0", "MX": [1, 0, 0, 1, 3, 4] } }]
              },
              {
                "I": 1,
                "DU": 1,
                "E": [{ "ASI": { "N": "front1", "MX": [1, 0, 0, 1, 5, 6] } }]
              }
            ]
          },
          {
            "LN": "back",
            "FR": [{
              "I": 0,
              "DU": 2,
              "E": [{ "ASI": { "N": "back", "MX": [2, 0, 0, 2, 0, 1] } }]
            }]
          }
        ]
      }
    }] }
  }
}"#;

#[test]
fn parses_animation_labels_and_symbols() {
    let animation = Animation::parse(ANIMATION).unwrap();
    assert_eq!(animation.name, "BoyFriend Assets_TA-Export");
    assert_eq!(animation.symbol_name, "BF ALL ANIMS");
    assert_eq!(animation.layers.len(), 1);
    let labels: Vec<_> = animation
        .labels
        .iter()
        .map(|label| (label.name.as_str(), label.index, label.duration))
        .collect();
    assert_eq!(labels, vec![("Idle", 0, 14), ("Left", 14, 8)]);
    assert_eq!(animation.label("Left").unwrap().duration, 8);
    assert!(animation.has_symbol("BF NOTE LEFT"));
    assert!(!animation.has_symbol("Missing"));

    let symbol = animation.symbol("BF idle dance").unwrap();
    assert_eq!(symbol.layers.len(), 1);
    assert_eq!(symbol.layers[0].name, "Layer 1");
    assert_eq!(symbol.layers[0].frames.len(), 1);
    let element = &symbol.layers[0].frames[0].elements[0];
    assert_eq!(
        element.matrix,
        [0.994, -0.105, 0.105, 0.994, 354.55, -165.35]
    );
    let ElementKind::Symbol(instance) = &element.kind else {
        panic!("expected symbol instance");
    };
    assert_eq!(instance.symbol_name, "BF Head default");
    assert_eq!(instance.first_frame, 4);
    assert_eq!(instance.transform_point, Vec2::new(532.6, -82.0));
    assert_eq!(instance.loop_mode.as_deref(), Some("LP"));

    let element = &symbol.layers[0].frames[0].elements[1];
    assert_eq!(element.matrix, [1.0, 0.0, 0.0, 1.0, 401.15, -123.0]);
    let ElementKind::Atlas(instance) = &element.kind else {
        panic!("expected atlas instance");
    };
    assert_eq!(instance.frame_name, "0");
}

#[test]
fn flattens_label_frames_in_draw_order() {
    let animation = Animation::parse(FLATTEN_ANIMATION).unwrap();

    let parts = animation.flatten_label_frame("Idle", 0).unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].frame_name, "back");
    assert_eq!(parts[0].matrix, [2.0, 0.0, 0.0, 2.0, 10.0, 21.0]);
    assert_eq!(parts[1].frame_name, "front0");
    assert_eq!(parts[1].matrix, [1.0, 0.0, 0.0, 1.0, 13.0, 24.0]);
}

#[test]
fn flattens_looping_symbol_frames() {
    let animation = Animation::parse(FLATTEN_ANIMATION).unwrap();

    let parts = animation.flatten_label_frame("Idle", 1).unwrap();
    assert_eq!(parts[0].frame_name, "back");
    assert_eq!(parts[1].frame_name, "front1");

    let parts = animation.flatten_label_frame("Idle", 3).unwrap();
    assert_eq!(parts[0].frame_name, "back");
    assert_eq!(parts[1].frame_name, "front1");
}

#[test]
fn follows_flxanimate_top_level_symbol_dictionary_overwrite_order() {
    let animation = Animation::parse(
        br#"{
          "AN": {
            "N": "duplicate-symbol-test",
            "SN": "root",
            "TL": { "L": [
              { "FR": [{ "N": "Idle", "I": 0, "DU": 1, "E": [] }] },
              { "FR": [{
                "I": 0,
                "DU": 1,
                "E": [{ "SI": { "SN": "body", "FF": 0, "LP": "LP" } }]
              }] }
            ] }
          },
          "SD": { "S": [
            { "SN": "body", "TL": { "L": [{ "FR": [{
              "I": 0,
              "DU": 1,
              "E": [{ "ASI": { "N": "old", "MX": [1, 0, 0, 1, 0, 0] } }]
            }] }] } },
            { "SN": "body", "TL": { "L": [{ "FR": [{
              "I": 0,
              "DU": 1,
              "E": [{ "ASI": { "N": "new", "MX": [1, 0, 0, 1, 0, 0] } }]
            }] }] } }
          ] }
        }"#,
    )
    .unwrap();

    assert_eq!(animation.symbol("body").unwrap().name, "body");
    let parts = animation.flatten_label_frame("Idle", 0).unwrap();
    assert_eq!(parts[0].frame_name, "new");
}

#[test]
fn follows_flxanimate_compact_symbol_loop_defaults() {
    assert_eq!(compact_loop_frame("", 2), "body0");
    assert_eq!(compact_loop_frame(r#""LP": "LPR","#, 2), "body0");
    assert_eq!(compact_loop_frame(r#""LP": "PO","#, 3), "body1");
    assert_eq!(
        compact_loop_frame(r#""ST": "MC", "FF": 1, "LP": "SF","#, 2),
        "body0"
    );
    assert_eq!(
        compact_loop_frame(r#""ST": "B", "FF": 1, "LP": "LP","#, 1),
        "body0"
    );
}

#[test]
fn follows_flxanimate_verbose_symbol_loop_defaults() {
    assert_eq!(verbose_loop_frame("", 2), "body0");
    assert_eq!(
        verbose_loop_frame(
            r#""symbolType": "movieclip", "firstFrame": 1, "loop": "singleframe","#,
            2
        ),
        "body0"
    );
}

#[test]
fn rejects_zero_duration_labels() {
    let bad = br#"{
      "AN": {
        "TL": { "L": [{ "FR": [{ "N": "Idle", "I": 0, "DU": 0 }] }] }
      }
    }"#;
    assert!(matches!(Animation::parse(bad), Err(AnimateError::Atlas(_))));
}

#[test]
fn rejects_empty_symbol_names() {
    let bad = br#"{
      "AN": {
        "TL": { "L": [] },
        "SD": { "S": [{ "SN": "" }] }
      }
    }"#;
    assert!(matches!(Animation::parse(bad), Err(AnimateError::Atlas(_))));
}

#[test]
fn rejects_empty_element_symbol_names() {
    let bad = br#"{
      "AN": {
        "TL": { "L": [] },
        "SD": {
          "S": [{
            "SN": "container",
            "TL": {
              "L": [{
                "FR": [{
                  "I": 0,
                  "DU": 1,
                  "E": [{ "SI": { "SN": "" } }]
                }]
              }]
            }
          }]
        }
      }
    }"#;
    assert!(matches!(Animation::parse(bad), Err(AnimateError::Atlas(_))));
}

#[test]
fn parses_matrix3d_as_affine() {
    let json = br#"{
      "AN": {
        "TL": { "L": [] },
        "SD": {
          "S": [{
            "SN": "container",
            "TL": {
              "L": [{
                "FR": [{
                  "I": 0,
                  "DU": 1,
                  "E": [{
                    "ASI": {
                      "N": "0000",
                      "M3D": [2, 0.5, 0, 0, -0.25, 3, 0, 0, 0, 0, 1, 0, 7, 8, 0, 1]
                    }
                  }]
                }]
              }]
            }
          }]
        }
      }
    }"#;
    let animation = Animation::parse(json).unwrap();
    let element = &animation.symbol("container").unwrap().layers[0].frames[0].elements[0];
    assert_eq!(element.matrix, [2.0, 0.5, -0.25, 3.0, 7.0, 8.0]);
}

#[test]
fn rejects_empty_element_atlas_frame_names() {
    let bad = br#"{
      "AN": {
        "TL": { "L": [] },
        "SD": {
          "S": [{
            "SN": "container",
            "TL": {
              "L": [{
                "FR": [{
                  "I": 0,
                  "DU": 1,
                  "E": [{ "ASI": { "N": "" } }]
                }]
              }]
            }
          }]
        }
      }
    }"#;
    assert!(matches!(Animation::parse(bad), Err(AnimateError::Atlas(_))));
}

fn compact_loop_frame(symbol_fields: &str, frame_offset: u32) -> String {
    let json = format!(
        r#"{{
  "AN": {{
    "N": "loop-test",
    "SN": "root",
    "TL": {{ "L": [
      {{ "LN": "Labels", "FR": [{{ "N": "Idle", "I": 0, "DU": 4, "E": [] }}] }},
      {{
        "LN": "Art",
        "FR": [{{
          "I": 0,
          "DU": 4,
          "E": [{{ "SI": {{ "SN": "body", {symbol_fields} "MX": [1, 0, 0, 1, 0, 0] }} }}]
        }}]
      }}
    ] }},
    "SD": {{ "S": [{{
      "SN": "body",
      "TL": {{ "L": [{{
        "LN": "art",
        "FR": [
          {{ "I": 0, "DU": 1, "E": [{{ "ASI": {{ "N": "body0" }} }}] }},
          {{ "I": 1, "DU": 1, "E": [{{ "ASI": {{ "N": "body1" }} }}] }}
        ]
      }}] }}
    }}] }}
  }}
}}"#
    );
    let animation = Animation::parse(json.as_bytes()).unwrap();
    animation
        .flatten_label_frame("Idle", frame_offset)
        .unwrap()
        .remove(0)
        .frame_name
}

fn verbose_loop_frame(symbol_fields: &str, frame_offset: u32) -> String {
    let json = format!(
        r#"{{
  "ANIMATION": {{
    "name": "loop-test",
    "SYMBOL_name": "root",
    "TIMELINE": {{ "LAYERS": [{{
      "Layer_name": "Art",
      "Frames": [{{
        "index": 0,
        "duration": 4,
        "elements": [{{
          "SYMBOL_Instance": {{
            "SYMBOL_name": "body",
            {symbol_fields}
            "Matrix3D": {{
              "m00": 1, "m01": 0, "m02": 0, "m03": 0,
              "m10": 0, "m11": 1, "m12": 0, "m13": 0,
              "m20": 0, "m21": 0, "m22": 1, "m23": 0,
              "m30": 0, "m31": 0, "m32": 0, "m33": 1
            }}
          }}
        }}]
      }}]
    }}] }}
  }},
  "SYMBOL_DICTIONARY": {{ "Symbols": [{{
    "SYMBOL_name": "body",
    "TIMELINE": {{ "LAYERS": [{{
      "Layer_name": "art",
      "Frames": [
        {{
          "index": 0,
          "duration": 1,
          "elements": [{{ "ATLAS_SPRITE_instance": {{ "name": "body0" }} }}]
        }},
        {{
          "index": 1,
          "duration": 1,
          "elements": [{{ "ATLAS_SPRITE_instance": {{ "name": "body1" }} }}]
        }}
      ]
    }}] }}
  }}] }}
}}"#
    );
    let animation = Animation::parse(json.as_bytes()).unwrap();
    animation
        .flatten_symbol_frame("root", frame_offset)
        .unwrap()
        .remove(0)
        .frame_name
}
