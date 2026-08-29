//! Floating macro overlay buttons and Phosphor icon catalog.
//!
//! Split from `sqyre-app` so overlay input/compositor work can iterate without
//! rebuilding the full desktop shell. Live buttons ship with `native-runtime`
//! (`overlay-buttons`). Fast local loop:
//! `cargo run -p sqyre-overlay --features sandbox --bin overlay_sandbox`.

#![cfg_attr(not(feature = "runtime"), allow(dead_code))]

pub mod icons;
pub mod theme;

#[cfg(feature = "runtime")]
mod macro_overlay;

#[cfg(all(feature = "runtime", target_os = "linux"))]
mod x11_buttons;
#[cfg(all(feature = "runtime", target_os = "linux"))]
mod raster;

#[cfg(feature = "runtime")]
pub use macro_overlay::MacroOverlay;

pub use icons::{
    catalog, glyph_font_id, register_phosphor_family, resolve, show_icon_picker_grid,
    style_preview_button, OverlayIcon, OverlayPaintStyle, DEFAULT_ICON_ID,
};
