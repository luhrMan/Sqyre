//! Nested variable-reference chips and unfocused entry overlays (Go VarEntry / VarNameEntry).

use eframe::egui::{self, Color32, Sense, Vec2};
use sqyre_domain::{action_pastel_color, is_known_variable, nested_var_ref_color, SummaryPill};
use sqyre_varref;
use std::collections::HashSet;

use crate::tree_chrome::{contrast_fg, rgba_pub};

const PILL_FONT_SIZE: f32 = 12.0;
const NESTED_MARGIN_X: i8 = 3;
const NESTED_MARGIN_Y: i8 = 0;
const NESTED_RADIUS: f32 = 4.0;
const OUTER_MARGIN_X: i8 = 3;
const OUTER_MARGIN_Y: i8 = 0;
const OUTER_RADIUS: f32 = 5.0;

fn nested_fill(unknown: bool, is_dark: bool) -> Color32 {
    if unknown {
        rgba_pub(action_pastel_color("warning", is_dark))
    } else {
        rgba_pub(nested_var_ref_color(is_dark))
    }
}

/// Compact nested chip for a variable name (Go `NewNestedVariableRefPill`).
pub fn paint_nested_var_chip(
    ui: &mut egui::Ui,
    name: &str,
    known: &HashSet<String>,
    is_dark: bool,
) -> egui::Response {
    let unknown = !is_known_variable(known, name);
    let fill = nested_fill(unknown, is_dark);
    let fg = contrast_fg(fill);
    let inner = egui::Frame::new()
        .fill(fill)
        .corner_radius(NESTED_RADIUS)
        .inner_margin(egui::Margin::symmetric(NESTED_MARGIN_X, NESTED_MARGIN_Y))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(name.trim())
                    .size(PILL_FONT_SIZE)
                    .color(fg),
            );
        });
    ui.interact(
        inner.response.rect,
        ui.id().with(("nested_var_chip", name)),
        Sense::hover(),
    )
}

/// Plain + nested-chip segments for a value that may contain `${}` / `{}` (Go `BuildVarRefPillContent`).
///
/// `plain_fg` colors non-ref text at full opacity (same as surrounding labels); variable
/// refs are distinguished only by their nested pill background.
pub fn paint_var_ref_content(
    ui: &mut egui::Ui,
    text: &str,
    known: &HashSet<String>,
    is_dark: bool,
    plain_fg: Color32,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
        for seg in sqyre_varref::segments(text) {
            if seg.is_ref {
                paint_nested_var_chip(ui, &seg.name, known, is_dark);
            } else if !seg.text.is_empty() {
                ui.label(
                    egui::RichText::new(seg.text)
                        .size(PILL_FONT_SIZE)
                        .color(plain_fg),
                );
            }
        }
    });
}

fn outer_frame(ui: &mut egui::Ui, fill: Color32, add_contents: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let inner = egui::Frame::new()
        .fill(fill)
        .corner_radius(OUTER_RADIUS)
        .inner_margin(egui::Margin::symmetric(OUTER_MARGIN_X, OUTER_MARGIN_Y))
        .show(ui, add_contents);
    ui.interact(
        inner.response.rect,
        ui.id().with("outer_var_pill"),
        Sense::hover(),
    )
}

/// Outer action pastel pill wrapping segmented var-ref content (Go `NewDisplayValuePill`).
pub fn paint_value_pill(
    ui: &mut egui::Ui,
    text: &str,
    action_type: &str,
    known: &HashSet<String>,
    is_dark: bool,
) -> egui::Response {
    let fill = rgba_pub(action_pastel_color(action_type, is_dark));
    let plain_fg = contrast_fg(fill);
    outer_frame(ui, fill, |ui| {
        paint_var_ref_content(ui, text, known, is_dark, plain_fg);
    })
}

/// Labeled outer pill whose value is a variable-name chip (Go `NewDisplayVariablePill`).
pub fn paint_variable_name_pill(
    ui: &mut egui::Ui,
    label: &str,
    var_name: &str,
    action_type: &str,
    known: &HashSet<String>,
    is_dark: bool,
) -> egui::Response {
    let fill = rgba_pub(action_pastel_color(action_type, is_dark));
    let name = var_name.trim();
    outer_frame(ui, fill, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
            if !label.is_empty() {
                ui.label(
                    egui::RichText::new(format!("{label}: "))
                        .size(PILL_FONT_SIZE)
                        .color(contrast_fg(fill)),
                );
            }
            if !name.is_empty() {
                paint_nested_var_chip(ui, name, known, is_dark);
            }
        });
    })
}

/// Tree-row summary pill: binding names get name chips; values get `${}` segmentation.
pub fn paint_summary_pill(
    ui: &mut egui::Ui,
    action_type: &str,
    pill: &SummaryPill,
    known: &HashSet<String>,
    is_dark: bool,
) -> egui::Response {
    match &pill.prefix {
        Some(label) => paint_variable_name_pill(ui, label, &pill.text, action_type, known, is_dark),
        None => paint_value_pill(ui, &pill.text, action_type, known, is_dark),
    }
}

fn should_show_var_ref_overlay(text: &str, focused: bool) -> bool {
    !focused && !text.is_empty() && sqyre_varref::contains(text)
}

fn should_show_var_name_overlay(text: &str, focused: bool) -> bool {
    !focused && !text.trim().is_empty()
}

/// Text field that shows nested `${}` chips when unfocused (Go BorderlessEntry / VarEntry).
pub fn var_ref_text_edit(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    known: &HashSet<String>,
    is_dark: bool,
    desired_width: f32,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let id = ui.id().with(("var_ref_edit", label));
        let focused = ui.memory(|m| m.has_focus(id));
        if should_show_var_ref_overlay(value, focused) {
            let plain_fg = ui.visuals().text_color();
            let resp = outer_frame(ui, Color32::TRANSPARENT, |ui| {
                paint_var_ref_content(ui, value, known, is_dark, plain_fg);
            });
            // Expand hit target for short values.
            let resp = ui.interact(
                resp.rect.expand2(Vec2::new(
                    (desired_width - resp.rect.width()).max(0.0),
                    2.0,
                )),
                id.with("overlay_hit"),
                Sense::click(),
            );
            if resp.clicked() {
                ui.memory_mut(|m| m.request_focus(id));
            }
        } else {
            ui.add(
                egui::TextEdit::singleline(value)
                    .id(id)
                    .desired_width(desired_width),
            );
        }
    });
}

/// Variable-name field that becomes a nested chip when unfocused (Go BorderlessVarNameEntry).
pub fn var_name_text_edit(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    known: &HashSet<String>,
    is_dark: bool,
    desired_width: f32,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let id = ui.id().with(("var_name_edit", label));
        let focused = ui.memory(|m| m.has_focus(id));
        if should_show_var_name_overlay(value, focused) {
            let resp = paint_nested_var_chip(ui, value, known, is_dark);
            let resp = ui.interact(
                resp.rect.expand2(Vec2::new(
                    (desired_width - resp.rect.width()).max(0.0),
                    2.0,
                )),
                id.with("overlay_hit"),
                Sense::click(),
            );
            if resp.clicked() {
                ui.memory_mut(|m| m.request_focus(id));
            }
        } else {
            ui.add(
                egui::TextEdit::singleline(value)
                    .id(id)
                    .desired_width(desired_width),
            );
        }
    });
}

/// Multiline / YAML value with unfocused `${}` overlay when applicable.
pub fn var_ref_multiline_edit(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    known: &HashSet<String>,
    is_dark: bool,
    desired_width: f32,
    rows: usize,
) {
    ui.label(label);
    let id = ui.id().with(("var_ref_multi", label));
    let focused = ui.memory(|m| m.has_focus(id));
    if should_show_var_ref_overlay(value, focused) {
        let plain_fg = ui.visuals().text_color();
        let resp = outer_frame(ui, Color32::TRANSPARENT, |ui| {
            ui.set_max_width(desired_width);
            // One line of chips per text line (Go multiLine overlay).
            for line in value.split('\n') {
                paint_var_ref_content(ui, line, known, is_dark, plain_fg);
            }
        });
        if resp.clicked() || ui.interact(resp.rect, id.with("overlay_hit"), Sense::click()).clicked()
        {
            ui.memory_mut(|m| m.request_focus(id));
        }
    } else {
        ui.add(
            egui::TextEdit::multiline(value)
                .id(id)
                .desired_width(desired_width)
                .desired_rows(rows),
        );
    }
}

/// Whether the overlay would show for a value field (tests / helpers).
pub fn shows_var_ref_overlay(text: &str, focused: bool) -> bool {
    should_show_var_ref_overlay(text, focused)
}

/// Whether the overlay would show for a name field.
pub fn shows_var_name_overlay(text: &str, focused: bool) -> bool {
    should_show_var_name_overlay(text, focused)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::known_variable_set;

    #[test]
    fn var_ref_overlay_only_when_unfocused_with_ref() {
        assert!(shows_var_ref_overlay("${x}", false));
        assert!(!shows_var_ref_overlay("${x}", true));
        assert!(!shows_var_ref_overlay("plain", false));
        assert!(!shows_var_ref_overlay("", false));
        assert!(shows_var_ref_overlay("1+${count}", false));
    }

    #[test]
    fn var_name_overlay_when_nonempty_unfocused() {
        assert!(shows_var_name_overlay("count", false));
        assert!(!shows_var_name_overlay("count", true));
        assert!(!shows_var_name_overlay("  ", false));
        assert!(!shows_var_name_overlay("", false));
    }

    #[test]
    fn paint_smoke_with_nested_refs() {
        let ctx = egui::Context::default();
        let known = known_variable_set(["count"]);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            paint_value_pill(ui, "1+${count}", "calculate", &known, false);
            paint_variable_name_pill(ui, "Variable", "count", "setvariable", &known, false);
            paint_nested_var_chip(ui, "missing", &known, false);
            let pill = SummaryPill {
                text: "${step}".into(),
                prefix: None,
            };
            paint_summary_pill(ui, "setvariable", &pill, &known, true);
        });
    }
}
