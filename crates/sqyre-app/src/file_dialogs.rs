//! Native file / folder dialogs via `rfd`.
//!
//! On Linux, `rfd` uses the XDG portal (`ashpd` / `zbus`) and blocks with
//! `pollster`. Keep `ksni` (and anything else using zbus) on the `async-io`
//! backend so nothing enables `zbus`'s `tokio` feature — otherwise sync
//! portal calls panic with "no reactor running".
//!
//! On WASM, sync `FileDialog` is unavailable — use `wasm_io` async dialogs.
//! `rfd` has no Android backend; pickers return `None` there until the shell
//! exposes the Storage Access Framework.

use std::path::PathBuf;

/// PNG open dialog (icon variants).
pub fn pick_png(start: &std::path::Path) -> Option<PathBuf> {
    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    {
        let _ = start;
        None
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        rfd::FileDialog::new()
            .set_directory(start)
            .add_filter("PNG", &["png"])
            .pick_file()
    }
}

/// Common raster formats (mask upload).
pub fn pick_image() -> Option<PathBuf> {
    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    {
        None
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
            .pick_file()
    }
}

/// Folder picker (settings: choose `.sqyre` location).
pub fn pick_folder(title: &str, start: &std::path::Path) -> Option<PathBuf> {
    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    {
        let _ = (title, start);
        None
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        rfd::FileDialog::new()
            .set_title(title)
            .set_directory(start)
            .pick_folder()
    }
}

/// Zip open dialog (settings: restore backup).
pub fn pick_zip(title: &str, start: &std::path::Path) -> Option<PathBuf> {
    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    {
        let _ = (title, start);
        None
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        rfd::FileDialog::new()
            .set_title(title)
            .set_directory(start)
            .add_filter("Zip archive", &["zip"])
            .pick_file()
    }
}
