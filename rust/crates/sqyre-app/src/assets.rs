//! Embedded brand assets (mirrors Go `internal/assets` embeds).

/// Raster app icon for the native window / taskbar (`eframe` requires PNG/`IconData`).
pub const APP_ICON_PNG: &[u8] =
    include_bytes!("../../../../internal/assets/icons/sqyre.png");

/// Native window icon, or empty if the embedded PNG fails to decode.
pub fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(APP_ICON_PNG).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sqyre app icon SVG (same bytes as Go `assets.AppIcon`); kept for embed parity.
    const APP_ICON_SVG: &[u8] =
        include_bytes!("../../../../internal/assets/icons/sqyre.svg");

    #[test]
    fn app_icon_svg_embedded() {
        assert!(!APP_ICON_SVG.is_empty());
        assert!(std::str::from_utf8(APP_ICON_SVG)
            .unwrap()
            .contains("<svg"));
    }

    #[test]
    fn app_icon_png_loads() {
        let icon = app_icon();
        assert!(icon.width > 0 && icon.height > 0);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    }
}
