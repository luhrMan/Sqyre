//! Embedded brand assets.

/// Sqyre app icon SVG (crate-local `assets/icons/`).
pub const APP_ICON_SVG: &[u8] = include_bytes!("../assets/icons/sqyre.svg");

/// Rasterize the brand SVG to unpremultiplied RGBA at `size`×`size`.
pub fn app_icon_rgba(size: u32) -> Option<(Vec<u8>, u32, u32)> {
    if size == 0 {
        return None;
    }
    let tree = resvg::usvg::Tree::from_data(APP_ICON_SVG, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
    let svg_w = tree.size().width();
    let svg_h = tree.size().height();
    if svg_w <= 0.0 || svg_h <= 0.0 {
        return None;
    }
    let scale = (size as f32 / svg_w).min(size as f32 / svg_h);
    let tx = (size as f32 - svg_w * scale) * 0.5;
    let ty = (size as f32 - svg_h * scale) * 0.5;
    let transform = resvg::tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, tx, ty);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let mut rgba = pixmap.take();
    unpremultiply_rgba(&mut rgba);
    Some((rgba, size, size))
}

/// Native window / taskbar icon from the brand SVG.
pub fn app_icon() -> egui::IconData {
    match app_icon_rgba(256) {
        Some((rgba, width, height)) => egui::IconData {
            rgba,
            width,
            height,
        },
        None => egui::IconData::default(),
    }
}

/// tiny-skia stores premultiplied RGBA; OS tray / egui IconData expect straight alpha.
fn unpremultiply_rgba(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3];
        if a == 0 {
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
        } else if a < 255 {
            let af = f32::from(a) / 255.0;
            px[0] = ((f32::from(px[0]) / af).round().clamp(0.0, 255.0)) as u8;
            px[1] = ((f32::from(px[1]) / af).round().clamp(0.0, 255.0)) as u8;
            px[2] = ((f32::from(px[2]) / af).round().clamp(0.0, 255.0)) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_icon_svg_embedded() {
        assert!(!APP_ICON_SVG.is_empty());
        assert!(std::str::from_utf8(APP_ICON_SVG).unwrap().contains("<svg"));
    }

    #[test]
    fn app_icon_rasterizes() {
        let (rgba, w, h) = app_icon_rgba(64).expect("rasterize");
        assert_eq!((w, h), (64, 64));
        assert_eq!(rgba.len(), 64 * 64 * 4);
        // Brand yellow / dark fills should produce some opaque pixels.
        assert!(rgba.chunks_exact(4).any(|p| p[3] > 0));
    }

    #[test]
    fn app_icon_loads() {
        let icon = app_icon();
        assert!(icon.width > 0 && icon.height > 0);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    }
}
