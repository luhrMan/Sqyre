//! Nested variable-reference chips and unfocused entry overlays.

use eframe::egui::{
    self, text::CCursor, text_edit::TextEditState, Color32, FontId, Key, Modifiers,
    PopupCloseBehavior, RectAlign, Sense, Stroke, Vec2,
};
use egui::text_selection::CCursorRange;
use sqyre_domain::{action_pastel_color, is_known_variable, nested_var_ref_color, SummaryPill};
use sqyre_validate::EntryValidation;
use sqyre_varref;
use std::collections::HashSet;
use std::sync::Arc;

use crate::tree_chrome::{contrast_fg, rgba_pub};

const VAR_AC_LIMIT: usize = 12;

const PILL_FONT_SIZE: f32 = 12.0;
const NESTED_MARGIN_X: i8 = 3;
const NESTED_MARGIN_Y: i8 = 1;
const NESTED_RADIUS: f32 = 4.0;
/// Outer display padding (4×2).
const OUTER_MARGIN_X: i8 = 4;
const OUTER_MARGIN_Y: i8 = 2;
const OUTER_RADIUS: f32 = 5.0;

fn nested_fill(unknown: bool, is_dark: bool) -> Color32 {
    if unknown {
        rgba_pub(action_pastel_color("warning", is_dark))
    } else {
        rgba_pub(nested_var_ref_color(is_dark))
    }
}

/// Place galley so its ink (mesh bounds) is centered in `rect`.
fn paint_galley_centered(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    galley: Arc<egui::Galley>,
    fallback: Color32,
) {
    let pos = if galley.mesh_bounds.is_positive() {
        rect.center() - galley.mesh_bounds.center().to_vec2()
    } else {
        egui::Align2::CENTER_CENTER
            .anchor_size(rect.center(), galley.size())
            .min
    };
    ui.painter().galley(pos, galley, fallback);
}

fn paint_text_chip(
    ui: &mut egui::Ui,
    text: &str,
    fill: Color32,
    radius: f32,
    margin_x: i8,
    margin_y: i8,
    id_salt: impl std::hash::Hash,
) -> egui::Response {
    // allocate_exact_size (not Frame::show / Label) so text can be centered in the chrome.
    let fg = contrast_fg(fill);
    let font = FontId::proportional(PILL_FONT_SIZE);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, fg);
    let pad = Vec2::new(margin_x as f32 * 2.0, margin_y as f32 * 2.0);
    let size = galley.size() + pad;
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect(rect, radius, fill, Stroke::NONE, egui::StrokeKind::Inside);
    paint_galley_centered(ui, rect, galley, fg);
    ui.interact(rect, ui.id().with(id_salt), Sense::hover())
}

/// Plain text segment (no chrome): sized to the galley and ink-centered so it
/// aligns with nested chips under parent `Align::Center`.
fn paint_plain_segment(ui: &mut egui::Ui, text: &str, color: Color32) {
    let font = FontId::proportional(PILL_FONT_SIZE);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    paint_galley_centered(ui, rect, galley, color);
}

/// Compact nested chip for a variable name.
pub fn paint_nested_var_chip(
    ui: &mut egui::Ui,
    name: &str,
    known: &HashSet<String>,
    is_dark: bool,
) -> egui::Response {
    let unknown = !is_known_variable(known, name);
    let fill = nested_fill(unknown, is_dark);
    paint_text_chip(
        ui,
        name.trim(),
        fill,
        NESTED_RADIUS,
        NESTED_MARGIN_X,
        NESTED_MARGIN_Y,
        ("nested_var_chip", name),
    )
}

/// Plain + nested-chip segments for a value that may contain `${}` / `{}`.
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
                paint_plain_segment(ui, &seg.text, plain_fg);
            }
        }
    });
}

fn measure_content_size(
    ui: &mut egui::Ui,
    mut add_contents: impl FnMut(&mut egui::Ui),
) -> Vec2 {
    let mut measure = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("var_pill_measure")
            .sizing_pass()
            .invisible()
            .max_rect(egui::Rect::from_min_size(
                ui.next_widget_position(),
                Vec2::new(ui.available_width().max(64.0), 1000.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    add_contents(&mut measure);
    measure.min_size()
}

fn outer_frame(
    ui: &mut egui::Ui,
    fill: Color32,
    mut add_contents: impl FnMut(&mut egui::Ui),
) -> egui::Response {
    // Measure content, then allocate chrome via allocate_exact_size (Frame::show
    // top-aligns and ignores parent Align::Center).
    let content_size = measure_content_size(ui, &mut add_contents);
    let margin = Vec2::new(OUTER_MARGIN_X as f32, OUTER_MARGIN_Y as f32);
    let size = content_size + margin * 2.0;
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if fill.a() > 0 {
        ui.painter().rect(
            rect,
            OUTER_RADIUS,
            fill,
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    }
    // Center the content band in the chrome (symmetric padding).
    let inner = egui::Rect::from_center_size(rect.center(), content_size);
    let mut content = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("var_pill_content")
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    add_contents(&mut content);
    ui.interact(rect, ui.id().with("outer_var_pill"), Sense::hover())
}

/// Outer action pastel pill wrapping segmented var-ref content.
pub fn paint_value_pill(
    ui: &mut egui::Ui,
    text: &str,
    action_type: &str,
    known: &HashSet<String>,
    is_dark: bool,
) -> egui::Response {
    let fill = rgba_pub(action_pastel_color(action_type, is_dark));
    // Plain text: single centered chip (avoids Label top-bias inside a composite frame).
    if !sqyre_varref::contains(text) {
        return paint_text_chip(
            ui,
            text,
            fill,
            OUTER_RADIUS,
            OUTER_MARGIN_X,
            OUTER_MARGIN_Y,
            ("value_pill", text),
        );
    }
    let plain_fg = contrast_fg(fill);
    outer_frame(ui, fill, |ui| {
        paint_var_ref_content(ui, text, known, is_dark, plain_fg);
    })
}

/// Labeled outer pill whose value is a variable-name chip.
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
    let fg = contrast_fg(fill);
    outer_frame(ui, fill, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
            if !label.is_empty() {
                paint_plain_segment(ui, &format!("{label}: "), fg);
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

/// Incomplete `${prefix` at the caret (no closing `}` yet).
#[derive(Debug, Clone, PartialEq, Eq)]
struct IncompleteDollarRef {
    /// Character index of `$` in `${`.
    start_char: usize,
    /// Text after `${` up to the caret.
    prefix: String,
}

/// Find an open `${…` span ending at `cursor_char` (character index).
fn find_incomplete_dollar_ref(text: &str, cursor_char: usize) -> Option<IncompleteDollarRef> {
    let before: String = text.chars().take(cursor_char).collect();
    let start_byte = before.rfind("${")?;
    let after = &before[start_byte + 2..];
    if after.contains('}') {
        return None;
    }
    // Abort if the prefix looks like it left the name (whitespace / newline).
    if after.chars().any(|c| c.is_whitespace()) {
        return None;
    }
    let start_char = before[..start_byte].chars().count();
    Some(IncompleteDollarRef {
        start_char,
        prefix: after.to_string(),
    })
}

fn byte_index_from_char_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Filter known names by prefix (case-insensitive); empty prefix → all (up to limit).
fn var_completion_options(prefix: &str, known: &HashSet<String>, limit: usize) -> Vec<String> {
    let p = prefix.to_ascii_lowercase();
    let mut names: Vec<String> = known.iter().cloned().collect();
    names.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    names
        .into_iter()
        .filter(|n| p.is_empty() || n.to_ascii_lowercase().starts_with(&p))
        .take(limit)
        .collect()
}

fn apply_var_completion(
    value: &mut String,
    start_char: usize,
    cursor_char: usize,
    name: &str,
) -> usize {
    let start_byte = byte_index_from_char_index(value, start_char);
    let end_byte = byte_index_from_char_index(value, cursor_char);
    let insertion = format!("${{{name}}}");
    value.replace_range(start_byte..end_byte, &insertion);
    start_char + insertion.chars().count()
}

#[derive(Clone, Default)]
struct VarAutocompleteNav {
    selected: usize,
    prefix: String,
}

/// Capture nav keys (and consume them) when autocomplete was open last frame.
fn take_var_ac_keys(ui: &mut egui::Ui, was_open: bool) -> (bool, bool, bool, bool) {
    if !was_open {
        return (false, false, false, false);
    }
    let down = ui.input(|i| i.key_pressed(Key::ArrowDown));
    let up = ui.input(|i| i.key_pressed(Key::ArrowUp));
    let accept = ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Tab));
    let dismiss = ui.input(|i| i.key_pressed(Key::Escape));
    for key in [
        Key::ArrowDown,
        Key::ArrowUp,
        Key::Enter,
        Key::Tab,
        Key::Escape,
    ] {
        let _ = ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        let _ = ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, key));
    }
    (down, up, accept, dismiss)
}

/// TextEdit + `${` autocomplete popup.
fn var_ref_text_edit(
    ui: &mut egui::Ui,
    id: egui::Id,
    value: &mut String,
    known: &HashSet<String>,
    desired_width: f32,
    multiline: Option<usize>,
) {
    let ac_id = id.with("var_ac");
    let was_open = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(ac_id.with("open")))
        .unwrap_or(false);
    let (down, up, accept, dismiss) = take_var_ac_keys(ui, was_open);

    let output = if let Some(rows) = multiline {
        egui::TextEdit::multiline(value)
            .id(id)
            .desired_width(desired_width)
            .desired_rows(rows)
            .show(ui)
    } else {
        egui::TextEdit::singleline(value)
            .id(id)
            .desired_width(desired_width)
            .show(ui)
    };

    let cursor_char = output.cursor_range.map(|r| r.primary.index);
    show_var_ref_autocomplete(
        ui,
        id,
        &output.response,
        value,
        cursor_char,
        known,
        down,
        up,
        accept,
        dismiss,
    );
}

fn show_var_ref_autocomplete(
    ui: &mut egui::Ui,
    edit_id: egui::Id,
    response: &egui::Response,
    value: &mut String,
    cursor_char: Option<usize>,
    known: &HashSet<String>,
    down: bool,
    up: bool,
    accept: bool,
    dismiss: bool,
) {
    let ac_id = edit_id.with("var_ac");
    let Some(cursor_char) = cursor_char else {
        ui.ctx()
            .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
        return;
    };
    let Some(incomplete) = find_incomplete_dollar_ref(value, cursor_char) else {
        ui.ctx()
            .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
        return;
    };
    let suggestions = var_completion_options(&incomplete.prefix, known, VAR_AC_LIMIT);
    if suggestions.is_empty() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
        return;
    }

    let mut nav = ui
        .ctx()
        .data(|d| d.get_temp::<VarAutocompleteNav>(ac_id))
        .unwrap_or_default();
    if nav.prefix != incomplete.prefix {
        nav.prefix = incomplete.prefix.clone();
        nav.selected = 0;
    }
    if nav.selected >= suggestions.len() {
        nav.selected = suggestions.len().saturating_sub(1);
    }

    if down {
        nav.selected = (nav.selected + 1) % suggestions.len();
    }
    if up {
        nav.selected = if nav.selected == 0 {
            suggestions.len() - 1
        } else {
            nav.selected - 1
        };
    }

    let mut chosen: Option<String> = None;
    if accept {
        chosen = suggestions.get(nav.selected).cloned();
    }
    if dismiss {
        ui.ctx().data_mut(|d| {
            d.insert_temp(ac_id.with("open"), false);
            d.insert_temp(ac_id, VarAutocompleteNav::default());
        });
        return;
    }

    ui.ctx()
        .data_mut(|d| d.insert_temp(ac_id.with("open"), true));

    let popup_width = response.rect.width().max(160.0);
    egui::Popup::from_response(response)
        .id(ac_id.with("popup"))
        .open(true)
        .align(RectAlign::BOTTOM_START)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .width(popup_width)
        .show(|ui| {
            ui.set_min_width(popup_width);
            ui.set_max_height(180.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, name) in suggestions.iter().enumerate() {
                    let selected = i == nav.selected;
                    let label = format!("${{{name}}}");
                    let resp =
                        ui.selectable_label(selected, egui::RichText::new(label).monospace());
                    if resp.clicked() {
                        chosen = Some(name.clone());
                    }
                    if selected {
                        resp.scroll_to_me(None);
                    }
                }
            });
        });

    if let Some(name) = chosen {
        let new_cursor = apply_var_completion(value, incomplete.start_char, cursor_char, &name);
        if let Some(mut state) = TextEditState::load(ui.ctx(), edit_id) {
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(new_cursor))));
            state.store(ui.ctx(), edit_id);
        }
        ui.memory_mut(|m| m.request_focus(edit_id));
        ui.ctx().data_mut(|d| {
            d.insert_temp(ac_id.with("open"), false);
            d.insert_temp(ac_id, VarAutocompleteNav::default());
        });
    } else {
        ui.ctx().data_mut(|d| d.insert_temp(ac_id, nav));
    }
}

/// Variable-name field that becomes a nested chip when unfocused.
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

/// Trailing error/warning icon. Errors take priority.
pub fn paint_entry_validation_icon(ui: &mut egui::Ui, v: &EntryValidation) {
    let (glyph, color, tip) = if !v.error.is_empty() {
        ("✕", Color32::from_rgb(220, 70, 70), v.error.as_str())
    } else if !v.warning.is_empty() {
        ("⚠", Color32::from_rgb(220, 170, 40), v.warning.as_str())
    } else {
        return;
    };
    ui.add(
        egui::Label::new(egui::RichText::new(glyph).color(color).size(14.0))
            .sense(Sense::hover()),
    )
    .on_hover_text(tip);
}

/// Stroke color for compact validated chips (error > warning > none).
pub fn entry_validation_stroke(v: &EntryValidation) -> Option<Stroke> {
    if !v.error.is_empty() {
        Some(Stroke::new(1.5, Color32::from_rgb(220, 70, 70)))
    } else if !v.warning.is_empty() {
        Some(Stroke::new(1.5, Color32::from_rgb(220, 170, 40)))
    } else {
        None
    }
}

/// Hover tip for a validated entry (error preferred).
pub fn entry_validation_tip(v: &EntryValidation) -> Option<&str> {
    if !v.error.is_empty() {
        Some(v.error.as_str())
    } else if !v.warning.is_empty() {
        Some(v.warning.as_str())
    } else {
        None
    }
}

/// Labeled var-ref field with live validation icon.
pub fn validated_var_ref_edit(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    known: &HashSet<String>,
    is_dark: bool,
    desired_width: f32,
    validation: &EntryValidation,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let id = ui.id().with(("validated_var_ref", label));
        let focused = ui.memory(|m| m.has_focus(id));
        if should_show_var_ref_overlay(value, focused) {
            let plain_fg = ui.visuals().text_color();
            let resp = outer_frame(ui, Color32::TRANSPARENT, |ui| {
                paint_var_ref_content(ui, value, known, is_dark, plain_fg);
            });
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
            var_ref_text_edit(ui, id, value, known, desired_width, None);
        }
        paint_entry_validation_icon(ui, validation);
    });
}

/// Multiline var-ref field with live validation icon.
pub fn validated_var_ref_multiline_edit(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    known: &HashSet<String>,
    is_dark: bool,
    desired_width: f32,
    rows: usize,
    validation: &EntryValidation,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        paint_entry_validation_icon(ui, validation);
    });
    let id = ui.id().with(("validated_var_ref_multi", label));
    let focused = ui.memory(|m| m.has_focus(id));
    if should_show_var_ref_overlay(value, focused) {
        let plain_fg = ui.visuals().text_color();
        let resp = outer_frame(ui, Color32::TRANSPARENT, |ui| {
            ui.set_max_width(desired_width);
            for line in value.split('\n') {
                paint_var_ref_content(ui, line, known, is_dark, plain_fg);
            }
        });
        if resp.clicked() || ui.interact(resp.rect, id.with("overlay_hit"), Sense::click()).clicked()
        {
            ui.memory_mut(|m| m.request_focus(id));
        }
    } else {
        var_ref_text_edit(ui, id, value, known, desired_width, Some(rows));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::known_variable_set;

    #[test]
    fn var_ref_overlay_only_when_unfocused_with_ref() {
        assert!(should_show_var_ref_overlay("${x}", false));
        assert!(!should_show_var_ref_overlay("${x}", true));
        assert!(!should_show_var_ref_overlay("plain", false));
        assert!(!should_show_var_ref_overlay("", false));
        assert!(should_show_var_ref_overlay("1+${count}", false));
    }

    #[test]
    fn var_name_overlay_when_nonempty_unfocused() {
        assert!(should_show_var_name_overlay("count", false));
        assert!(!should_show_var_name_overlay("count", true));
        assert!(!should_show_var_name_overlay("  ", false));
        assert!(!should_show_var_name_overlay("", false));
    }

    #[test]
    fn incomplete_dollar_ref_at_caret() {
        let r = find_incomplete_dollar_ref("${", 2).unwrap();
        assert_eq!(r.start_char, 0);
        assert_eq!(r.prefix, "");

        let r = find_incomplete_dollar_ref("x=${co", 6).unwrap();
        assert_eq!(r.start_char, 2);
        assert_eq!(r.prefix, "co");

        assert!(find_incomplete_dollar_ref("${count}", 8).is_none());
        assert!(find_incomplete_dollar_ref("plain", 5).is_none());
        assert!(find_incomplete_dollar_ref("${a b", 5).is_none());
    }

    #[test]
    fn completion_filters_and_applies() {
        let known: HashSet<String> = ["Count", "Cols", "other"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let opts = var_completion_options("c", &known, 10);
        assert_eq!(opts, vec!["Cols".to_string(), "Count".to_string()]);

        let mut text = "1+${co".to_string();
        let cursor = apply_var_completion(&mut text, 2, 6, "Count");
        assert_eq!(text, "1+${Count}");
        assert_eq!(cursor, 10);
    }

    #[test]
    fn paint_smoke_with_nested_refs() {
        let ctx = egui::Context::default();
        let known = known_variable_set(["count"]);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            paint_value_pill(ui, "1+${count}", "setvariable", &known, false);
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
