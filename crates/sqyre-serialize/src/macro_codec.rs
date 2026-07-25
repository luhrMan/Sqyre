//! Macro ↔ YAML map / string.

use crate::{Result, SerializeError};
use serde_yaml::{Mapping, Value};
use sqyre_domain::{ActionId, ActionKind, Macro, WIRE_TYPE_KEYS};

/// Reject unknown wire `type` keys under an action tree before serde (untagged
/// enums can otherwise mis-decode into the wrong variant).
fn validate_action_type_keys(value: &Value) -> Result<()> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| SerializeError::msg("expected mapping for action"))?;
    let type_key = mapping
        .get(Value::String("type".into()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| SerializeError::msg("action missing type"))?;
    if !WIRE_TYPE_KEYS.iter().any(|k| *k == type_key) {
        return Err(SerializeError::msg(format!(
            "unknown action type {type_key:?}"
        )));
    }
    for child_key in ["subactions", "elseactions"] {
        if let Some(Value::Sequence(seq)) = mapping.get(Value::String(child_key.into())) {
            for child in seq {
                validate_action_type_keys(child)?;
            }
        }
    }
    Ok(())
}

/// Decode a macro from a YAML mapping.
pub fn decode_macro_from_map(data: &Value) -> Result<Macro> {
    let raw = data
        .as_mapping()
        .ok_or_else(|| SerializeError::msg("expected mapping for macro"))?;
    let name_hint = raw
        .get(Value::String("name".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if let Some(root) = raw.get(Value::String("root".into())) {
        validate_action_type_keys(root)?;
    }
    let mut macro_: Macro = serde_yaml::from_value(Value::Mapping(raw.clone()))?;
    if macro_.name.is_empty() && !name_hint.is_empty() {
        macro_.name = name_hint;
    }
    if matches!(&macro_.root.kind, ActionKind::Loop { name, .. } if name == "root") {
        macro_.root.id = ActionId::root();
    }
    if !matches!(macro_.root.kind, ActionKind::Loop { .. }) {
        return Err(SerializeError::msg(format!(
            "macro \"{}\": root must be a loop",
            macro_.name
        )));
    }
    macro_.hotkey_trigger = Macro::parse_hotkey_trigger(&macro_.hotkey_trigger);
    macro_.init_runtime_variables();
    Ok(macro_)
}

/// Encode a macro to a YAML mapping (lowercase keys).
pub fn encode_macro_to_map(macro_: &Macro) -> Result<Mapping> {
    match serde_yaml::to_value(macro_)? {
        Value::Mapping(m) => Ok(m),
        other => Err(SerializeError::msg(format!(
            "expected mapping, got {other:?}"
        ))),
    }
}

pub fn decode_macro_from_yaml(yaml: &str) -> Result<Macro> {
    let value: Value = serde_yaml::from_str(yaml)?;
    decode_macro_from_map(&value)
}

pub fn encode_macro_to_yaml(macro_: &Macro) -> Result<String> {
    Ok(serde_yaml::to_string(&encode_macro_to_map(macro_)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use sqyre_domain::{Action, ActionKind, MouseButton, PressState, ScalarValue};

    #[test]
    fn roundtrip_wait_and_click() {
        let mut m = Macro::new("demo", 10, vec!["f1".into()]);
        m.root = sqyre_domain::root_loop(vec![
            Action {
                id: ActionId::new(),
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(5),
                },
            },
            Action {
                id: ActionId::new(),
                kind: ActionKind::Click {
                    button: MouseButton::Left,
                    state: PressState::Down,
                },
            },
        ]);
        let map = encode_macro_to_map(&m).unwrap();
        let restored = decode_macro_from_map(&Value::Mapping(map)).unwrap();
        assert_eq!(restored.name, "demo");
        assert_eq!(restored.global_delay, 10);
        assert_eq!(restored.root.children().len(), 2);
    }

    #[test]
    fn decode_fixture_macro_shape() {
        let yaml = r#"
name: wait-var
globaldelay: 0
keyboarddelay: 25
mousedelay: 25
hotkey: []
variables:
  - name: delay
    type: number
    initialvalue: "3"
    description: ""
root:
  type: loop
  name: root
  count: 1
  subactions:
    - type: wait
      time: ${delay}
    - type: loopjump
      mode: break
"#;
        let m = decode_macro_from_yaml(yaml).unwrap();
        assert_eq!(m.name, "wait-var");
        assert_eq!(m.variable_decls.len(), 1);
        assert_eq!(m.root.children().len(), 2);
    }

    #[test]
    fn rejects_unknown_action_type() {
        let yaml = r#"
name: bad
globaldelay: 0
keyboarddelay: 0
mousedelay: 0
hotkey: []
variables: []
root:
  type: loop
  name: root
  count: 1
  subactions:
    - type: notarealaction
      time: 1
"#;
        let err = decode_macro_from_yaml(yaml).unwrap_err().to_string();
        assert!(
            err.contains("unknown action type") && err.contains("notarealaction"),
            "{err}"
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn yaml_roundtrip_preserves_name_and_child_count(
            name in "[a-zA-Z][a-zA-Z0-9_ ]{0,24}",
            wait_ms in 0i64..10_000,
        ) {
            let mut m = Macro::new(name.trim(), 0, vec![]);
            prop_assume!(!m.name.is_empty());
            m.root = sqyre_domain::root_loop(vec![Action {
                id: ActionId::new(),
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(wait_ms),
                },
            }]);
            let yaml = encode_macro_to_yaml(&m).expect("encode yaml");
            let restored = decode_macro_from_yaml(&yaml).expect("decode yaml");
            prop_assert_eq!(&restored.name, &m.name);
            prop_assert_eq!(restored.root.children().len(), m.root.children().len());
            match &restored.root.children()[0].kind {
                ActionKind::Wait { time } => {
                    prop_assert_eq!(time, &ScalarValue::Int(wait_ms));
                }
                other => prop_assert!(false, "expected Wait, got {other:?}"),
            }
        }
    }
}
