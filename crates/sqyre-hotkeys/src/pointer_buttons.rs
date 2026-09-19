//! Global physical left-button state from the hotkey hook (evdev / rdev / Win32).
//!
//! X11 `QueryPointer` only sees buttons while the pointer is in the XWayland
//! seat. Overlay relocate over native Wayland surfaces (Cursor, GNOME desktop)
//! needs this bit instead.

use std::sync::atomic::{AtomicBool, Ordering};

static LEFT_DOWN: AtomicBool = AtomicBool::new(false);

/// Update from the hook thread on every left ButtonPress / ButtonRelease.
#[cfg_attr(not(feature = "hooks"), allow(dead_code))]
pub(crate) fn set_left_button_down(down: bool) {
    LEFT_DOWN.store(down, Ordering::Relaxed);
}

/// True while the physical left mouse button is held (any focused surface).
pub fn left_button_down() -> bool {
    LEFT_DOWN.load(Ordering::Relaxed)
}
