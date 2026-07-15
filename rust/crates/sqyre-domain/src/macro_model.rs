//! Macro aggregate.

use crate::{root_loop, Action, VariableDecl, VariableStore};

pub const DEFAULT_KEYBOARD_DELAY: i32 = 25;
pub const DEFAULT_MOUSE_DELAY: i32 = 25;

pub const HOTKEY_TRIGGER_PRESS: &str = "press";
pub const HOTKEY_TRIGGER_RELEASE: &str = "release";

#[derive(Debug, Clone, PartialEq)]
pub struct Macro {
    pub name: String,
    pub root: Action,
    pub global_delay: i32,
    pub keyboard_delay: i32,
    pub mouse_delay: i32,
    pub hotkey: Vec<String>,
    pub hotkey_trigger: String,
    pub tags: Vec<String>,
    pub variable_decls: Vec<VariableDecl>,
    /// Runtime store; never persisted.
    #[cfg_attr(test, allow(dead_code))]
    pub variables: VariableStore,
}

impl Macro {
    pub fn new(name: impl Into<String>, delay: i32, hotkey: Vec<String>) -> Self {
        Self {
            name: name.into(),
            root: root_loop(Vec::new()),
            global_delay: delay,
            keyboard_delay: DEFAULT_KEYBOARD_DELAY,
            mouse_delay: DEFAULT_MOUSE_DELAY,
            hotkey,
            hotkey_trigger: HOTKEY_TRIGGER_PRESS.to_string(),
            tags: Vec::new(),
            variable_decls: Vec::new(),
            variables: VariableStore::new(),
        }
    }

    pub fn init_runtime_variables(&mut self) {
        let mut vs = VariableStore::new();
        for d in &self.variable_decls {
            let name = d.name.trim();
            if name.is_empty() || d.initial_value.trim().is_empty() {
                continue;
            }
            vs.set(name, d.initial_stored_value());
        }
        self.variables = vs;
    }

    /// Insert or replace a declaration by name (case-insensitive). Go `UpsertVariable`.
    pub fn upsert_variable(&mut self, decl: VariableDecl) {
        let name = decl.name.trim();
        if name.is_empty() {
            return;
        }
        if let Some(existing) = self
            .variable_decls
            .iter_mut()
            .find(|d| d.name.eq_ignore_ascii_case(name))
        {
            *existing = decl;
        } else {
            self.variable_decls.push(decl);
        }
    }

    /// Remove a declaration by name (case-insensitive). Returns true if removed.
    pub fn remove_variable_decl(&mut self, name: &str) -> bool {
        let key = name.trim();
        if key.is_empty() {
            return false;
        }
        let before = self.variable_decls.len();
        self.variable_decls
            .retain(|d| !d.name.eq_ignore_ascii_case(key));
        self.variable_decls.len() != before
    }

    pub fn parse_hotkey_trigger(s: &str) -> String {
        match s.trim().to_ascii_lowercase().as_str() {
            HOTKEY_TRIGGER_RELEASE => HOTKEY_TRIGGER_RELEASE.to_string(),
            _ => HOTKEY_TRIGGER_PRESS.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VariableDecl, VariableType};

    #[test]
    fn upsert_and_remove_variable_decl() {
        let mut m = Macro::new("m", 0, vec![]);
        m.upsert_variable(VariableDecl {
            name: "Count".into(),
            type_: VariableType::Number,
            initial_value: "1".into(),
            description: String::new(),
        });
        m.upsert_variable(VariableDecl {
            name: "count".into(),
            type_: VariableType::Number,
            initial_value: "2".into(),
            description: "n".into(),
        });
        assert_eq!(m.variable_decls.len(), 1);
        assert_eq!(m.variable_decls[0].initial_value, "2");
        assert!(m.remove_variable_decl("COUNT"));
        assert!(m.variable_decls.is_empty());
    }
}
