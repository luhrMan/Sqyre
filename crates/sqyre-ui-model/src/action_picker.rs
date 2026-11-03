//! Add Action picker tile order.
//!
//! Category labels come from taxonomy [`sqyre_domain::action_color_category`] so the
//! picker and Appearance color buckets share one IA (no divergent lookup table).

use sqyre_domain::{action_color_category, ActionTemplate};

/// Picker category order (labels match taxonomy `color_category` / Appearance).
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

/// Tile order for the Add Action picker grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionPickerSort {
    /// [`ACTION_PICKER_CATEGORIES`] order; table order within each category.
    #[default]
    Category,
    NameAsc,
    NameDesc,
}

impl ActionPickerSort {
    pub const ALL: &'static [Self] = &[Self::Category, Self::NameAsc, Self::NameDesc];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Category => "Category",
            Self::NameAsc => "Name A→Z",
            Self::NameDesc => "Name Z→A",
        }
    }
}

/// Order filtered picker templates for the flat grid (`sort` semantics above).
/// Stable: equal keys keep the action-type-table order.
pub fn ordered_action_templates<'a>(
    templates: &[&'a ActionTemplate],
    sort: ActionPickerSort,
) -> Vec<&'a ActionTemplate> {
    let mut ordered = templates.to_vec();
    let name_key = |t: &&ActionTemplate| (t.label.to_ascii_lowercase(), t.action_type);
    match sort {
        ActionPickerSort::Category => ordered.sort_by_key(|t| {
            ACTION_PICKER_CATEGORIES
                .iter()
                .position(|c| *c == action_picker_category(t.action_type))
                .unwrap_or(ACTION_PICKER_CATEGORIES.len())
        }),
        ActionPickerSort::NameAsc => ordered.sort_by_key(name_key),
        ActionPickerSort::NameDesc => ordered.sort_by_key(|t| std::cmp::Reverse(name_key(t))),
    }
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{
        action_color_category, action_templates, action_type_table, ActionTemplate,
        ACTION_COLOR_CATEGORIES, ACTION_COLOR_KEY_DEFAULT, ACTION_COLOR_KEY_WAIT, WIRE_TYPE_KEYS,
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
    fn ordered_templates_category_groups_then_keeps_table_order() {
        let templates = action_templates();
        let refs: Vec<&ActionTemplate> = templates.iter().collect();
        let ordered = ordered_action_templates(&refs, ActionPickerSort::Category);
        assert_eq!(ordered.len(), templates.len());
        let mut last = 0usize;
        for t in &ordered {
            let pos = ACTION_PICKER_CATEGORIES
                .iter()
                .position(|c| *c == action_picker_category(t.action_type))
                .unwrap_or(usize::MAX);
            assert!(pos >= last, "category order regressed at {}", t.action_type);
            last = pos;
        }
    }

    #[test]
    fn ordered_templates_name_sorts_by_label() {
        let templates = action_templates();
        let refs: Vec<&ActionTemplate> = templates.iter().collect();
        let asc = ordered_action_templates(&refs, ActionPickerSort::NameAsc);
        let desc = ordered_action_templates(&refs, ActionPickerSort::NameDesc);
        for pair in asc.windows(2) {
            assert!(
                pair[0].label.to_ascii_lowercase() <= pair[1].label.to_ascii_lowercase(),
                "A→Z broke at {} → {}",
                pair[0].label,
                pair[1].label
            );
        }
        for pair in desc.windows(2) {
            assert!(
                pair[0].label.to_ascii_lowercase() >= pair[1].label.to_ascii_lowercase(),
                "Z→A broke at {} → {}",
                pair[0].label,
                pair[1].label
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
