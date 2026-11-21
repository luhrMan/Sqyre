//! Reusable field widgets and list editors.

use super::edit_pickers::{paint_coord_preview, search_area_picker_row};
use crate::action_tooltip::sections::{tip_section, tip_wrapped_section};
use crate::action_tooltip::{help, help as h};
use crate::paint_ctx::{CatalogPaint, VarTheme};
use crate::pickers::{options, ActivePicker};
use crate::preview_tooltip::PreviewKind;
use crate::theme;
use crate::var_pills::{self, VarFieldOpts};
use crate::widgets::{
    combo_condition_operator, combo_enum, combo_str_labeled, drag_field_enabled, num_or_var_field,
    pair_labeled, text_field, NumOrVarOpts, W_TEXT, W_VAR,
};
use eframe::egui;
use sqyre_domain::{
    ConditionBlock, ConditionClause, CoordinateOutputs, CoordinateRef, DetectionBranch,
    KnownVariableNames, ListColumn, Macro, MatchGrouping, MatchMode, MatchOrder, RepeatMode,
    ScalarValue, VariableAssignment, WaitTilFoundConfig, MAX_DETECTION_INTERVAL_MS,
    MIN_DETECTION_INTERVAL_MS,
};
use sqyre_persist::MAX_WHILE_MAX_ITERATIONS;
use sqyre_validate::{
    preview_calculate, validate_numeric_expression, validate_set_variable_value,
    validate_variable_references, EntryValidation,
};

#[expect(
    clippy::too_many_arguments,
    reason = "field editor: icon, label, help, value, and shared var theme context"
)]
pub(super) fn scalar_field(
    ui: &mut egui::Ui,
    icon: &str,
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
    num_or_var_field(
        ui,
        icon,
        label,
        &mut text,
        NumOrVarOpts {
            width: W_VAR,
            range: None,
            theme: VarTheme {
                known_vars,
                is_dark,
            },
            validation: &validation,
            help: help_text,
        },
    );
    if text != before {
        *value = ScalarValue::parse_edit(&text);
    }
}

/// Shared scalar edit body: validated var-ref text field + commit on change.
#[expect(
    clippy::too_many_arguments,
    reason = "field editor: icon, label, help, value, theme, and validation/commit"
)]
pub(super) fn scalar_value_edit(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut ScalarValue,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
    validate: fn(&str, Option<&Macro>) -> EntryValidation,
    commit: impl Fn(&mut ScalarValue, String),
) {
    let mut text = value.as_display();
    let before = text.clone();
    let validation = validate(&text, active_macro);
    var_pills::validated_var_ref_edit(
        ui,
        label,
        &mut text,
        VarTheme {
            known_vars,
            is_dark,
        },
        VarFieldOpts {
            desired_width: W_VAR,
            validation: &validation,
            icon,
            help: help_text,
        },
    );
    if text != before {
        commit(value, text);
    }
}

/// Condition L/R operands: plain text (including spaces) is allowed; only
/// arithmetic-looking values are expression-checked — same rules as Set value.
#[expect(
    clippy::too_many_arguments,
    reason = "field editor: icon, label, help, value, and shared var theme context"
)]
pub(super) fn condition_operand_field(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut ScalarValue,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    scalar_value_edit(
        ui,
        icon,
        label,
        help_text,
        value,
        known_vars,
        is_dark,
        active_macro,
        validate_set_variable_value,
        |v, text| *v = ScalarValue::String(text),
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "var-ref editor: icon, label, theme, help, width, and validation context"
)]
pub(super) fn var_ref_field(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    value: &mut String,
    theme: VarTheme<'_>,
    help_text: &str,
    desired_width: f32,
    active_macro: Option<&Macro>,
) {
    let validation = validate_variable_references(value, active_macro);
    var_pills::validated_var_ref_edit(
        ui,
        label,
        value,
        theme,
        VarFieldOpts {
            desired_width,
            validation: &validation,
            icon,
            help: help_text,
        },
    );
}

pub(super) fn string_list_field(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    values: &mut Vec<String>,
    help_text: &str,
) {
    string_list_field_opts(ui, icon, label, values, help_text, None);
}

/// Multiline one-per-line string list with an optional trailing control (e.g. record).
pub(super) fn string_list_field_opts(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    values: &mut Vec<String>,
    help_text: &str,
    mut trailing: Option<&mut dyn FnMut(&mut egui::Ui)>,
) {
    let mut text = values.join("\n");
    let reserve = if trailing.is_some() {
        crate::widgets::icon_btn_side(ui) + crate::widgets::PAIR_SLOT_GAP
    } else {
        0.0
    };
    let mut edit = |ui: &mut egui::Ui| {
        let width = (crate::widgets::fields::finite_room(ui) - reserve).max(24.0);
        help::tip(
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_width(width)
                    .desired_rows(3),
            ),
            help_text,
        )
    };
    let changed = if label.is_empty() {
        edit(ui).changed()
    } else if let Some(slot) = trailing.as_mut() {
        crate::widgets::pair_icon(
            ui,
            |ui| {
                help::label(ui, icon, label);
            },
            edit,
            |ui| slot(ui),
        )
        .inner
        .changed()
    } else {
        pair_labeled(ui, icon, label, edit).inner.changed()
    };
    if changed {
        *values = text
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
    }
}

/// Plain text for the Set value editor.
pub(super) fn set_value_edit_text(value: &ScalarValue) -> String {
    value.as_display()
}

pub(super) fn yaml_value_field(
    ui: &mut egui::Ui,
    icon: &str,
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
        ui.menu_button("f(x)", |ui| {
            for f in EXPRESSION_FUNCTIONS {
                if ui.button(format!("{f}( )")).clicked() {
                    insert = Some(format!("{f}()"));
                    ui.close();
                }
            }
            crate::widgets::section_separator(ui);
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
        VarTheme {
            known_vars,
            is_dark,
        },
        2,
        VarFieldOpts {
            desired_width: f32::INFINITY,
            validation: &validation,
            icon,
            help: h::SET_VALUE,
        },
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

const EXPRESSION_OPERATORS: &[&str] = &["+", "-", "*", "/", "^", "(", ")"];
const EXPRESSION_FUNCTIONS: &[&str] = &[
    "sqrt", "abs", "round", "floor", "ceil", "trunc", "sin", "cos", "tan", "ln",
];
const EXPRESSION_CONSTANTS: &[&str] = &["~pi", "~e"];

pub(super) fn condition_editor(
    ui: &mut egui::Ui,
    condition: &mut ConditionBlock,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
    extra: impl FnOnce(&mut egui::Ui),
) {
    tip_wrapped_section(ui, |ui| {
        text_field(
            ui,
            egui_phosphor::regular::TEXT_T,
            "Name",
            h::NAME,
            &mut condition.name,
        );
        let mut all = condition.match_mode != MatchMode::Any;
        if help::tip(
            ui.checkbox(&mut all, "Match all (uncheck = any)"),
            h::MATCH_ALL,
        )
        .changed()
        {
            condition.match_mode = if all { MatchMode::All } else { MatchMode::Any };
        }
        extra(ui);
    });
    tip_section(ui, |ui| {
        clauses_editor(
            ui,
            &mut condition.clauses,
            known_vars,
            is_dark,
            active_macro,
        );
    });
}

pub(super) fn search_area_section(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    search_area: &mut CoordinateRef,
    picker: &mut ActivePicker,
) {
    tip_section(ui, |ui| {
        search_area_picker_row(ui, paint, search_area, picker);
        paint_coord_preview(
            ui,
            paint.catalog,
            paint.previews,
            search_area,
            PreviewKind::SearchArea,
        );
    });
}

pub(super) fn detection_branch_editor(ui: &mut egui::Ui, detection: &mut DetectionBranch) {
    tip_wrapped_section(ui, |ui| wait_editor(ui, &mut detection.wait));
    tip_wrapped_section(ui, |ui| order_editor(ui, &mut detection.order));
}

/// Spaced repeat-mode label; *Until* italic, **While** bold.
pub(super) fn repeat_mode_label(ui: &egui::Ui, mode: RepeatMode) -> egui::WidgetText {
    let style = ui.style();
    let color = style.visuals.text_color();
    let (prefix, mid, mid_italics, suffix) = match mode {
        RepeatMode::Once => return "Once".into(),
        RepeatMode::WaitUntilFound => ("Wait ", "Until", true, " Found"),
        RepeatMode::WaitWhileFound => ("Wait ", "While", false, " Found"),
        RepeatMode::RepeatUntilFound => ("Repeat ", "Until", true, " Found"),
        RepeatMode::RepeatWhileFound => ("Repeat ", "While", false, " Found"),
    };
    let mut job = egui::text::LayoutJob::default();
    egui::RichText::new(prefix).color(color).append_to(
        &mut job,
        style,
        egui::FontSelection::Default,
        egui::Align::LEFT,
    );
    let mid_rt = if mid_italics {
        egui::RichText::new(mid).italics().color(color)
    } else {
        egui::RichText::new(mid).strong().color(color)
    };
    mid_rt.append_to(
        &mut job,
        style,
        egui::FontSelection::Default,
        egui::Align::LEFT,
    );
    egui::RichText::new(suffix).color(color).append_to(
        &mut job,
        style,
        egui::FontSelection::Default,
        egui::Align::LEFT,
    );
    job.into()
}

pub(super) fn repeat_mode_plain(mode: RepeatMode) -> &'static str {
    match mode {
        RepeatMode::Once => "Once",
        RepeatMode::WaitUntilFound => "Wait Until Found",
        RepeatMode::WaitWhileFound => "Wait While Found",
        RepeatMode::RepeatUntilFound => "Repeat Until Found",
        RepeatMode::RepeatWhileFound => "Repeat While Found",
    }
}

pub(super) fn wait_editor(ui: &mut egui::Ui, wait: &mut WaitTilFoundConfig) {
    pair_labeled(ui, egui_phosphor::regular::REPEAT, "Repeat mode", |ui| {
        let selected = repeat_mode_label(ui, wait.repeat_mode);
        let selected_plain = repeat_mode_plain(wait.repeat_mode);
        let response = egui::ComboBox::from_id_salt("Repeat mode")
            .selected_text(selected)
            .truncate()
            .show_ui(ui, |ui| {
                for mode in [
                    RepeatMode::Once,
                    RepeatMode::WaitUntilFound,
                    RepeatMode::WaitWhileFound,
                    RepeatMode::RepeatUntilFound,
                    RepeatMode::RepeatWhileFound,
                ] {
                    if ui
                        .selectable_label(wait.repeat_mode == mode, repeat_mode_label(ui, mode))
                        .clicked()
                    {
                        wait.repeat_mode = mode;
                    }
                }
            })
            .response;
        if crate::widgets::elide::combo_text_elided(ui, &response, selected_plain) {
            response.on_hover_text(format!("{selected_plain}\n\n{}", h::REPEAT_MODE));
        } else {
            help::tip(response, h::REPEAT_MODE);
        }
    });
    // Once → timing off; wait modes → timing only; repeat modes → timing + max iterations.
    let timing_enabled = wait.uses_timing();
    let max_enabled = wait.uses_max_iterations();
    drag_field_enabled(
        ui,
        egui_phosphor::regular::CLOCK,
        "Wait seconds",
        h::WAIT_SECONDS,
        &mut wait.wait_til_found_seconds,
        timing_enabled,
        |d| d.speed(0.1).range(0.0..=1_000_000.0).max_decimals(3),
    );
    drag_field_enabled(
        ui,
        egui_phosphor::regular::TIMER,
        "Interval ms",
        h::WAIT_INTERVAL,
        &mut wait.wait_til_found_interval_ms,
        timing_enabled,
        |d| {
            d.speed(1)
                .range(MIN_DETECTION_INTERVAL_MS..=MAX_DETECTION_INTERVAL_MS)
        },
    );
    drag_field_enabled(
        ui,
        egui_phosphor::regular::REPEAT,
        "Max iterations",
        h::WAIT_MAX_ITER,
        &mut wait.max_iterations,
        max_enabled,
        |d| d.speed(1).range(0..=MAX_WHILE_MAX_ITERATIONS),
    );
}

pub(super) fn coords_editor(
    ui: &mut egui::Ui,
    coords: &mut CoordinateOutputs,
    known_vars: &KnownVariableNames,
    is_dark: bool,
) {
    var_pills::var_name_text_edit(
        ui,
        egui_phosphor::regular::ARROWS_HORIZONTAL,
        "Output X",
        &mut coords.output_x_variable,
        known_vars,
        is_dark,
        W_VAR,
        h::OUT_X,
    );
    var_pills::var_name_text_edit(
        ui,
        egui_phosphor::regular::ARROWS_VERTICAL,
        "Output Y",
        &mut coords.output_y_variable,
        known_vars,
        is_dark,
        W_VAR,
        h::OUT_Y,
    );
}

pub(super) fn order_editor(ui: &mut egui::Ui, order: &mut MatchOrder) {
    combo_enum(
        ui,
        egui_phosphor::regular::SQUARES_FOUR,
        "Grouping",
        h::ORDER_GROUPING,
        &mut order.grouping,
        MatchGrouping::ALL,
        MatchGrouping::label,
    );
    combo_str_labeled(
        ui,
        egui_phosphor::regular::ARROWS_LEFT_RIGHT,
        "Horizontal",
        h::ORDER_HORIZONTAL,
        &mut order.horizontal,
        options::ORDER_HORIZONTAL,
        "left_to_right",
    );
    combo_str_labeled(
        ui,
        egui_phosphor::regular::ARROWS_DOWN_UP,
        "Vertical",
        h::ORDER_VERTICAL,
        &mut order.vertical,
        options::ORDER_VERTICAL,
        "top_to_bottom",
    );
}

/// Header row for repeatable list editors. Returns true when `+` was clicked.
pub(super) fn list_header(ui: &mut egui::Ui, title: &str, count: usize, add_help: &str) -> bool {
    let mut add = false;
    // Content-sized row (no `set_min_width(available)`) so the edit tip can
    // shrink horizontally — see egui-layout-gotchas.
    ui.horizontal(|ui| {
        ui.label(title);
        crate::widgets::count_suffix(ui, count);
        let btn = crate::widgets::add_button(ui, &format!("Add {}", title.to_lowercase()), true)
            .on_hover_text(add_help);
        if btn.clicked() {
            add = true;
        }
    });
    add
}

/// Right-click Remove menu for one repeatable list row; true when picked.
pub(super) fn remove_row_menu(ui: &egui::Ui, resp: &egui::Response) -> bool {
    crate::widgets::rect_danger_menu(
        ui,
        ui.id().with("remove_row_menu"),
        resp.rect,
        Some(resp),
        "Remove",
        true,
    )
}

pub(super) fn clauses_editor(
    ui: &mut egui::Ui,
    clauses: &mut Vec<ConditionClause>,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    if list_header(ui, "Clauses", clauses.len(), h::CLAUSE_ADD) {
        clauses.push(ConditionClause::default());
    }
    let mut remove: Option<usize> = None;
    let row_ids = crate::widgets::stable_row_ids(ui, "clauses", clauses.len());
    for (i, clause) in clauses.iter_mut().enumerate() {
        // Unique id so each clause's "op" ComboBox is distinct (same label salt).
        ui.push_id(row_ids[i], |ui| {
            let row = ui.horizontal(|ui| {
                condition_operand_field(
                    ui,
                    egui_phosphor::regular::FUNCTION,
                    "L",
                    h::CLAUSE_LEFT,
                    &mut clause.left,
                    known_vars,
                    is_dark,
                    active_macro,
                );
                combo_condition_operator(
                    ui,
                    egui_phosphor::regular::EQUALS,
                    "op",
                    h::CLAUSE_OP,
                    &mut clause.operator,
                );
                condition_operand_field(
                    ui,
                    egui_phosphor::regular::FUNCTION,
                    "R",
                    h::CLAUSE_RIGHT,
                    &mut clause.right,
                    known_vars,
                    is_dark,
                    active_macro,
                );
            });
            if remove_row_menu(ui, &row.response) {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        clauses.remove(i);
        crate::widgets::remove_row_id(ui, "clauses", i);
    }
}

pub(super) fn list_columns_editor(
    ui: &mut egui::Ui,
    sources: &mut Vec<ListColumn>,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    if list_header(ui, "Sources", sources.len(), h::FOREACH_ADD_SOURCE) {
        sources.push(ListColumn::default());
    }
    let mut remove: Option<usize> = None;
    let row_ids = crate::widgets::stable_row_ids(ui, "foreach_sources", sources.len());
    for (i, col) in sources.iter_mut().enumerate() {
        ui.push_id(row_ids[i], |ui| {
            if i > 0 {
                crate::widgets::section_separator(ui);
            }
            let frame = theme::section_frame(ui).show(ui, |ui| {
                var_ref_field(
                    ui,
                    egui_phosphor::regular::LIST_BULLETS,
                    "Source",
                    &mut col.source,
                    VarTheme {
                        known_vars,
                        is_dark,
                    },
                    h::FOREACH_SOURCE,
                    W_TEXT,
                    active_macro,
                );
                var_pills::var_name_text_edit(
                    ui,
                    egui_phosphor::regular::BRACKETS_CURLY,
                    "Output var",
                    &mut col.output_var,
                    known_vars,
                    is_dark,
                    W_VAR,
                    h::FOREACH_OUTPUT,
                );
                help::tip(ui.checkbox(&mut col.is_file, "Is file"), h::FOREACH_IS_FILE);
                help::tip(
                    ui.checkbox(&mut col.skip_blank_lines, "Skip blank lines"),
                    h::FOREACH_SKIP_BLANK,
                );
            });
            if remove_row_menu(ui, &frame.response) {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        sources.remove(i);
        crate::widgets::remove_row_id(ui, "foreach_sources", i);
    }
}

pub(super) fn assignments_editor(
    ui: &mut egui::Ui,
    assignments: &mut Vec<VariableAssignment>,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    if list_header(ui, "Assignments", assignments.len(), h::SET_ADD_ASSIGNMENT) {
        assignments.push(VariableAssignment::default());
    }
    let mut remove: Option<usize> = None;
    let can_remove = assignments.len() > 1;
    let row_ids = crate::widgets::stable_row_ids(ui, "assignments", assignments.len());
    for (i, a) in assignments.iter_mut().enumerate() {
        ui.push_id(row_ids[i], |ui| {
            if i > 0 {
                crate::widgets::section_separator(ui);
            }
            let frame = theme::section_frame(ui).show(ui, |ui| {
                var_pills::var_name_text_edit(
                    ui,
                    egui_phosphor::regular::BRACKETS_CURLY,
                    "Variable",
                    &mut a.variable_name,
                    known_vars,
                    is_dark,
                    W_VAR,
                    h::SET_VAR,
                );
                yaml_value_field(
                    ui,
                    egui_phosphor::regular::FUNCTION,
                    "Value (text, ${ref}, or expression)",
                    &mut a.value,
                    known_vars,
                    is_dark,
                    active_macro,
                );
            });
            if can_remove && remove_row_menu(ui, &frame.response) {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        assignments.remove(i);
        crate::widgets::remove_row_id(ui, "assignments", i);
    }
    if assignments.is_empty() {
        assignments.push(VariableAssignment::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scalar_int_and_string() {
        assert_eq!(ScalarValue::parse_edit("42"), ScalarValue::Int(42));
        assert_eq!(
            ScalarValue::parse_edit("${x}"),
            ScalarValue::String("${x}".into())
        );
        assert_eq!(ScalarValue::parse_edit(""), ScalarValue::Null);
        assert_eq!(ScalarValue::parse_edit("true"), ScalarValue::Bool(true));
    }

    #[test]
    fn set_value_edit_text_has_no_yaml_quotes() {
        assert_eq!(set_value_edit_text(&ScalarValue::Null), "");
        assert_eq!(set_value_edit_text(&ScalarValue::String(String::new())), "");
        assert_eq!(
            set_value_edit_text(&ScalarValue::String("true".into())),
            "true"
        );
        assert_eq!(set_value_edit_text(&ScalarValue::String("42".into())), "42");
        assert_eq!(
            set_value_edit_text(&ScalarValue::String("'hello'".into())),
            "'hello'"
        );
        assert_eq!(set_value_edit_text(&ScalarValue::Bool(true)), "true");
        assert_eq!(set_value_edit_text(&ScalarValue::Int(42)), "42");
    }

    #[test]
    fn set_value_edit_survives_deleting_quotes() {
        let mut value = ScalarValue::String(String::new());
        let mut text = set_value_edit_text(&value);
        assert_eq!(text, "");
        text.push('\'');
        value = ScalarValue::String(text.clone());
        assert_eq!(set_value_edit_text(&value), "'");
        text.pop();
        value = ScalarValue::String(text);
        assert_eq!(set_value_edit_text(&value), "");

        value = ScalarValue::String("true".into());
        text = set_value_edit_text(&value);
        assert_eq!(text, "true");
        // User wraps in quotes then deletes them — display must stay identity.
        text = format!("'{text}'");
        value = ScalarValue::String(text.clone());
        assert_eq!(set_value_edit_text(&value), "'true'");
        text.pop();
        value = ScalarValue::String(text.clone());
        assert_eq!(set_value_edit_text(&value), "'true");
        text.remove(0);
        value = ScalarValue::String(text);
        assert_eq!(set_value_edit_text(&value), "true");
    }
}
