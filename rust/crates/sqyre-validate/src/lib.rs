//! Validation helpers (Go `internal/validation`).

use sqyre_domain::{Action, ActionKind};
use sqyre_varref;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidateError {
    #[error("name cannot be empty")]
    EmptyName,
    #[error("invalid variable: {0}")]
    InvalidVariable(String),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, ValidateError>;

pub fn validate_entity_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        Err(ValidateError::EmptyName)
    } else {
        Ok(())
    }
}

pub fn validate_variable_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ValidateError::EmptyName);
    }
    if name.contains(['$', '{', '}']) {
        return Err(ValidateError::InvalidVariable(
            "must not contain $, {, or }".into(),
        ));
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(ValidateError::InvalidVariable(
            "must not contain control characters".into(),
        ));
    }
    Ok(())
}

/// True when `name` looks like an expression rather than a plain identifier.
pub fn looks_like_expression(name: &str) -> bool {
    let t = name.trim();
    if t.is_empty() {
        return false;
    }
    t.contains(['+', '-', '*', '/', '(', ')', '%']) || sqyre_varref::contains(t)
}

pub fn validate_variable_assignment_name(name: &str) -> Result<()> {
    validate_variable_name(name)?;
    if looks_like_expression(name) {
        return Err(ValidateError::InvalidVariable(
            "must be a simple variable name, not an expression".into(),
        ));
    }
    Ok(())
}

/// Checks minimum fields required to save/run an action (subset of Go ValidateAction).
pub fn validate_action(action: &Action) -> Result<()> {
    match &action.kind {
        ActionKind::Key { key, .. } => {
            if key.trim().is_empty() {
                return Err(ValidateError::Message(
                    "key: record a key before saving".into(),
                ));
            }
        }
        ActionKind::Calculate { expression, .. } => {
            if expression.trim().is_empty() {
                return Err(ValidateError::Message(
                    "calculate: expression cannot be empty".into(),
                ));
            }
        }
        ActionKind::SetVariable { variable_name, .. } => {
            validate_variable_assignment_name(variable_name).map_err(|e| {
                ValidateError::Message(format!("set variable: {e}"))
            })?;
        }
        ActionKind::Pause { continue_key, .. } => {
            if continue_key.is_empty() {
                return Err(ValidateError::Message(
                    "pause: continue key not set".into(),
                ));
            }
            let normalized: Vec<_> = continue_key
                .iter()
                .map(|k| k.trim().to_ascii_lowercase())
                .collect();
            let mut failsafe = vec![
                "esc".to_string(),
                "ctrl".to_string(),
                "shift".to_string(),
            ];
            failsafe.sort();
            let mut sorted = normalized.clone();
            sorted.sort();
            if sorted == failsafe {
                return Err(ValidateError::Message(
                    "pause: continue key cannot match the failsafe hotkey (esc + ctrl + shift)"
                        .into(),
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_name_rejects_braces() {
        assert!(validate_variable_name("${x}").is_err());
        assert!(validate_variable_name("ok").is_ok());
    }

    #[test]
    fn assignment_rejects_expression() {
        assert!(validate_variable_assignment_name("a+1").is_err());
    }
}
