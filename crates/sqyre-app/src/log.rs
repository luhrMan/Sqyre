//! User-facing warnings for the desktop shell (stderr).

/// Log a warning with the `sqyre:` prefix.
pub fn warn(msg: impl std::fmt::Display) {
    eprintln!("sqyre: {msg}");
}
