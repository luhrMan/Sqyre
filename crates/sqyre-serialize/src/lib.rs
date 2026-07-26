//! Encode/decode macros and actions via typed serde on domain types.
//!
//! Public map/YAML helpers wrap `serde_yaml::{to_value,from_value}` so
//! clipboard, undo, and persist keep the same API.

mod action;
mod macro_codec;

pub use action::{action_from_map, action_to_map, action_to_map_with_uid};
pub use macro_codec::{
    decode_macro_from_map, decode_macro_from_yaml, encode_macro_to_map, encode_macro_to_yaml,
};

use serde_yaml::Value;
use sqyre_domain::WIRE_TYPE_KEYS;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerializeError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, SerializeError>;

impl SerializeError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

/// Max nesting depth for YAML mappings/sequences (DoS guard). Shared by
/// action/macro decode paths here and by `sqyre-persist`'s `db.yaml` loader.
pub const MAX_YAML_NESTING_DEPTH: usize = 64;

/// Walk `value` and error if mapping/sequence nesting exceeds
/// [`MAX_YAML_NESTING_DEPTH`].
pub fn check_yaml_nesting_depth(value: &Value) -> Result<()> {
    fn walk(v: &Value, depth: usize) -> Result<()> {
        if depth > MAX_YAML_NESTING_DEPTH {
            return Err(SerializeError::msg(format!(
                "yaml nesting too deep (max {MAX_YAML_NESTING_DEPTH})"
            )));
        }
        match v {
            Value::Mapping(m) => {
                for (_, child) in m {
                    walk(child, depth + 1)?;
                }
            }
            Value::Sequence(s) => {
                for child in s {
                    walk(child, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    walk(value, 0)
}

/// Reject unknown wire `type` keys under an action tree before serde
/// (untagged enums can otherwise mis-decode into the wrong variant, or
/// serde's error text can leak internal shapes rather than naming the
/// action). Shared by [`action::action_from_map`] and
/// [`macro_codec::decode_macro_from_map`].
pub(crate) fn validate_action_type_keys(value: &Value) -> Result<()> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| SerializeError::msg("expected mapping for action"))?;
    let type_key = mapping
        .get(Value::String("type".into()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| SerializeError::msg("action missing type"))?;
    if !WIRE_TYPE_KEYS.contains(&type_key) {
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
