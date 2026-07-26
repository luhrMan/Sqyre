//! Shared typed error for domain-coupled ports (`CoordinateResolver`, `OcrEngine`,
//! `ContinueKeyWaiter` in `sqyre-executor`) and `ProgramCatalog` coordinate /
//! collection / atlas lookups in `sqyre-persist`.
//!
//! These ports live outside `sqyre-ports` because they take domain types
//! (`CoordinateRef`, `Macro`, …), but their failure modes are the same handful
//! of shapes, so they share one error type here rather than each growing its
//! own stringly `Result<_, String>`.

use thiserror::Error;

/// Failure resolving a coordinate/collection/atlas reference, running OCR, or
/// waiting on a key chord.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PortError {
    /// A named program / point / search area / collection / atlas does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// The port has no backing implementation for this operation (default trait
    /// methods, build without hooks, etc).
    #[error("not configured: {0}")]
    NotConfigured(String),
    /// Caller passed an unusable argument or reference shape.
    #[error("invalid: {0}")]
    Invalid(String),
    /// The run was stopped while waiting (continue-key / chord wait).
    #[error("stopped")]
    Stopped,
    /// Catch-all for backend-specific failures (OCR engine, lock poisoning, …).
    #[error("{0}")]
    Message(String),
}

impl PortError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn not_configured(msg: impl Into<String>) -> Self {
        Self::NotConfigured(msg.into())
    }

    pub fn invalid(msg: impl Into<String>) -> Self {
        Self::Invalid(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_has_fixed_message() {
        assert_eq!(PortError::Stopped.to_string(), "stopped");
    }

    #[test]
    fn constructors_wrap_message() {
        assert_eq!(
            PortError::not_found("program \"P\"").to_string(),
            "not found: program \"P\""
        );
        assert_eq!(
            PortError::not_configured("atlas lookup").to_string(),
            "not configured: atlas lookup"
        );
        assert_eq!(
            PortError::invalid("empty ref").to_string(),
            "invalid: empty ref"
        );
    }
}
