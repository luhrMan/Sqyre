//! Native file / folder dialogs via `rfd`.
//!
//! On Linux, `rfd`'s XDG portal backend uses `ashpd`/`zbus`. Accessibility
//! (accesskit) enables `zbus`'s Tokio feature for the whole process, so sync
//! `rfd` calls panic with "no reactor running" unless a Tokio runtime has been
//! entered. Keep a process-global runtime and enter it for every dialog.

use std::path::PathBuf;
use std::sync::OnceLock;

fn enter_tokio() -> tokio::runtime::EnterGuard<'static> {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    let rt = RT.get_or_init(|| {
        // Current-thread runtime is enough for rfd/ashpd; avoid a worker pool.
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime for native file dialogs")
    });
    rt.enter()
}

/// PNG open dialog (icon variants).
pub fn pick_png() -> Option<PathBuf> {
    let _guard = enter_tokio();
    rfd::FileDialog::new()
        .add_filter("PNG", &["png"])
        .pick_file()
}

/// Common raster formats (mask upload).
pub fn pick_image() -> Option<PathBuf> {
    let _guard = enter_tokio();
    rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
        .pick_file()
}

/// Folder picker (settings: choose `.sqyre` location).
pub fn pick_folder(title: &str, start: &std::path::Path) -> Option<PathBuf> {
    let _guard = enter_tokio();
    rfd::FileDialog::new()
        .set_title(title)
        .set_directory(start)
        .pick_folder()
}
