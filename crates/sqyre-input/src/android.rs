//! Android `AutomationBackend` over the Sqyre accessibility service.
//!
//! Pointer actions become accessibility gestures (see [`PointerPlanner`]). Android
//! has no injectable hardware keys for third-party apps, so key down/up is
//! unsupported; typing appends to the focused text field instead.

use sqyre_android::{bridge, AndroidError, PointerError, PointerPlanner};
use sqyre_ports::{AutomationBackend, AutomationError, MoveOptions};
use std::time::{Duration, Instant};

/// Gestures are dispatched whole, so nothing stays held after a hard exit.
pub fn release_held_inputs() {}

/// No per-thread input state to reset on Android.
pub fn prepare_for_automation() {}

pub struct OsAutomation {
    pointer: PointerPlanner,
}

impl OsAutomation {
    /// Fails (and opens Accessibility settings) while the service is off.
    pub fn new() -> Result<Self, AutomationError> {
        if !bridge::accessibility_enabled().map_err(backend)? {
            bridge::open_accessibility_settings().map_err(backend)?;
            return Err(backend(AndroidError::AccessibilityOff));
        }
        Ok(Self {
            pointer: PointerPlanner::default(),
        })
    }
}

fn backend(e: AndroidError) -> AutomationError {
    AutomationError::Backend(e.to_string())
}

fn pointer_error(e: PointerError) -> AutomationError {
    match e {
        PointerError::Unsupported(what) => AutomationError::Unsupported(what),
        PointerError::NoPosition => AutomationError::InvalidArg(e.to_string()),
    }
}

impl AutomationBackend for OsAutomation {
    fn milli_sleep(&mut self, ms: i32) {
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms as u64));
        }
    }

    /// Records the target only; a touch screen has no hovering pointer.
    fn move_to(&mut self, x: i32, y: i32, _opts: MoveOptions) {
        self.pointer.move_to(x, y);
    }

    fn click(&mut self, button: &str, down: bool) -> Result<(), AutomationError> {
        match self
            .pointer
            .click(button, down, Instant::now())
            .map_err(pointer_error)?
        {
            Some(gesture) => bridge::dispatch(gesture).map_err(backend),
            None => Ok(()),
        }
    }

    fn scroll(&mut self, up: bool) -> Result<(), AutomationError> {
        let screen = bridge::display_size().map_err(backend)?;
        bridge::dispatch(self.pointer.scroll(up, screen)).map_err(backend)
    }

    fn key_down(&mut self, _key: &str) -> Result<(), AutomationError> {
        Err(AutomationError::Unsupported("keyboard keys"))
    }

    fn key_up(&mut self, _key: &str) -> Result<(), AutomationError> {
        Err(AutomationError::Unsupported("keyboard keys"))
    }

    fn type_char(&mut self, ch: char) -> Result<(), AutomationError> {
        bridge::append_text(ch.encode_utf8(&mut [0; 4])).map_err(backend)
    }

    fn write_clipboard(&mut self, s: &str) -> Result<(), AutomationError> {
        bridge::set_clipboard(s).map_err(backend)
    }
}
