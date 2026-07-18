//! Single-key record dialog.

use crate::chord_record::{poll_waiting_release, record_modal};
use eframe::egui;
use sqyre_hotkeys::MacroHotkeyBridge;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub(crate) enum KeyRecordUi {
    #[default]
    Closed,
    /// Waiting for the first key press.
    Recording,
    /// Saved; wait for release before resuming macro hotkeys.
    WaitingRelease { key: String },
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
                let chord = [key.clone()];
                if poll_waiting_release(
                    ctx,
                    macro_hotkeys,
                    &chord,
                    "Record key",
                    "Release the key to finish…",
                ) {
                    *self = Self::Closed;
                }
                None
            }
            Self::Recording => {
                let pressed = macro_hotkeys.pressed_keys();
                let captured = pressed.first().cloned();

                let mut cancel = false;
                record_modal(ctx, "Record key", |ui| {
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
                    *self = Self::WaitingRelease { key: key.clone() };
                    ctx.request_repaint();
                    return Some(key);
                }

                ctx.request_repaint_after(Duration::from_millis(16));
                None
            }
        }
    }
}
