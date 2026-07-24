//! Single source of truth for action type metadata (label, description, categories,
//! icon, color key, delay class).
//!
//! Labels are used by the executor (action logs) as well as the UI. Picker column
//! order ([`ACTION_PICKER_CATEGORIES`]) is presentation-oriented and may move to
//! an app-facing module later; keep wire `type_key` values stable.
//!
//! # Adding a kind
//!
//! 1. Registry row in [`crate::action::wire_keys`] (`define_action_wire_keys!`)
//! 2. Row in [`ACTION_TYPE_TABLE`] below
//! 3. `ActionKind` variant + `blank_kind` defaults
//! 4. Serde wire struct / `From` mirrors in `action_serde`
//! 5. Exhaustive matches: edit UI, executor dispatch, `display_params` / `display_name` /
//!    `children` (and optional validate / rename / bindings)

use crate::color::{
    ACTION_COLOR_KEY_CONTROL_FLOW, ACTION_COLOR_KEY_DEFAULT, ACTION_COLOR_KEY_DETECTION,
    ACTION_COLOR_KEY_MISCELLANEOUS, ACTION_COLOR_KEY_MOUSE_KEYBOARD, ACTION_COLOR_KEY_VARIABLES,
    ACTION_COLOR_KEY_WAIT,
};

/// Number of addable [`crate::ActionKind`] variants / taxonomy rows.
///
/// Derived from [`ACTION_TYPE_TABLE`].
pub const ACTION_KIND_COUNT: usize = ACTION_TYPE_TABLE.len();

/// Picker column order (also used as color-bucket keys for most types).
pub const ACTION_PICKER_CATEGORIES: &[&str] = &[
    "Mouse & Keyboard",
    "Detection",
    "Variables",
    "Control flow",
    "Miscellaneous",
];

/// Post-action delay bucket for macro mouse/keyboard delay settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayClass {
    None,
    Mouse,
    Keyboard,
}

/// Static metadata for one action type key.
#[derive(Debug, Clone, Copy)]
pub struct ActionTypeMeta {
    pub type_key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// Add Action picker column.
    pub picker_category: &'static str,
    /// Pastel color bucket label (may differ from picker for loop/nav types).
    pub color_category: &'static str,
    /// Settings pastel override key ([`ACTION_COLOR_KEY_*`](crate::color)).
    pub color_key: &'static str,
    /// Static tree-pill glyph (Click/Key may override by press state in UI).
    pub icon: &'static str,
    pub delay_class: DelayClass,
}

const ACTION_TYPE_TABLE: &[ActionTypeMeta] = &[
    ActionTypeMeta {
        type_key: "move",
        label: "Mouse Move",
        description: "Moves the mouse cursor to a target position.",
        picker_category: "Mouse & Keyboard",
        color_category: "Mouse & Keyboard",
        color_key: ACTION_COLOR_KEY_MOUSE_KEYBOARD,
        icon: "➔",
        delay_class: DelayClass::Mouse,
    },
    ActionTypeMeta {
        type_key: "click",
        label: "Click",
        description: "Clicks a mouse button at the current cursor position.",
        picker_category: "Mouse & Keyboard",
        color_category: "Mouse & Keyboard",
        color_key: ACTION_COLOR_KEY_MOUSE_KEYBOARD,
        icon: "⬇",
        delay_class: DelayClass::Mouse,
    },
    ActionTypeMeta {
        type_key: "key",
        label: "Key",
        description: "Presses or releases a single keyboard key.",
        picker_category: "Mouse & Keyboard",
        color_category: "Mouse & Keyboard",
        color_key: ACTION_COLOR_KEY_MOUSE_KEYBOARD,
        icon: "⬇",
        delay_class: DelayClass::Keyboard,
    },
    ActionTypeMeta {
        type_key: "type",
        label: "Type",
        description: "Types out a string of text, one character at a time.",
        picker_category: "Mouse & Keyboard",
        color_category: "Mouse & Keyboard",
        color_key: ACTION_COLOR_KEY_MOUSE_KEYBOARD,
        icon: "⌨",
        delay_class: DelayClass::Keyboard,
    },
    ActionTypeMeta {
        type_key: "imagesearch",
        label: "Image Search",
        description: "Searches a screen region for images and saves match coordinates.",
        picker_category: "Detection",
        color_category: "Detection",
        color_key: ACTION_COLOR_KEY_DETECTION,
        icon: "🔍",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "ocr",
        label: "OCR",
        description:
            "Reads text from a screen region; runs nested actions when the target is found.",
        picker_category: "Detection",
        color_category: "Detection",
        color_key: ACTION_COLOR_KEY_DETECTION,
        icon: "🔤",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "findpixel",
        label: "Find pixel",
        description: "Scans a region for a pixel color; runs nested actions when found.",
        picker_category: "Detection",
        color_category: "Detection",
        color_key: ACTION_COLOR_KEY_DETECTION,
        icon: "🎨",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "setvariable",
        label: "Set",
        description:
            "Assigns a value to a variable; arithmetic expressions and ${refs} are evaluated.",
        picker_category: "Variables",
        color_category: "Variables",
        color_key: ACTION_COLOR_KEY_VARIABLES,
        icon: "x",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "savevariable",
        label: "Save to",
        description: "Writes a variable's value out to a file or the clipboard.",
        picker_category: "Variables",
        color_category: "Variables",
        color_key: ACTION_COLOR_KEY_VARIABLES,
        icon: "💾",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "loop",
        label: "Loop",
        description: "Repeats its sub-actions a set number of times.",
        picker_category: "Control flow",
        color_category: "Control flow",
        color_key: ACTION_COLOR_KEY_CONTROL_FLOW,
        icon: "↻",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "while",
        label: "While",
        description: "Repeats its sub-actions while conditions remain true.",
        picker_category: "Control flow",
        color_category: "Control flow",
        color_key: ACTION_COLOR_KEY_CONTROL_FLOW,
        icon: "↻",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "loopjump",
        label: "Break / Continue",
        description: "Break exits the innermost loop; Continue skips to its next iteration.",
        picker_category: "Control flow",
        color_category: "Control flow",
        color_key: ACTION_COLOR_KEY_CONTROL_FLOW,
        icon: "⏹",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "foreachrow",
        label: "For each row",
        description: "Runs its sub-actions once per row of a list source.",
        picker_category: "Control flow",
        color_category: "Control flow",
        color_key: ACTION_COLOR_KEY_CONTROL_FLOW,
        icon: "☰",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "conditional",
        label: "If",
        description: "Runs its sub-actions only when the conditions are true.",
        picker_category: "Control flow",
        color_category: "Control flow",
        color_key: ACTION_COLOR_KEY_CONTROL_FLOW,
        icon: "?",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "wait",
        label: "Wait",
        description: "Pauses for a fixed number of milliseconds, then continues.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
        color_key: ACTION_COLOR_KEY_WAIT,
        icon: "⏱",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "pause",
        label: "Pause",
        description: "Halts the macro until you press the continue key.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
        color_key: ACTION_COLOR_KEY_WAIT,
        icon: "⏸",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "focuswindow",
        label: "Focus window",
        description: "Brings a window to the front, matched by program and title.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
        color_key: ACTION_COLOR_KEY_MISCELLANEOUS,
        icon: "👁",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "runmacro",
        label: "Run macro",
        description: "Runs another macro inline as a sub-routine.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
        color_key: ACTION_COLOR_KEY_MISCELLANEOUS,
        icon: "▶",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "navigateselect",
        label: "Navigate Select",
        description:
            "Navigates a collection grid with chords; Nav Key children branch on custom keys.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
        color_key: ACTION_COLOR_KEY_MISCELLANEOUS,
        icon: "⌖",
        delay_class: DelayClass::None,
    },
    ActionTypeMeta {
        type_key: "navigatekey",
        label: "Nav Key",
        description: "Under Navigate Select: when this chord is pressed, runs nested actions.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
        color_key: ACTION_COLOR_KEY_MISCELLANEOUS,
        // ⌨-family / Misc Technical glyphs (⎇) often tofu in egui fonts.
        icon: "🔑",
        delay_class: DelayClass::None,
    },
];

fn lookup(action_type: &str) -> Option<&'static ActionTypeMeta> {
    let key = action_type.trim().to_ascii_lowercase();
    ACTION_TYPE_TABLE.iter().find(|m| m.type_key == key)
}

/// All known action type metadata rows (picker order).
pub fn action_type_table() -> &'static [ActionTypeMeta] {
    ACTION_TYPE_TABLE
}

pub fn action_type_label(action_type: &str) -> &'static str {
    lookup(action_type).map(|m| m.label).unwrap_or("Unknown")
}

pub fn action_type_description(action_type: &str) -> &'static str {
    lookup(action_type).map(|m| m.description).unwrap_or("")
}

/// Category for the Add Action picker grid.
pub fn action_picker_category(action_type: &str) -> &'static str {
    lookup(action_type)
        .map(|m| m.picker_category)
        .unwrap_or("Miscellaneous")
}

/// Pastel color bucket label for tree/UI badges.
pub fn action_color_category(action_type: &str) -> &'static str {
    lookup(action_type).map(|m| m.color_category).unwrap_or("")
}

/// Settings pastel override key for an action type.
pub fn action_color_key(action_type: &str) -> &'static str {
    lookup(action_type)
        .map(|m| m.color_key)
        .unwrap_or(ACTION_COLOR_KEY_DEFAULT)
}

/// Static tree-pill glyph for an action type key.
pub fn action_icon(action_type: &str) -> &'static str {
    lookup(action_type).map(|m| m.icon).unwrap_or("?")
}

/// Mouse/keyboard delay class for post-action sleep.
pub fn action_delay_class(action_type: &str) -> DelayClass {
    lookup(action_type)
        .map(|m| m.delay_class)
        .unwrap_or(DelayClass::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blank_action;

    #[test]
    fn table_len_matches_action_kind_count() {
        assert_eq!(
            ACTION_TYPE_TABLE.len(),
            ACTION_KIND_COUNT,
            "bump ACTION_KIND_COUNT when adding a taxonomy row / ActionKind"
        );
    }

    #[test]
    fn table_covers_picker_templates() {
        for m in ACTION_TYPE_TABLE {
            assert_eq!(action_type_label(m.type_key), m.label);
            assert_eq!(action_picker_category(m.type_key), m.picker_category);
            assert_eq!(action_color_category(m.type_key), m.color_category);
            assert_eq!(action_color_key(m.type_key), m.color_key);
            assert_eq!(action_icon(m.type_key), m.icon);
            assert_eq!(action_delay_class(m.type_key), m.delay_class);
        }
        assert_eq!(action_type_label("nope"), "Unknown");
        assert_eq!(action_color_category("nope"), "");
        assert_eq!(action_color_key("nope"), ACTION_COLOR_KEY_DEFAULT);
        assert_eq!(action_delay_class("nope"), DelayClass::None);
    }

    #[test]
    fn every_taxonomy_row_has_blank_factory() {
        for m in ACTION_TYPE_TABLE {
            let action = blank_action(m.type_key).unwrap_or_else(|| {
                panic!(
                    "blank_action missing for taxonomy type_key {:?}",
                    m.type_key
                )
            });
            assert_eq!(
                action.type_key(),
                m.type_key,
                "blank type_key mismatch for {:?}",
                m.type_key
            );
            assert_ne!(
                action_type_label(m.type_key),
                "Unknown",
                "taxonomy label missing for {:?}",
                m.type_key
            );
        }
    }

    #[test]
    fn taxonomy_type_keys_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for m in ACTION_TYPE_TABLE {
            assert!(
                seen.insert(m.type_key),
                "duplicate taxonomy type_key {:?}",
                m.type_key
            );
        }
    }

    #[test]
    fn taxonomy_type_keys_match_wire_registry() {
        use crate::WIRE_TYPE_KEYS;
        let table: std::collections::BTreeSet<_> =
            ACTION_TYPE_TABLE.iter().map(|m| m.type_key).collect();
        let wire: std::collections::BTreeSet<_> = WIRE_TYPE_KEYS.iter().copied().collect();
        assert_eq!(
            table, wire,
            "ACTION_TYPE_TABLE type_keys must match WIRE_TYPE_KEYS (wire-key registry)"
        );
        assert_eq!(WIRE_TYPE_KEYS.len(), ACTION_KIND_COUNT);
    }

    #[test]
    fn control_flow_picker_and_color_buckets() {
        assert_eq!(action_picker_category("loop"), "Control flow");
        assert_eq!(action_color_category("loop"), "Control flow");
        assert_eq!(action_color_key("loop"), ACTION_COLOR_KEY_CONTROL_FLOW);
        assert_eq!(action_picker_category("foreachrow"), "Control flow");
        assert_eq!(action_color_category("foreachrow"), "Control flow");
        assert_eq!(
            action_color_key("foreachrow"),
            ACTION_COLOR_KEY_CONTROL_FLOW
        );
        assert_eq!(action_picker_category("conditional"), "Control flow");
        assert_eq!(
            action_color_key("conditional"),
            ACTION_COLOR_KEY_CONTROL_FLOW
        );
        assert_eq!(action_picker_category("navigateselect"), "Miscellaneous");
        assert_eq!(
            action_color_key("navigateselect"),
            ACTION_COLOR_KEY_MISCELLANEOUS
        );
        assert_eq!(action_picker_category("navigatekey"), "Miscellaneous");
        assert_eq!(action_icon("navigatekey"), "🔑");
    }

    #[test]
    fn wait_pause_use_wait_color_key() {
        assert_eq!(action_color_key("wait"), ACTION_COLOR_KEY_WAIT);
        assert_eq!(action_color_key("pause"), ACTION_COLOR_KEY_WAIT);
    }

    #[test]
    fn mouse_keyboard_delay_classes() {
        assert_eq!(action_delay_class("move"), DelayClass::Mouse);
        assert_eq!(action_delay_class("click"), DelayClass::Mouse);
        assert_eq!(action_delay_class("key"), DelayClass::Keyboard);
        assert_eq!(action_delay_class("type"), DelayClass::Keyboard);
        assert_eq!(action_delay_class("wait"), DelayClass::None);
    }
}
