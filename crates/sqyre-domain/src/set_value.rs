//! Set-variable value resolution.

use crate::{evaluate_expression, numeric_to_scalar, ResolveError, ScalarValue, VariableStore};

type Result<T> = std::result::Result<T, ResolveError>;

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
        if is_expr_number_char(prev) || is_expr_number_char(next) || prev == b')' || next == b'(' {
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

/// Expand `${references}` once. Values that themselves contain refs are left as-is.
pub fn expand_variable_refs(text: &str, vars: &VariableStore) -> Result<String> {
    if !sqyre_varref::contains(text) {
        return Ok(sqyre_varref::unescape_plain(text));
    }
    let segs = sqyre_varref::segments(text);
    if segs.is_empty() {
        return Ok(sqyre_varref::unescape_plain(text));
    }
    let mut out = String::new();
    for seg in segs {
        if !seg.is_ref {
            out.push_str(&sqyre_varref::unescape_plain(&seg.text));
            continue;
        }
        let val = vars
            .get(&seg.name)
            .ok_or_else(|| ResolveError::UnresolvedVariable {
                name: seg.name.clone(),
            })?;
        out.push_str(&val.as_display());
    }
    Ok(out)
}

/// Resolve `${references}` in plain text; errors on unresolved or nested refs.
pub fn resolve_variables_in_text(text: &str, vars: &VariableStore) -> Result<String> {
    let out = expand_variable_refs(text, vars)?;
    if sqyre_varref::contains(&out) {
        return Err(ResolveError::NestedReference {
            text: text.to_string(),
        });
    }
    Ok(out)
}

/// Resolve a Set action value: literals, text, `${refs}`, and arithmetic expressions.
pub fn resolve_set_variable_value(
    value: &ScalarValue,
    vars: &VariableStore,
) -> Result<ScalarValue> {
    match value {
        ScalarValue::Bool(b) => Ok(ScalarValue::Bool(*b)),
        ScalarValue::Int(_) | ScalarValue::Float(_) | ScalarValue::Null => Ok(value.clone()),
        ScalarValue::String(s) => resolve_set_variable_string(s, vars),
    }
}

fn resolve_set_variable_string(text: &str, vars: &VariableStore) -> Result<ScalarValue> {
    let resolved = resolve_variables_in_text(text, vars)?;
    if resolved.is_empty() {
        return Ok(ScalarValue::String(String::new()));
    }
    if looks_like_arithmetic(&resolved) {
        if let Ok(f) = evaluate_expression(text, vars) {
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
pub fn resolve_scalar_int(v: &ScalarValue, vars: &VariableStore) -> Result<i32> {
    match v {
        ScalarValue::Int(i) => Ok(*i as i32),
        ScalarValue::Float(f) => Ok(*f as i32),
        ScalarValue::Bool(b) => Ok(if *b { 1 } else { 0 }),
        ScalarValue::Null => Ok(0),
        ScalarValue::String(s) => resolve_int_string(s, vars),
    }
}

fn resolve_int_string(text: &str, vars: &VariableStore) -> Result<i32> {
    let trimmed = text.trim();
    // Source may already be an expression with `${refs}` (evaluate_expression expands them).
    if looks_like_arithmetic(trimmed) {
        let f = evaluate_expression(trimmed, vars)?;
        return Ok(f as i32);
    }
    let resolved = resolve_variables_in_text(trimmed, vars)?;
    let resolved = resolved.trim();
    // A lone `${ref}` can expand to an expression (e.g. builtin-built formulas).
    if looks_like_arithmetic(resolved) {
        let f = evaluate_expression(resolved, vars)?;
        return Ok(f as i32);
    }
    resolved.parse().map_err(|_| ResolveError::ParseInt {
        value: resolved.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_plain_text_and_refs() {
        let mut vars = VariableStore::new();
        vars.set("x", ScalarValue::Int(5));
        let v = resolve_set_variable_value(&ScalarValue::String("${x}".into()), &vars).unwrap();
        assert_eq!(v, ScalarValue::Int(5));

        let v = resolve_set_variable_value(&ScalarValue::String("plain".into()), &vars).unwrap();
        assert_eq!(v, ScalarValue::String("plain".into()));
    }

    #[test]
    fn expand_unescapes_literal_refs() {
        let mut vars = VariableStore::new();
        vars.set("x", ScalarValue::Int(5));
        assert_eq!(
            expand_variable_refs("show $${x} and ${x}", &vars).unwrap(),
            "show ${x} and 5"
        );
        assert_eq!(
            expand_variable_refs("braces {{x}} vs {x}", &vars).unwrap(),
            "braces {x} vs 5"
        );
    }

    #[test]
    fn evaluates_arithmetic_expressions() {
        let mut vars = VariableStore::new();
        vars.set("x", ScalarValue::Int(5));
        let v = resolve_set_variable_value(&ScalarValue::String("1+${x}".into()), &vars).unwrap();
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
        let mut vars = VariableStore::new();
        vars.set("ox", ScalarValue::Int(2560));
        vars.set("w", ScalarValue::Int(1920));
        // Expression with refs (typical point formula).
        assert_eq!(
            resolve_scalar_int(&ScalarValue::String("${ox}+(${w}/2)".into()), &vars).unwrap(),
            3520
        );
        // Already-expanded expression (builtin resolution left a formula string).
        assert_eq!(
            resolve_scalar_int(&ScalarValue::String("2560+(1920/2)".into()), &vars).unwrap(),
            3520
        );
        // Ref whose value is itself an expression.
        vars.set("formula", ScalarValue::String("2560+(1920/2)".into()));
        assert_eq!(
            resolve_scalar_int(&ScalarValue::String("${formula}".into()), &vars).unwrap(),
            3520
        );
    }
}
