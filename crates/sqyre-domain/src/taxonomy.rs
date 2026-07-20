//! Single source of truth for action type metadata (label, description, categories).
//!
//! Labels are used by the executor (action logs) as well as the UI. Picker column
//! order ([`ACTION_PICKER_CATEGORIES`]) is presentation-oriented and may move to
//! an app-facing module later; keep wire `type_key` values stable.

/// Number of addable [`crate::ActionKind`] variants / taxonomy rows.
///
/// Derived from [`ACTION_TYPE_TABLE`]. When adding a kind, update the table,
/// `blank_kind` / serde tags, and exhaustive `ActionKind` matches.
pub const ACTION_KIND_COUNT: usize = ACTION_TYPE_TABLE.len();

/// Picker column order (also used as color-bucket keys for most types).
pub const ACTION_PICKER_CATEGORIES: &[&str] = &[
    "Mouse & Keyboard",
    "Detection",
    "Variables",
    "Loop flow",
    "Miscellaneous",
];

/// Static metadata for one action type key.
#[derive(Debug, Clone, Copy)]
pub struct ActionTypeMeta {
    pub type_key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// Add Action picker column.
    pub picker_category: &'static str,
    /// Pastel color bucket (may differ from picker for loop/nav types).
    pub color_category: &'static str,
}

const ACTION_TYPE_TABLE: &[ActionTypeMeta] = &[
    ActionTypeMeta {
        type_key: "move",
        label: "Mouse Move",
        description: "Moves the mouse cursor to a target position.",
        picker_category: "Mouse & Keyboard",
        color_category: "Mouse & Keyboard",
    },
    ActionTypeMeta {
        type_key: "click",
        label: "Click",
        description: "Clicks a mouse button at the current cursor position.",
        picker_category: "Mouse & Keyboard",
        color_category: "Mouse & Keyboard",
    },
    ActionTypeMeta {
        type_key: "key",
        label: "Key",
        description: "Presses or releases a single keyboard key.",
        picker_category: "Mouse & Keyboard",
        color_category: "Mouse & Keyboard",
    },
    ActionTypeMeta {
        type_key: "type",
        label: "Type",
        description: "Types out a string of text, one character at a time.",
        picker_category: "Mouse & Keyboard",
        color_category: "Mouse & Keyboard",
    },
    ActionTypeMeta {
        type_key: "imagesearch",
        label: "Image Search",
        description: "Searches a screen region for images and saves match coordinates.",
        picker_category: "Detection",
        color_category: "Detection",
    },
    ActionTypeMeta {
        type_key: "ocr",
        label: "OCR",
        description:
            "Reads text from a screen region; runs nested actions when the target is found.",
        picker_category: "Detection",
        color_category: "Detection",
    },
    ActionTypeMeta {
        type_key: "findpixel",
        label: "Find pixel",
        description: "Scans a region for a pixel color; runs nested actions when found.",
        picker_category: "Detection",
        color_category: "Detection",
    },
    ActionTypeMeta {
        type_key: "setvariable",
        label: "Set",
        description:
            "Assigns a value to a variable; arithmetic expressions and ${refs} are evaluated.",
        picker_category: "Variables",
        color_category: "Variables",
    },
    ActionTypeMeta {
        type_key: "foreachrow",
        label: "For each row",
        description: "Runs its sub-actions once per row of a list source.",
        picker_category: "Variables",
        color_category: "Variables",
    },
    ActionTypeMeta {
        type_key: "savevariable",
        label: "Save to",
        description: "Writes a variable's value out to a file or the clipboard.",
        picker_category: "Variables",
        color_category: "Variables",
    },
    ActionTypeMeta {
        type_key: "loop",
        label: "Loop",
        description: "Repeats its sub-actions a set number of times.",
        picker_category: "Loop flow",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "while",
        label: "While",
        description: "Repeats its sub-actions while conditions remain true.",
        picker_category: "Loop flow",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "break",
        label: "Break",
        description: "Exits the innermost enclosing loop immediately.",
        picker_category: "Loop flow",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "continue",
        label: "Continue",
        description: "Skips to the next iteration of the enclosing loop.",
        picker_category: "Loop flow",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "navigateselect",
        label: "Navigate Select",
        description:
            "Navigates a collection grid with chords; Nav Key children branch on custom keys.",
        picker_category: "Loop flow",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "navigatekey",
        label: "Nav Key",
        description: "Under Navigate Select: when this chord is pressed, runs nested actions.",
        picker_category: "Loop flow",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "wait",
        label: "Wait",
        description: "Pauses for a fixed number of milliseconds, then continues.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "pause",
        label: "Pause",
        description: "Halts the macro until you press the continue key.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "focuswindow",
        label: "Focus window",
        description: "Brings a window to the front, matched by program and title.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "runmacro",
        label: "Run macro",
        description: "Runs another macro inline as a sub-routine.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
    },
    ActionTypeMeta {
        type_key: "conditional",
        label: "If",
        description: "Runs its sub-actions only when the conditions are true.",
        picker_category: "Miscellaneous",
        color_category: "Miscellaneous",
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

/// Pastel color bucket for tree/UI badges.
pub fn action_color_category(action_type: &str) -> &'static str {
    lookup(action_type).map(|m| m.color_category).unwrap_or("")
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
        }
        assert_eq!(action_type_label("nope"), "Unknown");
        assert_eq!(action_color_category("nope"), "");
    }

    #[test]
    fn every_taxonomy_row_has_blank_factory() {
        for m in ACTION_TYPE_TABLE {
            let action = blank_action(m.type_key).unwrap_or_else(|| {
                panic!("blank_action missing for taxonomy type_key {:?}", m.type_key)
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
    fn loop_types_use_misc_color_bucket() {
        assert_eq!(action_picker_category("loop"), "Loop flow");
        assert_eq!(action_color_category("loop"), "Miscellaneous");
        assert_eq!(action_picker_category("navigateselect"), "Loop flow");
        assert_eq!(action_color_category("navigateselect"), "Miscellaneous");
    }
}
