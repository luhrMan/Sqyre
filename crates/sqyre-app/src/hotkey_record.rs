//! Macro hotkey record dialog.

use crate::chord_record::{poll_waiting_release, record_modal};
use crate::recorded_action::ChordRecordTarget;
use crate::widgets::dismiss_row;
use eframe::egui;
use sqyre_hotkeys::{format_hotkey, MacroHotkeyBridge};
use web_time::{Duration, Instant};

const STABLE_FOR: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Default)]
pub(crate) enum HotkeyRecordUi {
    #[default]
    Closed,
    /// Collecting a stable chord.
    Recording {
        last_chord: Vec<String>,
        stable_since: Option<Instant>,
        target: ChordRecordTarget,
    },
    /// Saved; wait for release before resuming macro hotkeys.
    WaitingRelease { chord: Vec<String> },
}

impl HotkeyRecordUi {
    /// Record for Pause / Navigate Key drafts, or the selected macro hotkey.
    pub fn open(&mut self, macro_hotkeys: &MacroHotkeyBridge) {
        self.open_for(macro_hotkeys, ChordRecordTarget::Inferred);
    }

    /// Record for a specific destination (e.g. a Navigate Select chord slot).
    pub fn open_for(&mut self, macro_hotkeys: &MacroHotkeyBridge, target: ChordRecordTarget) {
        if !matches!(self, Self::Closed) {
            return;
        }
        macro_hotkeys.suspend();
        *self = Self::Recording {
            last_chord: Vec::new(),
            stable_since: None,
            target,
        };
    }

    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Closed)
    }

    /// Draw modal; returns recorded chord + destination once when stable duration elapses.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        macro_hotkeys: &MacroHotkeyBridge,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
    ) -> Option<(Vec<String>, ChordRecordTarget)> {
        match self {
            Self::Closed => None,
            Self::WaitingRelease { chord, .. } => {
                if poll_waiting_release(
                    ctx,
                    macro_hotkeys,
                    chord,
                    "Record hotkey",
                    "Release the hotkey to finish…",
                    pending_scale,
                ) {
                    *self = Self::Closed;
                }
                None
            }
            Self::Recording {
                last_chord,
                stable_since,
                target,
            } => {
                let pressed = macro_hotkeys.pressed_keys();
                if pressed != *last_chord {
                    *last_chord = pressed.clone();
                    *stable_since = if pressed.is_empty() {
                        None
                    } else {
                        Some(Instant::now())
                    };
                }

                let progress = if let Some(since) = *stable_since {
                    (since.elapsed().as_secs_f32() / STABLE_FOR.as_secs_f32()).min(1.0)
                } else {
                    0.0
                };

                let ready = stable_since
                    .map(|s| s.elapsed() >= STABLE_FOR && !last_chord.is_empty())
                    .unwrap_or(false);

                // Esc dismisses only while no keys are held (Esc may join the chord).
                let esc_dismisses = last_chord.is_empty();
                let mut cancel = false;
                record_modal(ctx, "Record hotkey", pending_scale, |ui| {
                    ui.label(
                        "Hold your hotkey. When it stays unchanged for 1 second, it will be saved.\nUse Cancel to dismiss (Esc when no keys are held).",
                    );
                    crate::widgets::section_separator(ui);
                    let display = if last_chord.is_empty() {
                        "(no keys)".to_string()
                    } else {
                        format_hotkey(last_chord)
                    };
                    ui.monospace(display);
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .desired_width(280.0)
                            .show_percentage(),
                    );
                    cancel = dismiss_row(ui, esc_dismisses);
                });

                if cancel {
                    macro_hotkeys.resume();
                    *self = Self::Closed;
                    return None;
                }

                if ready {
                    let keys = last_chord.clone();
                    let target = *target;
                    *self = Self::WaitingRelease {
                        chord: keys.clone(),
                    };
                    ctx.request_repaint();
                    return Some((keys, target));
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
        ui: HotkeyRecordUi,
        keys: MacroHotkeyBridge,
    }

    fn harness() -> egui_kittest::Harness<'static, Host> {
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(640.0, 480.0))
            .build_ui_state(
                |ui, host: &mut Host| {
                    let _ = host.ui.show(ui.ctx(), &host.keys, None);
                },
                Host {
                    ui: HotkeyRecordUi::default(),
                    keys: MacroHotkeyBridge::new(),
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
    fn esc_with_empty_chord_dismisses_and_resumes() {
        let mut harness = harness();
        open(&mut harness);
        harness.run_steps(2);
        assert!(harness.state().keys.is_suspended());
        harness.key_press(egui::Key::Escape);
        harness.run_steps(2);
        assert!(!harness.state().ui.is_open());
        assert!(!harness.state().keys.is_suspended());
    }

    #[test]
    fn esc_held_joins_chord_without_dismiss() {
        let mut harness = harness();
        open(&mut harness);
        hold(&harness, &["esc"]);
        harness.run_steps(2);
        harness.key_press(egui::Key::Escape);
        harness.run_steps(2);
        assert!(harness.state().ui.is_open());
        assert!(harness.state().keys.is_suspended());
        harness.get_by_label("esc");
    }

    #[test]
    fn cancel_while_chord_held_dismisses_and_resumes() {
        let mut harness = harness();
        open(&mut harness);
        hold(&harness, &["ctrl", "a"]);
        harness.run_steps(2);
        harness.get_by_label("Cancel").click();
        harness.run_steps(2);
        assert!(!harness.state().ui.is_open());
        assert!(!harness.state().keys.is_suspended());
    }

    #[test]
    fn waiting_release_stays_suspended_until_release() {
        let mut harness = harness();
        open(&mut harness);
        harness.state_mut().ui = HotkeyRecordUi::WaitingRelease {
            chord: vec!["a".into()],
        };
        hold(&harness, &["a"]);
        harness.run_steps(2);
        assert!(harness.state().ui.is_open());
        assert!(harness.state().keys.is_suspended());
        hold(&harness, &[]);
        harness.run_steps(2);
        assert!(!harness.state().ui.is_open());
        assert!(!harness.state().keys.is_suspended());
    }
}
