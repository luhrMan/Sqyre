//! Macro aggregate.

use crate::{root_loop, Action, ActionKind, VariableDecl, VariableStore};
use serde::{Deserialize, Serialize};

pub const DEFAULT_KEYBOARD_DELAY: i32 = 25;
pub const DEFAULT_MOUSE_DELAY: i32 = 25;

pub const HOTKEY_TRIGGER_PRESS: &str = "press";
pub const HOTKEY_TRIGGER_RELEASE: &str = "release";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Macro {
    pub name: String,
    pub root: Action,
    #[serde(rename = "globaldelay", default)]
    pub global_delay: i32,
    #[serde(rename = "keyboarddelay", default = "default_keyboard_delay")]
    pub keyboard_delay: i32,
    #[serde(rename = "mousedelay", default = "default_mouse_delay")]
    pub mouse_delay: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hotkey: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hotkey_trigger: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(rename = "variables", default, skip_serializing_if = "Vec::is_empty")]
    pub variable_decls: Vec<VariableDecl>,
    /// Runtime store; never persisted.
    #[serde(skip)]
    pub variables: VariableStore,
}

fn default_keyboard_delay() -> i32 {
    DEFAULT_KEYBOARD_DELAY
}

fn default_mouse_delay() -> i32 {
    DEFAULT_MOUSE_DELAY
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

    /// Insert or replace a declaration by name (case-insensitive).
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

    /// True when the action tree reads the screen — an Image Search, OCR, or
    /// Find Pixel action — directly or through `Run Macro` references resolved
    /// against `macros` (matched case-insensitively). Cycles and missing targets
    /// are cut like the executor's run stack.
    pub fn uses_screen_capture<'a>(&'a self, macros: &'a [Macro]) -> bool {
        self.any_reached(macros, ActionKind::is_detection)
    }

    /// True when a reached detection polls faster than an accessibility screenshot
    /// ([`crate::SCREENSHOT_MIN_GAP_MS`] at the Android default interval).
    pub fn needs_fast_capture<'a>(&'a self, macros: &'a [Macro]) -> bool {
        self.any_reached(macros, detection_needs_fast_capture)
    }

    fn any_reached<'a>(&'a self, macros: &'a [Macro], pred: fn(&ActionKind) -> bool) -> bool {
        fn rec<'a>(
            macro_: &'a Macro,
            macros: &'a [Macro],
            stack: &mut Vec<String>,
            pred: fn(&ActionKind) -> bool,
        ) -> bool {
            let mut hit = false;
            let mut run_names = Vec::new();
            macro_.root.walk(&mut |action| match &action.kind {
                ActionKind::RunMacro { macro_name } => run_names.push(macro_name.clone()),
                kind if pred(kind) => hit = true,
                _ => {}
            });
            hit || run_names.into_iter().any(|name| {
                let name = name.trim();
                if name.is_empty() || stack.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                    return false;
                }
                let Some(target) = macros.iter().find(|m| m.name.eq_ignore_ascii_case(name)) else {
                    return false;
                };
                stack.push(target.name.clone());
                let found = rec(target, macros, stack, pred);
                stack.pop();
                found
            })
        }
        rec(self, macros, &mut vec![self.name.clone()], pred)
    }
}

fn detection_needs_fast_capture(kind: &ActionKind) -> bool {
    kind.detection()
        .is_some_and(|detection| detection.wait.needs_fast_capture())
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

    fn find_pixel(name: &str) -> Action {
        Action {
            id: crate::ActionId::new(),
            kind: ActionKind::FindPixel {
                name: name.into(),
                search_area: Default::default(),
                target_color: "#fff".into(),
                color_tolerance: 0,
                detection: Default::default(),
            },
        }
    }

    fn run_macro(name: &str) -> Action {
        Action {
            id: crate::ActionId::new(),
            kind: ActionKind::RunMacro {
                macro_name: name.into(),
            },
        }
    }

    fn push(macro_: &mut Macro, action: Action) {
        macro_
            .root
            .children_mut()
            .expect("root children")
            .push(action);
    }

    #[test]
    fn uses_screen_capture_flags_detection_steps() {
        let mut m = Macro::new("m", 0, vec![]);
        assert!(!m.uses_screen_capture(&[]));
        push(&mut m, run_macro("other"));
        assert!(!m.uses_screen_capture(&[]));
        push(&mut m, find_pixel("p"));
        assert!(m.uses_screen_capture(&[]));
    }

    #[test]
    fn uses_screen_capture_follows_run_macro() {
        let mut caller = Macro::new("caller", 0, vec![]);
        push(&mut caller, run_macro("helper"));
        let mut helper = Macro::new("helper", 0, vec![]);
        push(&mut helper, find_pixel("p"));
        let macros = vec![caller.clone(), helper];
        assert!(caller.uses_screen_capture(&macros));
    }

    #[test]
    fn uses_screen_capture_cuts_cycles_and_missing_targets() {
        let mut a = Macro::new("a", 0, vec![]);
        push(&mut a, run_macro("a"));
        push(&mut a, run_macro("missing"));
        let macros = vec![a.clone()];
        assert!(!a.uses_screen_capture(&macros));
        assert!(!a.needs_fast_capture(&macros));
    }

    fn fast_pixel(interval_ms: i32, seconds: f64, mode: crate::RepeatMode) -> Action {
        Action {
            id: crate::ActionId::new(),
            kind: ActionKind::FindPixel {
                name: "p".into(),
                search_area: Default::default(),
                target_color: "#fff".into(),
                color_tolerance: 0,
                detection: crate::DetectionBranch {
                    wait: crate::WaitTilFoundConfig {
                        repeat_mode: mode,
                        wait_til_found_seconds: seconds,
                        wait_til_found_interval_ms: interval_ms,
                        max_iterations: 0,
                    },
                    ..Default::default()
                },
            },
        }
    }

    #[test]
    fn needs_fast_capture_flags_a_sub_screenshot_gap() {
        let mut once = Macro::new("once", 0, vec![]);
        push(&mut once, find_pixel("p"));
        assert!(once.uses_screen_capture(&[]));
        assert!(!once.needs_fast_capture(&[]));

        let mut slow = Macro::new("slow", 0, vec![]);
        push(
            &mut slow,
            fast_pixel(0, 2.0, crate::RepeatMode::WaitUntilFound),
        );
        assert!(!slow.needs_fast_capture(&[]));

        let mut fast = Macro::new("fast", 0, vec![]);
        push(
            &mut fast,
            fast_pixel(200, 2.0, crate::RepeatMode::RepeatUntilFound),
        );
        assert!(fast.needs_fast_capture(&[]));

        let mut capped = Macro::new("capped", 0, vec![]);
        push(
            &mut capped,
            fast_pixel(0, 0.2, crate::RepeatMode::WaitUntilFound),
        );
        assert!(capped.needs_fast_capture(&[]));

        let mut wide = Macro::new("wide", 0, vec![]);
        push(
            &mut wide,
            fast_pixel(0, 0.4, crate::RepeatMode::WaitWhileFound),
        );
        assert!(!wide.needs_fast_capture(&[]));
    }

    #[test]
    fn needs_fast_capture_follows_run_macro_and_ignores_a_loop_of_one_shots() {
        let mut caller = Macro::new("caller", 0, vec![]);
        push(&mut caller, run_macro("helper"));
        let mut helper = Macro::new("helper", 0, vec![]);
        push(
            &mut helper,
            fast_pixel(200, 0.0, crate::RepeatMode::RepeatWhileFound),
        );
        let macros = vec![caller.clone(), helper];
        assert!(caller.needs_fast_capture(&macros));

        let mut looped = Macro::new("looped", 0, vec![]);
        push(
            &mut looped,
            Action {
                id: crate::ActionId::new(),
                kind: ActionKind::Loop {
                    name: "n".into(),
                    count: crate::ScalarValue::Int(3),
                    subactions: vec![find_pixel("p")],
                },
            },
        );
        assert!(looped.uses_screen_capture(&[]));
        assert!(!looped.needs_fast_capture(&[]));
    }
}
