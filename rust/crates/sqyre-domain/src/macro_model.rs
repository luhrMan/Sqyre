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

    pub fn parse_hotkey_trigger(s: &str) -> String {
        match s.trim().to_ascii_lowercase().as_str() {
            HOTKEY_TRIGGER_RELEASE => HOTKEY_TRIGGER_RELEASE.to_string(),
            _ => HOTKEY_TRIGGER_PRESS.to_string(),
        }
    }
}
