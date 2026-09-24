//! Full-document YAML schema validation for macros.
//!
//! Layer 1 of the Validate/Apply/Import pipeline: generated JSON Schema
//! (`additionalProperties: false`) before serde decode and [`crate::validate_macro`].

use crate::{Result, ValidateError};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use sqyre_domain::macro_document_json_schema;
use sqyre_serialize::{check_yaml_nesting_depth, decode_macro_from_yaml, MAX_YAML_NESTING_DEPTH};
use std::cell::OnceCell;

/// One schema / decode / semantic error with a YAML-ish instance path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlSchemaError {
    pub path: String,
    pub message: String,
    pub layer: YamlValidateLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlValidateLayer {
    Schema,
    Decode,
    Semantic,
}

impl std::fmt::Display for YamlSchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() || self.path == "/" {
            write!(f, "[{:?}] {}", self.layer, self.message)
        } else {
            write!(f, "[{:?}] {}: {}", self.layer, self.path, self.message)
        }
    }
}

/// Aggregate validation result for the editor status line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YamlValidateReport {
    pub errors: Vec<YamlSchemaError>,
}

impl YamlValidateReport {
    pub fn ok() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn summary(&self) -> String {
        match self.errors.as_slice() {
            [] => "Valid".into(),
            [one] => one.to_string(),
            many => format!("{} errors; first: {}", many.len(), many[0]),
        }
    }
}

fn with_compiled_schema<R>(f: impl FnOnce(&jsonschema::Validator) -> R) -> R {
    // `jsonschema` without resolve features uses `Rc` (not Sync) — thread_local
    // keeps a compiled schema per thread and works on wasm32.
    thread_local! {
        static VALIDATOR: OnceCell<jsonschema::Validator> = const { OnceCell::new() };
    }
    VALIDATOR.with(|cell| {
        let validator = cell.get_or_init(|| {
            let schema = macro_document_json_schema();
            jsonschema::options()
                .build(&schema)
                .expect("generated macro schema must compile")
        });
        f(validator)
    })
}

/// Convert YAML values to JSON for the schema validator.
pub fn yaml_to_json(value: &YamlValue) -> Result<JsonValue> {
    serde_json::to_value(yaml_to_json_value(value))
        .map_err(|e| ValidateError::Message(format!("yaml→json: {e}")))
}

fn yaml_to_json_value(value: &YamlValue) -> JsonValue {
    match value {
        YamlValue::Null => JsonValue::Null,
        YamlValue::Bool(b) => JsonValue::Bool(*b),
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                JsonValue::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                JsonValue::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                JsonValue::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into()))
            } else {
                JsonValue::Null
            }
        }
        YamlValue::String(s) => JsonValue::String(s.clone()),
        YamlValue::Sequence(seq) => JsonValue::Array(seq.iter().map(yaml_to_json_value).collect()),
        YamlValue::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    YamlValue::String(s) => s.clone(),
                    other => format!("{other:?}"),
                };
                obj.insert(key, yaml_to_json_value(v));
            }
            JsonValue::Object(obj)
        }
        YamlValue::Tagged(t) => yaml_to_json_value(&t.value),
    }
}

fn format_json_path(path: &str) -> String {
    if path.is_empty() {
        "/".into()
    } else {
        path.to_string()
    }
}

/// Schema-only check on a parsed YAML document.
pub fn validate_macro_yaml_schema(value: &YamlValue) -> YamlValidateReport {
    if let Err(e) = check_yaml_nesting_depth(value) {
        return YamlValidateReport {
            errors: vec![YamlSchemaError {
                path: "/".into(),
                message: format!("{e} (max {MAX_YAML_NESTING_DEPTH})"),
                layer: YamlValidateLayer::Schema,
            }],
        };
    }
    let json = match yaml_to_json(value) {
        Ok(j) => j,
        Err(e) => {
            return YamlValidateReport {
                errors: vec![YamlSchemaError {
                    path: "/".into(),
                    message: e.to_string(),
                    layer: YamlValidateLayer::Schema,
                }],
            };
        }
    };
    let mut errors = Vec::new();
    with_compiled_schema(|validator| {
        for err in validator.iter_errors(&json) {
            errors.push(YamlSchemaError {
                path: format_json_path(err.instance_path().as_str()),
                message: err.to_string(),
                layer: YamlValidateLayer::Schema,
            });
            if errors.len() >= 32 {
                break;
            }
        }
    });
    YamlValidateReport { errors }
}

/// Schema + decode + semantic validation. Does not uniquify the name.
pub fn validate_macro_yaml(yaml: &str) -> Result<sqyre_domain::Macro> {
    let trimmed = yaml.trim();
    if trimmed.is_empty() {
        return Err(ValidateError::Message("YAML document is empty".into()));
    }
    let value: YamlValue = serde_yaml::from_str(trimmed)
        .map_err(|e| ValidateError::Message(format!("YAML parse: {e}")))?;
    let report = validate_macro_yaml_schema(&value);
    if !report.is_ok() {
        return Err(ValidateError::Message(report.summary()));
    }
    let macro_ = decode_macro_from_yaml(trimmed)
        .map_err(|e| ValidateError::Message(format!("decode: {e}")))?;
    crate::validate_macro(&macro_).map_err(|e| ValidateError::Message(format!("semantic: {e}")))?;
    Ok(macro_)
}

/// Full validate report for the editor (collects first failure layer).
pub fn report_macro_yaml(yaml: &str) -> YamlValidateReport {
    let trimmed = yaml.trim();
    if trimmed.is_empty() {
        return YamlValidateReport {
            errors: vec![YamlSchemaError {
                path: "/".into(),
                message: "YAML document is empty".into(),
                layer: YamlValidateLayer::Schema,
            }],
        };
    }
    let value: YamlValue = match serde_yaml::from_str(trimmed) {
        Ok(v) => v,
        Err(e) => {
            return YamlValidateReport {
                errors: vec![YamlSchemaError {
                    path: "/".into(),
                    message: format!("YAML parse: {e}"),
                    layer: YamlValidateLayer::Schema,
                }],
            };
        }
    };
    let schema_report = validate_macro_yaml_schema(&value);
    if !schema_report.is_ok() {
        return schema_report;
    }
    let macro_ = match decode_macro_from_yaml(trimmed) {
        Ok(m) => m,
        Err(e) => {
            return YamlValidateReport {
                errors: vec![YamlSchemaError {
                    path: "/".into(),
                    message: e.to_string(),
                    layer: YamlValidateLayer::Decode,
                }],
            };
        }
    };
    if let Err(e) = crate::validate_macro(&macro_) {
        return YamlValidateReport {
            errors: vec![YamlSchemaError {
                path: "/".into(),
                message: e.to_string(),
                layer: YamlValidateLayer::Semantic,
            }],
        };
    }
    YamlValidateReport::ok()
}

/// Strip markdown fences, reject db.yaml wrappers, then validate.
pub fn prepare_macro_yaml(raw: &str) -> Result<sqyre_domain::Macro> {
    let yaml = strip_yaml_fences(raw);
    if yaml.is_empty() {
        return Err(ValidateError::Message(
            "Paste a macro YAML document first.".into(),
        ));
    }
    if looks_like_db_yaml(&yaml) {
        return Err(ValidateError::Message(
            "Paste a single macro document (with `name:` and `root:`), not a full db.yaml.".into(),
        ));
    }
    let macro_ = validate_macro_yaml(&yaml)?;
    if macro_.name.trim().is_empty() {
        return Err(ValidateError::Message("Macro is missing a name.".into()));
    }
    Ok(macro_)
}

/// Like [`prepare_macro_yaml`] but uniquifies `name` against `existing`.
pub fn prepare_import_macro_yaml(raw: &str, existing: &[String]) -> Result<sqyre_domain::Macro> {
    let mut macro_ = prepare_macro_yaml(raw)?;
    macro_.name = unique_macro_name(&macro_.name, existing);
    Ok(macro_)
}

/// Strip common markdown fences from pasted YAML.
pub fn strip_yaml_fences(raw: &str) -> String {
    let mut s = raw.trim();
    if let Some(rest) = s.strip_prefix("```yaml") {
        s = rest;
    } else if let Some(rest) = s.strip_prefix("```yml") {
        s = rest;
    } else if let Some(rest) = s.strip_prefix("```") {
        s = rest;
    }
    s = s.trim();
    if let Some(stripped) = s.strip_suffix("```") {
        s = stripped.trim();
    }
    s.to_string()
}

fn looks_like_db_yaml(yaml: &str) -> bool {
    let Ok(YamlValue::Mapping(map)) = serde_yaml::from_str(yaml) else {
        return false;
    };
    let has_macros = map.contains_key(YamlValue::String("macros".into()));
    let has_root = map.contains_key(YamlValue::String("root".into()));
    has_macros && !has_root
}

pub fn unique_macro_name(base: &str, existing: &[String]) -> String {
    if !existing.iter().any(|n| n == base) {
        return base.to_string();
    }
    for i in 2.. {
        let candidate = format!("{base} {i}");
        if !existing.iter().any(|n| n == &candidate) {
            return candidate;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{blank_action, ActionKind, Macro, WIRE_TYPE_KEYS};

    // Local helper: encode blank actions via domain + serialize.
    fn encode_blank(type_key: &str) -> String {
        let action = blank_action(type_key).expect(type_key);
        let mut m = Macro::new("stub", 0, vec![]);
        // Root must stay a loop; nest blank under root for non-loop types.
        if matches!(action.kind, ActionKind::Loop { .. }) && type_key == "loop" {
            m.root = action;
            // Ensure root loop name for decode.
            if let ActionKind::Loop { name, .. } = &mut m.root.kind {
                *name = "root".into();
            }
        } else {
            m.root = sqyre_domain::root_loop(vec![action]);
        }
        sqyre_serialize::encode_macro_to_yaml(&m).unwrap()
    }

    #[test]
    fn rejects_unknown_top_level_key() {
        let yaml = r#"
name: bad
root:
  type: loop
  name: root
  count: 1
  subactions: []
extra: true
"#;
        let report = report_macro_yaml(yaml);
        assert!(!report.is_ok(), "{report:?}");
        assert_eq!(report.errors[0].layer, YamlValidateLayer::Schema);
    }

    #[test]
    fn rejects_unknown_action_field() {
        let yaml = r#"
name: bad
root:
  type: loop
  name: root
  count: 1
  subactions:
    - type: wait
      time: 1
      notafield: 1
"#;
        let report = report_macro_yaml(yaml);
        assert!(!report.is_ok());
        assert_eq!(report.errors[0].layer, YamlValidateLayer::Schema);
    }

    #[test]
    fn accepts_minimal_macro() {
        let yaml = r#"
name: ok
root:
  type: loop
  name: root
  count: 1
  subactions: []
"#;
        assert!(validate_macro_yaml(yaml).is_ok());
    }

    #[test]
    fn every_blank_action_schema_and_decode() {
        for key in WIRE_TYPE_KEYS {
            let yaml = encode_blank(key);
            let value: YamlValue = serde_yaml::from_str(&yaml).unwrap();
            let report = validate_macro_yaml_schema(&value);
            assert!(
                report.is_ok(),
                "blank {key} schema failed: {}",
                report.summary()
            );
            decode_macro_from_yaml(&yaml).unwrap_or_else(|e| panic!("blank {key} decode: {e}"));
        }
    }

    #[test]
    fn prepare_import_uniquifies() {
        let yaml = r#"
name: demo
root:
  type: loop
  name: root
  count: 1
  subactions: []
"#;
        let m = prepare_import_macro_yaml(yaml, &["demo".into()]).unwrap();
        assert_eq!(m.name, "demo 2");
    }
}
