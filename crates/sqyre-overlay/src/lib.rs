//! Floating macro overlay buttons and Phosphor icon catalog.
//!
//! Split from `sqyre-app` so overlay input/compositor work can iterate without
//! rebuilding the full desktop shell. Live buttons ship with `native-runtime`
//! (`overlay-buttons`). Fast local loop:
//! `cargo run -p sqyre-overlay --features sandbox --bin overlay_sandbox`.

#![cfg_attr(
    not(feature = "runtime"),
    allow(dead_code, reason = "editor builds keep only the icon catalog")
)]

mod brand_s;
pub mod icons;

/// Positioned, styled overlay button face shared by the per-platform spec DTOs
/// (physical pixels, host-window/desktop coordinates).
///
/// `bg` is the fill RGBA. On ARGB faces this is real translucency. On the
/// 24-bit fallback alpha 0 = no fill (Shape punch-out from chrome ink only)
/// and any alpha > 0 paints opaque.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonFace {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub bg: [u8; 4],
    pub border: [u8; 4],
    pub border_width: f32,
    pub corner_radius: f32,
    pub icon_glyph: char,
    pub icon: [u8; 4],
    pub icon_hover: [u8; 4],
}

#[cfg(feature = "runtime")]
mod macro_overlay;

#[cfg(all(feature = "runtime", target_os = "android"))]
mod android_buttons;
#[cfg(all(feature = "runtime", any(target_os = "android", test)))]
mod button_visibility;
#[cfg(all(
    feature = "runtime",
    any(target_os = "linux", target_os = "android", target_os = "windows")
))]
mod raster;
#[cfg(all(feature = "runtime", target_os = "windows"))]
mod win_buttons;
#[cfg(all(feature = "runtime", target_os = "linux"))]
mod x11_buttons;

#[cfg(feature = "runtime")]
pub use macro_overlay::{MacroOverlay, NativeHotkeyChooserResult, OverlayButtonMove};

pub use icons::{
    catalog, glyph_font_id, register_phosphor_family, resolve, show_icon_picker_grid,
    style_preview_button, OverlayIcon, OverlayPaintStyle, DEFAULT_ICON_ID,
};
