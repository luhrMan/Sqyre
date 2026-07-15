//! Staged field editors and draft apply (preserve `subactions`).

use super::sections::{tip_section, tip_wrapped_section};
use crate::icon_cache::IconCache;
use crate::pickers::{self, options, ActivePicker};
use crate::preview_tooltip::{PreviewKind, PreviewTooltipCache};
use crate::var_pills;
use eframe::egui;
use sqyre_domain::{
    Action, ActionKind, ConditionClause, CoordinateOutputs, CoordinateRef, ListColumn, MatchOrder,
    ScalarValue, WaitTilFoundConfig,
};
use sqyre_persist::ProgramCatalog;
use std::collections::HashSet;

/// Copy draft fields onto `live`, keeping `live`'s children.
pub fn apply_draft_preserving_children(live: &mut Action, draft: Action) -> Result<(), String> {
    if live.id != draft.id {
        return Err(format!(
            "draft id {} != live id {}",
            draft.id.as_str(),
            live.id.as_str()
        ));
    }
    if std::mem::discriminant(&live.kind) != std::mem::discriminant(&draft.kind) {
        return Err("cannot change action type in tooltip edit".into());
    }
    let preserved = live.children().to_vec();
    live.kind = draft.kind;
    if let Some(kids) = live.children_mut() {
        *kids = preserved;
    }
    Ok(())
}

pub fn paint_edit_fields(
    ui: &mut egui::Ui,
    draft: &mut Action,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    previews: &mut PreviewTooltipCache,
    picker: &mut ActivePicker,
    _macros: &[(String, Vec<String>)],
    known_vars: &HashSet<String>,
    is_dark: bool,
) {
    match &mut draft.kind {
        ActionKind::Break | ActionKind::Continue => {
            tip_section(ui, |ui| {
                ui.label("Nothing to edit.");
            });
        }
        ActionKind::Wait { time } => {
            tip_wrapped_section(ui, |ui| {
                scalar_field(ui, "Time (ms)", time, known_vars, is_dark);
            });
        }
        ActionKind::Click { button, state } => {
            tip_wrapped_section(ui, |ui| {
                combo_str(ui, "Button", button, options::CLICK_BUTTONS);
                ui.checkbox(state, "Down");
            });
        }
        ActionKind::Key { key, state } => {
            tip_wrapped_section(ui, |ui| {
                var_pills::var_ref_text_edit(ui, "Key", key, known_vars, is_dark, 160.0);
                ui.checkbox(state, "Down");
            });
        }
        ActionKind::Type { text, delay_ms } => {
            tip_wrapped_section(ui, |ui| {
                var_pills::var_ref_text_edit(ui, "Text", text, known_vars, is_dark, 160.0);
                ui.add(egui::DragValue::new(delay_ms).prefix("Delay ms: ").speed(1));
            });
        }
        ActionKind::Move {
            point,
            smooth,
            smooth_low,
            smooth_high,
            smooth_delay_ms,
        } => {
            tip_section(ui, |ui| {
                point_picker_row(ui, point, picker);
                paint_coord_preview(ui, catalog, previews, point, PreviewKind::Point);
            });
            tip_wrapped_section(ui, |ui| {
                ui.checkbox(smooth, "Smooth");
                ui.add(
                    egui::DragValue::new(smooth_low)
                        .prefix("Smooth low: ")
                        .speed(0.01)
                        .range(0.0..=1.0),
                );
                ui.add(
                    egui::DragValue::new(smooth_high)
                        .prefix("Smooth high: ")
                        .speed(0.01)
                        .range(0.0..=1.0),
                );
                ui.add(
                    egui::DragValue::new(smooth_delay_ms)
                        .prefix("Smooth delay ms: ")
                        .speed(1),
                );
            });
        }
        ActionKind::Pause {
            message,
            continue_key,
            pass_through,
        } => {
            tip_section(ui, |ui| {
                var_pills::var_ref_text_edit(ui, "Message", message, known_vars, is_dark, 220.0);
            });
            tip_section(ui, |ui| {
                string_list_field(ui, "Continue keys (one per line)", continue_key);
            });
            tip_wrapped_section(ui, |ui| {
                ui.checkbox(pass_through, "Pass through");
            });
        }
        ActionKind::FocusWindow {
            process_path,
            window_title,
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Window title", window_title);
                ui.horizontal(|ui| {
                    ui.label("Process path");
                    ui.add(
                        egui::TextEdit::singleline(process_path)
                            .desired_width(180.0)
                            .hint_text("/path/to/app"),
                    );
                    if pick_icon_btn(ui).clicked() {
                        *picker = pickers::open_window_picker(process_path, window_title);
                    }
                });
            });
        }
        ActionKind::RunMacro { macro_name } => {
            tip_wrapped_section(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Macro");
                    ui.label(if macro_name.is_empty() {
                        "(unset)"
                    } else {
                        macro_name.as_str()
                    });
                    if pick_icon_btn(ui).clicked() {
                        *picker = ActivePicker::Macro {
                            search: String::new(),
                            value: macro_name.clone(),
                        };
                    }
                });
            });
        }
        ActionKind::SetVariable {
            variable_name,
            value,
        } => {
            tip_wrapped_section(ui, |ui| {
                var_pills::var_name_text_edit(ui, "Variable", variable_name, known_vars, is_dark, 160.0);
            });
            tip_section(ui, |ui| {
                yaml_value_field(ui, "Value (YAML)", value, known_vars, is_dark);
            });
        }
        ActionKind::Calculate {
            expression,
            output_var,
        } => {
            tip_section(ui, |ui| {
                var_pills::var_ref_text_edit(ui, "Expression", expression, known_vars, is_dark, 220.0);
            });
            tip_wrapped_section(ui, |ui| {
                var_pills::var_name_text_edit(ui, "Output variable", output_var, known_vars, is_dark, 160.0);
            });
        }
        ActionKind::SaveVariable {
            variable_name,
            destination,
            append,
            append_newline,
        } => {
            tip_wrapped_section(ui, |ui| {
                var_pills::var_name_text_edit(ui, "Variable", variable_name, known_vars, is_dark, 160.0);
                var_pills::var_ref_text_edit(ui, "Destination", destination, known_vars, is_dark, 160.0);
            });
            tip_wrapped_section(ui, |ui| {
                ui.checkbox(append, "Append");
                ui.checkbox(append_newline, "Append newline");
            });
        }
        ActionKind::Loop { name, count, .. } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
                scalar_field(ui, "Count", count, known_vars, is_dark);
            });
        }
        ActionKind::While {
            name,
            match_mode,
            clauses,
            max_iterations,
            ..
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
                let mut all = match_mode != "any";
                if ui.checkbox(&mut all, "Match all (uncheck = any)").changed() {
                    *match_mode = if all { "all".into() } else { "any".into() };
                }
                ui.add(egui::DragValue::new(max_iterations).prefix("Max iterations: "));
            });
            tip_section(ui, |ui| {
                clauses_editor(ui, clauses, known_vars, is_dark);
            });
        }
        ActionKind::Conditional {
            name,
            match_mode,
            clauses,
            ..
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
                let mut all = match_mode != "any";
                if ui.checkbox(&mut all, "Match all (uncheck = any)").changed() {
                    *match_mode = if all { "all".into() } else { "any".into() };
                }
            });
            tip_section(ui, |ui| {
                clauses_editor(ui, clauses, known_vars, is_dark);
            });
        }
        ActionKind::ForEachRow {
            name,
            sources,
            start_row,
            end_row,
            ..
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
                scalar_field(ui, "Start row", start_row, known_vars, is_dark);
                scalar_field(ui, "End row", end_row, known_vars, is_dark);
            });
            tip_section(ui, |ui| {
                list_columns_editor(ui, sources, known_vars, is_dark);
            });
        }
        ActionKind::ImageSearch {
            name,
            targets,
            search_area,
            tolerance,
            blur,
            wait,
            coords,
            run_branch_on_no_find,
            order,
            ..
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
            });
            tip_section(ui, |ui| {
                targets_editor(ui, catalog, icons, targets, picker);
            });
            tip_section(ui, |ui| {
                search_area_picker_row(ui, search_area, picker);
                paint_coord_preview(ui, catalog, previews, search_area, PreviewKind::SearchArea);
            });
            tip_wrapped_section(ui, |ui| {
                ui.add(
                    egui::DragValue::new(tolerance)
                        .prefix("Tolerance: ")
                        .speed(0.01)
                        .range(0.0..=1.0),
                );
                ui.add(egui::DragValue::new(blur).prefix("Blur: "));
            });
            tip_section(ui, |ui| {
                wait_editor(ui, wait);
            });
            tip_section(ui, |ui| {
                coords_editor(ui, coords, known_vars, is_dark);
            });
            tip_section(ui, |ui| {
                order_editor(ui, order);
            });
            tip_wrapped_section(ui, |ui| {
                ui.checkbox(run_branch_on_no_find, "Run branch on no find");
            });
        }
        ActionKind::Ocr {
            name,
            target,
            search_area,
            output_variable,
            coords,
            wait,
            run_branch_on_no_find,
            blur,
            min_threshold,
            resize,
            grayscale,
            threshold_otsu,
            threshold_invert,
            order,
            ..
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
                var_pills::var_ref_text_edit(ui, "Target", target, known_vars, is_dark, 160.0);
            });
            tip_section(ui, |ui| {
                search_area_picker_row(ui, search_area, picker);
                paint_coord_preview(ui, catalog, previews, search_area, PreviewKind::SearchArea);
            });
            tip_wrapped_section(ui, |ui| {
                var_pills::var_name_text_edit(
                    ui,
                    "Output variable",
                    output_variable,
                    known_vars,
                    is_dark,
                    160.0,
                );
            });
            tip_section(ui, |ui| {
                wait_editor(ui, wait);
            });
            tip_section(ui, |ui| {
                coords_editor(ui, coords, known_vars, is_dark);
            });
            tip_section(ui, |ui| {
                order_editor(ui, order);
            });
            tip_wrapped_section(ui, |ui| {
                ui.add(egui::DragValue::new(blur).prefix("Blur: "));
                ui.add(egui::DragValue::new(min_threshold).prefix("Min threshold: "));
                ui.add(egui::DragValue::new(resize).prefix("Resize: ").speed(0.01));
                ui.checkbox(grayscale, "Grayscale");
                ui.checkbox(threshold_otsu, "Threshold Otsu");
                ui.checkbox(threshold_invert, "Threshold invert");
                ui.checkbox(run_branch_on_no_find, "Run branch on no find");
            });
        }
        ActionKind::FindPixel {
            name,
            search_area,
            target_color,
            color_tolerance,
            wait,
            coords,
            run_branch_on_no_find,
            order,
            ..
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
            });
            tip_section(ui, |ui| {
                search_area_picker_row(ui, search_area, picker);
                paint_coord_preview(ui, catalog, previews, search_area, PreviewKind::SearchArea);
            });
            tip_wrapped_section(ui, |ui| {
                var_pills::var_ref_text_edit(ui, "Target color", target_color, known_vars, is_dark, 160.0);
                ui.add(egui::DragValue::new(color_tolerance).prefix("Color tolerance: "));
            });
            tip_section(ui, |ui| {
                wait_editor(ui, wait);
            });
            tip_section(ui, |ui| {
                coords_editor(ui, coords, known_vars, is_dark);
            });
            tip_section(ui, |ui| {
                order_editor(ui, order);
            });
            tip_wrapped_section(ui, |ui| {
                ui.checkbox(run_branch_on_no_find, "Run branch on no find");
            });
        }
        ActionKind::NavigateSelect {
            program,
            graph_name,
            chord_up,
            chord_down,
            chord_left,
            chord_right,
            chord_select,
            chord_back,
            wrap_edges,
            move_cursor_with_nav,
            smooth,
            pass_through,
            hold_repeat,
            select_device,
            select_button,
            select_key,
            select_press_mode,
            in_graph,
            in_row,
            in_col,
            in_collection,
            output_ref,
            output_graph,
            output_row,
            output_col,
            output_collection,
            subactions: _,
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Program", program);
                text_field(ui, "Graph", graph_name);
            });
            tip_section(ui, |ui| {
                string_list_field(ui, "Chord up", chord_up);
                string_list_field(ui, "Chord down", chord_down);
                string_list_field(ui, "Chord left", chord_left);
                string_list_field(ui, "Chord right", chord_right);
                string_list_field(ui, "Chord select", chord_select);
                string_list_field(ui, "Chord back", chord_back);
            });
            tip_wrapped_section(ui, |ui| {
                ui.checkbox(wrap_edges, "Wrap edges");
                ui.checkbox(move_cursor_with_nav, "Move cursor with nav");
                ui.checkbox(smooth, "Smooth");
                ui.checkbox(pass_through, "Pass through");
                ui.checkbox(hold_repeat, "Hold repeat");
            });
            tip_wrapped_section(ui, |ui| {
                combo_str(ui, "Select device", select_device, options::SELECT_DEVICES);
                combo_str(ui, "Select button", select_button, options::MOUSE_BUTTONS);
                text_field(ui, "Select key", select_key);
                combo_str(
                    ui,
                    "Select press mode",
                    select_press_mode,
                    options::SELECT_PRESS_MODES,
                );
            });
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "In graph", in_graph);
                text_field(ui, "In row", in_row);
                text_field(ui, "In col", in_col);
                text_field(ui, "In collection", in_collection);
            });
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Output ref", output_ref);
                text_field(ui, "Output graph", output_graph);
                text_field(ui, "Output row", output_row);
                text_field(ui, "Output col", output_col);
                text_field(ui, "Output collection", output_collection);
            });
            tip_section(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Nav Key children: nest Navigate Key actions under this node in the tree.",
                    )
                    .small()
                    .weak(),
                );
            });
        }
        ActionKind::NavigateKey {
            name,
            chord,
            exit,
            subactions: _,
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
                ui.checkbox(exit, "Exit Navigate Select after branch");
            });
            tip_section(ui, |ui| {
                string_list_field(ui, "Chord", chord);
            });
        }
    }
}

fn targets_editor(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    targets: &mut Vec<String>,
    picker: &mut ActivePicker,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Items").strong());
        if ui.button("Add / edit…").clicked() {
            *picker = ActivePicker::Items {
                search: String::new(),
                staged: targets.clone(),
            };
        }
    });
    if targets.is_empty() {
        ui.label("(none)");
        return;
    }
    let mut remove: Option<usize> = None;
    let snapshot = targets.clone();
    pickers::paint_even_icon_grid(
        ui,
        catalog,
        icons,
        &snapshot,
        |_| true,
        true,
        |_, _| {},
        |i| {
            remove = Some(i);
        },
    );
    if let Some(i) = remove {
        targets.remove(i);
    }
}

fn pick_icon_btn(ui: &mut egui::Ui) -> egui::Response {
    ui.add(egui::Button::new(egui::RichText::new("☰").size(14.0)).small())
        .on_hover_text("Pick…")
}

fn paint_coord_preview(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    previews: &mut PreviewTooltipCache,
    coord: &CoordinateRef,
    kind: PreviewKind,
) {
    if coord.is_empty() {
        return;
    }
    let mut force = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Preview").strong());
        if ui
            .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
            .on_hover_text("Refresh")
            .clicked()
        {
            force = true;
        }
    });
    previews.paint_for_coordinate_ref(ui, catalog, coord, kind, force);
}

fn point_picker_row(ui: &mut egui::Ui, point: &mut CoordinateRef, picker: &mut ActivePicker) {
    ui.horizontal(|ui| {
        ui.label("Point");
        ui.monospace(if point.is_empty() {
            "(unset)"
        } else {
            point.as_str()
        });
        if pick_icon_btn(ui).clicked() {
            *picker = ActivePicker::Point {
                search: String::new(),
                value: point.0.clone(),
                cell_pick: None,
            };
        }
    });
}

fn search_area_picker_row(
    ui: &mut egui::Ui,
    area: &mut CoordinateRef,
    picker: &mut ActivePicker,
) {
    ui.horizontal(|ui| {
        ui.label("Search area");
        ui.monospace(if area.is_empty() {
            "(unset)"
        } else {
            area.as_str()
        });
        if pick_icon_btn(ui).clicked() {
            *picker = ActivePicker::SearchArea {
                search: String::new(),
                value: area.0.clone(),
                cell_pick: None,
            };
        }
    });
}

fn combo_str(ui: &mut egui::Ui, label: &str, value: &mut String, options: &[&str]) {
    ui.horizontal(|ui| {
        ui.label(label);
        let display = if value.is_empty() {
            "(unset)".to_string()
        } else {
            value.clone()
        };
        let mut custom = None;
        if !options.iter().any(|o| *o == value.as_str()) && !value.is_empty() {
            custom = Some(value.clone());
        }
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
            });
    });
}

fn text_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(value).desired_width(220.0));
    });
}

fn scalar_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut ScalarValue,
    known_vars: &HashSet<String>,
    is_dark: bool,
) {
    let mut text = value.as_display();
    let before = text.clone();
    var_pills::var_ref_text_edit(ui, label, &mut text, known_vars, is_dark, 160.0);
    if text != before {
        *value = parse_scalar(&text);
    }
}

fn parse_scalar(text: &str) -> ScalarValue {
    let t = text.trim();
    if t.is_empty() {
        return ScalarValue::Null;
    }
    if let Ok(i) = t.parse::<i64>() {
        return ScalarValue::Int(i);
    }
    if let Ok(f) = t.parse::<f64>() {
        return ScalarValue::Float(f);
    }
    if t.eq_ignore_ascii_case("true") {
        return ScalarValue::Bool(true);
    }
    if t.eq_ignore_ascii_case("false") {
        return ScalarValue::Bool(false);
    }
    ScalarValue::String(t.to_string())
}

fn string_list_field(ui: &mut egui::Ui, label: &str, values: &mut Vec<String>) {
    let mut text = values.join("\n");
    ui.label(label);
    if ui
        .add(
            egui::TextEdit::multiline(&mut text)
                .desired_width(280.0)
                .desired_rows(3),
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

fn yaml_value_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut serde_yaml::Value,
    known_vars: &HashSet<String>,
    is_dark: bool,
) {
    let mut text = match serde_yaml::to_string(value) {
        Ok(s) => s.trim().to_string(),
        Err(_) => String::new(),
    };
    let before = text.clone();
    var_pills::var_ref_multiline_edit(ui, label, &mut text, known_vars, is_dark, 280.0, 2);
    if text != before {
        match serde_yaml::from_str::<serde_yaml::Value>(&text) {
            Ok(v) => *value = v,
            Err(_) => *value = serde_yaml::Value::String(text),
        }
    }
}

fn wait_editor(ui: &mut egui::Ui, wait: &mut WaitTilFoundConfig) {
    ui.label(egui::RichText::new("Wait / repeat").strong());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::splat(6.0);
        combo_str(ui, "Repeat mode", &mut wait.repeat_mode, options::REPEAT_MODES);
        ui.checkbox(&mut wait.wait_til_found, "Wait til found (legacy)");
        ui.add(egui::DragValue::new(&mut wait.wait_til_found_seconds).prefix("Wait seconds: "));
        ui.add(
            egui::DragValue::new(&mut wait.wait_til_found_interval_ms).prefix("Interval ms: "),
        );
        ui.add(egui::DragValue::new(&mut wait.max_iterations).prefix("Max iterations: "));
    });
}

fn coords_editor(
    ui: &mut egui::Ui,
    coords: &mut CoordinateOutputs,
    known_vars: &HashSet<String>,
    is_dark: bool,
) {
    ui.label(egui::RichText::new("Coordinate outputs").strong());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::splat(6.0);
        var_pills::var_name_text_edit(
            ui,
            "Output X",
            &mut coords.output_x_variable,
            known_vars,
            is_dark,
            160.0,
        );
        var_pills::var_name_text_edit(
            ui,
            "Output Y",
            &mut coords.output_y_variable,
            known_vars,
            is_dark,
            160.0,
        );
    });
}

fn order_editor(ui: &mut egui::Ui, order: &mut MatchOrder) {
    ui.label(egui::RichText::new("Match order").strong());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::splat(6.0);
        combo_str(ui, "Grouping", &mut order.grouping, options::ORDER_GROUPING);
        combo_str(
            ui,
            "Horizontal",
            &mut order.horizontal,
            options::ORDER_HORIZONTAL,
        );
        combo_str(ui, "Vertical", &mut order.vertical, options::ORDER_VERTICAL);
    });
}

fn clauses_editor(
    ui: &mut egui::Ui,
    clauses: &mut Vec<ConditionClause>,
    known_vars: &HashSet<String>,
    is_dark: bool,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("Clauses");
            if ui.small_button("+").clicked() {
                clauses.push(ConditionClause::default());
            }
        });
        let mut remove: Option<usize> = None;
        for (i, clause) in clauses.iter_mut().enumerate() {
            // Unique id so each clause's "op" ComboBox is distinct (same label salt).
            ui.push_id(i, |ui| {
                ui.horizontal(|ui| {
                    scalar_field(ui, "L", &mut clause.left, known_vars, is_dark);
                    combo_str(ui, "op", &mut clause.operator, options::CONDITIONAL_OPERATORS);
                    scalar_field(ui, "R", &mut clause.right, known_vars, is_dark);
                    if ui.small_button("−").clicked() {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove {
            clauses.remove(i);
        }
    });
}

fn list_columns_editor(
    ui: &mut egui::Ui,
    sources: &mut Vec<ListColumn>,
    known_vars: &HashSet<String>,
    is_dark: bool,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("Sources");
            if ui.small_button("+").clicked() {
                sources.push(ListColumn::default());
            }
        });
        let mut remove: Option<usize> = None;
        for (i, col) in sources.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                ui.group(|ui| {
                    var_pills::var_ref_text_edit(
                        ui,
                        "Source",
                        &mut col.source,
                        known_vars,
                        is_dark,
                        200.0,
                    );
                    var_pills::var_name_text_edit(
                        ui,
                        "Output var",
                        &mut col.output_var,
                        known_vars,
                        is_dark,
                        160.0,
                    );
                    ui.checkbox(&mut col.is_file, "Is file");
                    ui.checkbox(&mut col.skip_blank_lines, "Skip blank lines");
                    if ui.small_button("Remove").clicked() {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove {
            sources.remove(i);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::ActionId;

    #[test]
    fn parse_scalar_int_and_string() {
        assert_eq!(parse_scalar("42"), ScalarValue::Int(42));
        assert_eq!(
            parse_scalar("${x}"),
            ScalarValue::String("${x}".into())
        );
        assert_eq!(parse_scalar(""), ScalarValue::Null);
    }

    #[test]
    fn apply_rejects_type_change() {
        let mut live = Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        };
        let draft = Action {
            id: live.id,
            kind: ActionKind::Click {
                button: "left".into(),
                state: true,
            },
        };
        assert!(apply_draft_preserving_children(&mut live, draft).is_err());
    }
}
