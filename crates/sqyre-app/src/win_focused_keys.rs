//! When Sqyre is focused on Windows, `WH_KEYBOARD_LL` is suppressed because egui/winit
//! registers Raw Input — so feed egui's key state into the hotkey bridges instead.

use crate::SqyreApp;
use eframe::egui::{self, Key};
use std::collections::HashSet;
use std::sync::Arc;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LWIN, VK_RWIN};

/// Call once per frame before record UI / hotkey drains.
pub fn feed_focused_keyboard(app: &mut SqyreApp, ctx: &egui::Context) {
    let Some((mut pressed, esc_pressed)) = ctx.input(|i| {
        if !i.focused {
            return None;
        }
        let mut pressed = HashSet::new();
        // Use physical modifiers only. On Windows egui sets `command` == `ctrl`
        // for cross-platform shortcuts — do not map that to Sqyre "cmd".
        if i.modifiers.ctrl {
            pressed.insert("ctrl".into());
        }
        if i.modifiers.shift {
            pressed.insert("shift".into());
        }
        if i.modifiers.alt {
            pressed.insert("alt".into());
        }
        for key in &i.keys_down {
            if let Some(name) = egui_key_name(*key) {
                pressed.insert(name.into());
            }
        }
        Some((pressed, i.key_pressed(Key::Escape)))
    }) else {
        return;
    };

    // egui-winit drops the Win key on non-macOS; read it directly.
    let (lwin, rwin) = win_logo_down();
    if lwin {
        pressed.insert("cmd".into());
    }
    if rwin {
        pressed.insert("rcmd".into());
    }

    app.continue_wait.on_pressed_keys(&pressed);

    let pending = Arc::clone(&app.pending_hotkey_macros);
    let repaint = Arc::clone(&app.hotkey_repaint);
    app.macro_hotkeys.on_pressed_keys(&pressed, &move |name| {
        pending.lock().push(name);
        if let Some(ctx) = repaint.lock().as_ref() {
            ctx.request_repaint();
        }
    });

    if esc_pressed && !app.hotkey_record.is_open() && !app.key_record.is_open() {
        let ctrl = pressed.contains("ctrl");
        let shift = pressed.contains("shift") || pressed.contains("rshift");
        if app.screen_click.on_escape() {
            // Recording takes Esc; don't also stop macros.
        } else if ctrl && shift {
            eprintln!("failsafe Esc+Ctrl+Shift — exiting");
            std::process::exit(0);
        } else if !ctrl && !shift && !app.continue_wait.continue_is_escape() {
            app.request_stop();
        }
    }
}

fn win_logo_down() -> (bool, bool) {
    // SAFETY: GetAsyncKeyState is process-safe; high bit means currently down.
    let left = unsafe { GetAsyncKeyState(i32::from(VK_LWIN.0)) } < 0;
    let right = unsafe { GetAsyncKeyState(i32::from(VK_RWIN.0)) } < 0;
    (left, right)
}

fn egui_key_name(key: Key) -> Option<&'static str> {
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
