//! Action tree color keys, pastels, and hex helpers.

use crate::action_color_category;
use std::collections::HashMap;
use std::sync::RwLock;

/// Category keys for customizable macro-tree action colors.
pub const ACTION_COLOR_KEY_MOUSE_KEYBOARD: &str = "mouse_keyboard";
pub const ACTION_COLOR_KEY_DETECTION: &str = "detection";
pub const ACTION_COLOR_KEY_VARIABLES: &str = "variables";
pub const ACTION_COLOR_KEY_MISCELLANEOUS: &str = "miscellaneous";
pub const ACTION_COLOR_KEY_WAIT: &str = "wait";
pub const ACTION_COLOR_KEY_DEFAULT: &str = "default";

/// `(key, label)` for every customizable action color group.
pub const ACTION_COLOR_CATEGORIES: &[(&str, &str)] = &[
    (ACTION_COLOR_KEY_MOUSE_KEYBOARD, "Mouse & Keyboard"),
    (ACTION_COLOR_KEY_DETECTION, "Detection"),
    (ACTION_COLOR_KEY_VARIABLES, "Variables"),
    (ACTION_COLOR_KEY_MISCELLANEOUS, "Miscellaneous"),
    (ACTION_COLOR_KEY_WAIT, "Wait"),
    (ACTION_COLOR_KEY_DEFAULT, "Default"),
];

static CUSTOM_ACTION_COLORS: RwLock<Option<HashMap<String, [u8; 4]>>> = RwLock::new(None);

fn action_color_key(action_type: &str) -> &'static str {
    let t = action_type.trim().to_ascii_lowercase();
    if t == "wait" || t == "pause" {
        return ACTION_COLOR_KEY_WAIT;
    }
    match action_color_category(&t) {
        "Mouse & Keyboard" => ACTION_COLOR_KEY_MOUSE_KEYBOARD,
        "Detection" => ACTION_COLOR_KEY_DETECTION,
        "Variables" => ACTION_COLOR_KEY_VARIABLES,
        "Miscellaneous" => ACTION_COLOR_KEY_MISCELLANEOUS,
        _ => ACTION_COLOR_KEY_DEFAULT,
    }
}

/// Sample action type used when previewing a category swatch.
pub fn sample_action_type_for_color_key(category_key: &str) -> &'static str {
    match category_key {
        ACTION_COLOR_KEY_MOUSE_KEYBOARD => "click",
        ACTION_COLOR_KEY_DETECTION => "imagesearch",
        ACTION_COLOR_KEY_VARIABLES => "setvariable",
        ACTION_COLOR_KEY_MISCELLANEOUS => "loop",
        ACTION_COLOR_KEY_WAIT => "wait",
        _ => "",
    }
}

/// Format RGBA as `#rrggbb` (alpha ignored).
pub fn format_hex_color(rgba: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2])
}

/// Strip `#` and leading AA when 8 hex digits; return lowercase RGB body.
///
/// Does not validate length — callers that need a real color should use
/// [`parse_hex_color`].
pub fn normalize_hex_rgb(hex: &str) -> String {
    let mut h = hex.trim().trim_start_matches('#').to_ascii_lowercase();
    if h.len() == 8 {
        h = h[2..].to_string();
    }
    h
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
    let is_wait = t == "wait" || t == "pause";
    let category = action_color_category(&t);

    if is_dark {
        if is_wait {
            return [0x7B, 0x4E, 0x3E, 0xFF];
        }
        return match category {
            "Mouse & Keyboard" => [0x5E, 0x6B, 0x4A, 0xFF],
            "Detection" => [0x5A, 0x4A, 0x44, 0xFF],
            "Variables" => [0x2A, 0x42, 0x54, 0xFF],
            "Miscellaneous" => [0x6A, 0x5A, 0x3F, 0xFF],
            _ => [0x5C, 0x54, 0x49, 0xFF],
        };
    }
    if is_wait {
        return [0xC9, 0x8D, 0x6A, 0xFF];
    }
    match category {
        "Mouse & Keyboard" => [0xA1, 0xB0, 0x7A, 0xFF],
        "Detection" => [0xB4, 0x9A, 0x84, 0xFF],
        "Variables" => [0x5E, 0x8F, 0xB0, 0xFF],
        "Miscellaneous" => [0xB8, 0x9A, 0x6A, 0xFF],
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

/// Parse `#RGB`, `#RRGGBB`, or `#AARRGGBB` into RGBA (alpha forced to 255).
pub fn parse_hex_color(hex: &str) -> Option<[u8; 4]> {
    let h = normalize_hex_rgb(hex);
    if h.len() == 3 {
        let r = u8::from_str_radix(&h[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&h[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&h[2..3].repeat(2), 16).ok()?;
        return Some([r, g, b, 255]);
    }
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some([r, g, b, 255])
}
