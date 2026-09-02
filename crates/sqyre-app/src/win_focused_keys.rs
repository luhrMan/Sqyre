//! When Sqyre is focused on Windows, `WH_KEYBOARD_LL` is suppressed because egui/winit
//! registers Raw Input — so feed egui's key state into the hotkey bridges instead.
//!
//! While macro recording is armed, also poll `GetAsyncKeyState` so keys register
//! immediately even if a recording HUD / hidden root leaves us without a reliable
//! LL-hook or egui focus feed (previously required an extra click).

use crate::egui_keys::egui_key_name;
use crate::SqyreApp;
use eframe::egui::{self, Key};
use std::collections::HashSet;
use std::sync::Arc;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LWIN, VK_RWIN};

/// Call once per frame before record UI / hotkey drains.
pub fn feed_focused_keyboard(app: &mut SqyreApp, ctx: &egui::Context) {
    let recording = app.macro_record_bridge.is_armed();

    // While recording, prefer physical key state — focus/LL-hook delivery is unreliable
    // until the user clicks away from Sqyre (Raw Input suppresses WH_KEYBOARD_LL).
    if recording {
        let pressed = poll_async_pressed_keys();
        app.run_session.continue_wait.on_pressed_keys(&pressed);
        app.run_session
            .macro_hotkeys
            .on_pressed_keys(&pressed, &|_| {});
        app.macro_record_bridge.sync_pressed_keys(&pressed);
        if pressed.contains("esc") {
            let _ = app.macro_record_bridge.on_escape();
        }
        return;
    }

    let Some((mut pressed, esc_pressed)) = ctx.input(|i| {
        if !i.focused {
            return None;
        }
        let mut pressed: HashSet<&'static str> = HashSet::new();
        // Use physical modifiers only. On Windows egui sets `command` == `ctrl`
        // for cross-platform shortcuts — do not map that to Sqyre "cmd".
        if i.modifiers.ctrl {
            pressed.insert("ctrl");
        }
        if i.modifiers.shift {
            pressed.insert("shift");
        }
        if i.modifiers.alt {
            pressed.insert("alt");
        }
        for key in &i.keys_down {
            if let Some(name) = egui_key_name(*key) {
                pressed.insert(name);
            }
        }
        Some((pressed, i.key_pressed(Key::Escape)))
    }) else {
        return;
    };

    // egui-winit drops the Win key on non-macOS; read it directly.
    let (lwin, rwin) = win_logo_down();
    if lwin {
        pressed.insert("cmd");
    }
    if rwin {
        pressed.insert("rcmd");
    }

    app.run_session.continue_wait.on_pressed_keys(&pressed);

    let pending = Arc::clone(&app.pending_hotkey_macros);
    let repaint = Arc::clone(&app.hotkey_repaint);
    app.run_session
        .macro_hotkeys
        .on_pressed_keys(&pressed, &move |name| {
            crate::hotkey_wake::queue_macro_hotkey(&pending, &repaint, name);
        });

    if esc_pressed && !app.hotkey_record.is_open() && !app.key_record.is_open() {
        let ctrl = pressed.contains("ctrl");
        let shift = pressed.contains("shift") || pressed.contains("rshift");
        if app.screen_click.on_escape() {
            // Point/area recording takes Esc; don't also stop macros.
        } else if sqyre_hotkeys::failsafe_modifiers_held(&pressed) {
            crate::log::warn(format_args!(
                "failsafe {} — exiting",
                sqyre_hotkeys::FAILSAFE_LABEL
            ));
            sqyre_input::release_held_inputs();
            std::process::exit(0);
        } else if !ctrl && !shift && !app.run_session.continue_wait.continue_is_escape() {
            app.request_stop();
        }
    }
}

/// Virtual-key codes we care about while macro-recording (non-extended).
const RECORD_VKS: &[u32] = &[
    0x1B, // esc
    0xA2, 0xA3, // ctrl
    0xA0, 0xA1, // shift / rshift
    0xA4, 0xA5, // alt / ralt
    0x5B, 0x5C, // win
    0x20, 0x0D, 0x09, 0x08, 0x2E, // space enter tab backspace delete
    0x26, 0x28, 0x25, 0x27, // arrows
    0x24, 0x23, 0x21, 0x22, // home end pgup pgdn
    0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B, // f1-f12
    0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50,
    0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, // a-z
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, // 0-9
    0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, // num0-9
    0x6B, 0x6D, 0x6A, 0x6F, 0x6E, // num ops
];

fn poll_async_pressed_keys() -> HashSet<&'static str> {
    let mut pressed = HashSet::new();
    for &vk in RECORD_VKS {
        // SAFETY: GetAsyncKeyState is process-safe; high bit means currently down.
        if (unsafe { GetAsyncKeyState(vk as i32) }) < 0 {
            let extended = matches!(vk, 0x0D) && false; // plain Enter; num_enter via extended hook only
            if let Some(name) = sqyre_hotkeys::vk_key_name(vk, extended) {
                pressed.insert(name);
            }
        }
    }
    // Numpad Enter is VK_RETURN + extended; GetAsyncKeyState cannot distinguish —
    // plain enter mapping is enough for recording.
    let (lwin, rwin) = win_logo_down();
    if lwin {
        pressed.insert("cmd");
    }
    if rwin {
        pressed.insert("rcmd");
    }
    pressed
}

fn win_logo_down() -> (bool, bool) {
    // SAFETY: GetAsyncKeyState is process-safe; high bit means currently down.
    // Parens required: `unsafe { … } < 0` is parsed as a type, not a comparison.
    let left = (unsafe { GetAsyncKeyState(i32::from(VK_LWIN.0)) }) < 0;
    let right = (unsafe { GetAsyncKeyState(i32::from(VK_RWIN.0)) }) < 0;
    (left, right)
}
