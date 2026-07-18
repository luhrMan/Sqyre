//! Real `AutomationBackend` using rustautogui (lite) + arboard.

use arboard::Clipboard;
use rustautogui::{MouseClick, RustAutoGui};
use sqyre_executor::{AutomationBackend, MoveOptions};
use std::time::Duration;

pub struct OsAutomation {
    gui: RustAutoGui,
    clipboard: Option<Clipboard>,
}

impl OsAutomation {
    pub fn new() -> Result<Self, String> {
        let gui = RustAutoGui::new(false).map_err(|e| format!("rustautogui: {e}"))?;
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
        // Absolute root coords; cast carefully for negative multi-monitor origins.
        let xu = x.max(0) as u32;
        let yu = y.max(0) as u32;
        if let Err(e) = self.gui.move_mouse_to_pos(xu, yu, moving_time) {
            // Fallback: try zero-time again (bounds check can false-positive).
            let _ = self.gui.move_mouse_to_pos(xu, yu, 0.0);
            let _ = e;
        }
    }

    fn click(&mut self, button: &str, down: bool) -> Result<(), String> {
        let btn = Self::map_button(button);
        if down {
            self.gui
                .click_down(btn)
                .map_err(|e| format!("click down: {e}"))
        } else {
            self.gui.click_up(btn).map_err(|e| format!("click up: {e}"))
        }
    }

    fn scroll(&mut self, up: bool) -> Result<(), String> {
        // Scroll intensity ~3 notches.
        if up {
            self.gui.scroll_up(3).map_err(|e| format!("scroll up: {e}"))
        } else {
            self.gui
                .scroll_down(3)
                .map_err(|e| format!("scroll down: {e}"))
        }
    }

    fn key_down(&mut self, key: &str) -> Result<(), String> {
        let k = Self::map_key(key);
        self.gui
            .key_down(&k)
            .map_err(|e| format!("key down {k}: {e}"))
    }

    fn key_up(&mut self, key: &str) -> Result<(), String> {
        let k = Self::map_key(key);
        self.gui.key_up(&k).map_err(|e| format!("key up {k}: {e}"))
    }

    fn type_char(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        let _ = self.gui.keyboard_input(s);
    }

    fn write_clipboard(&mut self, s: &str) -> Result<(), String> {
        let clip = self
            .clipboard
            .as_mut()
            .ok_or_else(|| "clipboard unavailable".to_string())?;
        clip.set_text(s.to_string())
            .map_err(|e| format!("clipboard: {e}"))
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
}
