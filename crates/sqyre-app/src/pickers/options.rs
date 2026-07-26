//! Static option lists for ComboBox fields (>2 options).

pub const LOOP_JUMP_MODES: &[(&str, &str)] = &[("break", "Break"), ("continue", "Continue")];

/// Match-order options as `(stored value, display label)`.
pub const ORDER_GROUPING: &[(&str, &str)] =
    &[("row", "Row"), ("column", "Column"), ("none", "None")];
pub const ORDER_HORIZONTAL: &[(&str, &str)] = &[
    ("left_to_right", "Left → Right"),
    ("right_to_left", "Right → Left"),
];
pub const ORDER_VERTICAL: &[(&str, &str)] = &[
    ("top_to_bottom", "Top → Bottom"),
    ("bottom_to_top", "Bottom → Top"),
];

pub const SELECT_DEVICES: &[&str] = &["", "mouse", "keyboard"];
pub const SELECT_PRESS_MODES: &[&str] = &["", "click", "down", "up", "hold"];
pub const MOUSE_BUTTONS: &[&str] = &["", "left", "right", "center"];
