//! Persist error type (leaf so child modules need not import through `lib.rs`).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Serialize(#[from] sqyre_serialize::SerializeError),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, PersistError>;
