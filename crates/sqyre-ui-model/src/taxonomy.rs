//! Presentation-oriented action picker taxonomy (column order, category labels).

/// Add Action picker column order.
pub const ACTION_PICKER_CATEGORIES: &[&str] = &[
    "Mouse & Keyboard",
    "Detection",
    "Variables",
    "Control flow",
    "Miscellaneous",
];

/// Category for the Add Action picker grid.
pub fn action_picker_category(action_type: &str) -> &'static str {
    sqyre_domain::action_picker_category(action_type)
}
