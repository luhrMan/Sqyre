//! Sample a screen pixel as `rrggbb` (Find Pixel dropper).

use sqyre_capture::shared_capturer;
use sqyre_ports::DesktopRect;

#[cfg(any(test, target_arch = "wasm32"))]
use sqyre_ports::ScreenCapturer;

/// Capture the 1×1 pixel at `(x, y)` and return lowercase hex without `#`.
#[cfg(target_arch = "wasm32")]
pub fn sample_pixel_hex(x: i32, y: i32) -> Result<String, String> {
    let capturer = shared_capturer().map_err(|e| e.to_string())?;
    let mut wrap = sqyre_capture::SharedRunCapturer(capturer);
    sample_pixel_hex_with(&mut wrap, x, y)
}

#[cfg(any(test, target_arch = "wasm32"))]
pub fn sample_pixel_hex_with(
    capturer: &mut dyn ScreenCapturer,
    x: i32,
    y: i32,
) -> Result<String, String> {
    let rgb = capturer
        .capture_rect_rgb(DesktopRect { x, y, w: 1, h: 1 })
        .map_err(|e| e.to_string())?;
    if rgb.data.len() < 3 {
        return Err("empty pixel capture".into());
    }
    Ok(format!(
        "{:02x}{:02x}{:02x}",
        rgb.data[0], rgb.data[1], rgb.data[2]
    ))
}

/// Spawn a background 1×1 capture; poll the receiver from the UI thread.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_sample_pixel_hex(
    x: i32,
    y: i32,
) -> Result<std::sync::mpsc::Receiver<Result<String, String>>, String> {
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let capturer = shared_capturer().map_err(|e| e.to_string())?;
            let rgb = capturer
                .capture_rect_rgb_ref(DesktopRect { x, y, w: 1, h: 1 })
                .map_err(|e| e.to_string())?;
            if rgb.data.len() < 3 {
                return Err("empty pixel capture".into());
            }
            Ok(format!(
                "{:02x}{:02x}{:02x}",
                rgb.data[0], rgb.data[1], rgb.data[2]
            ))
        })();
        let _ = tx.send(result);
    });
    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use sqyre_capture::SolidCapturer;
    use sqyre_domain::normalize_hex_rgb;
    use sqyre_ports::DesktopRect;

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
        assert_eq!(normalize_hex_rgb("#FF00AA"), "ff00aa");
        assert_eq!(normalize_hex_rgb("ff00aabb"), "00aabb");
        assert_eq!(normalize_hex_rgb("  Abc "), "abc");
    }
}
