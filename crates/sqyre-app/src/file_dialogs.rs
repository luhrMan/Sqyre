//! Native file / folder dialogs via `rfd`.
//!
//! On Linux, `rfd` and Sqyre's Wayland portals use ashpd/zbus. Keep `ksni`
//! (and ashpd) on the `async-io` backend — enabling `zbus`'s `tokio` feature
//! makes sync portal calls panic with "no reactor running".
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

/// Zip open dialog (settings: restore backup).
pub fn pick_zip(title: &str, start: &std::path::Path) -> Option<PathBuf> {
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
            .add_filter("Zip archive", &["zip"])
            .pick_file()
    }
}
