//! Set-variable value resolution.

use crate::{evaluate_expression, numeric_to_scalar, Macro, ScalarValue};
use serde_yaml::Value;

type Result<T> = std::result::Result<T, String>;

/// Whether text will be evaluated as arithmetic at runtime.
pub fn looks_like_arithmetic(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if t.contains(['+', '*', '/', '^', '(', ')']) {
        return true;
    }
    let bytes = t.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'-' {
            continue;
        }
        if i == 0 {
            return true;
        }
        let prev = bytes[i - 1];
        let next = bytes.get(i + 1).copied().unwrap_or(0);
        if is_expr_number_char(prev)
            || is_expr_number_char(next)
            || prev == b')'
            || next == b'('
        {
            return true;
        }
    }
    let lower = t.to_ascii_lowercase();
    for fn_name in [
        "sqrt", "abs", "round", "floor", "ceil", "trunc", "sin", "cos", "tan", "ln",
    ] {
        if lower.contains(&format!("{fn_name}(")) {
            return true;
        }
    }
    t.contains("~pi") || t.contains("~e")
}

fn is_expr_number_char(b: u8) -> bool {
    b.is_ascii_digit() || b == b'.'
}

/// Resolve `${references}` in plain text; errors on unresolved refs.
pub fn resolve_variables_in_text(text: &str, macro_: &Macro) -> Result<String> {
    let segs = sqyre_varref::segments(text);
    if segs.is_empty() {
        return Ok(text.to_string());
    }
    let mut out = String::new();
    for seg in segs {
        if !seg.is_ref {
            out.push_str(&seg.text);
            continue;
        }
        let val = macro_.variables.get(&seg.name).ok_or_else(|| {
            format!("unresolved variable ${{{}}}", seg.name)
        })?;
        out.push_str(&val.as_display());
    }
    if sqyre_varref::contains(&out) {
        return Err(format!("unresolved variable reference in {text:?}"));
    }
    Ok(out)
}

/// Resolve a Set action value: literals, text, `${refs}`, and arithmetic expressions.
pub fn resolve_set_variable_value(value: &Value, macro_: &Macro) -> Result<ScalarValue> {
    match value {
        Value::Bool(b) => Ok(ScalarValue::Bool(*b)),
        Value::Number(_) => Ok(ScalarValue::from_yaml(value)),
        Value::String(s) => resolve_set_variable_string(s, macro_),
        other => Ok(ScalarValue::from_yaml(other)),
    }
}

fn resolve_set_variable_string(text: &str, macro_: &Macro) -> Result<ScalarValue> {
    let resolved = resolve_variables_in_text(text, macro_)?;
    if resolved.is_empty() {
        return Ok(ScalarValue::String(String::new()));
    }
    if looks_like_arithmetic(&resolved) {
        if let Ok(f) = evaluate_expression(text, macro_) {
            return Ok(numeric_to_scalar(f));
        }
    }
    if let Ok(i) = resolved.trim().parse::<i64>() {
        return Ok(ScalarValue::Int(i));
    }
    if let Ok(f) = resolved.trim().parse::<f64>() {
        return Ok(ScalarValue::Float(f));
    }
    Ok(ScalarValue::String(resolved))
}

/// Resolve a scalar to `i32`: literals, `${refs}`, and arithmetic expressions.
///
/// Used for point/search-area coordinates, wait times, loop counts, etc.
pub fn resolve_scalar_int(v: &ScalarValue, macro_: &Macro) -> Result<i32> {
    match v {
        ScalarValue::Int(i) => Ok(*i as i32),
        ScalarValue::Float(f) => Ok(*f as i32),
        ScalarValue::Bool(b) => Ok(if *b { 1 } else { 0 }),
        ScalarValue::Null => Ok(0),
        ScalarValue::String(s) => resolve_int_string(s, macro_),
    }
}

fn resolve_int_string(text: &str, macro_: &Macro) -> Result<i32> {
    let trimmed = text.trim();
    // Source may already be an expression with `${refs}` (evaluate_expression expands them).
    if looks_like_arithmetic(trimmed) {
        let f = evaluate_expression(trimmed, macro_)?;
        return Ok(f as i32);
    }
    let resolved = resolve_variables_in_text(trimmed, macro_)?;
    let resolved = resolved.trim();
    // A lone `${ref}` can expand to an expression (e.g. builtin-built formulas).
    if looks_like_arithmetic(resolved) {
        let f = evaluate_expression(resolved, macro_)?;
        return Ok(f as i32);
    }
    resolved
        .parse()
        .map_err(|_| format!("cannot parse int from {resolved:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VariableDecl, VariableType};

    #[test]
    fn resolves_plain_text_and_refs() {
        let mut m = Macro::new("t", 0, vec![]);
        m.variable_decls.push(VariableDecl {
            name: "x".into(),
            type_: VariableType::Number,
            initial_value: "5".into(),
            description: String::new(),
        });
        m.init_runtime_variables();
        let v = resolve_set_variable_value(
            &Value::String("${x}".into()),
            &m,
        )
        .unwrap();
        assert_eq!(v, ScalarValue::Int(5));

        let v = resolve_set_variable_value(&Value::String("plain".into()), &m).unwrap();
        assert_eq!(v, ScalarValue::String("plain".into()));
    }

    #[test]
    fn evaluates_arithmetic_expressions() {
        let mut m = Macro::new("t", 0, vec![]);
        m.variable_decls.push(VariableDecl {
            name: "x".into(),
            type_: VariableType::Number,
            initial_value: "5".into(),
            description: String::new(),
        });
        m.init_runtime_variables();
        let v = resolve_set_variable_value(&Value::String("1+${x}".into()), &m).unwrap();
        assert_eq!(v, ScalarValue::Int(6));
    }

    #[test]
    fn looks_like_arithmetic_detects_ops_and_fns() {
        assert!(looks_like_arithmetic("1+2"));
        assert!(!looks_like_arithmetic("hello"));
        assert!(looks_like_arithmetic("sqrt(4)"));
    }

    #[test]
    fn resolve_scalar_int_evaluates_arithmetic_after_refs() {
        let mut m = Macro::new("t", 0, vec![]);
        m.variables.set("ox", ScalarValue::Int(2560));
        m.variables.set("w", ScalarValue::Int(1920));
        // Expression with refs (typical point formula).
        assert_eq!(
            resolve_scalar_int(&ScalarValue::String("${ox}+(${w}/2)".into()), &m).unwrap(),
            3520
        );
        // Already-expanded expression (builtin resolution left a formula string).
        assert_eq!(
            resolve_scalar_int(&ScalarValue::String("2560+(1920/2)".into()), &m).unwrap(),
            3520
        );
        // Ref whose value is itself an expression.
        m.variables
            .set("formula", ScalarValue::String("2560+(1920/2)".into()));
        assert_eq!(
            resolve_scalar_int(&ScalarValue::String("${formula}".into()), &m).unwrap(),
            3520
        );
    }
}
