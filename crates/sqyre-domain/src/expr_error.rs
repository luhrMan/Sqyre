//! Typed errors for expression evaluation and variable resolution.

use thiserror::Error;

/// Failure parsing or evaluating a numeric expression.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExprError {
    #[error("empty expression")]
    Empty,
    #[error("expression too deeply nested (max {max})")]
    TooDeep { max: usize },
    #[error("failed to evaluate expression: unexpected input at {tail:?}")]
    UnexpectedInput { tail: String },
    #[error("failed to evaluate expression: expected '{expected}', got {got:?}")]
    ExpectedToken { expected: char, got: Option<char> },
    #[error("failed to evaluate expression: unexpected {token:?}")]
    UnexpectedToken { token: Option<char> },
    #[error("failed to evaluate expression: invalid number")]
    InvalidNumber,
    #[error("failed to evaluate expression: invalid number {value:?}")]
    InvalidNumberValue { value: String },
    #[error("failed to evaluate expression: bad ident")]
    BadIdent,
    #[error("failed to evaluate expression: unknown identifier {name:?}")]
    UnknownIdent { name: String },
    #[error("failed to evaluate expression: unknown function {name:?}")]
    UnknownFunction { name: String },
}

/// Failure expanding `${variable}` references or coercing resolved text.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ResolveError {
    #[error("unresolved variable ${{{name}}}")]
    UnresolvedVariable { name: String },
    #[error("unresolved variable reference in {text:?}")]
    NestedReference { text: String },
    #[error("cannot parse int from {value:?}")]
    ParseInt { value: String },
    #[error(transparent)]
    Expr(#[from] ExprError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expr_error_display_roundtrip() {
        let err = ExprError::UnknownFunction { name: "foo".into() };
        assert!(err.to_string().contains("unknown function"));
    }

    #[test]
    fn resolve_error_wraps_expr() {
        let err = ResolveError::from(ExprError::Empty);
        assert!(matches!(err, ResolveError::Expr(ExprError::Empty)));
    }
}
