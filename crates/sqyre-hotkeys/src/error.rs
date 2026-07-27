//! Typed errors for global hotkey installation.

use thiserror::Error;

/// Failure starting the platform hotkey hook thread.
#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("hotkey thread: {0}")]
    ThreadSpawn(String),
    #[error("{0}")]
    Install(String),
}
