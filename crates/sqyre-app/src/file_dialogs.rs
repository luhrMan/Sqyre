//! Native file / folder dialogs via `rfd`.
//!
//! On Linux, `rfd` uses the XDG portal (`ashpd` / `zbus`) and blocks with
//! `pollster`. Keep `ksni` (and anything else using zbus) on the `async-io`
//! backend so nothing enables `zbus`'s `tokio` feature — otherwise sync
//! portal calls panic with "no reactor running".
//!
//! On WASM, sync `FileDialog` is unavailable — use `wasm_io` async dialogs.

use std::path::PathBuf;

/// PNG open dialog (icon variants).
pub fn pick_png() -> Option<PathBuf> {
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .pick_file()
    }
}

/// Common raster formats (mask upload).
pub fn pick_image() -> Option<PathBuf> {
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
            .pick_file()
    }
}

/// Folder picker (settings: choose `.sqyre` location).
pub fn pick_folder(title: &str, start: &std::path::Path) -> Option<PathBuf> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (title, start);
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rfd::FileDialog::new()
            .set_title(title)
            .set_directory(start)
            .pick_folder()
    }
}
