//! Labeled form field helpers shared by action edit, variables, and data editor.

use crate::action_tooltip::help;
use eframe::egui;

/// Standard single-line text edit width.
pub const W_TEXT: f32 = 220.0;
/// Standard variable / scalar ref edit width.
pub const W_VAR: f32 = 160.0;
/// Standard multiline edit width.
pub const W_MULTILINE: f32 = 280.0;

pub fn text_field(ui: &mut egui::Ui, label: &str, help_text: &str, value: &mut String) {
    text_field_width(ui, label, help_text, value, W_TEXT);
}

pub fn text_field_width(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    value: &mut String,
    width: f32,
) {
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        help::tip(
            ui.add(egui::TextEdit::singleline(value).desired_width(width)),
            help_text,
        );
    });
}

/// Labeled DragValue (no `.prefix`); configure speed/range via `configure`.
pub fn drag_field<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    value: &mut Num,
    configure: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
) {
    drag_field_enabled(ui, label, help_text, value, true, configure);
}

pub fn drag_field_enabled<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    value: &mut Num,
    enabled: bool,
    configure: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
) {
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        help::tip(
            ui.add_enabled(enabled, configure(egui::DragValue::new(value))),
            help_text,
        );
    });
}

pub fn combo_str(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    value: &mut String,
    options: &[&str],
) {
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        let display = if value.is_empty() {
            "(unset)".to_string()
        } else {
            value.clone()
        };
        let mut custom = None;
        if !options.contains(&value.as_str()) && !value.is_empty() {
            custom = Some(value.clone());
        }
        help::tip(
            egui::ComboBox::from_id_salt(label)
                .selected_text(display)
                .show_ui(ui, |ui| {
                    for opt in options {
                        let text = if opt.is_empty() { "(unset)" } else { opt };
                        ui.selectable_value(value, (*opt).to_string(), text);
                    }
                    if let Some(c) = custom {
                        ui.selectable_value(value, c.clone(), c);
                    }
                })
                .response,
            help_text,
        );
    });
}
