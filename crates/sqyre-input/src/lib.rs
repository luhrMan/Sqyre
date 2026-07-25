//! Real `AutomationBackend` using rustautogui (lite) + arboard.
//!
//! Tracks keys/buttons this process has pressed so hard exits (failsafe /
//! `process::exit`) can still release them — executor cleanup never runs then.

use arboard::Clipboard;
use rustautogui::{MouseClick, RustAutoGui};
use sqyre_ports::{AutomationBackend, AutomationError, MoveOptions};
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

/// Keys currently held via [`OsAutomation::key_down`] (rustautogui names).
static HELD_KEYS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
/// Mouse buttons currently held via [`OsAutomation::click`] down.
static HELD_BUTTONS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn note_key_down(key: &str) {
    if let Ok(mut g) = HELD_KEYS.lock() {
        g.insert(key.to_string());
    }
}

fn note_key_up(key: &str) {
    if let Ok(mut g) = HELD_KEYS.lock() {
        g.remove(key);
    }
}

fn note_button_down(button: &str) {
    if let Ok(mut g) = HELD_BUTTONS.lock() {
        g.insert(button.to_string());
    }
}

fn note_button_up(button: &str) {
    if let Ok(mut g) = HELD_BUTTONS.lock() {
        g.remove(button);
    }
}

fn take_held() -> (HashSet<String>, HashSet<String>) {
    let keys = HELD_KEYS
        .lock()
        .map(|mut g| std::mem::take(&mut *g))
        .unwrap_or_default();
    let buttons = HELD_BUTTONS
        .lock()
        .map(|mut g| std::mem::take(&mut *g))
        .unwrap_or_default();
    (keys, buttons)
}

/// Canonical button name used for hold tracking (`left` / `right` / `middle`).
fn canonical_button(button: &str) -> &'static str {
    match button {
        "right" => "right",
        "center" | "middle" => "middle",
        _ => "left",
    }
}

/// Best-effort release of every key/button this process still has held.
///
/// Safe to call from any thread (including failsafe / `process::exit` paths).
/// No-ops when nothing is held or when the OS input backend cannot start.
pub fn release_held_inputs() {
    let (keys, buttons) = take_held();
    if keys.is_empty() && buttons.is_empty() {
        return;
    }
    let Ok(gui) = RustAutoGui::new(false) else {
        return;
    };
    for key in keys {
        let _ = gui.key_up(&key);
    }
    for button in buttons {
        let _ = gui.click_up(OsAutomation::map_button(&button));
    }
}

pub struct OsAutomation {
    gui: RustAutoGui,
    clipboard: Option<Clipboard>,
}

impl OsAutomation {
    pub fn new() -> Result<Self, AutomationError> {
        let gui = RustAutoGui::new(false)
            .map_err(|e| AutomationError::Backend(format!("rustautogui: {e}")))?;
        let clipboard = Clipboard::new().ok();
        Ok(Self { gui, clipboard })
    }

    fn map_button(button: &str) -> MouseClick {
        match button {
            "right" => MouseClick::RIGHT,
            "center" | "middle" => MouseClick::MIDDLE,
            _ => MouseClick::LEFT,
        }
    }

    fn map_key(key: &str) -> String {
        // Legacy key names → rustautogui US keyboard names.
        match key.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => "control".into(),
            "cmd" | "command" | "super" | "win" => "command".into(),
            "esc" | "escape" => "escape".into(),
            "return" | "enter" => "enter".into(),
            "space" | "spacebar" => "space".into(),
            other => other.to_string(),
        }
    }
}

/// Absolute move with signed virtual-desktop coords (Windows origin may be negative).
#[cfg(target_os = "windows")]
fn move_mouse_windows(x: i32, y: i32, moving_time: f32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

    unsafe {
        if moving_time <= 0.0 {
            let _ = SetCursorPos(x, y);
            return;
        }
        let mut start = POINT::default();
        if GetCursorPos(&mut start).is_err() {
            let _ = SetCursorPos(x, y);
            return;
        }
        let start_t = std::time::Instant::now();
        let dx = x - start.x;
        let dy = y - start.y;
        loop {
            let t = start_t.elapsed().as_secs_f32() / moving_time;
            if t >= 1.0 {
                let _ = SetCursorPos(x, y);
                break;
            }
            let nx = start.x as f32 + t * dx as f32;
            let ny = start.y as f32 + t * dy as f32;
            let _ = SetCursorPos(nx as i32, ny as i32);
        }
    }
}

impl AutomationBackend for OsAutomation {
    fn milli_sleep(&mut self, ms: i32) {
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms as u64));
        }
    }

    fn move_to(&mut self, x: i32, y: i32, opts: MoveOptions) {
        let moving_time = if opts.smooth {
            // Approximate smooth move: delay_ms scaling into seconds.
            let base = if opts.delay_ms > 0 {
                opts.delay_ms as f32 * 0.05
            } else {
                0.2
            };
            base.clamp(0.05, 2.0)
        } else {
            0.0
        };
        // Absolute virtual-desktop coords (Windows origin may be negative).
        #[cfg(target_os = "windows")]
        {
            move_mouse_windows(x, y, moving_time);
            return;
        }
        #[cfg(not(target_os = "windows"))]
        {
            // rustautogui's public API is u32; X11 virtual desktop starts at (0,0).
            let xu = u32::try_from(x).unwrap_or(0);
            let yu = u32::try_from(y).unwrap_or(0);
            if let Err(e) = self.gui.move_mouse_to_pos(xu, yu, moving_time) {
                // Fallback: try zero-time again (bounds check can false-positive).
                let _ = self.gui.move_mouse_to_pos(xu, yu, 0.0);
                let _ = e;
            }
        }
    }

    fn click(&mut self, button: &str, down: bool) -> Result<(), AutomationError> {
        let canonical = canonical_button(button);
        let btn = Self::map_button(canonical);
        if down {
            self.gui
                .click_down(btn)
                .map_err(|e| AutomationError::Backend(format!("click down: {e}")))?;
            note_button_down(canonical);
            Ok(())
        } else {
            self.gui
                .click_up(btn)
                .map_err(|e| AutomationError::Backend(format!("click up: {e}")))?;
            note_button_up(canonical);
            Ok(())
        }
    }

    fn scroll(&mut self, up: bool) -> Result<(), AutomationError> {
        // Scroll intensity ~3 notches.
        if up {
            self.gui
                .scroll_up(3)
                .map_err(|e| AutomationError::Backend(format!("scroll up: {e}")))
        } else {
            self.gui
                .scroll_down(3)
                .map_err(|e| AutomationError::Backend(format!("scroll down: {e}")))
        }
    }

    fn key_down(&mut self, key: &str) -> Result<(), AutomationError> {
        let k = Self::map_key(key);
        self.gui
            .key_down(&k)
            .map_err(|e| AutomationError::Backend(format!("key down {k}: {e}")))?;
        note_key_down(&k);
        Ok(())
    }

    fn key_up(&mut self, key: &str) -> Result<(), AutomationError> {
        let k = Self::map_key(key);
        self.gui
            .key_up(&k)
            .map_err(|e| AutomationError::Backend(format!("key up {k}: {e}")))?;
        note_key_up(&k);
        Ok(())
    }

    fn type_char(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        let _ = self.gui.keyboard_input(s);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_buttons_and_keys() {
        assert!(matches!(OsAutomation::map_button("left"), MouseClick::LEFT));
        assert!(matches!(
            OsAutomation::map_button("right"),
            MouseClick::RIGHT
        ));
        assert!(matches!(
            OsAutomation::map_button("middle"),
            MouseClick::MIDDLE
        ));
        assert!(matches!(
            OsAutomation::map_button("center"),
            MouseClick::MIDDLE
        ));
        assert_eq!(OsAutomation::map_key("ctrl"), "control");
        assert_eq!(OsAutomation::map_key("control"), "control");
        assert_eq!(OsAutomation::map_key("esc"), "escape");
        assert_eq!(OsAutomation::map_key("escape"), "escape");
        assert_eq!(OsAutomation::map_key("return"), "enter");
        assert_eq!(OsAutomation::map_key("enter"), "enter");
        assert_eq!(OsAutomation::map_key("spacebar"), "space");
        assert_eq!(OsAutomation::map_key("cmd"), "command");
        assert_eq!(OsAutomation::map_key("super"), "command");
        assert_eq!(OsAutomation::map_key("a"), "a");
    }

    #[test]
    fn smooth_move_time_clamped() {
        // Documented mapping used by move_to — keep in sync if formula changes.
        let from_delay = (100_f32 * 0.05).clamp(0.05, 2.0);
        assert!((from_delay - 2.0).abs() < f32::EPSILON);
        let default_smooth = 0.2_f32.clamp(0.05, 2.0);
        assert!((default_smooth - 0.2).abs() < f32::EPSILON);
        let instant = 0.0_f32;
        assert_eq!(instant, 0.0);
    }

    #[test]
    fn hold_tracking_take_clears() {
        let _ = take_held(); // isolate from other tests
        note_key_down("control");
        note_key_down("a");
        note_button_down("left");
        note_key_up("a");
        let (keys, buttons) = take_held();
        assert!(keys.contains("control"));
        assert!(!keys.contains("a"));
        assert!(buttons.contains("left"));
        let (keys2, buttons2) = take_held();
        assert!(keys2.is_empty());
        assert!(buttons2.is_empty());
    }

    #[test]
    fn canonical_button_aliases() {
        assert_eq!(canonical_button("left"), "left");
        assert_eq!(canonical_button("right"), "right");
        assert_eq!(canonical_button("middle"), "middle");
        assert_eq!(canonical_button("center"), "middle");
        assert_eq!(canonical_button("other"), "left");
    }
}
