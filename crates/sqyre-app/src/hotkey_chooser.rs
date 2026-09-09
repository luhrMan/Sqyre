//! Near-cursor menu when multiple macros share a hotkey chord.

use crate::SqyreApp;
use eframe::egui::{self, Pos2, Vec2, ViewportBuilder, ViewportId};
use sqyre_hotkeys::{HotkeyTrigger, MacroHotkeyBinding};
use std::collections::BTreeMap;

const CHOOSER_ID: &str = "sqyre_hotkey_chooser";
const MENU_OFFSET: Vec2 = Vec2::new(12.0, 12.0);
const ROW_H: f32 = 28.0;
const PAD: f32 = 8.0;
const MIN_W: f32 = 180.0;
const MAX_W: f32 = 360.0;

#[derive(Debug, Clone)]
pub(crate) struct HotkeyChooserState {
    pub names: Vec<String>,
    pub chord_label: String,
    /// Screen position in egui points, captured once when the menu opens.
    pub anchor: Pos2,
    /// True when the native X11 host owns the menu (no egui viewport).
    pub native: bool,
}

/// Group pending macro names by normalized chord + trigger.
/// Returns groups of unique macro names (order preserved within each group).
pub(crate) fn group_pending_by_chord(
    pending: &[String],
    macros: &[sqyre_domain::Macro],
) -> Vec<(String, HotkeyTrigger, Vec<String>)> {
    let by_name: BTreeMap<&str, &sqyre_domain::Macro> =
        macros.iter().map(|m| (m.name.as_str(), m)).collect();

    // Preserve first-seen chord order from pending.
    let mut order: Vec<(String, HotkeyTrigger)> = Vec::new();
    let mut groups: BTreeMap<(String, HotkeyTrigger), Vec<String>> = BTreeMap::new();

    for name in pending {
        let Some(m) = by_name.get(name.as_str()) else {
            continue;
        };
        if m.hotkey.is_empty() {
            continue;
        }
        let chord = MacroHotkeyBinding::new(
            m.name.clone(),
            m.hotkey.clone(),
            HotkeyTrigger::parse(&m.hotkey_trigger),
        )
        .chord;
        if chord.is_empty() {
            continue;
        }
        let chord_key = chord.join("+");
        let trigger = HotkeyTrigger::parse(&m.hotkey_trigger);
        let key = (chord_key.clone(), trigger);
        let entry = groups.entry(key.clone()).or_default();
        if !entry.iter().any(|n| n == name) {
            if entry.is_empty() {
                order.push((chord_key, trigger));
            }
            entry.push(name.clone());
        }
    }

    order
        .into_iter()
        .filter_map(|(chord_key, trigger)| {
            let names = groups.remove(&(chord_key.clone(), trigger))?;
            Some((chord_key, trigger, names))
        })
        .collect()
}

/// Desktop cursor in physical pixels, when available.
fn desktop_cursor_physical() -> Option<(i32, i32)> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    {
        sqyre_capture::desktop_cursor_position()
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "native-runtime")))]
    {
        None
    }
}

/// egui viewport `with_position` uses logical points (`physical / pixels_per_point`).
fn cursor_screen_points(ctx: &egui::Context) -> Pos2 {
    let ppp = ctx.pixels_per_point().max(0.01);
    if let Some((x, y)) = desktop_cursor_physical() {
        return Pos2::new(x as f32 / ppp, y as f32 / ppp) + MENU_OFFSET;
    }
    // Last resort: egui pointer is already in points, but window-local — map to
    // screen via the root viewport outer rect when known.
    let local = ctx
        .pointer_latest_pos()
        .or_else(|| ctx.input(|i| i.pointer.hover_pos()))
        .unwrap_or(Pos2::new(80.0, 80.0));
    if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
        return Pos2::new(outer.min.x + local.x, outer.min.y + local.y) + MENU_OFFSET;
    }
    local + MENU_OFFSET
}

impl SqyreApp {
    pub(crate) fn open_hotkey_chooser(
        &mut self,
        names: Vec<String>,
        chord_label: String,
        ctx: &egui::Context,
    ) {
        let row_count = names.len().max(1) as f32;
        let title_h = 22.0;
        let height = PAD * 2.0 + title_h + row_count * ROW_H + 4.0;
        let width = names
            .iter()
            .chain(std::iter::once(&chord_label))
            .map(|s| s.len())
            .max()
            .unwrap_or(12) as f32
            * 8.0
            + PAD * 2.0;
        let width = width.clamp(MIN_W, MAX_W);
        let anchor = clamp_to_monitor(ctx, cursor_screen_points(ctx), width, height);
        let ppp = ctx.pixels_per_point().max(0.01);
        let phys = desktop_cursor_physical().unwrap_or_else(|| {
            (
                (anchor.x * ppp).round() as i32,
                (anchor.y * ppp).round() as i32,
            )
        });
        let phys = (
            phys.0 + (MENU_OFFSET.x * ppp).round() as i32,
            phys.1 + (MENU_OFFSET.y * ppp).round() as i32,
        );

        #[cfg(all(
            feature = "native-runtime",
            feature = "overlay-buttons",
            target_os = "linux"
        ))]
        let native = {
            let title = if chord_label.is_empty() {
                "Choose macro".to_string()
            } else {
                format!("Hotkey: {chord_label}")
            };
            self.macro_overlay.show_hotkey_chooser(
                ctx,
                &self.pending_hotkey_macros,
                title,
                names.clone(),
                phys.0,
                phys.1,
            )
        };
        #[cfg(not(all(
            feature = "native-runtime",
            feature = "overlay-buttons",
            target_os = "linux"
        )))]
        let native = false;

        self.hotkey_chooser = Some(HotkeyChooserState {
            names,
            chord_label,
            anchor,
            native,
        });
        ctx.request_repaint();
    }

    /// Paint / poll the conflict chooser when open. Returns a macro name to start.
    pub(crate) fn paint_hotkey_chooser(&mut self, ctx: &egui::Context) -> Option<String> {
        let state = self.hotkey_chooser.clone()?;

        if state.native {
            #[cfg(all(
                feature = "native-runtime",
                feature = "overlay-buttons",
                target_os = "linux"
            ))]
            {
                let result = self.macro_overlay.take_hotkey_chooser_result();
                match result {
                    Some(sqyre_overlay::NativeHotkeyChooserResult::Picked(name)) => {
                        self.hotkey_chooser = None;
                        return Some(name);
                    }
                    Some(sqyre_overlay::NativeHotkeyChooserResult::Dismissed) => {
                        self.hotkey_chooser = None;
                        return None;
                    }
                    None => {
                        // Native host owns input; light wake so we keep polling the result.
                        ctx.request_repaint_after(std::time::Duration::from_millis(50));
                        return None;
                    }
                }
            }
            #[cfg(not(all(
                feature = "native-runtime",
                feature = "overlay-buttons",
                target_os = "linux"
            )))]
            {
                self.hotkey_chooser = None;
                return None;
            }
        }

        let row_count = state.names.len().max(1) as f32;
        let title_h = 22.0;
        let height = PAD * 2.0 + title_h + row_count * ROW_H + 4.0;
        let width = state
            .names
            .iter()
            .chain(std::iter::once(&state.chord_label))
            .map(|s| s.len())
            .max()
            .unwrap_or(12) as f32
            * 8.0
            + PAD * 2.0;
        let width = width.clamp(MIN_W, MAX_W);

        let id = ViewportId::from_hash_of(CHOOSER_ID);
        let builder = ViewportBuilder::default()
            .with_title("Choose macro")
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top()
            .with_taskbar(false)
            .with_active(true)
            .with_inner_size([width, height])
            .with_position(state.anchor);

        let names = state.names.clone();
        let chord_label = state.chord_label.clone();
        let mut picked = None;
        let mut dismiss = false;
        ctx.show_viewport_immediate(id, builder, |ctx, _class| {
            egui::CentralPanel::default()
                .frame(egui::Frame::popup(ctx.style()).inner_margin(PAD))
                .show(ctx, |ui| {
                    paint_chooser_body(ui, &chord_label, &names, &mut picked, &mut dismiss);
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                dismiss = true;
            }
        });

        if picked.is_some() || dismiss {
            self.hotkey_chooser = None;
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        picked
    }
}

fn clamp_to_monitor(ctx: &egui::Context, pos: Pos2, w: f32, h: f32) -> Pos2 {
    let Some(size) = ctx.input(|i| i.viewport().monitor_size) else {
        return pos;
    };
    Pos2::new(
        pos.x.clamp(0.0, (size.x - w).max(0.0)),
        pos.y.clamp(0.0, (size.y - h).max(0.0)),
    )
}

fn paint_chooser_body(
    ui: &mut egui::Ui,
    chord_label: &str,
    names: &[String],
    picked: &mut Option<String>,
    dismiss: &mut bool,
) {
    ui.label(
        egui::RichText::new(if chord_label.is_empty() {
            "Choose macro".to_string()
        } else {
            format!("Hotkey: {chord_label}")
        })
        .small()
        .weak(),
    );
    ui.add_space(4.0);
    for name in names {
        if ui
            .add_sized(
                [ui.available_width(), ROW_H],
                egui::Button::new(name.as_str()).wrap_mode(egui::TextWrapMode::Truncate),
            )
            .clicked()
        {
            *picked = Some(name.clone());
        }
    }
    ui.add_space(2.0);
    if ui.small_button("Cancel").clicked() {
        *dismiss = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::Macro;

    fn m(name: &str, hotkey: &[&str], trigger: &str) -> Macro {
        let mut macro_ = Macro::new(name, 0, Vec::new());
        macro_.hotkey = hotkey.iter().map(|s| (*s).to_string()).collect();
        macro_.hotkey_trigger = trigger.into();
        macro_
    }

    #[test]
    fn groups_same_chord_together() {
        let macros = vec![
            m("A", &["ctrl", "a"], "press"),
            m("B", &["ctrl", "a"], "press"),
            m("C", &["ctrl", "b"], "press"),
        ];
        let pending = vec!["A".into(), "B".into(), "C".into()];
        let groups = group_pending_by_chord(&pending, &macros);
        assert_eq!(groups.len(), 2);
        let ab = groups
            .iter()
            .find(|(_, _, names)| names.len() == 2)
            .expect("pair");
        assert_eq!(ab.2, vec!["A".to_string(), "B".to_string()]);
        let c = groups
            .iter()
            .find(|(_, _, names)| names.len() == 1)
            .unwrap();
        assert_eq!(c.2, vec!["C".to_string()]);
    }

    #[test]
    fn different_triggers_stay_separate() {
        let macros = vec![m("A", &["f1"], "press"), m("B", &["f1"], "release")];
        let pending = vec!["A".into(), "B".into()];
        let groups = group_pending_by_chord(&pending, &macros);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn dedups_repeated_pending_names() {
        let macros = vec![m("A", &["f1"], "press"), m("B", &["f1"], "press")];
        let pending = vec!["A".into(), "A".into(), "B".into()];
        let groups = group_pending_by_chord(&pending, &macros);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].2, vec!["A".to_string(), "B".to_string()]);
    }
}
