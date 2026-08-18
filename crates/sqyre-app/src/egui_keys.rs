//! Map egui [`Key`] values to Sqyre hotkey names (same strings as rdev / Win32 hooks).

use eframe::egui::Key;

pub(crate) fn egui_key_name(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::Escape => "esc",
        Key::Tab => "tab",
        Key::Backspace | Key::Delete => "delete",
        Key::Enter => "enter",
        Key::Space => "space",
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
    }
}
