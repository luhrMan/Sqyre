//! On Wayland, rdev/evdev does not see keys delivered to the focused Sqyre window.
//! Mirror egui's key state into the hotkey bridges (same approach as Windows Raw Input).

use crate::egui_keys::egui_key_name;
use crate::SqyreApp;
use eframe::egui::{self, Key};
use sqyre_capture::{LinuxSessionInfo, LinuxSessionKind};
use std::collections::HashSet;
use std::sync::Arc;

/// Call once per frame before record UI / hotkey drains.
pub fn feed_focused_keyboard(app: &mut SqyreApp, ctx: &egui::Context) {
    if LinuxSessionInfo::detect().session_kind != LinuxSessionKind::Wayland {
        return;
    }

    let record_ui_open = app.key_record.is_open() || app.hotkey_record.is_open();
    let recording = app.macro_record_bridge.is_armed();
    let focused = ctx.input(|i| i.focused);

    // Unfocused macro recording relies on rdev/evdev; do not overwrite with an empty egui snapshot.
    if recording && !focused && !record_ui_open {
        return;
    }
    if !focused && !record_ui_open {
        return;
    }

    let (pressed, esc_pressed) = ctx.input(|i| {
        let mut pressed: HashSet<&'static str> = HashSet::new();
        if i.modifiers.ctrl {
            pressed.insert("ctrl");
        }
        if i.modifiers.shift {
            pressed.insert("shift");
        }
        if i.modifiers.alt {
            pressed.insert("alt");
        }
        // On Linux Wayland, egui-winit maps Super to `command`.
        if i.modifiers.command {
            pressed.insert("cmd");
        }
        for key in &i.keys_down {
            if let Some(name) = egui_key_name(*key) {
                pressed.insert(name);
            }
        }
        (pressed, i.key_pressed(Key::Escape))
    });

    if recording {
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

    app.run_session.continue_wait.on_pressed_keys(&pressed);

    let pending = Arc::clone(&app.pending_hotkey_macros);
    let repaint = Arc::clone(&app.hotkey_repaint);
    app.run_session
        .macro_hotkeys
        .on_pressed_keys(&pressed, &move |name| {
            pending.lock().push(name);
            if let Some(ctx) = repaint.lock().as_ref() {
                ctx.request_repaint();
            }
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
