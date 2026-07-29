//! Wayland RemoteDesktop-backed [`AutomationBackend`].

use crate::{canonical_button, note_button_down, note_button_up, note_key_down, note_key_up};
use arboard::Clipboard;
use sqyre_capture::{input_control_enabled, wayland_input_session};
use sqyre_ports::{AutomationBackend, AutomationError, MoveOptions};
use std::time::Duration;

/// Linux button codes (evdev) for the RemoteDesktop portal.
const BTN_LEFT: i32 = 0x110;
const BTN_RIGHT: i32 = 0x111;
const BTN_MIDDLE: i32 = 0x112;

pub struct WaylandAutomation {
    clipboard: Option<Clipboard>,
}

impl WaylandAutomation {
    pub fn new() -> Result<Self, AutomationError> {
        if !input_control_enabled() {
            return Err(AutomationError::PermissionDenied {
                capability: "input control",
            });
        }
        wayland_input_session::ensure_session().map_err(|e| match e {
            sqyre_ports::CaptureError::PermissionDenied { capability } => {
                AutomationError::PermissionDenied { capability }
            }
            other => AutomationError::PortalUnavailable(other.to_string()),
        })?;
        Ok(Self {
            clipboard: Clipboard::new().ok(),
        })
    }

    fn map_button(button: &str) -> i32 {
        match canonical_button(button) {
            "right" => BTN_RIGHT,
            "middle" => BTN_MIDDLE,
            _ => BTN_LEFT,
        }
    }

    pub(crate) fn map_keysym_pub(key: &str) -> i32 {
        Self::map_keysym(key)
    }

    fn map_keysym(key: &str) -> i32 {
        // XKB keysyms for common keys (US layout).
        match key.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => 0xffe3, // Control_L
            "alt" => 0xffe9,              // Alt_L
            "shift" => 0xffe1,            // Shift_L
            "cmd" | "command" | "super" | "win" => 0xffeb, // Super_L
            "esc" | "escape" => 0xff1b,
            "return" | "enter" => 0xff0d,
            "space" | "spacebar" => 0x0020,
            "tab" => 0xff09,
            "backspace" => 0xff08,
            "delete" | "del" => 0xffff,
            "up" => 0xff52,
            "down" => 0xff54,
            "left" => 0xff51,
            "right" => 0xff53,
            "home" => 0xff50,
            "end" => 0xff57,
            "pageup" => 0xff55,
            "pagedown" => 0xff56,
            other if other.len() == 1 => {
                let c = other.chars().next().unwrap();
                c as i32
            }
            _ => 0,
        }
    }
}

impl AutomationBackend for WaylandAutomation {
    fn milli_sleep(&mut self, ms: i32) {
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms as u64));
        }
    }

    fn move_to(&mut self, x: i32, y: i32, opts: MoveOptions) {
        let moving_time = if opts.smooth {
            let base = if opts.delay_ms > 0 {
                opts.delay_ms as f32 * 0.05
            } else {
                0.2
            };
            base.clamp(0.05, 2.0)
        } else {
            0.0
        };
        if moving_time <= 0.0 {
            let _ = wayland_input_session::notify_pointer_motion_absolute(x as f64, y as f64);
            return;
        }
        // Approximate smooth move with stepped absolute motions.
        let steps = (moving_time * 60.0).ceil().max(2.0) as i32;
        let start = std::time::Instant::now();
        // Without a pointer query API, jump in equal time steps toward the target.
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let nx = x as f64 * t;
            let ny = y as f64 * t;
            let _ = wayland_input_session::notify_pointer_motion_absolute(nx, ny);
            let target = start + Duration::from_secs_f32(moving_time * (i as f32 / steps as f32));
            let now = std::time::Instant::now();
            if target > now {
                std::thread::sleep(target - now);
            }
        }
        let _ = wayland_input_session::notify_pointer_motion_absolute(x as f64, y as f64);
    }

    fn click(&mut self, button: &str, down: bool) -> Result<(), AutomationError> {
        let canonical = canonical_button(button);
        let code = Self::map_button(canonical);
        wayland_input_session::notify_pointer_button(code, down).map_err(|e| {
            AutomationError::PortalUnavailable(e.to_string())
        })?;
        if down {
            note_button_down(canonical);
        } else {
            note_button_up(canonical);
        }
        Ok(())
    }

    fn scroll(&mut self, up: bool) -> Result<(), AutomationError> {
        let dy = if up { -15.0 } else { 15.0 };
        wayland_input_session::notify_pointer_axis(0.0, dy)
            .map_err(|e| AutomationError::PortalUnavailable(e.to_string()))
    }

    fn key_down(&mut self, key: &str) -> Result<(), AutomationError> {
        let keysym = Self::map_keysym(key);
        if keysym == 0 {
            return Err(AutomationError::InvalidArg(format!("unknown key: {key}")));
        }
        wayland_input_session::notify_keyboard_keysym(keysym, true)
            .map_err(|e| AutomationError::PortalUnavailable(e.to_string()))?;
        note_key_down(key);
        Ok(())
    }

    fn key_up(&mut self, key: &str) -> Result<(), AutomationError> {
        let keysym = Self::map_keysym(key);
        if keysym == 0 {
            return Err(AutomationError::InvalidArg(format!("unknown key: {key}")));
        }
        wayland_input_session::notify_keyboard_keysym(keysym, false)
            .map_err(|e| AutomationError::PortalUnavailable(e.to_string()))?;
        note_key_up(key);
        Ok(())
    }

    fn type_char(&mut self, ch: char) {
        let keysym = ch as i32;
        let _ = wayland_input_session::notify_keyboard_keysym(keysym, true);
        let _ = wayland_input_session::notify_keyboard_keysym(keysym, false);
    }

    fn write_clipboard(&mut self, s: &str) -> Result<(), AutomationError> {
        let clip = self
            .clipboard
            .as_mut()
            .ok_or(AutomationError::Unsupported("clipboard"))?;
        clip.set_text(s.to_string())
            .map_err(|e| AutomationError::Backend(format!("clipboard: {e}")))
    }
}
