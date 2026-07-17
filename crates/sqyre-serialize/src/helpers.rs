use crate::{Result, SerializeError};
use serde_yaml::{Mapping, Value};
use sqyre_domain::{CoordinateRef, ScalarValue};

pub fn string_from_map(m: &Mapping, key: &str) -> String {
    m.get(Value::String(key.into()))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn expect_string(m: &Mapping, key: &str) -> Result<String> {
    match m.get(Value::String(key.into())) {
        None | Some(Value::Null) => Err(SerializeError::msg(format!("missing field \"{key}\""))),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(SerializeError::msg(format!(
            "field \"{key}\": expected string, got {other:?}"
        ))),
    }
}

pub fn expect_bool(m: &Mapping, key: &str) -> Result<bool> {
    match m.get(Value::String(key.into())) {
        None | Some(Value::Null) => Err(SerializeError::msg(format!("missing field \"{key}\""))),
        Some(Value::Bool(b)) => Ok(*b),
        Some(other) => Err(SerializeError::msg(format!(
            "field \"{key}\": expected bool, got {other:?}"
        ))),
    }
}

pub fn bool_from_map(m: &Mapping, key: &str) -> bool {
    matches!(m.get(Value::String(key.into())), Some(Value::Bool(true)))
}

pub fn int_from_value(v: &Value) -> i32 {
    match v {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().map(|u| u as i64))
            .unwrap_or(0) as i32,
        _ => 0,
    }
}

pub fn float_from_value(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

pub fn optional_int(m: &Mapping, key: &str) -> Option<i32> {
    m.get(Value::String(key.into()))
        .filter(|v| !v.is_null())
        .map(int_from_value)
}

pub fn string_slice_from_value(v: &Value) -> Vec<String> {
    match v {
        Value::Sequence(seq) => seq
            .iter()
            .filter_map(|e| e.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

pub fn parse_coordinate_ref(v: Option<&Value>) -> CoordinateRef {
    match v {
        None | Some(Value::Null) => CoordinateRef::default(),
        Some(Value::String(s)) => CoordinateRef(s.clone()),
        Some(Value::Mapping(m)) => CoordinateRef(string_from_map(m, "name")),
        _ => CoordinateRef::default(),
    }
}

pub fn coordinate_ref_to_value(r: &CoordinateRef) -> Value {
    if r.is_empty() {
        Value::Null
    } else {
        Value::String(r.0.clone())
    }
}

pub fn scalar_from_value(v: Option<&Value>) -> ScalarValue {
    match v {
        None | Some(Value::Null) => ScalarValue::Null,
        Some(v) => ScalarValue::from_yaml(v),
    }
}

pub fn insert(m: &mut Mapping, key: &str, value: Value) {
    m.insert(Value::String(key.into()), value);
}

pub fn insert_str(m: &mut Mapping, key: &str, value: impl Into<String>) {
    insert(m, key, Value::String(value.into()));
}

pub fn insert_bool(m: &mut Mapping, key: &str, value: bool) {
    insert(m, key, Value::Bool(value));
}

pub fn insert_i32(m: &mut Mapping, key: &str, value: i32) {
    insert(m, key, Value::Number(value.into()));
}

pub fn insert_f64(m: &mut Mapping, key: &str, value: f64) {
    insert(m, key, Value::Number(serde_yaml::Number::from(value)));
}

pub fn as_mapping(v: &Value) -> Result<&Mapping> {
    v.as_mapping()
        .ok_or_else(|| SerializeError::msg(format!("expected mapping, got {v:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_int_and_coercion() {
        let mut m = Mapping::new();
        insert_i32(&mut m, "a", 42);
        insert(&mut m, "b", Value::String("nope".into()));
        assert_eq!(optional_int(&m, "a"), Some(42));
        assert_eq!(optional_int(&m, "missing"), None);
        assert_eq!(optional_int(&m, "b"), Some(0));
    }

    #[test]
    fn parse_coordinate_ref_shapes() {
        assert!(parse_coordinate_ref(None).is_empty());
        assert_eq!(
            parse_coordinate_ref(Some(&Value::String("Game~Spot".into()))).as_str(),
            "Game~Spot"
        );
        let mut nested = Mapping::new();
        insert_str(&mut nested, "name", "Arena");
        assert_eq!(
            parse_coordinate_ref(Some(&Value::Mapping(nested))).as_str(),
            "Arena"
        );
    }

    #[test]
    fn expect_string_errors() {
        let m = Mapping::new();
        assert!(expect_string(&m, "x").is_err());
        let mut m = Mapping::new();
        insert(&mut m, "x", Value::Bool(true));
        assert!(expect_string(&m, "x")
            .unwrap_err()
            .to_string()
            .contains("string"));
    }
}
