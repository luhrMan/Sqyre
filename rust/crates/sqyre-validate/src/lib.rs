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

pub fn parse_positive_i32(s: &str) -> Result<i32> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ValidateError::Message("must be a positive integer".into()));
    }
    let v: i32 = s
        .parse()
        .map_err(|_| ValidateError::Message(format!("must be a positive integer: {s:?}")))?;
    if v <= 0 {
        return Err(ValidateError::Message(format!(
            "must be a positive integer: {s:?}"
        )));
    }
    Ok(v)
}

pub fn parse_non_negative_i32(s: &str) -> Result<i32> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ValidateError::Message(
            "must be a non-negative integer".into(),
        ));
    }
    let v: i32 = s.parse().map_err(|_| {
        ValidateError::Message(format!("must be a non-negative integer: {s:?}"))
    })?;
    if v < 0 {
        return Err(ValidateError::Message(format!(
            "must be a non-negative integer: {s:?}"
        )));
    }
    Ok(v)
}

/// Item grid cols/rows > 0 and stack_max ≥ 0.
pub fn validate_item_grid_fields(cols: &str, rows: &str, stack_max: &str) -> Result<()> {
    parse_positive_i32(cols).map_err(|e| ValidateError::Message(format!("cols: {e}")))?;
    parse_positive_i32(rows).map_err(|e| ValidateError::Message(format!("rows: {e}")))?;
    parse_non_negative_i32(stack_max)
        .map_err(|e| ValidateError::Message(format!("stack max: {e}")))?;
    Ok(())
}

/// When all four coords are numeric literals, require positive width/height.
/// Variable refs (`${…}`) skip the bounds check.
pub fn validate_search_area_literal_bounds(left: &str, top: &str, right: &str, bottom: &str) -> Result<()> {
    let Some(lx) = parse_coord_literal(left) else {
        return Ok(());
    };
    let Some(ty) = parse_coord_literal(top) else {
        return Ok(());
    };
    let Some(rx) = parse_coord_literal(right) else {
        return Ok(());
    };
    let Some(by) = parse_coord_literal(bottom) else {
        return Ok(());
    };
    let (lx, rx) = if lx <= rx { (lx, rx) } else { (rx, lx) };
    let (ty, by) = if ty <= by { (ty, by) } else { (by, ty) };
    let w = rx - lx;
    let h = by - ty;
    if w <= 0 || h <= 0 {
        return Err(ValidateError::Message(format!(
            "invalid search area (width={w} height={h}); need positive dimensions"
        )));
    }
    if w > 1 << 16 || h > 1 << 16 {
        return Err(ValidateError::Message(format!(
            "search area dimensions too large ({w}x{h})"
        )));
    }
    Ok(())
}

fn parse_coord_literal(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() || sqyre_varref::contains(s) {
        return None;
    }
    if let Ok(i) = s.parse::<i32>() {
        return Some(i);
    }
    s.parse::<f64>().ok().map(|f| f as i32)
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

    #[test]
    fn item_grid_and_search_area_bounds() {
        assert!(validate_item_grid_fields("2", "3", "0").is_ok());
        assert!(validate_item_grid_fields("0", "3", "0").is_err());
        assert!(validate_item_grid_fields("2", "3", "-1").is_err());
        assert!(validate_search_area_literal_bounds("0", "0", "10", "10").is_ok());
        assert!(validate_search_area_literal_bounds("10", "10", "10", "10").is_err());
        assert!(validate_search_area_literal_bounds("${a}", "0", "10", "10").is_ok());
    }
}
