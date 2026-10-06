//! User-facing warnings for the desktop shell (stderr) and the WASM editor (browser console).

/// Log a warning with the `sqyre:` prefix.
#[cfg(not(target_arch = "wasm32"))]
pub fn warn(msg: impl std::fmt::Display) {
    eprintln!("sqyre: {msg}");
}

/// Log a warning with the `sqyre:` prefix.
#[cfg(target_arch = "wasm32")]
pub fn warn(msg: impl std::fmt::Display) {
    log::warn!("sqyre: {msg}");
}
