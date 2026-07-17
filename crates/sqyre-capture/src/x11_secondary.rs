//! Track Sqyre's secondary X11 `Display*` connections.
//!
//! winit installs a process-global `XSetErrorHandler` that stores *any* Xlib
//! error into its connection slot. GetProperty on a racing/destroyed window
//! from our focus/capture/outline Displays then poisons that slot and later
//! panics inside IME `remove_context` (`Failed to destroy input context`).
//!
//! The app registers a [`winit::platform::x11::register_xlib_error_hook`] that
//! returns true for displays listed here so those errors are not stored.

use parking_lot::Mutex;
use std::os::raw::c_void;
use x11::xlib::Display;

static SECONDARY: Mutex<Vec<usize>> = Mutex::new(Vec::new());

/// Remember a Sqyre-owned Display so its X errors can be filtered in the app.
pub fn register(display: *mut Display) {
    if display.is_null() {
        return;
    }
    let key = display as usize;
    let mut list = SECONDARY.lock();
    if !list.contains(&key) {
        list.push(key);
    }
}

/// Drop a Display from the filter set (e.g. after `XCloseDisplay`).
pub fn unregister(display: *mut Display) {
    if display.is_null() {
        return;
    }
    let key = display as usize;
    SECONDARY.lock().retain(|&d| d != key);
}

/// True if `display` is a Sqyre secondary connection (not winit's).
pub fn owns(display: *mut c_void) -> bool {
    if display.is_null() {
        return false;
    }
    let key = display as usize;
    SECONDARY.lock().iter().any(|&d| d == key)
}
