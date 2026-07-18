//! Shared chord-record modal chrome for key and hotkey capture.

use eframe::egui;
use sqyre_hotkeys::{chord_fully_released, MacroHotkeyBridge};
use std::collections::HashSet;

/// Centered non-resizable record dialog.
pub(crate) fn record_modal(
    ctx: &egui::Context,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, add_contents);
}

/// Waiting-for-release step: resume hotkeys when chord is up, else show hint.
///
/// Returns `true` when release completed (caller should close).
pub(crate) fn poll_waiting_release(
    ctx: &egui::Context,
    macro_hotkeys: &MacroHotkeyBridge,
    chord: &[String],
    title: &str,
    message: &str,
) -> bool {
    let pressed: HashSet<String> = macro_hotkeys.pressed_keys().into_iter().collect();
    if chord_fully_released(&pressed, chord) {
        macro_hotkeys.resume();
        return true;
    }
    record_modal(ctx, title, |ui| {
        ui.label(message);
    });
    ctx.request_repaint();
    false
}
