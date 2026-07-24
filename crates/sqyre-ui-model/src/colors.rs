//! Action tree color keys, pastels, and custom overrides.
//!
//! Hex parse/format helpers and category keys live in [`sqyre_domain::color`]
//! so vision and persist can use them without depending on UI chrome.

use sqyre_domain::{
    action_color_key as taxonomy_color_key, ACTION_COLOR_KEY_CONTROL_FLOW,
    ACTION_COLOR_KEY_DETECTION, ACTION_COLOR_KEY_MISCELLANEOUS, ACTION_COLOR_KEY_MOUSE_KEYBOARD,
    ACTION_COLOR_KEY_VARIABLES, ACTION_COLOR_KEY_WAIT,
};
use std::collections::HashMap;
use std::sync::RwLock;

static CUSTOM_ACTION_COLORS: RwLock<Option<HashMap<String, [u8; 4]>>> = RwLock::new(None);

fn action_color_key(action_type: &str) -> &'static str {
    taxonomy_color_key(action_type)
}

/// Sample action type used when previewing a category swatch.
pub fn sample_action_type_for_color_key(category_key: &str) -> &'static str {
    match category_key {
        ACTION_COLOR_KEY_MOUSE_KEYBOARD => "click",
        ACTION_COLOR_KEY_DETECTION => "imagesearch",
        ACTION_COLOR_KEY_VARIABLES => "setvariable",
        ACTION_COLOR_KEY_CONTROL_FLOW => "loop",
        ACTION_COLOR_KEY_MISCELLANEOUS => "focuswindow",
        ACTION_COLOR_KEY_WAIT => "wait",
        _ => "",
    }
}

/// Store a user-chosen color for a category key.
pub fn set_custom_action_color(category_key: &str, rgba: [u8; 4]) {
    let mut guard = CUSTOM_ACTION_COLORS.write().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(category_key.to_string(), rgba);
}

/// Remove a user override for a category key.
pub fn clear_custom_action_color(category_key: &str) {
    let mut guard = CUSTOM_ACTION_COLORS.write().unwrap();
    if let Some(map) = guard.as_mut() {
        map.remove(category_key);
    }
}

/// Remove every user override.
pub fn clear_all_custom_action_colors() {
    *CUSTOM_ACTION_COLORS.write().unwrap() = None;
}

/// Category pastel color, light/dark theme.
/// Uses a user override when one is set; otherwise the built-in pastel.
pub fn action_pastel_color(action_type: &str, is_dark: bool) -> [u8; 4] {
    let t = action_type.trim().to_ascii_lowercase();
    if t != "warning" {
        let key = action_color_key(&t);
        if let Some(c) = CUSTOM_ACTION_COLORS
            .read()
            .unwrap()
            .as_ref()
            .and_then(|m| m.get(key).copied())
        {
            return c;
        }
    }
    default_action_pastel_color(action_type, is_dark)
}

/// Built-in pastel, ignoring user overrides.
pub fn default_action_pastel_color(action_type: &str, is_dark: bool) -> [u8; 4] {
    let t = action_type.trim().to_ascii_lowercase();
    if t == "warning" {
        return if is_dark {
            [0x8A, 0x5A, 0x2A, 0xFF]
        } else {
            [0xF0, 0xC0, 0x6A, 0xFF]
        };
    }
    let key = action_color_key(&t);

    if is_dark {
        return match key {
            ACTION_COLOR_KEY_WAIT => [0x7B, 0x4E, 0x3E, 0xFF],
            ACTION_COLOR_KEY_MOUSE_KEYBOARD => [0x5E, 0x6B, 0x4A, 0xFF],
            ACTION_COLOR_KEY_DETECTION => [0x5A, 0x4A, 0x44, 0xFF],
            ACTION_COLOR_KEY_VARIABLES => [0x2A, 0x42, 0x54, 0xFF],
            ACTION_COLOR_KEY_CONTROL_FLOW => [0x3A, 0x5A, 0x58, 0xFF],
            ACTION_COLOR_KEY_MISCELLANEOUS => [0x8A, 0x45, 0x68, 0xFF],
            _ => [0x5C, 0x54, 0x49, 0xFF],
        };
    }
    match key {
        ACTION_COLOR_KEY_WAIT => [0xC9, 0x8D, 0x6A, 0xFF],
        ACTION_COLOR_KEY_MOUSE_KEYBOARD => [0xA1, 0xB0, 0x7A, 0xFF],
        ACTION_COLOR_KEY_DETECTION => [0xB4, 0x9A, 0x84, 0xFF],
        ACTION_COLOR_KEY_VARIABLES => [0x5E, 0x8F, 0xB0, 0xFF],
        ACTION_COLOR_KEY_CONTROL_FLOW => [0x7A, 0xB8, 0xB0, 0xFF],
        ACTION_COLOR_KEY_MISCELLANEOUS => [0xE0, 0x90, 0xB0, 0xFF],
        _ => [0xB2, 0xA4, 0x8E, 0xFF],
    }
}

/// Nested `${var}` chip fill.
pub fn nested_var_ref_color(is_dark: bool) -> [u8; 4] {
    if is_dark {
        [0x46, 0x62, 0x78, 0xFF]
    } else {
        [0x9E, 0xC4, 0xE3, 0xFF]
    }
}
