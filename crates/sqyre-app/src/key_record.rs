//! Single-key record dialog.

use crate::chord_record::{poll_waiting_release, record_modal};
use crate::widgets::dismiss_row;
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
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
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
                    pending_scale,
                ) {
                    *self = Self::Closed;
                }
                None
            }
            Self::Recording => {
                let pressed = macro_hotkeys.pressed_keys();
                let captured = pressed.first().cloned();

                let mut cancel = false;
                record_modal(ctx, "Record key", pending_scale, |ui| {
                    ui.label(
                        "Press the key you want to use.\nThe first key you press is saved.\nUse Cancel to dismiss without saving.",
                    );
                    crate::widgets::section_separator(ui);
                    ui.monospace("(no key)");
                    // Escape is recordable — button only (no Esc dismiss).
                    cancel = dismiss_row(ui, false);
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

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    use egui_kittest::kittest::Queryable;
    use std::collections::HashSet;

    struct Host {
        ui: KeyRecordUi,
        keys: MacroHotkeyBridge,
        saved: Option<String>,
    }

    fn harness() -> egui_kittest::Harness<'static, Host> {
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(640.0, 480.0))
            .build_ui_state(
                |ui, host: &mut Host| {
                    if let Some(key) = host.ui.show(ui.ctx(), &host.keys, None) {
                        host.saved = Some(key);
                    }
                },
                Host {
                    ui: KeyRecordUi::default(),
                    keys: MacroHotkeyBridge::new(),
                    saved: None,
                },
            )
    }

    fn open(harness: &mut egui_kittest::Harness<'static, Host>) {
        let keys = harness.state().keys.clone();
        harness.state_mut().ui.open(&keys);
    }

    fn hold(harness: &egui_kittest::Harness<'static, Host>, names: &[&'static str]) {
        let pressed: HashSet<&'static str> = names.iter().copied().collect();
        harness.state().keys.on_pressed_keys(&pressed, &|_| {});
    }

    #[test]
    fn esc_is_recorded_and_release_resumes() {
        let mut harness = harness();
        open(&mut harness);
        hold(&harness, &["esc"]);
        harness.run_steps(2);
        assert_eq!(harness.state().saved.as_deref(), Some("esc"));
        assert!(harness.state().ui.is_open());
        assert!(harness.state().keys.is_suspended());
        harness.key_press(egui::Key::Escape);
        harness.run_steps(2);
        assert!(harness.state().ui.is_open());
        assert!(harness.state().keys.is_suspended());
        hold(&harness, &[]);
        harness.run_steps(2);
        assert!(!harness.state().ui.is_open());
        assert!(!harness.state().keys.is_suspended());
        assert_eq!(harness.state().saved.as_deref(), Some("esc"));
    }

    #[test]
    fn escape_key_does_not_dismiss_cancel_does() {
        let mut harness = harness();
        open(&mut harness);
        harness.run_steps(2);
        harness.key_press(egui::Key::Escape);
        harness.run_steps(2);
        assert!(harness.state().ui.is_open());
        assert!(harness.state().keys.is_suspended());
        assert!(harness.state().saved.is_none());
        harness.get_by_label("Cancel").click();
        harness.run_steps(2);
        assert!(!harness.state().ui.is_open());
        assert!(!harness.state().keys.is_suspended());
        assert!(harness.state().saved.is_none());
    }
}
