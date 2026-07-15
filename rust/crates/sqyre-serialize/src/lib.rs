//! Encode/decode macros and actions for YAML / map round-trips.
//!
//! Field names match Go `internal/models/serialize` codecs (including tip
//! `while` / `navigateselect` and tolerant ImageSearch loading).

mod action;
mod helpers;
mod macro_codec;

pub use action::{action_from_map, action_to_map, action_to_map_with_uid};
pub use macro_codec::{decode_macro_from_map, decode_macro_from_yaml, encode_macro_to_map, encode_macro_to_yaml};

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
