//! Macro hotkey record dialog.

use crate::chord_record::{poll_waiting_release, record_modal};
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
    },
    /// Saved; wait for release before resuming macro hotkeys.
    WaitingRelease { chord: Vec<String> },
}

impl HotkeyRecordUi {
    pub fn open(&mut self, macro_hotkeys: &MacroHotkeyBridge) {
        if !matches!(self, Self::Closed) {
            return;
        }
        macro_hotkeys.suspend();
        *self = Self::Recording {
            last_chord: Vec::new(),
            stable_since: None,
        };
    }

    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Closed)
    }

    /// Draw modal; returns recorded chord once when stable duration elapses.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        macro_hotkeys: &MacroHotkeyBridge,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
    ) -> Option<Vec<String>> {
        match self {
            Self::Closed => None,
            Self::WaitingRelease { chord } => {
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
                    ui.separator();
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
                    *self = Self::WaitingRelease {
                        chord: keys.clone(),
                    };
                    ctx.request_repaint();
                    return Some(keys);
                }

                ctx.request_repaint_after(Duration::from_millis(16));
                None
            }
        }
    }
}
