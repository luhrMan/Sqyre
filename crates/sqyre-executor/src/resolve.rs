//! Clause evaluation and scalar/text resolution for action execution.

use crate::error::{ExecError, Result};
use sqyre_domain::{resolve_scalar_int, Macro, MatchMode, ScalarValue};

pub(crate) fn eval_clauses(
    match_mode: MatchMode,
    clauses: &[sqyre_domain::ConditionClause],
    macro_: &Macro,
) -> Result<bool> {
    if clauses.is_empty() {
        return Ok(true);
    }
    let want_any = match_mode == MatchMode::Any;
    for c in clauses {
        let ok = eval_one_clause(c, macro_)?;
        if want_any {
            if ok {
                return Ok(true);
            }
        } else if !ok {
            return Ok(false);
        }
    }
    Ok(!want_any)
}

fn eval_one_clause(c: &sqyre_domain::ConditionClause, macro_: &Macro) -> Result<bool> {
    use sqyre_domain::ConditionOperator;

    if c.operator.reads_variable_name() {
        let raw = c.left.as_display();
        let name = variable_name_for_is_set(&raw);
        return Ok(macro_.variables.get(name).is_some());
    }

    let left = resolve_text(&c.left.as_display(), macro_)?;
    Ok(match c.operator {
        ConditionOperator::IsEmpty => left.trim().is_empty(),
        ConditionOperator::Equals
        | ConditionOperator::NotEquals
        | ConditionOperator::Contains
        | ConditionOperator::StartsWith
        | ConditionOperator::EndsWith
        | ConditionOperator::LessThan
        | ConditionOperator::LessOrEqual
        | ConditionOperator::GreaterThan
        | ConditionOperator::GreaterOrEqual => {
            let right = resolve_text(&c.right.as_display(), macro_)?;
            match c.operator {
                ConditionOperator::Equals => left == right,
                ConditionOperator::NotEquals => left != right,
                ConditionOperator::Contains => left.contains(&right),
                ConditionOperator::StartsWith => left.starts_with(&right),
                ConditionOperator::EndsWith => left.ends_with(&right),
                ConditionOperator::LessThan => compare_ordered(&left, &right, OrderingOp::Less),
                ConditionOperator::LessOrEqual => {
                    compare_ordered(&left, &right, OrderingOp::LessOrEqual)
                }
                ConditionOperator::GreaterThan => {
                    compare_ordered(&left, &right, OrderingOp::Greater)
                }
                ConditionOperator::GreaterOrEqual => {
                    compare_ordered(&left, &right, OrderingOp::GreaterOrEqual)
                }
                _ => unreachable!(),
            }
        }
        ConditionOperator::IsSet => unreachable!("handled above"),
    })
}

/// Name for `is set`: strip `${…}` / `{…}`, otherwise use the trimmed literal.
fn variable_name_for_is_set(raw: &str) -> &str {
    let t = raw.trim();
    if let Some(inner) = t.strip_prefix("${").and_then(|x| x.strip_suffix('}')) {
        return inner.trim();
    }
    if let Some(inner) = t.strip_prefix('{').and_then(|x| x.strip_suffix('}')) {
        return inner.trim();
    }
    t
}

#[derive(Clone, Copy)]
enum OrderingOp {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

/// Numeric compare when both sides parse as `f64`; otherwise lexicographic.
fn compare_ordered(left: &str, right: &str, op: OrderingOp) -> bool {
    use std::cmp::Ordering;
    let ord = match (left.trim().parse::<f64>(), right.trim().parse::<f64>()) {
        (Ok(a), Ok(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => left.cmp(right),
    };
    match op {
        OrderingOp::Less => ord == Ordering::Less,
        OrderingOp::LessOrEqual => ord != Ordering::Greater,
        OrderingOp::Greater => ord == Ordering::Greater,
        OrderingOp::GreaterOrEqual => ord != Ordering::Less,
    }
}

pub(crate) fn resolve_int(v: &ScalarValue, macro_: &Macro) -> Result<i32> {
    resolve_scalar_int(v, &macro_.variables).map_err(|e| ExecError::Message(e.to_string()))
}

pub(crate) fn resolve_text(text: &str, macro_: &Macro) -> Result<String> {
    sqyre_domain::expand_variable_refs(text, &macro_.variables)
        .map_err(|e| ExecError::Message(e.to_string()))
}
