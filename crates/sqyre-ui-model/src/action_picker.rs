//! Add Action picker column order.
//!
//! Column labels come from taxonomy [`sqyre_domain::action_color_category`] so the
//! picker and Appearance color buckets share one IA (no divergent lookup table).

use sqyre_domain::action_color_category;

/// Picker column order (labels match taxonomy `color_category` / Appearance).
pub const ACTION_PICKER_CATEGORIES: &[&str] = &[
    "Mouse & Keyboard",
    "Detection",
    "Variables",
    "Control flow",
    "Wait",
    "Miscellaneous",
];

/// Category for the Add Action picker grid.
pub fn action_picker_category(action_type: &str) -> &'static str {
    let cat = action_color_category(action_type);
    if cat.is_empty() {
        "Miscellaneous"
    } else {
        cat
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{
        action_color_category, action_type_table, ACTION_COLOR_CATEGORIES,
        ACTION_COLOR_KEY_DEFAULT, ACTION_COLOR_KEY_WAIT, WIRE_TYPE_KEYS,
    };

    #[test]
    fn control_flow_picker_buckets() {
        assert_eq!(action_picker_category("loop"), "Control flow");
        assert_eq!(action_picker_category("foreachrow"), "Control flow");
        assert_eq!(action_picker_category("foreachcell"), "Control flow");
        assert_eq!(action_picker_category("conditional"), "Control flow");
        assert_eq!(action_picker_category("loopjump"), "Control flow");
        assert_eq!(action_picker_category("navigateselect"), "Miscellaneous");
        assert_eq!(action_picker_category("navigatekey"), "Miscellaneous");
    }

    #[test]
    fn wait_pause_picker_bucket_matches_appearance() {
        assert_eq!(action_picker_category("wait"), "Wait");
        assert_eq!(action_picker_category("pause"), "Wait");
        assert!(ACTION_PICKER_CATEGORIES.contains(&"Wait"));
        assert!(
            ACTION_COLOR_CATEGORIES
                .iter()
                .any(|&(key, label)| key == ACTION_COLOR_KEY_WAIT && label == "Wait"),
            "Appearance must keep a Wait color row"
        );
    }

    #[test]
    fn every_wire_key_has_picker_category_in_columns() {
        for key in WIRE_TYPE_KEYS {
            let cat = action_picker_category(key);
            assert!(
                ACTION_PICKER_CATEGORIES.contains(&cat),
                "picker category {cat:?} for {key} missing from ACTION_PICKER_CATEGORIES"
            );
        }
    }

    #[test]
    fn picker_category_matches_taxonomy_color_category() {
        for m in action_type_table() {
            assert_eq!(
                action_picker_category(m.type_key),
                action_color_category(m.type_key),
                "picker vs taxonomy diverge for {}",
                m.type_key
            );
        }
    }

    #[test]
    fn picker_columns_match_appearance_labels() {
        let appearance: Vec<&str> = ACTION_COLOR_CATEGORIES
            .iter()
            .filter(|(key, _)| *key != ACTION_COLOR_KEY_DEFAULT)
            .map(|(_, label)| *label)
            .collect();
        assert_eq!(
            ACTION_PICKER_CATEGORIES,
            appearance.as_slice(),
            "picker columns must match Appearance category labels (minus Default)"
        );
    }
}
