//! Global physical button state from the hotkey hook (evdev / rdev / Win32).
//!
//! X11 `QueryPointer` only sees buttons while the pointer is in the XWayland
//! seat. Overlay relocate / click-through over native Wayland surfaces (Cursor,
//! GNOME desktop) needs these bits instead.

use std::sync::atomic::{AtomicBool, Ordering};

static LEFT_DOWN: AtomicBool = AtomicBool::new(false);
static RIGHT_DOWN: AtomicBool = AtomicBool::new(false);

/// Update from the hook thread on every left ButtonPress / ButtonRelease.
#[cfg_attr(
    not(feature = "hooks"),
    expect(dead_code, reason = "only the OS hook threads report button state")
)]
pub(crate) fn set_left_button_down(down: bool) {
    LEFT_DOWN.store(down, Ordering::Relaxed);
}

/// Update from the hook thread on every right ButtonPress / ButtonRelease.
#[cfg_attr(
    all(not(feature = "hooks"), not(test)),
    expect(dead_code, reason = "only the OS hook threads report button state")
)]
pub(crate) fn set_right_button_down(down: bool) {
    RIGHT_DOWN.store(down, Ordering::Relaxed);
}

/// True while the physical left mouse button is held (any focused surface).
pub fn left_button_down() -> bool {
    LEFT_DOWN.load(Ordering::Relaxed)
}

/// True while the physical right mouse button is held (any focused surface).
pub fn right_button_down() -> bool {
    RIGHT_DOWN.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_button_tracks_set_clear() {
        set_right_button_down(true);
        assert!(right_button_down());
        set_right_button_down(false);
        assert!(!right_button_down());
    }
}
