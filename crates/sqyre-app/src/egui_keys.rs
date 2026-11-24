//! Map egui [`Key`] values to Sqyre hotkey names (same strings as rdev / Win32 hooks).

use eframe::egui::Key;

pub(crate) fn egui_key_name(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::Escape => "esc",
        Key::Tab => "tab",
        Key::Backspace => "backspace",
        Key::Delete => "delete",
        Key::Enter => "enter",
        Key::Space => "space",
        Key::Insert => "insert",
        Key::ArrowUp => "up",
        Key::ArrowDown => "down",
        Key::ArrowLeft => "left",
        Key::ArrowRight => "right",
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
        Key::A => "a",
        Key::B => "b",
        Key::C => "c",
        Key::D => "d",
        Key::E => "e",
        Key::F => "f",
        Key::G => "g",
        Key::H => "h",
        Key::I => "i",
        Key::J => "j",
        Key::K => "k",
        Key::L => "l",
        Key::M => "m",
        Key::N => "n",
        Key::O => "o",
        Key::P => "p",
        Key::Q => "q",
        Key::R => "r",
        Key::S => "s",
        Key::T => "t",
        Key::U => "u",
        Key::V => "v",
        Key::W => "w",
        Key::X => "x",
        Key::Y => "y",
        Key::Z => "z",
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
        // Punctuation — fold shift-glyph variants onto the physical key name so
        // focused egui recording matches hook/evdev chords (US layout).
        Key::Minus => "minus",
        Key::Equals | Key::Plus => "equal",
        Key::OpenBracket | Key::OpenCurlyBracket => "leftbracket",
        Key::CloseBracket | Key::CloseCurlyBracket => "rightbracket",
        Key::Semicolon | Key::Colon => "semicolon",
        Key::Quote => "quote",
        Key::Backtick => "grave",
        Key::Backslash | Key::Pipe => "backslash",
        Key::Comma => "comma",
        Key::Period => "period",
        Key::Slash | Key::Questionmark => "slash",
        Key::IntlBackslash => "intlbackslash",
        // Physical modifiers (egui 0.36+). Left/right Ctrl share one chord name;
        // Shift/Alt keep the side so chords can require the right-hand key.
        Key::ShiftLeft => "shift",
        Key::ShiftRight => "rshift",
        Key::ControlLeft | Key::ControlRight => "ctrl",
        Key::AltLeft => "alt",
        Key::AltRight => "ralt",
        Key::SuperLeft => "cmd",
        Key::SuperRight => "rcmd",
        // egui collapses DigitN and NumpadN into Key::NumN — numpad digits are
        // only distinct on the hook/evdev/Win32 path (`num0`…`num9`).
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::egui_key_name;
    use eframe::egui::Key;

    #[test]
    fn maps_common_record_keys() {
        assert_eq!(egui_key_name(Key::Escape), Some("esc"));
        assert_eq!(egui_key_name(Key::A), Some("a"));
        assert_eq!(egui_key_name(Key::F5), Some("f5"));
        assert_eq!(egui_key_name(Key::Enter), Some("enter"));
        assert_eq!(egui_key_name(Key::Backspace), Some("backspace"));
        assert_eq!(egui_key_name(Key::Delete), Some("delete"));
        assert_ne!(egui_key_name(Key::Backspace), egui_key_name(Key::Delete));
        assert_eq!(egui_key_name(Key::ShiftLeft), Some("shift"));
        assert_eq!(egui_key_name(Key::ShiftRight), Some("rshift"));
        assert_eq!(egui_key_name(Key::ControlLeft), Some("ctrl"));
        assert_eq!(egui_key_name(Key::AltRight), Some("ralt"));
        assert_eq!(egui_key_name(Key::SuperLeft), Some("cmd"));
        assert_eq!(egui_key_name(Key::SuperRight), Some("rcmd"));
    }

    #[test]
    fn maps_operator_and_punctuation_keys() {
        assert_eq!(egui_key_name(Key::Minus), Some("minus"));
        assert_eq!(egui_key_name(Key::Equals), Some("equal"));
        assert_eq!(egui_key_name(Key::Plus), Some("equal"));
        assert_eq!(egui_key_name(Key::OpenBracket), Some("leftbracket"));
        assert_eq!(egui_key_name(Key::OpenCurlyBracket), Some("leftbracket"));
        assert_eq!(egui_key_name(Key::CloseCurlyBracket), Some("rightbracket"));
        assert_eq!(egui_key_name(Key::Backtick), Some("grave"));
        assert_eq!(egui_key_name(Key::Semicolon), Some("semicolon"));
        assert_eq!(egui_key_name(Key::Slash), Some("slash"));
    }

    #[test]
    fn mapped_names_unique_aside_from_shift_glyphs_and_ctrl() {
        use std::collections::HashMap;
        let mut by_name: HashMap<&str, Vec<Key>> = HashMap::new();
        for key in Key::ALL {
            if let Some(name) = egui_key_name(*key) {
                by_name.entry(name).or_default().push(*key);
            }
        }
        // Intentional folds: shift-glyph variants share a physical key name; L/R Ctrl share "ctrl".
        let allowed_multi = [
            "equal",
            "leftbracket",
            "rightbracket",
            "semicolon",
            "backslash",
            "slash",
            "ctrl",
        ];
        for (name, keys) in &by_name {
            if allowed_multi.contains(name) {
                assert!(
                    keys.len() >= 2,
                    "{name} expected to fold multiple egui keys, got {keys:?}"
                );
                continue;
            }
            assert_eq!(
                keys.len(),
                1,
                "name {name:?} shared by multiple egui keys: {keys:?}"
            );
        }
        // Digits stay distinct from each other. egui has no separate numpad Key
        // (DigitN and NumpadN both become Key::NumN) — numpad distinction lives
        // on the hook/evdev/VK path as `num0`…`num9`.
        assert_eq!(egui_key_name(Key::Num0), Some("0"));
        assert_eq!(egui_key_name(Key::Num1), Some("1"));
        assert_ne!(egui_key_name(Key::Num0), egui_key_name(Key::Num1));
    }
}
