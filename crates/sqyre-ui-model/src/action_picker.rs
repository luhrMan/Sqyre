//! Add Action picker column order and per-type category labels.

/// Picker column order (also used as color-bucket keys for most types).
pub const ACTION_PICKER_CATEGORIES: &[&str] = &[
    "Mouse & Keyboard",
    "Detection",
    "Variables",
    "Control flow",
    "Miscellaneous",
];

fn lookup_category(type_key: &str) -> Option<&'static str> {
    let key = type_key.trim().to_ascii_lowercase();
    Some(match key.as_str() {
        "move" | "click" | "key" | "type" => "Mouse & Keyboard",
        "imagesearch" | "ocr" | "findpixel" => "Detection",
        "setvariable" | "savevariable" => "Variables",
        "loop" | "while" | "conditional" | "foreachrow" => "Control flow",
        "wait" | "pause" | "focuswindow" | "runmacro" | "navigateselect" | "navigatekey"
        | "loopjump" => "Miscellaneous",
        _ => return None,
    })
}

/// Category for the Add Action picker grid.
pub fn action_picker_category(action_type: &str) -> &'static str {
    lookup_category(action_type).unwrap_or("Miscellaneous")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_flow_picker_buckets() {
        assert_eq!(action_picker_category("loop"), "Control flow");
        assert_eq!(action_picker_category("foreachrow"), "Control flow");
        assert_eq!(action_picker_category("conditional"), "Control flow");
        assert_eq!(action_picker_category("navigateselect"), "Miscellaneous");
        assert_eq!(action_picker_category("navigatekey"), "Miscellaneous");
    }

    #[test]
    fn every_wire_key_has_category() {
        for key in sqyre_domain::WIRE_TYPE_KEYS {
            assert!(
                lookup_category(key).is_some(),
                "missing picker category for {key}"
            );
        }
    }
}
