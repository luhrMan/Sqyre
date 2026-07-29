//! Runtime gates for Wayland portal capabilities (synced from User Settings).

use std::sync::atomic::{AtomicBool, Ordering};

static SCREEN_CAPTURE: AtomicBool = AtomicBool::new(true);
static INPUT_CONTROL: AtomicBool = AtomicBool::new(true);
static GLOBAL_SHORTCUTS: AtomicBool = AtomicBool::new(true);
static WINDOW_MANAGEMENT: AtomicBool = AtomicBool::new(true);

/// Apply User Settings toggles for Wayland portal use.
pub fn apply_from_settings(
    screen_capture: bool,
    input_control: bool,
    global_shortcuts: bool,
    window_management: bool,
) {
    SCREEN_CAPTURE.store(screen_capture, Ordering::SeqCst);
    INPUT_CONTROL.store(input_control, Ordering::SeqCst);
    GLOBAL_SHORTCUTS.store(global_shortcuts, Ordering::SeqCst);
    WINDOW_MANAGEMENT.store(window_management, Ordering::SeqCst);
}

pub fn screen_capture_enabled() -> bool {
    SCREEN_CAPTURE.load(Ordering::SeqCst)
}

pub fn input_control_enabled() -> bool {
    INPUT_CONTROL.load(Ordering::SeqCst)
}

pub fn global_shortcuts_enabled() -> bool {
    GLOBAL_SHORTCUTS.load(Ordering::SeqCst)
}

pub fn window_management_enabled() -> bool {
    WINDOW_MANAGEMENT.load(Ordering::SeqCst)
}
