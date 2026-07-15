//! Validation helpers (Go `internal/validation` + `macro` entry validation).

use sqyre_domain::{
    collect_known_variable_names, evaluate_expression, Action, ActionKind, Macro, ScalarValue,
};
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
pub fn validate_search_area_literal_bounds(
    left: &str,
    top: &str,
    right: &str,
    bottom: &str,
) -> Result<()> {
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

/// True when `name` looks like an expression rather than a plain identifier
/// (assignment-name check; Go `LooksLikeExpression` for names is broader — see
/// [`looks_like_arithmetic`]).
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

/// Outcome of validating a variable entry value in the UI (Go `EntryValidation`).
/// Warnings (unknown `${var}`) do not block submit; errors do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryValidation {
    pub warning: String,
    pub error: String,
}

impl EntryValidation {
    pub fn blocks_submit(&self) -> bool {
        !self.error.is_empty()
    }
}

/// Warning when text references undefined variables (Go `UnknownVariableWarning`).
pub fn unknown_variable_warning(text: &str, macro_: Option<&Macro>) -> String {
    let Some(macro_) = macro_ else {
        return String::new();
    };
    if text.trim().is_empty() {
        return String::new();
    }
    let known = collect_known_variable_names(macro_);
    let mut unknown: Vec<String> = Vec::new();
    for name in sqyre_varref::names(text) {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if !known.contains(&name.to_ascii_lowercase()) {
            unknown.push(name.to_string());
        }
    }
    unknown.sort();
    unknown.dedup();
    match unknown.as_slice() {
        [] => String::new(),
        [one] => format!("unknown variable {one:?}"),
        many => format!("unknown variables: {}", many.join(", ")),
    }
}

/// Whether text will be evaluated as arithmetic at runtime (re-exported from domain).
pub use sqyre_domain::looks_like_arithmetic;

/// Calculate expression check (Go `ValidateCalculateExpression`) — alias for Set value validation.
pub fn validate_calculate_expression(text: &str, macro_: Option<&Macro>) -> EntryValidation {
    validate_set_variable_value(text, macro_)
}

/// Parse/evaluate with placeholders for missing vars (Go `validateExpressionStructure`).
/// Does not mutate the caller's runtime store — works on a scratch clone.
/// When `macro_` is `None`, still validates literal/arithmetic structure on an empty scratch
/// (so `"1 + "` blocks even with no active macro).
fn validate_expression_structure(expr: &str, macro_: Option<&Macro>) -> Result<()> {
    if expr.trim().is_empty() {
        return Ok(());
    }

    let mut scratch = match macro_ {
        Some(m) => {
            let mut scratch = Macro::new(m.name.clone(), m.global_delay, vec![]);
            scratch.variable_decls = m.variable_decls.clone();
            scratch.init_runtime_variables();
            for (name, val) in m.variables.iter() {
                scratch.variables.set(name, val.clone());
            }
            scratch
        }
        None => Macro::new(String::new(), 0, vec![]),
    };
    // Seed missing refs as 0 so structure (not unknown-var) is what we check.
    for name in sqyre_varref::names(expr) {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if scratch.variables.get(name).is_none() {
            scratch.variables.set(name, ScalarValue::Int(0));
        }
    }
    evaluate_expression(expr, &scratch).map_err(ValidateError::Message)?;
    Ok(())
}

/// Set-variable value: plain text allowed; invalid arithmetic blocks
/// (Go `ValidateSetVariableValue`).
pub fn validate_set_variable_value(text: &str, macro_: Option<&Macro>) -> EntryValidation {
    if text.trim().is_empty() {
        return EntryValidation::default();
    }
    let mut v = EntryValidation {
        warning: unknown_variable_warning(text, macro_),
        error: String::new(),
    };
    if looks_like_arithmetic(text) {
        if let Err(e) = validate_expression_structure(text, macro_) {
            v.error = e.to_string();
        }
    }
    v
}

/// Numeric field: empty, literal number, or valid arithmetic
/// (Go `ValidateNumericExpression`).
pub fn validate_numeric_expression(text: &str, macro_: Option<&Macro>) -> EntryValidation {
    if text.trim().is_empty() {
        return EntryValidation::default();
    }
    let mut v = EntryValidation {
        warning: unknown_variable_warning(text, macro_),
        error: String::new(),
    };
    if let Err(e) = validate_expression_structure(text, macro_) {
        v.error = e.to_string();
    }
    v
}

fn variable_binding_label(name: &str, role: &str) -> String {
    let name = name.trim();
    match role {
        "value" => format!("variable {name:?}"),
        "output" => format!("output variable {name:?}"),
        "output_x" => format!("output X variable {name:?}"),
        "output_y" => format!("output Y variable {name:?}"),
        _ => format!("variable {name:?}"),
    }
}

fn yaml_string_value(v: &serde_yaml::Value) -> Option<&str> {
    match v {
        serde_yaml::Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn validate_continue_key(keys: &[String]) -> Result<()> {
    let normalized: Vec<String> = keys
        .iter()
        .map(|k| k.trim().to_ascii_lowercase())
        .filter(|k| !k.is_empty())
        .collect();
    if normalized.is_empty() {
        return Err(ValidateError::Message(
            "pause: continue key not set".into(),
        ));
    }
    let mut sorted = normalized;
    sorted.sort();
    let mut failsafe = vec![
        "esc".to_string(),
        "ctrl".to_string(),
        "shift".to_string(),
    ];
    failsafe.sort();
    if sorted == failsafe {
        return Err(ValidateError::Message(
            "pause: continue key cannot match the failsafe hotkey (esc + ctrl + shift)".into(),
        ));
    }
    Ok(())
}

/// Checks minimum fields required to save/run an action (Go `ValidateAction`).
///
/// `macro_` enables Set expression structure checks; when
/// `None`, those structure checks are skipped (empty-expression / name rules
/// still apply).
pub fn validate_action(action: &Action, macro_: Option<&Macro>) -> Result<()> {
    for b in action.variable_bindings() {
        if b.name.trim().is_empty() {
            continue;
        }
        validate_variable_assignment_name(&b.name).map_err(|e| {
            ValidateError::Message(format!("{}: {e}", variable_binding_label(&b.name, &b.role)))
        })?;
    }

    match &action.kind {
        ActionKind::Key { key, .. } => {
            if key.trim().is_empty() {
                return Err(ValidateError::Message(
                    "key: record a key before saving".into(),
                ));
            }
        }
        ActionKind::SetVariable {
            variable_name,
            value,
        } => {
            validate_variable_assignment_name(variable_name).map_err(|e| {
                ValidateError::Message(format!("set variable: {e}"))
            })?;
            if let Some(text) = yaml_string_value(value) {
                let v = validate_set_variable_value(text, macro_);
                if v.blocks_submit() {
                    return Err(ValidateError::Message(format!("set variable: {}", v.error)));
                }
            }
        }
        ActionKind::Pause { continue_key, .. } => {
            validate_continue_key(continue_key)?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{ActionId, VariableDecl, VariableType};

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

    fn pause(keys: &[&str]) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Pause {
                message: String::new(),
                continue_key: keys.iter().map(|s| (*s).to_string()).collect(),
                pass_through: false,
            },
        }
    }

    fn key(k: &str) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: k.into(),
                state: true,
            },
        }
    }

    fn set_var(name: &str, value: serde_yaml::Value) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                variable_name: name.into(),
                value,
            },
        }
    }

    #[test]
    fn validate_action_pause_requires_continue_key() {
        assert!(validate_action(&pause(&[]), None).is_err());
    }

    #[test]
    fn validate_action_key_requires_key() {
        assert!(validate_action(&key(""), None).is_err());
    }

    #[test]
    fn validate_action_set_allows_empty_value() {
        assert!(validate_action(
            &set_var("out", serde_yaml::Value::String(String::new())),
            None
        )
        .is_ok());
    }

    #[test]
    fn validate_action_set_variable_requires_name() {
        assert!(validate_action(
            &set_var("", serde_yaml::Value::String("1".into())),
            None
        )
        .is_err());
    }

    #[test]
    fn validate_action_set_valid_expression() {
        let mut m = Macro::new("test", 0, vec![]);
        m.init_runtime_variables();
        assert!(validate_action(
            &set_var("sum", serde_yaml::Value::String("1 + 2".into())),
            Some(&m)
        )
        .is_ok());
    }

    #[test]
    fn validate_action_set_rejects_malformed_expression() {
        let mut m = Macro::new("test", 0, vec![]);
        m.init_runtime_variables();
        let err = validate_action(
            &set_var("sum", serde_yaml::Value::String("1 + ".into())),
            Some(&m),
        )
        .unwrap_err();
        assert!(err.to_string().contains("set variable:"), "{err}");
    }

    #[test]
    fn validate_set_variable_value_parity() {
        let mut m = Macro::new("t", 0, vec![]);
        m.variable_decls.push(VariableDecl {
            name: "x".into(),
            type_: VariableType::Number,
            initial_value: "5".into(),
            description: String::new(),
        });
        m.init_runtime_variables();

        assert!(!validate_set_variable_value("hello", Some(&m)).blocks_submit());
        assert!(!validate_set_variable_value("1+${x}", Some(&m)).blocks_submit());
        let missing = validate_set_variable_value("${missing}", Some(&m));
        assert!(!missing.blocks_submit());
        assert!(!missing.warning.is_empty());
        assert!(validate_set_variable_value("1 + ", Some(&m)).blocks_submit());
    }

    #[test]
    fn validate_action_rejects_bad_variable_name() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                variable_name: "a+b".into(),
                value: serde_yaml::Value::String("1+1".into()),
            },
        };
        let err = validate_action(&a, None).unwrap_err();
        assert!(err.to_string().contains("variable \"a+b\""), "{err}");
    }

    #[test]
    fn looks_like_arithmetic_detects_ops_and_fns() {
        assert!(looks_like_arithmetic("1+2"));
        assert!(!looks_like_arithmetic("hello"));
        assert!(looks_like_arithmetic("sqrt(4)"));
    }

    #[test]
    fn validate_numeric_without_macro_still_checks_structure() {
        assert!(!validate_numeric_expression("100", None).blocks_submit());
        assert!(!validate_numeric_expression("1+2", None).blocks_submit());
        assert!(validate_numeric_expression("1 + ", None).blocks_submit());
        assert!(!validate_numeric_expression("${x}", None).blocks_submit());
    }
}
