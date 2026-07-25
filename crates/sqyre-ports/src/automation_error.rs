//! Typed errors for [`crate::AutomationBackend`] and [`crate::WindowFocuser`].

use thiserror::Error;

/// Failure driving mouse / keyboard / clipboard input or window focus.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AutomationError {
    /// The operation has no implementation on this platform / build.
    #[error("{0}: not supported on this platform")]
    Unsupported(&'static str),
    /// Caller passed an unusable argument (empty path, unknown key, …).
    #[error("invalid argument: {0}")]
    InvalidArg(String),
    /// No window matched the requested executable path + title.
    #[error("no window with title {title:?} from {process_path:?}")]
    WindowNotFound { process_path: String, title: String },
    /// The underlying OS input / window-manager call failed.
    #[error("{0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_not_found_names_target() {
        let msg = AutomationError::WindowNotFound {
            process_path: "/usr/bin/app".into(),
            title: "Title".into(),
        }
        .to_string();
        assert!(msg.contains("/usr/bin/app"), "{msg}");
        assert!(msg.contains("Title"), "{msg}");
    }

    #[test]
    fn unsupported_names_operation() {
        assert_eq!(
            AutomationError::Unsupported("clipboard").to_string(),
            "clipboard: not supported on this platform"
        );
    }
}
