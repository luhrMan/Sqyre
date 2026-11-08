//! Field that is either a numeric stepper or a variable text entry.
//!
//! Right-click switches. A value that is not a plain number stays on the
//! variable entry until the user picks Number, which replaces it with a literal.

use crate::help_text as ht;
use crate::paint_ctx::VarTheme;
use crate::var_pills::{self, VarFieldOpts};
use eframe::egui;
use sqyre_validate::EntryValidation;
use std::ops::RangeInclusive;

/// Which editor the field is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumOrVarChoice {
    Number,
    Variable,
}

/// Presentation of one number-or-variable field, separate from its value.
pub struct NumOrVarOpts<'a> {
    /// Finite width, or [`f32::INFINITY`] to fill the remaining row.
    pub width: f32,
    /// Clamp for the numeric stepper. `None` leaves it unbounded.
    pub range: Option<RangeInclusive<f64>>,
    pub theme: VarTheme<'a>,
    pub validation: &'a EntryValidation,
    /// Field description. The switch hint is appended for the hover tip.
    pub help: &'a str,
}

/// Labeled number stepper or variable text entry. Right-click switches.
pub fn num_or_var_field(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    value: &mut String,
    opts: NumOrVarOpts<'_>,
) {
    let NumOrVarOpts {
        width,
        range,
        theme,
        validation,
        help,
    } = opts;
    let help = num_or_var_hover(help);
    super::pair_labeled(ui, icon, label, |ui| {
        let mode_id = ui.id().with(("num_or_var_mode", label));
        let edit_id = ui.id().with(("validated_var_ref", label));
        let field_w = var_pills::resolve_edit_width(
            ui,
            width,
            var_pills::validation_icon_reserve(ui, validation),
        );
        let use_var = variable_entry(ui, mode_id, value);
        let resp = if use_var {
            let resp = var_pills::paint_validated_var_ref(
                ui,
                label,
                value,
                theme,
                VarFieldOpts {
                    desired_width: field_w,
                    validation,
                    icon: "",
                    help: &help,
                },
            );
            // Focus the editor that exists this frame. Requesting it on the
            // click frame points at a widget that has not been built yet.
            if take_var_focus(ui, mode_id) {
                if resp.id == edit_id {
                    resp.request_focus();
                } else {
                    ui.memory_mut(|m| m.request_focus(edit_id));
                }
            }
            resp
        } else {
            paint_number(ui, value, field_w, range.as_ref(), validation, &help)
        };
        if let Some(choice) = super::response_context_menu(ui, &resp, |ui| {
            num_or_var_menu(ui, use_var, non_literal(value))
        })
        .flatten()
        {
            apply_num_or_var_choice(ui, mode_id, value, range.as_ref(), choice);
        }
    });
}

/// Hover text plus the shared right-click hint.
pub(crate) fn num_or_var_hover(help: &str) -> String {
    if help.is_empty() {
        ht::NUM_OR_VAR_SWITCH.to_owned()
    } else {
        format!("{help}\n\n{}", ht::NUM_OR_VAR_SWITCH)
    }
}

/// True when `value` is non-empty and not a finite numeric literal.
pub(crate) fn non_literal(value: &str) -> bool {
    let t = value.trim();
    !t.is_empty() && parse_literal(t).is_none()
}

/// Variable text entry: the user asked for it, or the value is not a literal.
pub(crate) fn variable_entry(ui: &egui::Ui, mode_id: egui::Id, value: &str) -> bool {
    non_literal(value) || ui.data(|d| d.get_temp::<bool>(mode_id)).unwrap_or(false)
}

/// Number / Variable menu. `variable_mode` marks the entry that is showing.
pub(crate) fn num_or_var_menu(
    ui: &mut egui::Ui,
    variable_mode: bool,
    non_literal: bool,
) -> Option<NumOrVarChoice> {
    let number_tip = if non_literal {
        ht::NUM_OR_VAR_NUMBER_REPLACE
    } else {
        ht::NUM_OR_VAR_NUMBER
    };
    let number = ui
        .selectable_label(!variable_mode, "Number")
        .on_hover_text(number_tip);
    let variable = ui
        .selectable_label(variable_mode, "Variable")
        .on_hover_text(ht::NUM_OR_VAR_VARIABLE);
    if number.clicked() {
        Some(NumOrVarChoice::Number)
    } else if variable.clicked() {
        Some(NumOrVarChoice::Variable)
    } else {
        None
    }
}

/// Apply a menu choice. Number on a non-literal replaces it with `0`, or the
/// range start when that is higher. Variable asks the next frame's editor to
/// take focus ([`take_var_focus`]).
pub(crate) fn apply_num_or_var_choice(
    ui: &egui::Ui,
    mode_id: egui::Id,
    value: &mut String,
    range: Option<&RangeInclusive<f64>>,
    choice: NumOrVarChoice,
) {
    match choice {
        NumOrVarChoice::Number => {
            ui.data_mut(|d| d.insert_temp(mode_id, false));
            if non_literal(value) {
                *value = format_literal(number_fallback(range));
            }
        }
        NumOrVarChoice::Variable => {
            ui.data_mut(|d| {
                d.insert_temp(mode_id, true);
                d.insert_temp(mode_id.with("focus_var"), true);
            });
        }
    }
}

/// One-shot: the user just chose Variable, so the editor built this frame
/// should take keyboard focus. Clears the flag.
pub(crate) fn take_var_focus(ui: &egui::Ui, mode_id: egui::Id) -> bool {
    let key = mode_id.with("focus_var");
    let pending = ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    if pending {
        ui.data_mut(|d| d.remove_temp::<bool>(key));
    }
    pending
}

fn paint_number(
    ui: &mut egui::Ui,
    value: &mut String,
    width: f32,
    range: Option<&RangeInclusive<f64>>,
    validation: &EntryValidation,
    help: &str,
) -> egui::Response {
    let mut n = parse_literal(value.trim()).unwrap_or_else(|| number_fallback(range));
    if let Some(r) = range {
        n = n.clamp(*r.start(), *r.end());
    }
    let initial = n;
    let slot = ui.scope(|ui| {
        ui.set_min_width(width);
        let mut entry = super::num_entry(&mut n).speed(1.0).max_decimals(3);
        if let Some(r) = range {
            entry = entry.range(r.clone());
        }
        let resp = ui.add(entry);
        // Ignore a clamp or a reformat that did not move the number, so an
        // empty field stays empty until the user edits it.
        if (n - initial).abs() > 1e-9 {
            *value = format_literal(n);
        }
        resp.on_hover_text(help)
    });
    var_pills::paint_entry_validation_icon(ui, validation);
    slot.inner | slot.response
}

fn parse_literal(text: &str) -> Option<f64> {
    if sqyre_varref::contains(text) {
        return None;
    }
    let n: f64 = text.parse().ok()?;
    n.is_finite().then_some(n)
}

fn number_fallback(range: Option<&RangeInclusive<f64>>) -> f64 {
    let Some(range) = range else {
        return 0.0;
    };
    range.start().max(0.0).clamp(*range.start(), *range.end())
}

pub(crate) fn format_literal(v: f64) -> String {
    if !v.is_finite() {
        return "0".to_owned();
    }
    if v.fract() == 0.0 && v.abs() <= i64::MAX as f64 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint_ctx::VarTheme;
    use egui_kittest::kittest::Queryable;
    use sqyre_domain::KnownVariableNames;

    fn show_field(
        initial: String,
        range: Option<RangeInclusive<f64>>,
    ) -> egui_kittest::Harness<'static, String> {
        egui_kittest::Harness::builder()
            .with_size([480.0, 320.0])
            .build_ui_state(
                move |ui, value: &mut String| {
                    let known = KnownVariableNames::default();
                    let validation = EntryValidation::default();
                    num_or_var_field(
                        ui,
                        "#",
                        "Count",
                        value,
                        NumOrVarOpts {
                            width: 160.0,
                            range: range.clone(),
                            theme: VarTheme {
                                known_vars: &known,
                                is_dark: true,
                            },
                            validation: &validation,
                            help: "How many.",
                        },
                    );
                },
                initial,
            )
    }

    fn has_role(h: &egui_kittest::Harness<'_, String>, role: egui::accesskit::Role) -> bool {
        h.query_all_by_role(role).next().is_some()
    }

    fn pointer_click(
        h: &mut egui_kittest::Harness<'_, String>,
        at: egui::Pos2,
        button: egui::PointerButton,
    ) {
        h.hover_at(at);
        h.run_steps(1);
        for pressed in [true, false] {
            h.input_mut().events.push(egui::Event::PointerButton {
                pos: at,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
            h.run_steps(1);
        }
        h.run_steps(2);
    }

    #[test]
    fn literal_right_click_stays_on_variable_entry() {
        let mut h = show_field("12".into(), None);
        h.run_steps(2);
        assert!(has_role(&h, egui::accesskit::Role::SpinButton));
        assert!(!has_role(&h, egui::accesskit::Role::TextInput));
        let at = h
            .get_by_role(egui::accesskit::Role::SpinButton)
            .rect()
            .center();
        pointer_click(&mut h, at, egui::PointerButton::Secondary);
        h.get_by_label("Variable").click();
        h.run_steps(2);
        assert_eq!(h.state(), "12");
        assert!(has_role(&h, egui::accesskit::Role::TextInput));
        assert!(!has_role(&h, egui::accesskit::Role::SpinButton));
        assert!(
            h.get_by_role(egui::accesskit::Role::TextInput).is_focused(),
            "choosing Variable focuses the text entry"
        );
        pointer_click(
            &mut h,
            egui::pos2(400.0, 280.0),
            egui::PointerButton::Primary,
        );
        assert_eq!(h.state(), "12");
        assert!(
            has_role(&h, egui::accesskit::Role::TextInput),
            "variable mode sticks after the field loses focus"
        );
        assert!(!has_role(&h, egui::accesskit::Role::SpinButton));
    }

    #[test]
    fn expression_right_click_becomes_a_number() {
        let mut h = show_field("${x}".into(), Some(5.0..=9.0));
        h.run_steps(2);
        assert!(has_role(&h, egui::accesskit::Role::TextInput));
        assert!(!has_role(&h, egui::accesskit::Role::SpinButton));
        let at = h
            .get_by_role(egui::accesskit::Role::TextInput)
            .rect()
            .center();
        pointer_click(&mut h, at, egui::PointerButton::Secondary);
        h.get_by_label("Number").click();
        h.run_steps(2);
        assert_eq!(h.state(), "5");
        assert!(has_role(&h, egui::accesskit::Role::SpinButton));
        assert!(!has_role(&h, egui::accesskit::Role::TextInput));
    }

    #[test]
    fn number_fields_in_a_row_share_a_baseline() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 200.0));
        for pass in 0..2 {
            let out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    crate::widgets::wrapped_row(ui, |ui| {
                        let known = KnownVariableNames::default();
                        let validation = EntryValidation::default();
                        for label in ["In row", "In col", "In extra"] {
                            let mut value = "1".to_owned();
                            num_or_var_field(
                                ui,
                                "#",
                                label,
                                &mut value,
                                NumOrVarOpts {
                                    width: 160.0,
                                    range: Some(1.0..=100.0),
                                    theme: VarTheme {
                                        known_vars: &known,
                                        is_dark: true,
                                    },
                                    validation: &validation,
                                    help: "cell",
                                },
                            );
                        }
                    });
                },
            );
            let mut frames: Vec<egui::Rect> = out
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect) if rect.fill.a() > 200 && rect.rect.width() > 80.0 => {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .collect();
            out.drop_without_applying_deltas();
            frames.sort_by(|a, b| a.left().total_cmp(&b.left()));
            assert_eq!(frames.len(), 3, "expected one frame per field: {frames:?}");
            let top = frames[0].top();
            for (i, rect) in frames.iter().enumerate() {
                assert!(
                    (rect.top() - top).abs() < 1.0,
                    "field {i} stepped down on pass {pass}: {frames:?}"
                );
            }
        }
    }

    #[test]
    fn empty_number_mode_does_not_write_zero() {
        let mut h = show_field(String::new(), None);
        h.run_steps(4);
        assert_eq!(h.state(), "");
        assert!(has_role(&h, egui::accesskit::Role::SpinButton));
    }
}
