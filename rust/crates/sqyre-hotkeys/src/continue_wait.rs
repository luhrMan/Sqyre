//! Shared continue-chord wait used by Pause actions (Go `WaitForContinueKey`).

use parking_lot::{Condvar, Mutex};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Bridge between the hotkey listener thread and Pause waiters.
#[derive(Clone)]
pub struct ContinueWaitBridge {
    inner: Arc<Inner>,
    hooks_enabled: bool,
}

struct Inner {
    state: Mutex<WaitState>,
    cv: Condvar,
}

#[derive(Default)]
struct WaitState {
    waiting: bool,
    keys: Vec<String>,
    /// Continue chord is a lone Escape — Esc should resume, not stop.
    continue_is_esc: bool,
    signaled: bool,
    chord_was_pressed: bool,
}

impl ContinueWaitBridge {
    pub fn new(hooks_enabled: bool) -> Self {
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(WaitState::default()),
                cv: Condvar::new(),
            }),
            hooks_enabled,
        }
    }

    /// Whether a lone Escape should resume Pause instead of stopping the macro.
    pub fn continue_is_escape(&self) -> bool {
        let g = self.inner.state.lock();
        g.waiting && g.continue_is_esc
    }

    /// Called from the hotkey thread when the set of pressed keys changes.
    pub fn on_pressed_keys(&self, pressed: &HashSet<String>) {
        let mut g = self.inner.state.lock();
        if !g.waiting || g.signaled {
            return;
        }
        let all = !g.keys.is_empty() && g.keys.iter().all(|k| pressed.contains(k));
        if all && !g.chord_was_pressed {
            g.signaled = true;
            g.chord_was_pressed = true;
            self.inner.cv.notify_all();
        } else if !all {
            g.chord_was_pressed = false;
        }
    }

    /// Block until the continue chord is pressed or `stop` is set.
    pub fn wait_for_continue(
        &self,
        keys: &[String],
        _pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<(), String> {
        if !self.hooks_enabled {
            return Err("pause: continue key wait is not available in this build".into());
        }
        let normalized = normalize_keys(keys);
        if normalized.is_empty() {
            return Err("pause: continue key not set".into());
        }
        validate_not_failsafe(&normalized)?;

        {
            let mut g = self.inner.state.lock();
            g.waiting = true;
            g.keys = normalized.clone();
            g.continue_is_esc = normalized == ["esc".to_string()];
            g.signaled = false;
            g.chord_was_pressed = false;
        }

        let result = loop {
            if stop.load(Ordering::SeqCst) {
                break Err("stopped".into());
            }
            {
                let mut g = self.inner.state.lock();
                if g.signaled {
                    break Ok(());
                }
                self.inner
                    .cv
                    .wait_for(&mut g, Duration::from_millis(50));
                if g.signaled {
                    break Ok(());
                }
            }
        };

        {
            let mut g = self.inner.state.lock();
            g.waiting = false;
            g.keys.clear();
            g.continue_is_esc = false;
            g.signaled = false;
            g.chord_was_pressed = false;
        }
        result
    }
}

fn normalize_keys(keys: &[String]) -> Vec<String> {
    keys.iter()
        .map(|k| normalize_key_name(k))
        .filter(|k| !k.is_empty())
        .collect()
}

pub fn normalize_key_name(key: &str) -> String {
    match key.trim().to_ascii_lowercase().as_str() {
        "escape" => "esc".into(),
        "control" => "ctrl".into(),
        "return" => "enter".into(),
        other => other.to_string(),
    }
}

fn validate_not_failsafe(keys: &[String]) -> Result<(), String> {
    let mut sorted = keys.to_vec();
    sorted.sort();
    let mut failsafe: Vec<String> = vec!["ctrl".into(), "esc".into(), "shift".into()];
    failsafe.sort();
    if sorted == failsafe {
        return Err(
            "pause: continue key cannot match the failsafe hotkey (esc + ctrl + shift)".into(),
        );
    }
    Ok(())
}

/// Map an rdev key to a Sqyre continue-key name (lowercase).
#[cfg(feature = "hooks")]
pub fn rdev_key_name(key: rdev::Key) -> Option<String> {
    use rdev::Key;
    let name = match key {
        Key::Escape => "esc",
        Key::ControlLeft | Key::ControlRight => "ctrl",
        Key::ShiftLeft | Key::ShiftRight => "shift",
        Key::Alt | Key::AltGr => "alt",
        Key::MetaLeft | Key::MetaRight => "super",
        Key::Space => "space",
        Key::Return => "enter",
        Key::Tab => "tab",
        Key::Backspace => "backspace",
        Key::Delete => "delete",
        Key::UpArrow => "up",
        Key::DownArrow => "down",
        Key::LeftArrow => "left",
        Key::RightArrow => "right",
        Key::Home => "home",
        Key::End => "end",
        Key::PageUp => "pageup",
        Key::PageDown => "pagedown",
        Key::F1 => "f1",
        Key::F2 => "f2",
        Key::F3 => "f3",
        Key::F4 => "f4",
        Key::F5 => "f5",
        Key::F6 => "f6",
        Key::F7 => "f7",
        Key::F8 => "f8",
        Key::F9 => "f9",
        Key::F10 => "f10",
        Key::F11 => "f11",
        Key::F12 => "f12",
        Key::KeyA => "a",
        Key::KeyB => "b",
        Key::KeyC => "c",
        Key::KeyD => "d",
        Key::KeyE => "e",
        Key::KeyF => "f",
        Key::KeyG => "g",
        Key::KeyH => "h",
        Key::KeyI => "i",
        Key::KeyJ => "j",
        Key::KeyK => "k",
        Key::KeyL => "l",
        Key::KeyM => "m",
        Key::KeyN => "n",
        Key::KeyO => "o",
        Key::KeyP => "p",
        Key::KeyQ => "q",
        Key::KeyR => "r",
        Key::KeyS => "s",
        Key::KeyT => "t",
        Key::KeyU => "u",
        Key::KeyV => "v",
        Key::KeyW => "w",
        Key::KeyX => "x",
        Key::KeyY => "y",
        Key::KeyZ => "z",
        Key::Num0 | Key::Kp0 => "0",
        Key::Num1 | Key::Kp1 => "1",
        Key::Num2 | Key::Kp2 => "2",
        Key::Num3 | Key::Kp3 => "3",
        Key::Num4 | Key::Kp4 => "4",
        Key::Num5 | Key::Kp5 => "5",
        Key::Num6 | Key::Kp6 => "6",
        Key::Num7 | Key::Kp7 => "7",
        Key::Num8 | Key::Kp8 => "8",
        Key::Num9 | Key::Kp9 => "9",
        _ => return None,
    };
    Some(name.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn wait_errors_without_hooks() {
        let b = ContinueWaitBridge::new(false);
        let stop = AtomicBool::new(false);
        let err = b
            .wait_for_continue(&["f9".into()], false, &stop)
            .unwrap_err();
        assert!(err.contains("not available"));
    }

    #[test]
    fn wait_signaled_by_pressed_keys() {
        let b = ContinueWaitBridge::new(true);
        let stop = AtomicBool::new(false);
        let bridge = b.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            let mut pressed = HashSet::new();
            pressed.insert("f9".into());
            bridge.on_pressed_keys(&pressed);
        });
        b.wait_for_continue(&["f9".into()], false, &stop).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn wait_stops_on_flag() {
        let b = ContinueWaitBridge::new(true);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let s = Arc::clone(&stop_flag);
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            s.store(true, Ordering::SeqCst);
        });
        let err = b
            .wait_for_continue(&["f9".into()], false, stop_flag.as_ref())
            .unwrap_err();
        assert!(err.contains("stopped"));
        handle.join().unwrap();
    }
}
