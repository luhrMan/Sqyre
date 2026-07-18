//! Staged field editors and draft apply (preserve `subactions`).

use super::sections::{tip_section, tip_wrapped_section};
use crate::icon_cache::IconCache;
use crate::paint_ctx::{CatalogPaint, EditFieldsCtx, RecordBridges, VarTheme};
use crate::pickers::{self, options, ActivePicker, CoordKind};
use crate::preview_tooltip::{PreviewKind, PreviewTooltipCache};
use crate::theme;
use crate::tree_chrome;
use crate::var_pills;
use eframe::egui;
use sqyre_domain::{
    parse_hex_color, Action, ActionKind, ConditionBlock, ConditionClause, CoordinateOutputs,
    CoordinateRef, DetectionBranch, ListColumn, Macro, MatchMode, MatchOrder, MouseButton,
    RepeatMode, ScalarValue, WaitTilFoundConfig,
};
use sqyre_persist::ProgramCatalog;
use sqyre_validate::{
    preview_calculate, validate_numeric_expression, validate_set_variable_value,
    validate_variable_references,
};
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
    picker: &mut ActivePicker,
    ctx: &mut EditFieldsCtx<'_>,
) {
    let EditFieldsCtx {
        paint,
        bridges,
        theme,
        macros: _macros,
        active_macro,
    } = ctx;
    let CatalogPaint {
        catalog,
        icons,
        previews,
    } = paint;
    let VarTheme {
        known_vars,
        is_dark,
    } = *theme;
    let RecordBridges {
        key_record,
        hotkey_record,
        macro_hotkeys,
        screen_click,
    } = bridges;
    let active_macro = *active_macro;
    match &mut draft.kind {
        ActionKind::Break | ActionKind::Continue => {
            tip_section(ui, |ui| {
                ui.label("Nothing to edit.");
            });
        }
        ActionKind::Wait { time } => {
            tip_wrapped_section(ui, |ui| {
                scalar_field(ui, "Time (ms)", time, known_vars, is_dark, active_macro);
            });
        }
        ActionKind::Click { button, state } => {
            tip_wrapped_section(ui, |ui| {
                let mut btn = button.as_str().to_string();
                combo_str(ui, "Button", &mut btn, options::CLICK_BUTTONS);
                *button = MouseButton::parse(&btn);
                ui.vertical(|ui| {
                    ui.small("Up");
                    theme::up_down_toggle(ui, state);
                    ui.small("Down");
                });
            });
        }
        ActionKind::Key { key, state } => {
            tip_wrapped_section(ui, |ui| {
                ui.horizontal(|ui| {
                    var_ref_field(ui, "Key", key, known_vars, is_dark, 160.0, active_macro);
                    if theme::record_icon_button(ui, "Record a key", !key_record.is_open())
                        .clicked()
                    {
                        key_record.open(macro_hotkeys);
                    }
                });
                ui.vertical(|ui| {
                    ui.small("Up");
                    theme::up_down_toggle(ui, state);
                    ui.small("Down");
                });
            });
        }
        ActionKind::Type { text, delay_ms } => {
            tip_wrapped_section(ui, |ui| {
                var_ref_field(ui, "Text", text, known_vars, is_dark, 160.0, active_macro);
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
                var_ref_field(
                    ui,
                    "Message",
                    message,
                    known_vars,
                    is_dark,
                    220.0,
                    active_macro,
                );
            });
            tip_section(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Continue keys (one per line)");
                    if theme::record_icon_button(
                        ui,
                        "Record continue chord",
                        !hotkey_record.is_open() && !key_record.is_open(),
                    )
                    .clicked()
                    {
                        hotkey_record.open(macro_hotkeys);
                    }
                });
                string_list_field(ui, "", continue_key);
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
                            scroll_to_selection: true,
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
                var_pills::var_name_text_edit(
                    ui,
                    "Variable",
                    variable_name,
                    known_vars,
                    is_dark,
                    160.0,
                );
            });
            tip_section(ui, |ui| {
                yaml_value_field(
                    ui,
                    "Value (text, ${ref}, or expression)",
                    value,
                    known_vars,
                    is_dark,
                    active_macro,
                );
            });
        }
        ActionKind::SaveVariable {
            variable_name,
            destination,
            append,
            append_newline,
        } => {
            tip_wrapped_section(ui, |ui| {
                var_pills::var_name_text_edit(
                    ui,
                    "Variable",
                    variable_name,
                    known_vars,
                    is_dark,
                    160.0,
                );
                var_ref_field(
                    ui,
                    "Destination",
                    destination,
                    known_vars,
                    is_dark,
                    160.0,
                    active_macro,
                );
            });
            tip_wrapped_section(ui, |ui| {
                ui.checkbox(append, "Append");
                ui.checkbox(append_newline, "Append newline");
            });
        }
        ActionKind::Loop { name, count, .. } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
                scalar_field(ui, "Count", count, known_vars, is_dark, active_macro);
            });
        }
        ActionKind::While {
            condition,
            max_iterations,
            ..
        } => {
            condition_editor(ui, condition, known_vars, is_dark, active_macro, |ui| {
                ui.add(egui::DragValue::new(max_iterations).prefix("Max iterations: "));
            });
        }
        ActionKind::Conditional { condition, .. } => {
            condition_editor(ui, condition, known_vars, is_dark, active_macro, |_| {});
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
                scalar_field(
                    ui,
                    "Start row",
                    start_row,
                    known_vars,
                    is_dark,
                    active_macro,
                );
                scalar_field(ui, "End row", end_row, known_vars, is_dark, active_macro);
            });
            tip_section(ui, |ui| {
                list_columns_editor(ui, sources, known_vars, is_dark, active_macro);
            });
        }
        ActionKind::ImageSearch {
            name,
            targets,
            search_area,
            tolerance,
            blur,
            detection,
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
            });
            tip_section(ui, |ui| {
                targets_editor(ui, catalog, icons, targets, picker);
            });
            search_area_section(ui, catalog, previews, search_area, picker);
            tip_wrapped_section(ui, |ui| {
                ui.add(
                    egui::DragValue::new(tolerance)
                        .prefix("Tolerance: ")
                        .speed(0.01)
                        .range(0.0..=1.0),
                );
                ui.add(egui::DragValue::new(blur).prefix("Blur: "));
            });
            detection_branch_editor(ui, detection, known_vars, is_dark);
        }
        ActionKind::Ocr {
            name,
            target,
            search_area,
            output_variable,
            blur,
            min_threshold,
            resize,
            grayscale,
            threshold_otsu,
            threshold_invert,
            detection,
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
                var_ref_field(
                    ui,
                    "Target",
                    target,
                    known_vars,
                    is_dark,
                    160.0,
                    active_macro,
                );
            });
            search_area_section(ui, catalog, previews, search_area, picker);
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
            detection_branch_editor(ui, detection, known_vars, is_dark);
            tip_wrapped_section(ui, |ui| {
                ui.add(egui::DragValue::new(blur).prefix("Blur: "));
                ui.add(egui::DragValue::new(min_threshold).prefix("Min threshold: "));
                ui.add(egui::DragValue::new(resize).prefix("Resize: ").speed(0.01));
                ui.checkbox(grayscale, "Grayscale");
                ui.checkbox(threshold_otsu, "Threshold Otsu");
                ui.checkbox(threshold_invert, "Threshold invert");
            });
        }
        ActionKind::FindPixel {
            name,
            search_area,
            target_color,
            color_tolerance,
            detection,
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", name);
            });
            search_area_section(ui, catalog, previews, search_area, picker);
            tip_wrapped_section(ui, |ui| {
                ui.horizontal(|ui| {
                    var_ref_field(
                        ui,
                        "Target color",
                        target_color,
                        known_vars,
                        is_dark,
                        160.0,
                        active_macro,
                    );
                    if let Some(rgba) = parse_hex_color(target_color) {
                        let size = egui::vec2(16.0, 16.0);
                        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                        ui.painter().rect(
                            rect,
                            3.0,
                            tree_chrome::rgba_pub(rgba),
                            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if theme::record_icon_button(
                        ui,
                        "Click on screen to sample pixel color",
                        !screen_click.is_armed(),
                    )
                    .clicked()
                    {
                        screen_click.arm_color();
                    }
                });
                ui.add(egui::DragValue::new(color_tolerance).prefix("Color tolerance: "));
            });
            detection_branch_editor(ui, detection, known_vars, is_dark);
        }
        ActionKind::NavigateSelect(data) => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Program", &mut data.program);
                text_field(ui, "Graph", &mut data.graph_name);
            });
            tip_section(ui, |ui| {
                string_list_field(ui, "Chord up", &mut data.chords.up);
                string_list_field(ui, "Chord down", &mut data.chords.down);
                string_list_field(ui, "Chord left", &mut data.chords.left);
                string_list_field(ui, "Chord right", &mut data.chords.right);
                string_list_field(ui, "Chord select", &mut data.chords.select);
                string_list_field(ui, "Chord back", &mut data.chords.back);
            });
            tip_wrapped_section(ui, |ui| {
                ui.checkbox(&mut data.options.wrap_edges, "Wrap edges");
                ui.checkbox(
                    &mut data.options.move_cursor_with_nav,
                    "Move cursor with nav",
                );
                ui.checkbox(&mut data.options.smooth, "Smooth");
                ui.checkbox(&mut data.options.pass_through, "Pass through");
                ui.checkbox(&mut data.options.hold_repeat, "Hold repeat");
            });
            tip_wrapped_section(ui, |ui| {
                combo_str(
                    ui,
                    "Select device",
                    &mut data.select.device,
                    options::SELECT_DEVICES,
                );
                combo_str(
                    ui,
                    "Select button",
                    &mut data.select.button,
                    options::MOUSE_BUTTONS,
                );
                text_field(ui, "Select key", &mut data.select.key);
                combo_str(
                    ui,
                    "Select press mode",
                    &mut data.select.press_mode,
                    options::SELECT_PRESS_MODES,
                );
            });
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "In graph", &mut data.inputs.graph);
                text_field(ui, "In row", &mut data.inputs.row);
                text_field(ui, "In col", &mut data.inputs.col);
                text_field(ui, "In collection", &mut data.inputs.collection);
            });
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Output ref", &mut data.outputs.output_ref);
                text_field(ui, "Output graph", &mut data.outputs.output_graph);
                text_field(ui, "Output row", &mut data.outputs.output_row);
                text_field(ui, "Output col", &mut data.outputs.output_col);
                text_field(ui, "Output collection", &mut data.outputs.output_collection);
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
    coord_picker_row(ui, "Point", point, CoordKind::Point, picker);
}

fn search_area_picker_row(ui: &mut egui::Ui, area: &mut CoordinateRef, picker: &mut ActivePicker) {
    coord_picker_row(ui, "Search area", area, CoordKind::SearchArea, picker);
}

fn coord_picker_row(
    ui: &mut egui::Ui,
    label: &str,
    coord: &mut CoordinateRef,
    kind: CoordKind,
    picker: &mut ActivePicker,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.monospace(if coord.is_empty() {
            "(unset)"
        } else {
            coord.as_str()
        });
        if pick_icon_btn(ui).clicked() {
            *picker = ActivePicker::Coord {
                kind,
                search: String::new(),
                value: coord.0.clone(),
                cell_pick: None,
                scroll_to_selection: true,
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
        if !options.contains(&value.as_str()) && !value.is_empty() {
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
        160.0,
        &validation,
    );
    if text != before {
        *value = ScalarValue::parse_edit(&text);
    }
}

fn var_ref_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    known_vars: &HashSet<String>,
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
    );
}

fn string_list_field(ui: &mut egui::Ui, label: &str, values: &mut Vec<String>) {
    let mut text = values.join("\n");
    if !label.is_empty() {
        ui.label(label);
    }
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

/// Plain text for the Set value editor.
fn set_value_edit_text(value: &ScalarValue) -> String {
    value.as_display()
}

fn yaml_value_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut ScalarValue,
    known_vars: &HashSet<String>,
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
        .on_hover_text("Insert function or constant");
        for op in EXPRESSION_OPERATORS {
            if ui.small_button(*op).clicked() {
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
        280.0,
        2,
        &validation,
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

fn condition_editor(
    ui: &mut egui::Ui,
    condition: &mut ConditionBlock,
    known_vars: &HashSet<String>,
    is_dark: bool,
    active_macro: Option<&Macro>,
    extra: impl FnOnce(&mut egui::Ui),
) {
    tip_wrapped_section(ui, |ui| {
        text_field(ui, "Name", &mut condition.name);
        let mut all = condition.match_mode != MatchMode::Any;
        if ui.checkbox(&mut all, "Match all (uncheck = any)").changed() {
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

fn search_area_section(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    previews: &mut PreviewTooltipCache,
    search_area: &mut CoordinateRef,
    picker: &mut ActivePicker,
) {
    tip_section(ui, |ui| {
        search_area_picker_row(ui, search_area, picker);
        paint_coord_preview(ui, catalog, previews, search_area, PreviewKind::SearchArea);
    });
}

fn detection_branch_editor(
    ui: &mut egui::Ui,
    detection: &mut DetectionBranch,
    known_vars: &HashSet<String>,
    is_dark: bool,
) {
    tip_section(ui, |ui| wait_editor(ui, &mut detection.wait));
    tip_section(ui, |ui| {
        coords_editor(ui, &mut detection.coords, known_vars, is_dark);
    });
    tip_section(ui, |ui| order_editor(ui, &mut detection.order));
    tip_wrapped_section(ui, |ui| {
        ui.checkbox(
            &mut detection.run_branch_on_no_find,
            "Run branch on no find",
        );
    });
}

fn wait_editor(ui: &mut egui::Ui, wait: &mut WaitTilFoundConfig) {
    ui.label(egui::RichText::new("Wait / repeat").strong());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::splat(6.0);
        let mut mode = wait.repeat_mode.as_str().to_string();
        combo_str(ui, "Repeat mode", &mut mode, options::REPEAT_MODES);
        wait.repeat_mode = RepeatMode::parse(&mode);
        // Once → all off; waituntilfound → timing only;
        // repeatwhilefound → timing + max iterations.
        let timing_enabled = wait.repeat_mode != RepeatMode::Once;
        let max_enabled = wait.is_repeat_while_found();
        ui.add_enabled(
            timing_enabled,
            egui::DragValue::new(&mut wait.wait_til_found_seconds).prefix("Wait seconds: "),
        );
        ui.add_enabled(
            timing_enabled,
            egui::DragValue::new(&mut wait.wait_til_found_interval_ms).prefix("Interval ms: "),
        );
        ui.add_enabled(
            max_enabled,
            egui::DragValue::new(&mut wait.max_iterations).prefix("Max iterations: "),
        );
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
    active_macro: Option<&Macro>,
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
                    scalar_field(ui, "L", &mut clause.left, known_vars, is_dark, active_macro);
                    combo_str(
                        ui,
                        "op",
                        &mut clause.operator,
                        options::CONDITIONAL_OPERATORS,
                    );
                    scalar_field(
                        ui,
                        "R",
                        &mut clause.right,
                        known_vars,
                        is_dark,
                        active_macro,
                    );
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
    active_macro: Option<&Macro>,
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
                    var_ref_field(
                        ui,
                        "Source",
                        &mut col.source,
                        known_vars,
                        is_dark,
                        200.0,
                        active_macro,
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
