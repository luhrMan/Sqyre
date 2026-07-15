//! Sample a screen pixel as `rrggbb` (Go `screen.GetPixelColor` + Find Pixel dropper).

use sqyre_capture::X11Capturer;
use sqyre_executor::{DesktopRect, ScreenCapturer};

/// Capture the 1×1 pixel at `(x, y)` and return lowercase hex without `#`.
///
/// Matches Go dropper normalization: strip `#`, drop leading alpha when 8 chars.
pub fn sample_pixel_hex(x: i32, y: i32) -> Result<String, String> {
    let mut capturer = X11Capturer::open()?;
    sample_pixel_hex_with(&mut capturer, x, y)
}

pub fn sample_pixel_hex_with(
    capturer: &mut dyn ScreenCapturer,
    x: i32,
    y: i32,
) -> Result<String, String> {
    let img = capturer.capture_rect(DesktopRect { x, y, w: 1, h: 1 })?;
    if img.width() < 1 || img.height() < 1 {
        return Err("empty pixel capture".into());
    }
    let px = img.get_pixel(0, 0).0;
    Ok(format!("{:02x}{:02x}{:02x}", px[0], px[1], px[2]))
}

/// Normalize a pasted/typed color like the Go Find Pixel apply path.
pub fn normalize_target_color(hex: &str) -> String {
    let mut h = hex.trim().trim_start_matches('#').to_ascii_lowercase();
    if h.len() == 8 {
        h = h[2..].to_string();
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use sqyre_capture::SolidCapturer;
    use sqyre_executor::DesktopRect;

    #[test]
    fn samples_solid_pixel() {
        let mut c = SolidCapturer {
            color: Rgba([0xab, 0xcd, 0xef, 0xff]),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 10,
                h: 10,
            },
        };
        assert_eq!(sample_pixel_hex_with(&mut c, 3, 4).unwrap(), "abcdef");
    }

    #[test]
    fn normalize_strips_hash_and_alpha() {
        assert_eq!(normalize_target_color("#FF00AA"), "ff00aa");
        assert_eq!(normalize_target_color("ff00aabb"), "00aabb");
        assert_eq!(normalize_target_color("  Abc "), "abc");
    }
}
