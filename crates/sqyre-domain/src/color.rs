//! Hex color parsing shared by vision (find-pixel) and UI chrome.
//!
//! Kept outside [`crate::display`] so non-UI crates can depend on it without
//! pulling presentation helpers.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_strips_alpha_and_short_form() {
        assert_eq!(parse_hex_color("#ff112233"), Some([0x11, 0x22, 0x33, 255]));
        assert_eq!(parse_hex_color("aabbcc"), Some([0xaa, 0xbb, 0xcc, 255]));
        assert_eq!(parse_hex_color("#abc"), Some([0xaa, 0xbb, 0xcc, 255]));
        assert_eq!(parse_hex_color("not-hex"), None);
    }

    #[test]
    fn format_roundtrips_rgb() {
        assert_eq!(format_hex_color([0xab, 0xcd, 0xef, 255]), "#abcdef");
    }
}
