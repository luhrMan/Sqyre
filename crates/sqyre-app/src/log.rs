//! User-facing stderr messages for the desktop shell.

/// Log a message to stderr with the `sqyre:` prefix.
pub fn warn(msg: impl std::fmt::Display) {
    eprintln!("sqyre: {msg}");
}
