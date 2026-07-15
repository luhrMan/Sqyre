//! Declared macro variables and runtime store.

use crate::{
    Action, ActionKind, Macro, ScalarValue, FOREACH_ROW_BUILTIN_ROW, FOREACH_ROW_BUILTIN_ROW_COUNT,
};
use std::collections::HashSet;

/// Image Search builtins set inside sub-actions (Go `ImageSearchBuiltinVars`).
pub const IMAGE_SEARCH_BUILTIN_VARS: &[&str] = &[
    "StackMax",
    "Cols",
    "Rows",
    "ItemName",
    "ImagePixelWidth",
    "ImagePixelHeight",
];

/// Monitor builtin names for `num_monitors` displays (1-based; Go `MonitorBuiltinVarNames`).
/// When `num_monitors < 1`, returns names for one monitor.
pub fn monitor_builtin_var_names(num_monitors: usize) -> Vec<String> {
    let n = num_monitors.max(1);
    let mut names = Vec::with_capacity(n * 2);
    for i in 1..=n {
        names.push(format!("monitor{i}Width"));
        names.push(format!("monitor{i}Height"));
    }
    names
}

/// Lowercase name set for known/unknown nested variable chips (Go `KnownVariableSet`).
pub fn known_variable_set(names: impl IntoIterator<Item = impl AsRef<str>>) -> HashSet<String> {
    names
        .into_iter()
        .map(|n| n.as_ref().trim().to_ascii_lowercase())
        .filter(|n| !n.is_empty())
        .collect()
}

/// Collect defined variable names from decls, action bindings, and relevant builtins.
///
/// Includes `monitor1Width` / `monitor1Height` (one display). Prefer
/// [`collect_known_variable_names_with_monitors`] when the live display count is known.
pub fn collect_known_variable_names(macro_: &Macro) -> HashSet<String> {
    collect_known_variable_names_with_monitors(macro_, 1)
}

/// Like [`collect_known_variable_names`], plus monitor builtins for `num_monitors`.
pub fn collect_known_variable_names_with_monitors(
    macro_: &Macro,
    num_monitors: usize,
) -> HashSet<String> {
    let mut names = HashSet::new();
    let mut has_image_search = false;
    let mut has_for_each_row = false;

    for d in &macro_.variable_decls {
        let n = d.name.trim();
        if !n.is_empty() {
            names.insert(n.to_ascii_lowercase());
        }
    }

    macro_.root.walk(&mut |action: &Action| {
        match &action.kind {
            ActionKind::ImageSearch { .. } => has_image_search = true,
            ActionKind::ForEachRow { .. } => has_for_each_row = true,
            _ => {}
        }
        for b in action.variable_bindings() {
            let n = b.name.trim();
            if !n.is_empty() {
                names.insert(n.to_ascii_lowercase());
            }
        }
    });

    if has_image_search {
        for n in IMAGE_SEARCH_BUILTIN_VARS {
            names.insert((*n).to_ascii_lowercase());
        }
    }
    if has_for_each_row {
        names.insert(FOREACH_ROW_BUILTIN_ROW.to_ascii_lowercase());
        names.insert(FOREACH_ROW_BUILTIN_ROW_COUNT.to_ascii_lowercase());
    }

    for name in monitor_builtin_var_names(num_monitors) {
        names.insert(name.to_ascii_lowercase());
    }

    names
}

/// True when `name` is in the known set (case-insensitive).
pub fn is_known_variable(known: &HashSet<String>, name: &str) -> bool {
    known.contains(&name.trim().to_ascii_lowercase())
}

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

    /// Remove a variable by name (case-insensitive). No-op when name is empty or missing.
    pub fn delete(&mut self, name: &str) {
        let key = name.trim();
        if key.is_empty() {
            return;
        }
        self.entries
            .retain(|(n, _)| !n.eq_ignore_ascii_case(key));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &ScalarValue)> {
        self.entries.iter().map(|(n, v)| (n.as_str(), v))
    }
}

#[cfg(test)]
mod known_tests {
    use super::*;
    use crate::{root_loop, Action, ActionId, ActionKind};

    #[test]
    fn collect_includes_decls_bindings_and_builtins() {
        let mut m = Macro::new("m", 0, vec![]);
        m.variable_decls.push(VariableDecl {
            name: "Seed".into(),
            ..Default::default()
        });
        m.root = root_loop(vec![
            Action {
                id: ActionId::new(),
                kind: ActionKind::SetVariable {
                    variable_name: "Count".into(),
                    value: serde_yaml::Value::Null,
                },
            },
            Action {
                id: ActionId::new(),
                kind: ActionKind::ForEachRow {
                    name: "rows".into(),
                    sources: vec![],
                    start_row: ScalarValue::Null,
                    end_row: ScalarValue::Null,
                    subactions: vec![],
                },
            },
        ]);
        let known = collect_known_variable_names(&m);
        assert!(is_known_variable(&known, "seed"));
        assert!(is_known_variable(&known, "COUNT"));
        assert!(is_known_variable(&known, "Row"));
        assert!(is_known_variable(&known, "RowCount"));
        assert!(is_known_variable(&known, "monitor1Width"));
        assert!(is_known_variable(&known, "monitor1Height"));
    }

    #[test]
    fn monitor_builtin_names_scale_with_count() {
        assert_eq!(
            monitor_builtin_var_names(2),
            vec![
                "monitor1Width",
                "monitor1Height",
                "monitor2Width",
                "monitor2Height",
            ]
        );
        assert_eq!(
            monitor_builtin_var_names(0),
            vec!["monitor1Width", "monitor1Height"]
        );
        let known = collect_known_variable_names_with_monitors(&Macro::new("m", 0, vec![]), 2);
        assert!(is_known_variable(&known, "monitor2Width"));
    }
}
