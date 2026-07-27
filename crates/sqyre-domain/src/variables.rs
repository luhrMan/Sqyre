//! Declared macro variables and runtime store.

use crate::{
    Action, ActionKind, Macro, ScalarValue, FOREACH_ROW_BUILTIN_ROW, FOREACH_ROW_BUILTIN_ROW_COUNT,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Image Search builtins set inside sub-actions.
pub const IMAGE_SEARCH_BUILTIN_VARS: &[&str] = &[
    "StackMax",
    "Cols",
    "Rows",
    "ItemName",
    "ImagePixelWidth",
    "ImagePixelHeight",
];

/// Fixed descriptions for Image Search builtins (same order as [`IMAGE_SEARCH_BUILTIN_VARS`]).
const IMAGE_SEARCH_BUILTIN_DESCS: &[&str] = &[
    "Max stack depth for the matched image (Image Search)",
    "Column count of the matched grid (Image Search)",
    "Row count of the matched grid (Image Search)",
    "Name of the matched item (Image Search)",
    "Template image width in pixels (Image Search)",
    "Template image height in pixels (Image Search)",
];

/// Name + description for a system-provided runtime variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinVariableInfo {
    pub name: String,
    pub description: &'static str,
}

/// Monitor builtin names for `num_monitors` displays (1-based).
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

/// Full reference catalog of builtins for the Variables panel (not filtered by macro content).
pub fn builtin_variable_catalog(num_monitors: usize) -> Vec<BuiltinVariableInfo> {
    let n = num_monitors.max(1);
    let mut out = Vec::with_capacity(n * 2 + IMAGE_SEARCH_BUILTIN_VARS.len() + 2);
    for i in 1..=n {
        out.push(BuiltinVariableInfo {
            name: format!("monitor{i}Width"),
            description: "Display width in pixels (set at macro start)",
        });
        out.push(BuiltinVariableInfo {
            name: format!("monitor{i}Height"),
            description: "Display height in pixels (set at macro start)",
        });
    }
    for (name, description) in IMAGE_SEARCH_BUILTIN_VARS
        .iter()
        .zip(IMAGE_SEARCH_BUILTIN_DESCS.iter())
    {
        out.push(BuiltinVariableInfo {
            name: (*name).to_string(),
            description,
        });
    }
    out.push(BuiltinVariableInfo {
        name: FOREACH_ROW_BUILTIN_ROW.to_string(),
        description: "Current 1-based row index (ForEachRow)",
    });
    out.push(BuiltinVariableInfo {
        name: FOREACH_ROW_BUILTIN_ROW_COUNT.to_string(),
        description: "Total row count of the driving source (ForEachRow)",
    });
    out
}

/// Case-insensitive set of known variable names with O(1) lookup.
///
/// Keyed internally by ascii-lowercased name; preserves the first-seen declared/canonical
/// casing for display (autocomplete, chips) via [`KnownVariableNames::iter`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KnownVariableNames {
    by_lower: HashMap<String, String>,
}

impl KnownVariableNames {
    /// Insert `name`, keeping the first-seen casing for an already-present (case-insensitive) key.
    fn insert(&mut self, name: &str) {
        let n = name.trim();
        if n.is_empty() {
            return;
        }
        self.by_lower
            .entry(n.to_ascii_lowercase())
            .or_insert_with(|| n.to_string());
    }

    /// True when `name` is in the set (case-insensitive), in O(1).
    pub fn contains(&self, name: &str) -> bool {
        let needle = name.trim().to_ascii_lowercase();
        !needle.is_empty() && self.by_lower.contains_key(&needle)
    }

    /// Display-cased names, in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.by_lower.values().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.by_lower.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_lower.is_empty()
    }
}

impl<S: AsRef<str>> FromIterator<S> for KnownVariableNames {
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let mut out = Self::default();
        for name in iter {
            out.insert(name.as_ref());
        }
        out
    }
}

/// Known-name set for known/unknown nested variable chips.
pub fn known_variable_set(names: impl IntoIterator<Item = impl AsRef<str>>) -> KnownVariableNames {
    names.into_iter().collect()
}

/// Collect defined variable names from decls, action bindings, and relevant builtins.
///
/// Includes `monitor1Width` / `monitor1Height` (one display). Prefer
/// [`collect_known_variable_names_with_monitors`] when the live display count is known.
pub fn collect_known_variable_names(macro_: &Macro) -> KnownVariableNames {
    collect_known_variable_names_with_monitors(macro_, 1)
}

/// Like [`collect_known_variable_names`], plus monitor builtins for `num_monitors`.
///
/// Names keep their declared/canonical casing for display (autocomplete, chips).
/// Lookup remains case-insensitive and O(1) via [`is_known_variable`].
pub fn collect_known_variable_names_with_monitors(
    macro_: &Macro,
    num_monitors: usize,
) -> KnownVariableNames {
    let mut known = KnownVariableNames::default();
    let mut has_image_search = false;
    let mut has_for_each_row = false;

    for d in &macro_.variable_decls {
        known.insert(&d.name);
    }

    macro_.root.walk(&mut |action: &Action| {
        match &action.kind {
            ActionKind::ImageSearch { .. } => has_image_search = true,
            ActionKind::ForEachRow { .. } => has_for_each_row = true,
            _ => {}
        }
        for b in action.variable_bindings() {
            known.insert(&b.name);
        }
    });

    if has_image_search {
        for n in IMAGE_SEARCH_BUILTIN_VARS {
            known.insert(n);
        }
    }
    if has_for_each_row {
        known.insert(FOREACH_ROW_BUILTIN_ROW);
        known.insert(FOREACH_ROW_BUILTIN_ROW_COUNT);
    }

    for name in monitor_builtin_var_names(num_monitors) {
        known.insert(&name);
    }

    known
}

/// True when `name` is in the known set (case-insensitive), in O(1).
pub fn is_known_variable(known: &KnownVariableNames, name: &str) -> bool {
    known.contains(name)
}

/// True when `name` collides with a runtime builtin (Image Search / ForEachRow / monitors).
pub fn is_reserved_runtime_variable_name(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    if IMAGE_SEARCH_BUILTIN_VARS
        .iter()
        .any(|n| n.eq_ignore_ascii_case(name))
    {
        return true;
    }
    if name.eq_ignore_ascii_case(FOREACH_ROW_BUILTIN_ROW)
        || name.eq_ignore_ascii_case(FOREACH_ROW_BUILTIN_ROW_COUNT)
    {
        return true;
    }
    // monitorNWidth / monitorNHeight for any positive N.
    let lower = name.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("monitor") {
        if let Some(num) = rest
            .strip_suffix("width")
            .or_else(|| rest.strip_suffix("height"))
        {
            return !num.is_empty() && num.chars().all(|c| c.is_ascii_digit());
        }
    }
    false
}

crate::string_enum! {
    /// Declared value type of a user-defined macro variable.
    pub enum VariableType {
        #[default]
        Auto = "auto",
        Text = "text",
        Number = "number",
    }
}

/// User-declared macro variable (persisted).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct VariableDecl {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(rename = "type", default)]
    pub type_: VariableType,
    #[serde(
        rename = "initialvalue",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub initial_value: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
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

/// Process-wide source of [`VariableStore::revision`] stamps.
///
/// Global (not per-store) so revisions from different stores never collide —
/// observers such as the live-variables sink compare a single last-seen stamp
/// while the executor switches between a caller's and a sub-macro's store.
static NEXT_STORE_REVISION: AtomicU64 = AtomicU64::new(1);

/// Case-insensitive runtime variable store (not persisted).
///
/// Keyed internally by ascii-lowercased name for O(1) get/set/delete.
#[derive(Debug, Clone, Default)]
pub struct VariableStore {
    entries: HashMap<String, (String, ScalarValue)>,
    /// Bumped whenever the contents actually change; see [`Self::revision`].
    revision: u64,
}

/// Contents only — [`VariableStore::revision`] is observer bookkeeping, not state.
impl PartialEq for VariableStore {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl VariableStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opaque stamp that changes whenever the contents change.
    ///
    /// Lets observers skip work when nothing has been written since last time.
    /// Only equality is meaningful — do not compare with `<` / `>`.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn touch(&mut self) {
        self.revision = NEXT_STORE_REVISION.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self, name: &str) -> Option<&ScalarValue> {
        let key = name.trim().to_ascii_lowercase();
        self.entries.get(&key).map(|(_, v)| v)
    }

    pub fn set(&mut self, name: impl Into<String>, value: ScalarValue) {
        let name = name.into();
        let key = name.trim().to_ascii_lowercase();
        match self.entries.get_mut(&key) {
            Some((_, v)) => {
                if *v == value {
                    return;
                }
                *v = value;
            }
            None => {
                self.entries.insert(key, (name, value));
            }
        }
        self.touch();
    }

    /// Remove a variable by name (case-insensitive). No-op when name is empty or missing.
    pub fn delete(&mut self, name: &str) {
        let key = name.trim().to_ascii_lowercase();
        if key.is_empty() {
            return;
        }
        if self.entries.remove(&key).is_some() {
            self.touch();
        }
    }

    pub fn clear(&mut self) {
        if !self.entries.is_empty() {
            self.entries.clear();
            self.touch();
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &ScalarValue)> {
        self.entries.values().map(|(n, v)| (n.as_str(), v))
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
                    assignments: vec![crate::VariableAssignment::new("Count", ScalarValue::Null)],
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
        assert!(known.iter().any(|n| n == "Seed"));
        assert!(known.iter().any(|n| n == "Count"));
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

    #[test]
    fn builtin_catalog_includes_monitors_image_search_and_foreach() {
        let cat = builtin_variable_catalog(2);
        let names: Vec<&str> = cat.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"monitor1Width"));
        assert!(names.contains(&"monitor2Height"));
        assert!(names.contains(&"StackMax"));
        assert!(names.contains(&"Row"));
        assert!(names.contains(&"RowCount"));
        assert_eq!(cat.len(), 2 * 2 + IMAGE_SEARCH_BUILTIN_VARS.len() + 2);
    }
}
