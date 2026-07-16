//! Shared continue-chord wait used by Pause and NavigateSelect.

use parking_lot::{Condvar, Mutex};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Bridge between the hotkey listener thread and key waiters.
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
    /// When non-empty, waiting for any of these chords (longest match wins).
    chords: Vec<Vec<String>>,
    /// Parallel to `chords`: which indices re-fire while held.
    hold_repeat: Vec<bool>,
    hold_repeat_ms: u64,
    /// Continue chord is a lone Escape — Esc should resume, not stop.
    continue_is_esc: bool,
    /// Matched chord index when `signaled`.
    matched: Option<usize>,
    signaled: bool,
    /// Indices currently considered “held since last edge”.
    latched: HashSet<usize>,
    /// For hold-repeat: last fire time per index.
    last_fire: Vec<Option<Instant>>,
    /// Latest pressed-key snapshot from the hook thread.
    last_pressed: HashSet<String>,
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
        g.last_pressed = pressed.clone();
        if !g.waiting || g.signaled {
            return;
        }
        Self::try_match_locked(&mut g);
        if g.signaled {
            self.inner.cv.notify_all();
        }
    }

    fn try_match_locked(g: &mut WaitState) {
        if g.chords.is_empty() || g.signaled {
            return;
        }
        let pressed = g.last_pressed.clone();

        let mut best: Option<(usize, usize)> = None; // (len, index)
        for (i, chord) in g.chords.iter().enumerate() {
            if chord.is_empty() {
                continue;
            }
            let all = chord.iter().all(|k| pressed.contains(k));
            if all {
                let len = chord.len();
                match best {
                    Some((best_len, _)) if len <= best_len => {}
                    _ => best = Some((len, i)),
                }
            } else {
                g.latched.remove(&i);
            }
        }

        let Some((_, idx)) = best else {
            return;
        };

        let hold = g.hold_repeat.get(idx).copied().unwrap_or(false);
        if g.latched.contains(&idx) {
            if !hold {
                return;
            }
            let ms = g.hold_repeat_ms.max(1);
            let due = g
                .last_fire
                .get(idx)
                .and_then(|t| *t)
                .map(|t| t.elapsed() >= Duration::from_millis(ms))
                .unwrap_or(true);
            if !due {
                return;
            }
        }

        g.latched.insert(idx);
        if let Some(slot) = g.last_fire.get_mut(idx) {
            *slot = Some(Instant::now());
        }
        g.matched = Some(idx);
        g.signaled = true;
    }

    /// Block until the continue chord is pressed or `stop` is set.
    pub fn wait_for_continue(
        &self,
        keys: &[String],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<(), String> {
        self.wait_for_any_chord(&[keys.to_vec()], &[], pass_through, stop)
            .map(|_| ())
    }

    /// Block until one of `chords` is pressed. Returns the matched chord index.
    ///
    /// `hold_repeat` is parallel to `chords` (missing entries = false). While a
    /// hold-repeat chord stays pressed, it re-fires every ~180ms after the first edge.
    pub fn wait_for_any_chord(
        &self,
        chords: &[Vec<String>],
        hold_repeat: &[bool],
        _pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<usize, String> {
        if !self.hooks_enabled {
            return Err("key wait is not available in this build".into());
        }
        let normalized: Vec<Vec<String>> = chords
            .iter()
            .map(|c| normalize_keys(c))
            .collect();
        if normalized.iter().all(|c| c.is_empty()) {
            return Err("key wait: no chords configured".into());
        }
        for c in &normalized {
            if !c.is_empty() {
                validate_not_failsafe(c)?;
            }
        }

        let hold: Vec<bool> = (0..normalized.len())
            .map(|i| hold_repeat.get(i).copied().unwrap_or(false))
            .collect();

        {
            let mut g = self.inner.state.lock();
            g.waiting = true;
            g.chords = normalized.clone();
            g.hold_repeat = hold;
            g.hold_repeat_ms = 180;
            g.continue_is_esc = normalized.len() == 1 && normalized[0] == ["esc".to_string()];
            g.matched = None;
            g.signaled = false;
            // Re-latch from current physical keys so hold-repeat can continue
            // across wait calls without needing a new key event.
            g.latched.clear();
            g.last_fire = vec![None; normalized.len()];
            let already: Vec<usize> = g
                .chords
                .iter()
                .enumerate()
                .filter(|(_, chord)| {
                    !chord.is_empty() && chord.iter().all(|k| g.last_pressed.contains(k))
                })
                .map(|(i, _)| i)
                .collect();
            for i in already {
                g.latched.insert(i);
                if g.hold_repeat.get(i).copied().unwrap_or(false) {
                    if let Some(slot) = g.last_fire.get_mut(i) {
                        *slot = Some(Instant::now());
                    }
                }
            }
            Self::try_match_locked(&mut g);
        }

        let result = loop {
            if stop.load(Ordering::SeqCst) {
                break Err("stopped".into());
            }
            {
                let mut g = self.inner.state.lock();
                if !g.signaled {
                    // Hold-repeat can become due without a new key event.
                    Self::try_match_locked(&mut g);
                }
                if g.signaled {
                    break Ok(g.matched.unwrap_or(0));
                }
                self.inner
                    .cv
                    .wait_for(&mut g, Duration::from_millis(50));
                if !g.signaled {
                    Self::try_match_locked(&mut g);
                }
                if g.signaled {
                    break Ok(g.matched.unwrap_or(0));
                }
            }
        };

        {
            let mut g = self.inner.state.lock();
            g.waiting = false;
            g.chords.clear();
            g.hold_repeat.clear();
            g.continue_is_esc = false;
            g.matched = None;
            g.signaled = false;
            g.latched.clear();
            g.last_fire.clear();
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
        "control" | "rcontrol" | "controlleft" | "controlright" => "ctrl".into(),
        "return" => "enter".into(),
        "super" | "meta" | "win" | "windows" | "meta_left" | "metaleft" => "cmd".into(),
        "meta_right" | "metaright" => "rcmd".into(),
        "backspace" | "back_space" => "delete".into(),
        "shiftleft" | "shift_left" => "shift".into(),
        "shiftright" | "shift_right" => "rshift".into(),
        "altleft" | "alt_left" => "alt".into(),
        "altright" | "alt_right" | "altgr" => "ralt".into(),
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
            "key wait: chord cannot match the failsafe hotkey (esc + ctrl + shift)".into(),
        );
    }
    Ok(())
}

/// Map an rdev key to a Sqyre hotkey name (lowercase).
/// Stable key names for `db.yaml` chords (X11 keysym → hook name).
#[cfg(feature = "hooks")]
pub fn rdev_key_name(key: rdev::Key) -> Option<String> {
    use rdev::Key;
    let name = match key {
        Key::Escape => "esc",
        Key::ControlLeft | Key::ControlRight => "ctrl",
        Key::ShiftLeft => "shift",
        Key::ShiftRight => "rshift",
        Key::Alt => "alt",
        Key::AltGr => "ralt",
        Key::MetaLeft => "cmd",
        Key::MetaRight => "rcmd",
        Key::Space => "space",
        Key::Return => "enter",
        Key::Tab => "tab",
        // XK_BackSpace → "delete".
        Key::Backspace => "delete",
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
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::Kp0 => "num0",
        Key::Kp1 => "num1",
        Key::Kp2 => "num2",
        Key::Kp3 => "num3",
        Key::Kp4 => "num4",
        Key::Kp5 => "num5",
        Key::Kp6 => "num6",
        Key::Kp7 => "num7",
        Key::Kp8 => "num8",
        Key::Kp9 => "num9",
        Key::KpReturn => "num_enter",
        Key::KpPlus => "num_plus",
        Key::KpMinus => "num_minus",
        Key::KpMultiply => "num_asterisk",
        Key::KpDivide => "num_slash",
        Key::KpDelete => "num_period",
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
    fn wait_any_picks_longest_chord() {
        let b = ContinueWaitBridge::new(true);
        let stop = AtomicBool::new(false);
        let bridge = b.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            let mut pressed = HashSet::new();
            pressed.insert("ctrl".into());
            pressed.insert("a".into());
            bridge.on_pressed_keys(&pressed);
        });
        let idx = b
            .wait_for_any_chord(
                &[vec!["a".into()], vec!["ctrl".into(), "a".into()]],
                &[],
                false,
                &stop,
            )
            .unwrap();
        assert_eq!(idx, 1);
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
