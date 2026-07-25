//! Labeled form field helpers shared by action edit, variables, and data editor.

use crate::action_tooltip::help;
use crate::pickers::fuzzy_match_fold;
use eframe::egui;
use std::hash::Hash;

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

/// Like [`combo_str`], but each option is `(stored value, display label)`.
///
/// When `value` is empty, the closed button and open-list highlight use
/// `empty_default` / its label without writing into `value` until the user picks.
pub fn combo_str_labeled(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    value: &mut String,
    options: &[(&str, &str)],
    empty_default: &str,
) {
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        let empty_label = options
            .iter()
            .find(|(v, _)| *v == empty_default)
            .map(|(_, l)| *l)
            .unwrap_or(empty_default);
        let display = if value.is_empty() {
            empty_label.to_string()
        } else {
            options
                .iter()
                .find(|(v, _)| *v == value.as_str())
                .map(|(_, l)| (*l).to_string())
                .unwrap_or_else(|| value.clone())
        };
        let current = if value.is_empty() {
            empty_default.to_string()
        } else {
            value.clone()
        };
        let mut custom = None;
        if !value.is_empty() && !options.iter().any(|(v, _)| *v == value.as_str()) {
            custom = Some(value.clone());
        }
        help::tip(
            egui::ComboBox::from_id_salt(label)
                .selected_text(display)
                .show_ui(ui, |ui| {
                    for &(stored, shown) in options {
                        if ui.selectable_label(current == stored, shown).clicked() {
                            *value = stored.to_string();
                        }
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

/// Callback for option (and closed-button) hover in [`searchable_combo_with`].
pub type ComboOptionHover<'a> = dyn FnMut(&mut egui::Ui, &egui::Response, &str) + 'a;

/// Leading icon for combo options / closed selection in [`searchable_combo_with`].
pub type ComboOptionIcon<'a> =
    dyn FnMut(&egui::Context, &str) -> Option<eframe::egui::TextureHandle> + 'a;

/// Searchable combo for unbounded / growing option lists (programs, masks, macros, …).
///
/// Uses fuzzy subsequence matching. `empty_text` is the closed-button label when `value` is
/// empty. When `none_label` is `Some`, an empty-string option is offered with that label.
///
/// Returns the combo button response; `changed` is set when the selection changes.
pub fn searchable_combo(
    ui: &mut egui::Ui,
    id_salt: impl Hash + std::fmt::Debug,
    value: &mut String,
    options: &[String],
    empty_text: &str,
    none_label: Option<&str>,
) -> egui::Response {
    searchable_combo_with(
        ui, id_salt, value, options, empty_text, none_label, None, None, None,
    )
}

/// [`searchable_combo`] with an optional fixed button/popup width.
pub fn searchable_combo_width(
    ui: &mut egui::Ui,
    id_salt: impl Hash + std::fmt::Debug,
    value: &mut String,
    options: &[String],
    empty_text: &str,
    none_label: Option<&str>,
    width: Option<f32>,
) -> egui::Response {
    searchable_combo_with(
        ui, id_salt, value, options, empty_text, none_label, width, None, None,
    )
}

/// [`searchable_combo_width`] plus per-option hover callbacks (also on the closed button when
/// `value` is non-empty). Use for image/preview tooltips while browsing the popup.
///
/// When `option_icon` is set, a leading icon is drawn on each option row and beside the
/// closed combo button for the current value.
#[allow(clippy::too_many_arguments)]
pub fn searchable_combo_with(
    ui: &mut egui::Ui,
    id_salt: impl Hash + std::fmt::Debug,
    value: &mut String,
    options: &[String],
    empty_text: &str,
    none_label: Option<&str>,
    width: Option<f32>,
    mut on_option_hover: Option<&mut ComboOptionHover<'_>>,
    mut option_icon: Option<&mut ComboOptionIcon<'_>>,
) -> egui::Response {
    // Match `ComboBox::from_id_salt(salt)` → `make_persistent_id(salt)`.
    let button_id = ui.make_persistent_id(&id_salt);
    let popup_id = button_id.with("popup");
    let search_id = button_id.with("search");
    let focused_id = button_id.with("search_focused");

    let display = if value.is_empty() {
        empty_text.to_string()
    } else {
        value.clone()
    };

    let orphan = (!value.is_empty() && !options.iter().any(|o| o == value)).then(|| value.clone());

    let closed_icon = option_icon
        .as_mut()
        .filter(|_| !value.is_empty())
        .and_then(|cb| cb(ui.ctx(), value.as_str()));

    let mut changed = false;
    let response = ui
        .horizontal(|ui| {
            if let Some(tex) = closed_icon.as_ref() {
                crate::icon_cache::paint_process_icon(
                    ui,
                    tex,
                    crate::icon_cache::PROCESS_ICON_SIDE,
                );
            }
            let mut combo = egui::ComboBox::from_id_salt(&id_salt)
                .selected_text(display)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
            if let Some(w) = width {
                combo = combo.width(w);
            }

            let mut ir = combo.show_ui(ui, |ui| {
                let mut search = ui
                    .ctx()
                    .data(|d| d.get_temp::<String>(search_id))
                    .unwrap_or_default();
                let search_resp = ui.add(
                    egui::TextEdit::singleline(&mut search)
                        .id(search_id)
                        .hint_text("Search…")
                        .desired_width(ui.available_width()),
                );
                let already_focused = ui
                    .ctx()
                    .data(|d| d.get_temp::<bool>(focused_id))
                    .unwrap_or(false);
                if !already_focused {
                    search_resp.request_focus();
                    ui.ctx().data_mut(|d| d.insert_temp(focused_id, true));
                }
                ui.ctx()
                    .data_mut(|d| d.insert_temp(search_id, search.clone()));

                ui.separator();

                let q = search.trim().to_ascii_lowercase();
                let mut any = false;

                if let Some(label) = none_label {
                    if q.is_empty() || fuzzy_match_fold(&q, label) {
                        any = true;
                        if ui.selectable_value(value, String::new(), label).clicked() {
                            changed = true;
                            egui::Popup::close_id(ui.ctx(), popup_id);
                        }
                    }
                }

                if let Some(ref custom) = orphan {
                    if q.is_empty() || fuzzy_match_fold(&q, custom) {
                        any = true;
                        let resp = paint_combo_option(
                            ui,
                            value,
                            custom.as_str(),
                            custom.as_str(),
                            &mut option_icon,
                        );
                        if let Some(cb) = on_option_hover.as_mut() {
                            cb(ui, &resp, custom.as_str());
                        }
                        if resp.clicked() {
                            changed = true;
                            egui::Popup::close_id(ui.ctx(), popup_id);
                        }
                    }
                }

                for opt in options {
                    if !q.is_empty() && !fuzzy_match_fold(&q, opt) {
                        continue;
                    }
                    any = true;
                    let resp =
                        paint_combo_option(ui, value, opt.as_str(), opt.as_str(), &mut option_icon);
                    if let Some(cb) = on_option_hover.as_mut() {
                        cb(ui, &resp, opt.as_str());
                    }
                    if resp.clicked() {
                        changed = true;
                        egui::Popup::close_id(ui.ctx(), popup_id);
                    }
                }

                if !any {
                    ui.weak("No matches");
                }
            });

            if !egui::ComboBox::is_open(ui.ctx(), button_id) {
                ui.ctx().data_mut(|d| {
                    d.remove::<String>(search_id);
                    d.remove::<bool>(focused_id);
                });
            }

            if !value.is_empty() {
                if let Some(cb) = on_option_hover.as_mut() {
                    cb(ui, &ir.response, value.as_str());
                }
            }

            if changed {
                ir.response.mark_changed();
            }
            ir.response
        })
        .inner;
    response
}

fn paint_combo_option(
    ui: &mut egui::Ui,
    value: &mut String,
    option_value: &str,
    label: &str,
    option_icon: &mut Option<&mut ComboOptionIcon<'_>>,
) -> egui::Response {
    let selected = value.as_str() == option_value;
    let icon = option_icon
        .as_mut()
        .and_then(|cb| cb(ui.ctx(), option_value));
    let resp = ui
        .horizontal(|ui| {
            if let Some(tex) = icon.as_ref() {
                crate::icon_cache::paint_process_icon(
                    ui,
                    tex,
                    crate::icon_cache::PROCESS_ICON_SIDE,
                );
            }
            ui.selectable_label(selected, label)
        })
        .inner;
    if resp.clicked() {
        *value = option_value.to_string();
    }
    resp
}
