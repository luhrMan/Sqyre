//! Encode/decode a single action (and subtree) to/from YAML mappings.

use crate::{Result, SerializeError};
use serde_yaml::{Mapping, Value};
use sqyre_domain::{Action, ActionId, ActionKind};
use uuid::Uuid;

/// Encode an action (and subtree) to a YAML mapping.
pub fn action_to_map(action: &Action) -> Result<Mapping> {
    value_as_mapping(serde_yaml::to_value(action)?)
}

/// Encode including `uid` on every node (for undo/clipboard snapshots).
///
/// Normal [`action_to_map`] omits UIDs so copy/paste gets fresh identities;
/// undo must restore stable IDs, so this walks the live tree and injects them.
pub fn action_to_map_with_uid(action: &Action) -> Result<Mapping> {
    let mut m = action_to_map(action)?;
    inject_action_uid(&mut m, action);
    Ok(m)
}

fn inject_action_uid(m: &mut Mapping, action: &Action) {
    if !action.id.is_root() {
        m.insert(
            Value::String("uid".into()),
            Value::String(action.id.as_str()),
        );
    }
    let Some(Value::Sequence(seq)) = m.get_mut(Value::String("subactions".into())) else {
        return;
    };
    for (i, child) in action.children().iter().enumerate() {
        if let Some(Value::Mapping(sub)) = seq.get_mut(i) {
            inject_action_uid(sub, child);
        }
    }
}

/// Decode an action from a YAML mapping. Assigns a new UID unless `uid` is set.
pub fn action_from_map(raw: &Mapping) -> Result<Action> {
    let type_name = raw
        .get(Value::String("type".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if type_name.is_empty() {
        return Err(SerializeError::msg("missing field \"type\""));
    }
    let mut action: Action = serde_yaml::from_value(Value::Mapping(raw.clone())).map_err(|e| {
        if e.to_string().contains("untagged") || e.to_string().contains("data did not match") {
            SerializeError::msg(format!("unknown action type {type_name}"))
        } else {
            SerializeError::Yaml(e)
        }
    })?;
    action.id = restore_uid(raw, type_name, &action.kind);
    Ok(action)
}

fn restore_uid(raw: &Mapping, type_name: &str, kind: &ActionKind) -> ActionId {
    if let Some(Value::String(uid)) = raw.get(Value::String("uid".into())) {
        if let Ok(u) = Uuid::parse_str(uid) {
            return ActionId(u);
        }
    }
    if type_name == "loop" {
        if let ActionKind::Loop { name, .. } = kind {
            if name == "root" {
                return ActionId::root();
            }
        }
    }
    // Prefer id already deserialized from `uid` when present; else new.
    ActionId::new()
}

fn value_as_mapping(v: Value) -> Result<Mapping> {
    match v {
        Value::Mapping(m) => Ok(m),
        other => Err(SerializeError::msg(format!(
            "expected mapping, got {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{
        root_loop, CoordinateOutputs, CoordinateRef, DetectionBranch, NavigateSelectData,
        ScalarValue,
    };

    #[test]
    fn action_to_map_with_uid_preserves_nested_uids() {
        let child = Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        };
        let child_id = child.id;
        let nested = Action {
            id: ActionId::new(),
            kind: ActionKind::Loop {
                name: "inner".into(),
                count: ScalarValue::Int(1),
                subactions: vec![child],
            },
        };
        let nested_id = nested.id;
        let root = root_loop(vec![nested]);
        let m = action_to_map_with_uid(&root).unwrap();
        let restored = action_from_map(&m).unwrap();
        assert_eq!(restored.children()[0].id, nested_id);
        assert_eq!(restored.children()[0].children()[0].id, child_id);
    }

    #[test]
    fn navigate_select_with_key_branch_roundtrips() {
        let kid = Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        };
        let branch = Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateKey {
                name: "Inspect".into(),
                chord: vec!["i".into()],
                exit: true,
                subactions: vec![kid],
            },
        };
        let branch_id = branch.id;
        let nav = Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateSelect(Box::new(NavigateSelectData {
                program: "P".into(),
                graph_name: "bag".into(),
                chords: sqyre_domain::NavChords {
                    up: vec!["up".into()],
                    down: vec!["down".into()],
                    left: vec![],
                    right: vec![],
                    select: vec!["enter".into()],
                    back: vec!["esc".into()],
                },
                options: sqyre_domain::NavOptions {
                    wrap_edges: true,
                    move_cursor_with_nav: true,
                    smooth: false,
                    pass_through: false,
                    hold_repeat: true,
                },
                select: sqyre_domain::NavSelectAction {
                    device: "mouse".into(),
                    button: "left".into(),
                    key: String::new(),
                    press_mode: "click".into(),
                },
                inputs: sqyre_domain::NavInputs::default(),
                outputs: sqyre_domain::NavOutputs {
                    output_ref: "ref".into(),
                    output_graph: String::new(),
                    output_row: "r".into(),
                    output_col: "c".into(),
                    output_collection: String::new(),
                },
                subactions: vec![branch],
            })),
        };
        let nav_id = nav.id;
        let m = action_to_map_with_uid(&nav).unwrap();
        let restored = action_from_map(&m).unwrap();
        assert_eq!(restored.id, nav_id);
        assert!(restored.is_branch());
        match &restored.kind {
            ActionKind::NavigateSelect(data) => {
                assert_eq!(data.program, "P");
                assert!(data.options.wrap_edges);
                assert!(data.options.hold_repeat);
                assert_eq!(data.subactions.len(), 1);
                assert_eq!(data.subactions[0].id, branch_id);
                match &data.subactions[0].kind {
                    ActionKind::NavigateKey {
                        name,
                        chord,
                        exit,
                        subactions: kids,
                    } => {
                        assert_eq!(name, "Inspect");
                        assert_eq!(chord, &vec!["i".to_string()]);
                        assert!(*exit);
                        assert_eq!(kids.len(), 1);
                    }
                    other => panic!("expected NavigateKey, got {other:?}"),
                }
            }
            other => panic!("expected NavigateSelect, got {other:?}"),
        }
    }

    #[test]
    fn blank_action_kinds_roundtrip_encode_decode() {
        use sqyre_domain::action_templates;
        for tmpl in action_templates() {
            let action = tmpl.create();
            let encoded = action_to_map(&action).unwrap();
            let decoded = action_from_map(&encoded).unwrap();
            assert_eq!(
                decoded.type_key(),
                tmpl.action_type,
                "type_key mismatch for {}",
                tmpl.action_type
            );
            let reencoded = action_to_map(&decoded).unwrap();
            let redecoded = action_from_map(&reencoded).unwrap();
            assert_eq!(
                decoded.kind, redecoded.kind,
                "encode/decode not idempotent for {}",
                tmpl.action_type
            );
        }
    }

    #[test]
    fn decode_rejects_missing_type() {
        let mut m = Mapping::new();
        m.insert(Value::String("name".into()), Value::String("x".into()));
        let err = action_from_map(&m).unwrap_err();
        assert!(err.to_string().contains("type"), "{err}");
    }

    #[test]
    fn decode_rejects_unknown_type() {
        let mut m = Mapping::new();
        m.insert(
            Value::String("type".into()),
            Value::String("notarealaction".into()),
        );
        let err = action_from_map(&m).unwrap_err();
        assert!(
            err.to_string().to_ascii_lowercase().contains("unknown")
                || err.to_string().contains("notarealaction"),
            "{err}"
        );
    }

    #[test]
    fn image_search_populated_fields_roundtrip() {
        use sqyre_domain::{MatchOrder, RepeatMode, WaitTilFoundConfig};
        let action = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "find sword".into(),
                targets: vec!["Game~Sword".into(), "Game~Shield".into()],
                search_area: CoordinateRef("Game~Arena".into()),
                tolerance: 0.87,
                blur: 3,
                detection: DetectionBranch {
                    wait: WaitTilFoundConfig {
                        repeat_mode: RepeatMode::WaitUntilFound,
                        wait_til_found_seconds: 5,
                        wait_til_found_interval_ms: 100,
                        max_iterations: 0,
                    },
                    coords: CoordinateOutputs {
                        output_x_variable: "sx".into(),
                        output_y_variable: "sy".into(),
                    },
                    run_branch_on_no_find: true,
                    order: MatchOrder {
                        grouping: "row".into(),
                        horizontal: "ltr".into(),
                        vertical: "ttb".into(),
                    },
                    subactions: vec![Action {
                        id: ActionId::new(),
                        kind: ActionKind::Wait {
                            time: ScalarValue::Int(1),
                        },
                    }],
                },
            },
        };
        let map = action_to_map_with_uid(&action).unwrap();
        assert!(
            !map.contains_key(Value::String("detection".into())),
            "detection must be flattened"
        );
        assert_eq!(
            map.get(Value::String("repeatmode".into()))
                .and_then(|v| v.as_str()),
            Some("waituntilfound")
        );
        let restored = action_from_map(&map).unwrap();
        assert_eq!(restored.kind, action.kind);
    }

    #[test]
    fn while_found_preserves_max_iterations() {
        use sqyre_domain::{RepeatMode, WaitTilFoundConfig};
        let action = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "x".into(),
                targets: vec!["t".into()],
                search_area: CoordinateRef::default(),
                tolerance: 0.0,
                blur: 5,
                detection: DetectionBranch {
                    wait: WaitTilFoundConfig {
                        repeat_mode: RepeatMode::WhileFound,
                        wait_til_found_seconds: 0,
                        wait_til_found_interval_ms: 0,
                        max_iterations: 42,
                    },
                    coords: CoordinateOutputs::defaults(),
                    run_branch_on_no_find: false,
                    order: Default::default(),
                    subactions: Vec::new(),
                },
            },
        };
        let map = action_to_map(&action).unwrap();
        let restored = action_from_map(&map).unwrap();
        match restored.kind {
            ActionKind::ImageSearch { detection, .. } => {
                assert_eq!(detection.wait.max_iterations, 42);
            }
            other => panic!("expected ImageSearch, got {other:?}"),
        }
    }

    #[test]
    fn image_search_targets_roundtrip() {
        let yaml = r#"
type: imagesearch
name: find
targets: [Game~Sword]
searcharea: Game~Arena
tolerance: 0.5
blur: 5
"#;
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let map = value.as_mapping().unwrap();
        let action = action_from_map(map).unwrap();
        match action.kind {
            ActionKind::ImageSearch {
                targets,
                search_area,
                ..
            } => {
                assert_eq!(targets, vec!["Game~Sword".to_string()]);
                assert_eq!(search_area.as_str(), "Game~Arena");
            }
            other => panic!("expected ImageSearch, got {other:?}"),
        }
    }
}
