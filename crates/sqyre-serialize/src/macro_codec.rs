use crate::action::{action_from_map, action_to_map};
use crate::helpers::*;
use crate::{Result, SerializeError};
use serde_yaml::{Mapping, Value};
use sqyre_domain::{
    ActionKind, Macro, VariableDecl, VariableType, DEFAULT_KEYBOARD_DELAY, DEFAULT_MOUSE_DELAY,
};

fn decode_macro_from_map_inner(raw: &Mapping, name_hint: &str) -> Result<Macro> {
    let name = {
        let n = string_from_map(raw, "name");
        if n.is_empty() {
            name_hint.to_string()
        } else {
            n
        }
    };
    let mut macro_ = Macro::new(&name, 0, Vec::new());

    macro_.global_delay = optional_int(raw, "globaldelay").unwrap_or(0);
    macro_.keyboard_delay = optional_int(raw, "keyboarddelay").unwrap_or(DEFAULT_KEYBOARD_DELAY);
    macro_.mouse_delay = optional_int(raw, "mousedelay").unwrap_or(DEFAULT_MOUSE_DELAY);

    macro_.hotkey = raw
        .get(Value::String("hotkey".into()))
        .map(string_slice_from_value)
        .unwrap_or_default();

    macro_.hotkey_trigger = Macro::parse_hotkey_trigger(&string_from_map(raw, "hotkey_trigger"));

    macro_.tags = raw
        .get(Value::String("tags".into()))
        .map(string_slice_from_value)
        .unwrap_or_default();

    if let Some(v) = raw.get(Value::String("variables".into())) {
        macro_.variable_decls = decode_variable_decls(v)?;
    }

    let root_raw = raw
        .get(Value::String("root".into()))
        .ok_or_else(|| SerializeError::msg(format!("macro \"{name}\": missing or invalid root")))?;
    let root_map = as_mapping(root_raw)
        .map_err(|_| SerializeError::msg(format!("macro \"{name}\": missing or invalid root")))?;
    let root = action_from_map(root_map)
        .map_err(|e| SerializeError::msg(format!("macro \"{name}\" root: {e}")))?;
    if !matches!(root.kind, ActionKind::Loop { .. }) {
        return Err(SerializeError::msg(format!(
            "macro \"{name}\": root must be a loop"
        )));
    }
    macro_.root = root;
    macro_.init_runtime_variables();
    Ok(macro_)
}

/// Decode a macro from a YAML mapping.
pub fn decode_macro_from_map(data: &Value) -> Result<Macro> {
    let raw = as_mapping(data)?;
    decode_macro_from_map_inner(raw, "")
}

fn decode_variable_decls(v: &Value) -> Result<Vec<VariableDecl>> {
    let Some(seq) = v.as_sequence() else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for item in seq {
        let Some(m) = item.as_mapping() else {
            continue;
        };
        out.push(VariableDecl {
            name: string_from_map(m, "name"),
            type_: VariableType::parse(&string_from_map(m, "type")),
            initial_value: string_from_map(m, "initialvalue"),
            description: string_from_map(m, "description"),
        });
    }
    Ok(out)
}

/// Encode a macro to a YAML mapping (lowercase keys).
pub fn encode_macro_to_map(macro_: &Macro) -> Result<Mapping> {
    let mut m = Mapping::new();
    insert_str(&mut m, "name", &macro_.name);
    insert_i32(&mut m, "globaldelay", macro_.global_delay);
    insert_i32(&mut m, "keyboarddelay", macro_.keyboard_delay);
    insert_i32(&mut m, "mousedelay", macro_.mouse_delay);
    insert(
        &mut m,
        "hotkey",
        Value::Sequence(macro_.hotkey.iter().cloned().map(Value::String).collect()),
    );
    if !macro_.hotkey_trigger.is_empty() {
        insert_str(&mut m, "hotkey_trigger", &macro_.hotkey_trigger);
    }
    insert(
        &mut m,
        "tags",
        Value::Sequence(macro_.tags.iter().cloned().map(Value::String).collect()),
    );
    let mut vars = serde_yaml::Sequence::new();
    for d in &macro_.variable_decls {
        let mut vm = Mapping::new();
        insert_str(&mut vm, "name", &d.name);
        insert_str(&mut vm, "type", d.type_.as_str());
        insert_str(&mut vm, "initialvalue", &d.initial_value);
        insert_str(&mut vm, "description", &d.description);
        vars.push(Value::Mapping(vm));
    }
    insert(&mut m, "variables", Value::Sequence(vars));
    insert(&mut m, "root", Value::Mapping(action_to_map(&macro_.root)?));
    Ok(m)
}

pub fn decode_macro_from_yaml(yaml: &str) -> Result<Macro> {
    let value: Value = serde_yaml::from_str(yaml)?;
    decode_macro_from_map(&value)
}

pub fn encode_macro_to_yaml(macro_: &Macro) -> Result<String> {
    let map = encode_macro_to_map(macro_)?;
    Ok(serde_yaml::to_string(&Value::Mapping(map))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{action_from_map, action_to_map};
    use sqyre_domain::{root_loop, Action, ActionKind, ScalarValue};

    #[test]
    fn roundtrip_wait_and_click() {
        let root = root_loop(vec![
            Action {
                id: sqyre_domain::ActionId::new(),
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(100),
                },
            },
            Action {
                id: sqyre_domain::ActionId::new(),
                kind: ActionKind::Click {
                    button: "right".into(),
                    state: true,
                },
            },
        ]);
        let map = action_to_map(&root).unwrap();
        let back = action_from_map(&map).unwrap();
        assert_eq!(back.children().len(), 2);
        assert_eq!(back.type_key(), "loop");
    }

    #[test]
    fn decode_fixture_macro_shape() {
        let yaml = r#"
name: wait-var
globaldelay: 50
hotkey: [ctrl, a]
variables:
  - name: delay
    type: number
    initialvalue: "100"
    description: ""
root:
  type: loop
  name: root
  count: 1
  subactions:
    - type: wait
      time: ${delay}
    - type: break
"#;
        let m = decode_macro_from_yaml(yaml).unwrap();
        assert_eq!(m.name, "wait-var");
        assert_eq!(m.global_delay, 50);
        assert_eq!(m.variable_decls.len(), 1);
        assert_eq!(m.root.children().len(), 2);
    }
}
