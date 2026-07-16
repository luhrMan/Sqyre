//! Scalar values that YAML may store as int, float, string, or bool.

use serde_yaml::Value;

/// Operand / count / time value: literal number or string (often `${var}`).
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum ScalarValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    #[default]
    Null,
}

impl ScalarValue {
    pub fn from_yaml(v: &Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Bool(b) => Self::Bool(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Self::Int(i)
                } else if let Some(u) = n.as_u64() {
                    Self::Int(u as i64)
                } else if let Some(f) = n.as_f64() {
                    // Prefer int when YAML gave a whole number as float.
                    if f.fract() == 0.0 && f.abs() <= i64::MAX as f64 {
                        Self::Int(f as i64)
                    } else {
                        Self::Float(f)
                    }
                } else {
                    Self::Null
                }
            }
            Value::String(s) => Self::String(s.clone()),
            other => Self::String(format!("{other:?}")),
        }
    }

    pub fn to_yaml(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(b) => Value::Bool(*b),
            Self::Int(i) => Value::Number((*i).into()),
            Self::Float(f) => Value::Number(serde_yaml::Number::from(*f)),
            Self::String(s) => Value::String(s.clone()),
        }
    }

    pub fn as_display(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Bool(b) => b.to_string(),
            Self::Int(i) => i.to_string(),
            Self::Float(f) => f.to_string(),
            Self::String(s) => s.clone(),
        }
    }

    /// True when a ForEachRow row bound (or similar) was present in YAML.
    pub fn is_set(&self) -> bool {
        !matches!(self, Self::Null)
    }
}


/// Delimiter between program and entity in coordinate / target refs.
pub const PROGRAM_DELIMITER: &str = "~";

/// Coordinate / search-area reference: `program~entity` or legacy name string.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoordinateRef(pub String);

impl CoordinateRef {
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn display_label(&self) -> &str {
        if self.is_empty() {
            "(unset)"
        } else {
            self.0.as_str()
        }
    }

    /// Program portion before `~`, if present.
    pub fn program(&self) -> Option<&str> {
        let (prog, _) = self.split_program_name()?;
        if prog.is_empty() {
            None
        } else {
            Some(prog)
        }
    }

    /// Entity name (point / search area / collection), without `@cell` suffix.
    pub fn name(&self) -> &str {
        let rest = match self.split_program_name() {
            Some((_, rest)) => rest,
            None => self.0.as_str(),
        };
        rest.split('@').next().unwrap_or(rest)
    }

    pub fn is_collection(&self) -> bool {
        self.cell_range().is_some()
    }

    /// Build `program~name@r1,c1-r2,c2` (1-based inclusive; corners normalized).
    pub fn collection(program: &str, name: &str, r1: i32, c1: i32, r2: i32, c2: i32) -> Self {
        let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
        let (c1, c2) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
        let base = if program.is_empty() {
            name.to_string()
        } else {
            format!("{program}{PROGRAM_DELIMITER}{name}")
        };
        Self(format!("{base}@{r1},{c1}-{r2},{c2}"))
    }

    /// Parses `@r1,c1-r2,c2` (1-based inclusive). Returns `(r1,c1,r2,c2)`.
    pub fn cell_range(&self) -> Option<(i32, i32, i32, i32)> {
        let s = self.0.as_str();
        let (_, suffix) = s.split_once('@')?;
        let (start, end) = suffix.split_once('-')?;
        let (r1, c1) = parse_cell_pair(start)?;
        let (r2, c2) = parse_cell_pair(end)?;
        let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
        let (c1, c2) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
        Some((r1, c1, r2, c2))
    }

    /// Replace program/entity portions, preserving any `@cell` suffix.
    pub fn with_entity_name(&self, program: &str, new_name: &str) -> Self {
        let range = self.cell_range();
        let base = if program.is_empty() {
            new_name.to_string()
        } else {
            format!("{program}{PROGRAM_DELIMITER}{new_name}")
        };
        if let Some((r1, c1, r2, c2)) = range {
            Self(format!("{base}@{r1},{c1}-{r2},{c2}"))
        } else {
            Self(base)
        }
    }

    fn split_program_name(&self) -> Option<(&str, &str)> {
        self.0.split_once(PROGRAM_DELIMITER)
    }
}

fn parse_cell_pair(s: &str) -> Option<(i32, i32)> {
    let (a, b) = s.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_ref_normalizes_corners() {
        let r = CoordinateRef::collection("Demo", "bag", 2, 3, 1, 1);
        assert_eq!(r.as_str(), "Demo~bag@1,1-2,3");
        assert!(r.is_collection());
        assert_eq!(r.program(), Some("Demo"));
        assert_eq!(r.name(), "bag");
        assert_eq!(r.cell_range(), Some((1, 1, 2, 3)));
    }
}

