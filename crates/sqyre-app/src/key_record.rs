//! Single-key record dialog.

use eframe::egui;
use sqyre_hotkeys::{chord_fully_released, MacroHotkeyBridge};
use std::collections::HashSet;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub(crate) enum KeyRecordUi {
    #[default]
    Closed,
    /// Waiting for the first key press.
    Recording,
    /// Saved; wait for release before resuming macro hotkeys.
    WaitingRelease {
        key: String,
    },
}

impl KeyRecordUi {
    pub fn open(&mut self, macro_hotkeys: &MacroHotkeyBridge) {
        if !matches!(self, Self::Closed) {
            return;
        }
        macro_hotkeys.suspend();
        *self = Self::Recording;
    }

    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Closed)
    }

    /// Draw modal; returns the recorded key once when the first key is pressed.
    ///
    /// Escape is a recordable key. Use Cancel to dismiss without saving.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        macro_hotkeys: &MacroHotkeyBridge,
    ) -> Option<String> {
        match self {
            Self::Closed => None,
            Self::WaitingRelease { key } => {
                let pressed: HashSet<String> =
                    macro_hotkeys.pressed_keys().into_iter().collect();
                let chord = [key.clone()];
                if chord_fully_released(&pressed, &chord) {
                    macro_hotkeys.resume();
                    *self = Self::Closed;
                } else {
                    egui::Window::new("Record key")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.label("Release the key to finish…");
                        });
                    ctx.request_repaint();
                }
                None
            }
            Self::Recording => {
                let pressed = macro_hotkeys.pressed_keys();
                let captured = pressed.first().cloned();

                let mut cancel = false;
                egui::Window::new("Record key")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(
                            "Press the key you want to use.\nThe first key you press is saved.\nUse Cancel to dismiss without saving.",
                        );
                        ui.separator();
                        ui.monospace("(no key)");
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });

                if cancel {
                    macro_hotkeys.resume();
                    *self = Self::Closed;
                    return None;
                }

                if let Some(key) = captured {
                    *self = Self::WaitingRelease {
                        key: key.clone(),
                    };
                    ctx.request_repaint();
                    return Some(key);
                }

                ctx.request_repaint_after(Duration::from_millis(16));
                None
            }
        }
    }
}
