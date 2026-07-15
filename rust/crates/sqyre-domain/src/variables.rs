//! Declared macro variables and runtime store.

use crate::ScalarValue;

/// Declared value type of a user-defined macro variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VariableType {
    #[default]
    Auto,
    Text,
    Number,
}

impl VariableType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Text => "text",
            Self::Number => "number",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "text" => Self::Text,
            "number" => Self::Number,
            _ => Self::Auto,
        }
    }
}

/// User-declared macro variable (persisted).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VariableDecl {
    pub name: String,
    pub type_: VariableType,
    pub initial_value: String,
    pub description: String,
}

impl VariableDecl {
    pub fn initial_stored_value(&self) -> ScalarValue {
        match self.type_ {
            VariableType::Number => {
                let trimmed = self.initial_value.trim();
                if let Ok(i) = trimmed.parse::<i64>() {
                    return ScalarValue::Int(i);
                }
                if let Ok(f) = trimmed.parse::<f64>() {
                    return ScalarValue::Float(f);
                }
                ScalarValue::String(self.initial_value.clone())
            }
            _ => ScalarValue::String(self.initial_value.clone()),
        }
    }
}

/// Case-insensitive runtime variable store (not persisted).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VariableStore {
    entries: Vec<(String, ScalarValue)>,
}

impl VariableStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&ScalarValue> {
        let key = name.trim();
        self.entries
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    pub fn set(&mut self, name: impl Into<String>, value: ScalarValue) {
        let name = name.into();
        if let Some((_, v)) = self
            .entries
            .iter_mut()
            .find(|(n, _)| n.eq_ignore_ascii_case(name.trim()))
        {
            *v = value;
        } else {
            self.entries.push((name, value));
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
