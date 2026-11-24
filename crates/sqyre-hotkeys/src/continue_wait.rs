//! Shared continue-chord wait used by Pause and NavigateSelect.

use parking_lot::{Condvar, Mutex};
use sqyre_domain::{normalize_keys, validate_not_failsafe};

use crate::HotkeyError;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use web_time::{Duration, Instant};

/// What can end a continue wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinueSource {
    /// No global keys and no shell signal: every wait fails.
    Unavailable,
    /// Global key hooks feed [`ContinueWaitBridge::on_pressed_keys`].
    Keys,
    /// No global keys: only [`ContinueWaitBridge::signal_continue`] (a notification
    /// action) ends a single-chord wait. Multi-chord waits fail.
    Signal,
}

/// Bridge between the hotkey listener thread and key waiters.
#[derive(Clone)]
pub struct ContinueWaitBridge {
    inner: Arc<Inner>,
    source: ContinueSource,
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
    last_pressed: HashSet<&'static str>,
}

impl ContinueWaitBridge {
    pub fn new(source: ContinueSource) -> Self {
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(WaitState::default()),
                cv: Condvar::new(),
            }),
            source,
        }
    }

    /// End the current single-chord wait (Pause) as if its chord were pressed.
    /// No-op when nothing waits or the wait has several chords.
    pub fn signal_continue(&self) {
        let mut g = self.inner.state.lock();
        if !g.waiting || g.signaled || g.chords.len() != 1 {
            return;
        }
        g.matched = Some(0);
        g.signaled = true;
        self.inner.cv.notify_all();
    }

    /// Whether a lone Escape should resume Pause instead of stopping the macro.
    pub fn continue_is_escape(&self) -> bool {
        let g = self.inner.state.lock();
        g.waiting && g.continue_is_esc
    }

    /// Called from the hotkey thread when the set of pressed keys changes.
    pub fn on_pressed_keys(&self, pressed: &HashSet<&'static str>) {
        let mut g = self.inner.state.lock();
        // Always refresh — wait arming reads last_pressed for already-held chords.
        g.last_pressed.clone_from(pressed);
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
        let pressed = &g.last_pressed;

        let mut best: Option<(usize, usize)> = None; // (len, index)
        for (i, chord) in g.chords.iter().enumerate() {
            if chord.is_empty() {
                continue;
            }
            let all = chord.iter().all(|k| pressed.contains(k.as_str()));
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
    ) -> Result<(), HotkeyError> {
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
    ) -> Result<usize, HotkeyError> {
        if self.source == ContinueSource::Unavailable {
            return Err(HotkeyError::WaitUnavailable);
        }
        let normalized: Vec<Vec<String>> = chords.iter().map(|c| normalize_keys(c)).collect();
        if normalized.iter().all(|c| c.is_empty()) {
            return Err(HotkeyError::NoChords);
        }
        if self.source == ContinueSource::Signal && normalized.len() != 1 {
            return Err(HotkeyError::WaitUnavailable);
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
                    !chord.is_empty() && chord.iter().all(|k| g.last_pressed.contains(k.as_str()))
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
                break Err(HotkeyError::Stopped);
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
                self.inner.cv.wait_for(&mut g, Duration::from_millis(50));
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

/// Map an rdev key to a Sqyre hotkey name (lowercase).
/// Stable key names for `db.yaml` chords (X11 keysym → hook name).
///
/// Punctuation uses word names (`minus`, `equal`, …) so chords never collide with
/// the `+` join / `-` empty-hotkey sentinels in [`crate::format_hotkey`] / parse.
#[cfg(all(feature = "hooks", not(target_os = "windows")))]
pub fn rdev_key_name(key: rdev::Key) -> Option<&'static str> {
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
        Key::Insert => "insert",
        Key::CapsLock => "capslock",
        Key::NumLock => "numlock",
        Key::ScrollLock => "scrolllock",
        Key::PrintScreen => "printscreen",
        Key::Pause => "pause",
        Key::Function => "fn",
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
        Key::Minus => "minus",
        Key::Equal => "equal",
        Key::LeftBracket => "leftbracket",
        Key::RightBracket => "rightbracket",
        Key::SemiColon => "semicolon",
        Key::Quote => "quote",
        Key::BackQuote => "grave",
        Key::BackSlash => "backslash",
        Key::IntlBackslash => "intlbackslash",
        Key::Comma => "comma",
        Key::Dot => "period",
        Key::Slash => "slash",
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
        // OS-specific scan codes — no stable cross-platform name.
        Key::Unknown(_) => return None,
    };
    Some(name)
}

/// Map a Win32 virtual-key code to a Sqyre hotkey name (same strings as [`rdev_key_name`]).
///
/// `extended` is the LLKHF_EXTENDED bit from `KBDLLHOOKSTRUCT::flags` (numpad Enter, etc.).
/// Pure lookup — available on every target so Linux CI can assert the map.
pub fn vk_key_name(vk: u32, extended: bool) -> Option<&'static str> {
    // https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
    let name = match vk {
        0x1B => "esc",                   // VK_ESCAPE
        0xA2 | 0xA3 => "ctrl",           // VK_LCONTROL | VK_RCONTROL
        0xA0 => "shift",                 // VK_LSHIFT
        0xA1 => "rshift",                // VK_RSHIFT
        0xA4 => "alt",                   // VK_LMENU
        0xA5 => "ralt",                  // VK_RMENU
        0x5B => "cmd",                   // VK_LWIN
        0x5C => "rcmd",                  // VK_RWIN
        0x20 => "space",                 // VK_SPACE
        0x0D if extended => "num_enter", // VK_RETURN + extended
        0x0D => "enter",                 // VK_RETURN
        0x09 => "tab",                   // VK_TAB
        0x08 => "backspace",             // VK_BACK
        0x2E => "delete",                // VK_DELETE
        0x26 => "up",                    // VK_UP
        0x28 => "down",                  // VK_DOWN
        0x25 => "left",                  // VK_LEFT
        0x27 => "right",                 // VK_RIGHT
        0x24 => "home",                  // VK_HOME
        0x23 => "end",                   // VK_END
        0x21 => "pageup",                // VK_PRIOR
        0x22 => "pagedown",              // VK_NEXT
        0x2D => "insert",                // VK_INSERT
        0x14 => "capslock",              // VK_CAPITAL
        0x90 => "numlock",               // VK_NUMLOCK
        0x91 => "scrolllock",            // VK_SCROLL
        0x2C => "printscreen",           // VK_SNAPSHOT
        0x13 => "pause",                 // VK_PAUSE
        0x70 => "f1",
        0x71 => "f2",
        0x72 => "f3",
        0x73 => "f4",
        0x74 => "f5",
        0x75 => "f6",
        0x76 => "f7",
        0x77 => "f8",
        0x78 => "f9",
        0x79 => "f10",
        0x7A => "f11",
        0x7B => "f12",
        0x41 => "a",
        0x42 => "b",
        0x43 => "c",
        0x44 => "d",
        0x45 => "e",
        0x46 => "f",
        0x47 => "g",
        0x48 => "h",
        0x49 => "i",
        0x4A => "j",
        0x4B => "k",
        0x4C => "l",
        0x4D => "m",
        0x4E => "n",
        0x4F => "o",
        0x50 => "p",
        0x51 => "q",
        0x52 => "r",
        0x53 => "s",
        0x54 => "t",
        0x55 => "u",
        0x56 => "v",
        0x57 => "w",
        0x58 => "x",
        0x59 => "y",
        0x5A => "z",
        0x30 => "0",
        0x31 => "1",
        0x32 => "2",
        0x33 => "3",
        0x34 => "4",
        0x35 => "5",
        0x36 => "6",
        0x37 => "7",
        0x38 => "8",
        0x39 => "9",
        0xBD => "minus",         // VK_OEM_MINUS
        0xBB => "equal",         // VK_OEM_PLUS (= / +)
        0xDB => "leftbracket",   // VK_OEM_4 ([ / {)
        0xDD => "rightbracket",  // VK_OEM_6 (] / })
        0xBA => "semicolon",     // VK_OEM_1 (; / :)
        0xDE => "quote",         // VK_OEM_7 (' / ")
        0xC0 => "grave",         // VK_OEM_3 (` / ~)
        0xDC => "backslash",     // VK_OEM_5 (\ / |)
        0xE2 => "intlbackslash", // VK_OEM_102
        0xBC => "comma",         // VK_OEM_COMMA
        0xBE => "period",        // VK_OEM_PERIOD
        0xBF => "slash",         // VK_OEM_2 (/ / ?)
        0x60 => "num0",
        0x61 => "num1",
        0x62 => "num2",
        0x63 => "num3",
        0x64 => "num4",
        0x65 => "num5",
        0x66 => "num6",
        0x67 => "num7",
        0x68 => "num8",
        0x69 => "num9",
        0x6B => "num_plus",     // VK_ADD
        0x6D => "num_minus",    // VK_SUBTRACT
        0x6A => "num_asterisk", // VK_MULTIPLY
        0x6F => "num_slash",    // VK_DIVIDE
        0x6E => "num_period",   // VK_DECIMAL
        _ => return None,
    };
    Some(name)
}

#[cfg(all(test, feature = "hooks", not(target_os = "windows")))]
mod rdev_key_name_tests {
    use super::rdev_key_name;
    use rdev::Key;
    use std::collections::HashMap;

    #[test]
    fn maps_operator_and_lock_keys() {
        assert_eq!(rdev_key_name(Key::Minus), Some("minus"));
        assert_eq!(rdev_key_name(Key::Equal), Some("equal"));
        assert_eq!(rdev_key_name(Key::LeftBracket), Some("leftbracket"));
        assert_eq!(rdev_key_name(Key::RightBracket), Some("rightbracket"));
        assert_eq!(rdev_key_name(Key::BackQuote), Some("grave"));
        assert_eq!(rdev_key_name(Key::CapsLock), Some("capslock"));
        assert_eq!(rdev_key_name(Key::NumLock), Some("numlock"));
        assert_eq!(rdev_key_name(Key::KpPlus), Some("num_plus"));
        assert_eq!(rdev_key_name(Key::Backspace), Some("backspace"));
        assert_eq!(rdev_key_name(Key::Delete), Some("delete"));
        assert_eq!(rdev_key_name(Key::Unknown(42)), None);
    }

    /// Every physical key we care about for recording (Unknown excluded).
    fn recordable_keys() -> &'static [Key] {
        &[
            Key::Escape,
            Key::ControlLeft,
            Key::ControlRight,
            Key::ShiftLeft,
            Key::ShiftRight,
            Key::Alt,
            Key::AltGr,
            Key::MetaLeft,
            Key::MetaRight,
            Key::Space,
            Key::Return,
            Key::Tab,
            Key::Backspace,
            Key::Delete,
            Key::UpArrow,
            Key::DownArrow,
            Key::LeftArrow,
            Key::RightArrow,
            Key::Home,
            Key::End,
            Key::PageUp,
            Key::PageDown,
            Key::Insert,
            Key::CapsLock,
            Key::NumLock,
            Key::ScrollLock,
            Key::PrintScreen,
            Key::Pause,
            Key::Function,
            Key::F1,
            Key::F2,
            Key::F3,
            Key::F4,
            Key::F5,
            Key::F6,
            Key::F7,
            Key::F8,
            Key::F9,
            Key::F10,
            Key::F11,
            Key::F12,
            Key::KeyA,
            Key::KeyB,
            Key::KeyC,
            Key::KeyD,
            Key::KeyE,
            Key::KeyF,
            Key::KeyG,
            Key::KeyH,
            Key::KeyI,
            Key::KeyJ,
            Key::KeyK,
            Key::KeyL,
            Key::KeyM,
            Key::KeyN,
            Key::KeyO,
            Key::KeyP,
            Key::KeyQ,
            Key::KeyR,
            Key::KeyS,
            Key::KeyT,
            Key::KeyU,
            Key::KeyV,
            Key::KeyW,
            Key::KeyX,
            Key::KeyY,
            Key::KeyZ,
            Key::Num0,
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
            Key::Num9,
            Key::Minus,
            Key::Equal,
            Key::LeftBracket,
            Key::RightBracket,
            Key::SemiColon,
            Key::Quote,
            Key::BackQuote,
            Key::BackSlash,
            Key::IntlBackslash,
            Key::Comma,
            Key::Dot,
            Key::Slash,
            Key::Kp0,
            Key::Kp1,
            Key::Kp2,
            Key::Kp3,
            Key::Kp4,
            Key::Kp5,
            Key::Kp6,
            Key::Kp7,
            Key::Kp8,
            Key::Kp9,
            Key::KpReturn,
            Key::KpPlus,
            Key::KpMinus,
            Key::KpMultiply,
            Key::KpDivide,
            Key::KpDelete,
        ]
    }

    #[test]
    fn every_recordable_key_has_a_name() {
        for key in recordable_keys() {
            assert!(
                rdev_key_name(*key).is_some(),
                "missing rdev name for {key:?}"
            );
        }
    }

    #[test]
    fn mapped_names_unique_except_ctrl_sides() {
        let mut by_name: HashMap<&str, Vec<Key>> = HashMap::new();
        for key in recordable_keys() {
            let name = rdev_key_name(*key).expect("mapped");
            by_name.entry(name).or_default().push(*key);
        }
        for (name, keys) in &by_name {
            if *name == "ctrl" {
                assert_eq!(
                    keys.len(),
                    2,
                    "ctrl should fold only left+right Control: {keys:?}"
                );
                continue;
            }
            assert_eq!(
                keys.len(),
                1,
                "name {name:?} shared by multiple keys: {keys:?}"
            );
        }
    }

    #[test]
    fn top_row_digits_distinct_from_numpad() {
        let pairs = [
            (Key::Num0, Key::Kp0, "0", "num0"),
            (Key::Num1, Key::Kp1, "1", "num1"),
            (Key::Num2, Key::Kp2, "2", "num2"),
            (Key::Num3, Key::Kp3, "3", "num3"),
            (Key::Num4, Key::Kp4, "4", "num4"),
            (Key::Num5, Key::Kp5, "5", "num5"),
            (Key::Num6, Key::Kp6, "6", "num6"),
            (Key::Num7, Key::Kp7, "7", "num7"),
            (Key::Num8, Key::Kp8, "8", "num8"),
            (Key::Num9, Key::Kp9, "9", "num9"),
        ];
        for (row, pad, want_row, want_pad) in pairs {
            assert_eq!(rdev_key_name(row), Some(want_row));
            assert_eq!(rdev_key_name(pad), Some(want_pad));
            assert_ne!(want_row, want_pad);
        }
        assert_eq!(rdev_key_name(Key::Return), Some("enter"));
        assert_eq!(rdev_key_name(Key::KpReturn), Some("num_enter"));
        assert_ne!(rdev_key_name(Key::Backspace), rdev_key_name(Key::Delete));
        assert_ne!(rdev_key_name(Key::Minus), rdev_key_name(Key::KpMinus));
        assert_ne!(rdev_key_name(Key::Dot), rdev_key_name(Key::KpDelete));
    }
}

#[cfg(test)]
mod vk_key_name_tests {
    use super::vk_key_name;
    use std::collections::HashMap;

    #[test]
    fn top_row_digits_distinct_from_numpad() {
        for i in 0u32..10 {
            let row = vk_key_name(0x30 + i, false).unwrap_or_else(|| panic!("digit {i}"));
            let pad = vk_key_name(0x60 + i, false).unwrap_or_else(|| panic!("num{i}"));
            let want_row = format!("{i}");
            let want_pad = format!("num{i}");
            assert_eq!(row, want_row.as_str());
            assert_eq!(pad, want_pad.as_str());
            assert_ne!(row, pad);
        }
        assert_eq!(vk_key_name(0x0D, false), Some("enter"));
        assert_eq!(vk_key_name(0x0D, true), Some("num_enter"));
        assert_eq!(vk_key_name(0x08, false), Some("backspace"));
        assert_eq!(vk_key_name(0x2E, false), Some("delete"));
        assert_ne!(vk_key_name(0x08, false), vk_key_name(0x2E, false));
    }

    #[test]
    fn non_extended_vk_names_are_unique() {
        let vks: &[u32] = &[
            0x1B, 0xA2, 0xA3, 0xA0, 0xA1, 0xA4, 0xA5, 0x5B, 0x5C, 0x20, 0x0D, 0x09, 0x08, 0x2E,
            0x26, 0x28, 0x25, 0x27, 0x24, 0x23, 0x21, 0x22, 0x2D, 0x14, 0x90, 0x91, 0x2C, 0x13,
            0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B, 0x41, 0x42,
            0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50,
            0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x30, 0x31, 0x32, 0x33,
            0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0xBD, 0xBB, 0xDB, 0xDD, 0xBA, 0xDE, 0xC0, 0xDC,
            0xE2, 0xBC, 0xBE, 0xBF, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
            0x6B, 0x6D, 0x6A, 0x6F, 0x6E,
        ];
        let mut by_name: HashMap<&str, Vec<u32>> = HashMap::new();
        for &vk in vks {
            let name = vk_key_name(vk, false).unwrap_or_else(|| panic!("unmapped VK 0x{vk:02X}"));
            by_name.entry(name).or_default().push(vk);
        }
        for (name, codes) in &by_name {
            if *name == "ctrl" {
                assert_eq!(
                    codes.len(),
                    2,
                    "ctrl should fold L/R control only: {codes:?}"
                );
                continue;
            }
            assert_eq!(
                codes.len(),
                1,
                "name {name:?} shared by multiple VKs: {codes:?}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    /// Spawn a thread that sets the stop flag once the bridge starts waiting.
    fn spawn_stop_on_waiting(b: &ContinueWaitBridge) -> (Arc<AtomicBool>, thread::JoinHandle<()>) {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let s = Arc::clone(&stop_flag);
        let bridge = b.clone();
        let handle = thread::spawn(move || {
            while !bridge.inner.state.lock().waiting {
                thread::yield_now();
            }
            s.store(true, Ordering::SeqCst);
        });
        (stop_flag, handle)
    }

    #[test]
    fn wait_errors_without_hooks() {
        let b = ContinueWaitBridge::new(ContinueSource::Unavailable);
        let stop = AtomicBool::new(false);
        let err = b
            .wait_for_continue(&["f9".into()], false, &stop)
            .unwrap_err();
        assert!(matches!(err, HotkeyError::WaitUnavailable));
    }

    #[test]
    fn signal_ends_a_pause_wait() {
        let b = ContinueWaitBridge::new(ContinueSource::Signal);
        let stop = AtomicBool::new(false);
        let bridge = b.clone();
        let handle = thread::spawn(move || {
            while !bridge.inner.state.lock().waiting {
                thread::yield_now();
            }
            bridge.signal_continue();
        });
        b.wait_for_continue(&["f9".into()], false, &stop).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn signal_before_a_wait_is_dropped() {
        let b = ContinueWaitBridge::new(ContinueSource::Signal);
        b.signal_continue();
        let g = b.inner.state.lock();
        assert!(!g.signaled);
        assert!(g.matched.is_none());
    }

    #[test]
    fn signal_mode_rejects_multi_chord_waits() {
        let b = ContinueWaitBridge::new(ContinueSource::Signal);
        let stop = AtomicBool::new(false);
        let err = b
            .wait_for_any_chord(&[vec!["up".into()], vec!["down".into()]], &[], false, &stop)
            .unwrap_err();
        assert!(matches!(err, HotkeyError::WaitUnavailable));
    }

    #[test]
    fn signal_mode_wait_stops_on_flag() {
        let b = ContinueWaitBridge::new(ContinueSource::Signal);
        let (stop_flag, handle) = spawn_stop_on_waiting(&b);
        let err = b
            .wait_for_continue(&["f9".into()], false, stop_flag.as_ref())
            .unwrap_err();
        assert!(matches!(err, HotkeyError::Stopped));
        handle.join().unwrap();
    }

    #[test]
    fn wait_signaled_by_pressed_keys() {
        let b = ContinueWaitBridge::new(ContinueSource::Keys);
        let stop = AtomicBool::new(false);
        let bridge = b.clone();
        let handle = thread::spawn(move || {
            while !bridge.inner.state.lock().waiting {
                thread::yield_now();
            }
            let mut pressed = HashSet::new();
            pressed.insert("f9");
            bridge.on_pressed_keys(&pressed);
        });
        b.wait_for_continue(&["f9".into()], false, &stop).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn wait_any_picks_longest_chord() {
        let b = ContinueWaitBridge::new(ContinueSource::Keys);
        let stop = AtomicBool::new(false);
        let bridge = b.clone();
        let handle = thread::spawn(move || {
            while !bridge.inner.state.lock().waiting {
                thread::yield_now();
            }
            let mut pressed = HashSet::new();
            pressed.insert("ctrl");
            pressed.insert("a");
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
        let b = ContinueWaitBridge::new(ContinueSource::Keys);
        let (stop_flag, handle) = spawn_stop_on_waiting(&b);
        let err = b
            .wait_for_continue(&["f9".into()], false, stop_flag.as_ref())
            .unwrap_err();
        assert!(matches!(err, HotkeyError::Stopped));
        handle.join().unwrap();
    }
}
