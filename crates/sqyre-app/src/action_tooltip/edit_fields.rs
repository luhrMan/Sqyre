use crate::paint_ctx::VarTheme;
use crate::var_pills;
use crate::widgets::{
    drag_field, drag_field_enabled, text_field, W_TEXT, W_VAR,
};
use eframe::egui;
use sqyre_domain::{KnownVariableNames, ScalarValue, VariableAssignment};
use sqyre_validate::{
    preview_calculate, validate_numeric_expression, validate_set_variable_value,
    validate_variable_references,
};

fn pick_icon_btn(ui: &mut egui::Ui) -> egui::Response {
    crate::theme::icon_button(ui, "☰").on_hover_text("Pick…")
}

fn scalar_field(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    value: &mut ScalarValue,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    let mut text = value.as_display();
    let before = text.clone();
    let validation = validate_numeric_expression(&text, active_macro);
    var_pills::validated_var_ref_edit(
        ui,
        label,
        &mut text,
        known_vars,
        is_dark,
        W_VAR,
        &validation,
        help_text,
    );
    if text != before {
        *value = ScalarValue::parse_edit(&text);
    }
}

#[allow(clippy::too_many_arguments)]
fn var_ref_field(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    value: &mut String,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    desired_width: f32,
    active_macro: Option<&Macro>,
) {
    let validation = validate_variable_references(value, active_macro);
    var_pills::validated_var_ref_edit(
        ui,
        label,
        value,
        known_vars,
        is_dark,
        desired_width,
        &validation,
        help_text,
    );
}

fn string_list_field(ui: &mut egui::Ui, label: &str, values: &mut Vec<String>, help_text: &str) {
    let mut text = values.join("\n");
    if !label.is_empty() {
        help::label(ui, label, help_text);
    }
    if help::tip(
        ui.add(
            egui::TextEdit::multiline(&mut text)
                .desired_width(f32::INFINITY)
                .desired_rows(3),
        ),
        help_text,
    )
    .changed()
    {
        *values = text
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
    }
}

/// Plain text for the Set value editor.
fn set_value_edit_text(value: &ScalarValue) -> String {
    value.as_display()
}

fn yaml_value_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut ScalarValue,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    let mut text = set_value_edit_text(value);
    let before = text.clone();

    // Expression builder toolbar.
    let mut insert: Option<String> = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.menu_button("f(x)", |ui| {
            for f in EXPRESSION_FUNCTIONS {
                if ui.button(format!("{f}( )")).clicked() {
                    insert = Some(format!("{f}()"));
                    ui.close();
                }
            }
            ui.separator();
            for c in EXPRESSION_CONSTANTS {
                if ui.button(*c).clicked() {
                    insert = Some((*c).to_string());
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(h::SET_FX);
        for op in EXPRESSION_OPERATORS {
            if ui.small_button(*op).on_hover_text(h::SET_FX).clicked() {
                insert = Some((*op).to_string());
            }
        }
    });

    if let Some(token) = insert {
        text.push_str(&token);
    }

    let validation = validate_set_variable_value(&text, active_macro);
    var_pills::validated_var_ref_multiline_edit(
        ui,
        label,
        &mut text,
        known_vars,
        is_dark,
        f32::INFINITY,
        2,
        &validation,
        h::SET_VALUE,
    );

    // Live preview.
    if let Some(m) = active_macro {
        if let Ok(preview) = preview_calculate(&text, m) {
            if !preview.is_empty() {
                ui.weak(format!("Preview: {preview}"));
            }
        }
    }

    if text != before {
        // Store as plain string. Runtime resolve parses numbers/expressions.
        *value = ScalarValue::String(text);
    }
}
fn assignments_editor(
    ui: &mut egui::Ui,
    assignments: &mut Vec<VariableAssignment>,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    if list_header(ui, "Assignments", h::SET_ADD_ASSIGNMENT) {
        assignments.push(VariableAssignment::default());
    }
    let mut remove: Option<usize> = None;
    let can_remove = assignments.len() > 1;
    for (i, a) in assignments.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            if i > 0 {
                ui.separator();
            }
            theme::section_frame(ui.style()).show(ui, |ui| {
                var_pills::var_name_text_edit(
                    ui,
                    "Variable",
                    &mut a.variable_name,
                    known_vars,
                    is_dark,
                    W_VAR,
                    h::SET_VAR,
                );
                yaml_value_field(
                    ui,
                    "Value (text, ${ref}, or expression)",
                    &mut a.value,
                    known_vars,
                    is_dark,
                    active_macro,
                );
                if can_remove
                    && ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Remove").color(theme::MACRO_STOP),
                            )
                            .small(),
                        )
                        .on_hover_text(h::SET_REMOVE_ASSIGNMENT)
                        .clicked()
                {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove {
        assignments.remove(i);
    }
    if assignments.is_empty() {
        assignments.push(VariableAssignment::default());
    }
}
