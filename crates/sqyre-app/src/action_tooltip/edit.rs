//! Staged field editors and draft apply (preserve `subactions`).

use super::sections::{tip_section, tip_wrapped_section};
use super::{help, help as h};
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
    RepeatMode, ScalarValue, VariableAssignment, WaitTilFoundConfig,
};
use sqyre_persist::ProgramCatalog;
use sqyre_validate::{
    preview_calculate, validate_numeric_expression, validate_set_variable_value,
    validate_variable_references,
};
use std::collections::HashSet;

/// Standard single-line text edit width.
const W_TEXT: f32 = 220.0;
/// Standard variable / scalar ref edit width.
const W_VAR: f32 = 160.0;
/// Standard multiline edit width.
const W_MULTILINE: f32 = 280.0;

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
                help::label(ui, "Nothing to edit.", h::NOTHING_TO_EDIT);
            });
        }
        ActionKind::Wait { time } => {
            tip_wrapped_section(ui, |ui| {
                scalar_field(
                    ui,
                    "Time (ms)",
                    h::WAIT_TIME,
                    time,
                    known_vars,
                    is_dark,
                    active_macro,
                );
            });
        }
        ActionKind::Click { button, state } => {
            tip_wrapped_section(ui, |ui| {
                let mut btn = button.as_str().to_string();
                combo_str(
                    ui,
                    "Button",
                    h::CLICK_BUTTON,
                    &mut btn,
                    options::CLICK_BUTTONS,
                );
                *button = MouseButton::parse(&btn);
                ui.vertical(|ui| {
                    help::tip(ui.small("Up"), h::CLICK_STATE);
                    help::tip(theme::up_down_toggle(ui, state), h::CLICK_STATE);
                    help::tip(ui.small("Down"), h::CLICK_STATE);
                });
            });
        }
        ActionKind::Key { key, state } => {
            tip_wrapped_section(ui, |ui| {
                ui.horizontal(|ui| {
                    var_ref_field(
                        ui,
                        "Key",
                        h::KEY,
                        key,
                        known_vars,
                        is_dark,
                        W_VAR,
                        active_macro,
                    );
                    if theme::record_icon_button(ui, "Record a key", !key_record.is_open())
                        .clicked()
                    {
                        key_record.open(macro_hotkeys);
                    }
                });
                ui.vertical(|ui| {
                    help::tip(ui.small("Up"), h::KEY_STATE);
                    help::tip(theme::up_down_toggle(ui, state), h::KEY_STATE);
                    help::tip(ui.small("Down"), h::KEY_STATE);
                });
            });
        }
        ActionKind::Type { text, delay_ms } => {
            tip_wrapped_section(ui, |ui| {
                var_ref_field(
                    ui,
                    "Text",
                    h::TYPE_TEXT,
                    text,
                    known_vars,
                    is_dark,
                    W_VAR,
                    active_macro,
                );
                drag_field(ui, "Delay ms", h::TYPE_DELAY, delay_ms, |d| d.speed(1));
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
                help::tip(ui.checkbox(smooth, "Smooth"), h::MOVE_SMOOTH);
                drag_field(ui, "Smooth low", h::MOVE_SMOOTH_LOW, smooth_low, |d| {
                    d.speed(0.01).range(0.0..=1.0)
                });
                drag_field(ui, "Smooth high", h::MOVE_SMOOTH_HIGH, smooth_high, |d| {
                    d.speed(0.01).range(0.0..=1.0)
                });
                drag_field(
                    ui,
                    "Smooth delay ms",
                    h::MOVE_SMOOTH_DELAY,
                    smooth_delay_ms,
                    |d| d.speed(1),
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
                    h::PAUSE_MESSAGE,
                    message,
                    known_vars,
                    is_dark,
                    W_TEXT,
                    active_macro,
                );
            });
            tip_section(ui, |ui| {
                ui.horizontal(|ui| {
                    help::label(ui, "Continue keys (one per line)", h::PAUSE_CONTINUE);
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
                string_list_field(ui, "", continue_key, h::PAUSE_CONTINUE);
            });
            tip_wrapped_section(ui, |ui| {
                help::tip(
                    ui.checkbox(pass_through, "Pass through"),
                    h::PAUSE_PASS_THROUGH,
                );
            });
        }
        ActionKind::FocusWindow {
            process_path,
            window_title,
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Window title", h::FOCUS_TITLE, window_title);
                if picker_edit_row(
                    ui,
                    "Process path",
                    h::FOCUS_PROCESS,
                    process_path,
                    "/path/to/app",
                ) {
                    *picker = pickers::open_window_picker(process_path, window_title);
                }
            });
        }
        ActionKind::RunMacro { macro_name } => {
            tip_wrapped_section(ui, |ui| {
                if picker_display_row(ui, "Macro", h::RUN_MACRO, macro_name.as_str()) {
                    *picker = ActivePicker::Macro {
                        search: String::new(),
                        value: macro_name.clone(),
                        scroll_to_selection: true,
                    };
                }
            });
        }
        ActionKind::SetVariable { assignments } => {
            tip_section(ui, |ui| {
                assignments_editor(ui, assignments, known_vars, is_dark, active_macro);
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
                    W_VAR,
                    h::SAVE_VAR,
                );
                var_ref_field(
                    ui,
                    "Destination",
                    h::SAVE_DEST,
                    destination,
                    known_vars,
                    is_dark,
                    W_VAR,
                    active_macro,
                );
            });
            tip_wrapped_section(ui, |ui| {
                help::tip(ui.checkbox(append, "Append"), h::SAVE_APPEND);
                help::tip(
                    ui.checkbox(append_newline, "Append newline"),
                    h::SAVE_NEWLINE,
                );
            });
        }
        ActionKind::Loop { name, count, .. } => {
            tip_wrapped_section(ui, |ui| {
                scalar_field(
                    ui,
                    "Count",
                    h::LOOP_COUNT,
                    count,
                    known_vars,
                    is_dark,
                    active_macro,
                );
                text_field(ui, "Name", h::NAME, name);
            });
        }
        ActionKind::While {
            condition,
            max_iterations,
            ..
        } => {
            condition_editor(ui, condition, known_vars, is_dark, active_macro, |ui| {
                drag_field(
                    ui,
                    "Max iterations",
                    h::MAX_ITERATIONS,
                    max_iterations,
                    |d| d,
                );
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
                text_field(ui, "Name", h::NAME, name);
                scalar_field(
                    ui,
                    "Start row",
                    h::FOREACH_START,
                    start_row,
                    known_vars,
                    is_dark,
                    active_macro,
                );
                scalar_field(
                    ui,
                    "End row",
                    h::FOREACH_END,
                    end_row,
                    known_vars,
                    is_dark,
                    active_macro,
                );
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
                text_field(ui, "Name", h::NAME, name);
            });
            tip_section(ui, |ui| {
                targets_editor(ui, catalog, icons, targets, picker);
            });
            search_area_section(ui, catalog, previews, search_area, picker);
            tip_wrapped_section(ui, |ui| {
                drag_field(ui, "Tolerance", h::IS_TOLERANCE, tolerance, |d| {
                    d.speed(0.01).range(0.0..=1.0)
                });
                drag_field(ui, "Blur", h::IS_BLUR, blur, |d| d);
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
                text_field(ui, "Name", h::NAME, name);
                var_ref_field(
                    ui,
                    "Target",
                    h::OCR_TARGET,
                    target,
                    known_vars,
                    is_dark,
                    W_VAR,
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
                    W_VAR,
                    h::OCR_OUTPUT,
                );
            });
            detection_branch_editor(ui, detection, known_vars, is_dark);
            tip_wrapped_section(ui, |ui| {
                drag_field(ui, "Blur", h::OCR_BLUR, blur, |d| d);
                drag_field(
                    ui,
                    "Min threshold",
                    h::OCR_MIN_THRESHOLD,
                    min_threshold,
                    |d| d,
                );
                drag_field(ui, "Resize", h::OCR_RESIZE, resize, |d| d.speed(0.01));
                help::tip(ui.checkbox(grayscale, "Grayscale"), h::OCR_GRAYSCALE);
                help::tip(ui.checkbox(threshold_otsu, "Threshold Otsu"), h::OCR_OTSU);
                help::tip(
                    ui.checkbox(threshold_invert, "Threshold invert"),
                    h::OCR_INVERT,
                );
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
                text_field(ui, "Name", h::NAME, name);
            });
            search_area_section(ui, catalog, previews, search_area, picker);
            tip_wrapped_section(ui, |ui| {
                ui.horizontal(|ui| {
                    var_ref_field(
                        ui,
                        "Target color",
                        h::PIXEL_COLOR,
                        target_color,
                        known_vars,
                        is_dark,
                        W_VAR,
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
                drag_field(
                    ui,
                    "Color tolerance",
                    h::PIXEL_TOLERANCE,
                    color_tolerance,
                    |d| d,
                );
            });
            detection_branch_editor(ui, detection, known_vars, is_dark);
        }
        ActionKind::NavigateSelect(data) => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Program", h::NAV_PROGRAM, &mut data.program);
                text_field(ui, "Graph", h::NAV_GRAPH, &mut data.graph_name);
            });
            tip_section(ui, |ui| {
                string_list_field(ui, "Chord up", &mut data.chords.up, h::NAV_CHORD_UP);
                string_list_field(ui, "Chord down", &mut data.chords.down, h::NAV_CHORD_DOWN);
                string_list_field(ui, "Chord left", &mut data.chords.left, h::NAV_CHORD_LEFT);
                string_list_field(
                    ui,
                    "Chord right",
                    &mut data.chords.right,
                    h::NAV_CHORD_RIGHT,
                );
                string_list_field(
                    ui,
                    "Chord select",
                    &mut data.chords.select,
                    h::NAV_CHORD_SELECT,
                );
                string_list_field(ui, "Chord back", &mut data.chords.back, h::NAV_CHORD_BACK);
            });
            tip_wrapped_section(ui, |ui| {
                help::tip(
                    ui.checkbox(&mut data.options.wrap_edges, "Wrap edges"),
                    h::NAV_WRAP,
                );
                help::tip(
                    ui.checkbox(
                        &mut data.options.move_cursor_with_nav,
                        "Move cursor with nav",
                    ),
                    h::NAV_MOVE_CURSOR,
                );
                help::tip(
                    ui.checkbox(&mut data.options.smooth, "Smooth"),
                    h::NAV_SMOOTH,
                );
                help::tip(
                    ui.checkbox(&mut data.options.pass_through, "Pass through"),
                    h::NAV_PASS_THROUGH,
                );
                help::tip(
                    ui.checkbox(&mut data.options.hold_repeat, "Hold repeat"),
                    h::NAV_HOLD_REPEAT,
                );
            });
            tip_wrapped_section(ui, |ui| {
                combo_str(
                    ui,
                    "Select device",
                    h::NAV_SELECT_DEVICE,
                    &mut data.select.device,
                    options::SELECT_DEVICES,
                );
                combo_str(
                    ui,
                    "Select button",
                    h::NAV_SELECT_BUTTON,
                    &mut data.select.button,
                    options::MOUSE_BUTTONS,
                );
                text_field(ui, "Select key", h::NAV_SELECT_KEY, &mut data.select.key);
                combo_str(
                    ui,
                    "Select press mode",
                    h::NAV_SELECT_PRESS,
                    &mut data.select.press_mode,
                    options::SELECT_PRESS_MODES,
                );
            });
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "In graph", h::NAV_IN_GRAPH, &mut data.inputs.graph);
                text_field(ui, "In row", h::NAV_IN_ROW, &mut data.inputs.row);
                text_field(ui, "In col", h::NAV_IN_COL, &mut data.inputs.col);
                text_field(
                    ui,
                    "In collection",
                    h::NAV_IN_COLLECTION,
                    &mut data.inputs.collection,
                );
            });
            tip_wrapped_section(ui, |ui| {
                text_field(
                    ui,
                    "Output ref",
                    h::NAV_OUT_REF,
                    &mut data.outputs.output_ref,
                );
                text_field(
                    ui,
                    "Output graph",
                    h::NAV_OUT_GRAPH,
                    &mut data.outputs.output_graph,
                );
                text_field(
                    ui,
                    "Output row",
                    h::NAV_OUT_ROW,
                    &mut data.outputs.output_row,
                );
                text_field(
                    ui,
                    "Output col",
                    h::NAV_OUT_COL,
                    &mut data.outputs.output_col,
                );
                text_field(
                    ui,
                    "Output collection",
                    h::NAV_OUT_COLLECTION,
                    &mut data.outputs.output_collection,
                );
            });
            tip_section(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Nav Key children: nest Navigate Key actions under this node in the tree.",
                    )
                    .small()
                    .weak(),
                )
                .on_hover_text(h::NAV_KEY_CHILDREN);
            });
        }
        ActionKind::NavigateKey {
            name,
            chord,
            exit,
            subactions: _,
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, "Name", h::NAME, name);
                help::tip(
                    ui.checkbox(exit, "Exit Navigate Select after branch"),
                    h::NAV_KEY_EXIT,
                );
            });
            tip_section(ui, |ui| {
                string_list_field(ui, "Chord", chord, h::NAV_KEY_CHORD);
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
        help::tip(ui.label(egui::RichText::new("Items").strong()), h::IS_ITEMS);
        if ui
            .button("Add / edit…")
            .on_hover_text(h::IS_ITEMS)
            .clicked()
        {
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

/// Label + read-only value + pick button. Returns true when pick was clicked.
fn picker_display_row(ui: &mut egui::Ui, label: &str, help_text: &str, display: &str) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        ui.label(if display.is_empty() {
            "(unset)"
        } else {
            display
        });
        if pick_icon_btn(ui).clicked() {
            clicked = true;
        }
    });
    clicked
}

/// Label + editable text + pick button. Returns true when pick was clicked.
fn picker_edit_row(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    value: &mut String,
    hint: &str,
) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        help::tip(
            ui.add(
                egui::TextEdit::singleline(value)
                    .desired_width(W_TEXT)
                    .hint_text(hint),
            ),
            help_text,
        );
        if pick_icon_btn(ui).clicked() {
            clicked = true;
        }
    });
    clicked
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
    coord_picker_row(ui, "Point", h::MOVE_POINT, point, CoordKind::Point, picker);
}

fn search_area_picker_row(ui: &mut egui::Ui, area: &mut CoordinateRef, picker: &mut ActivePicker) {
    coord_picker_row(
        ui,
        "Search area",
        h::SEARCH_AREA,
        area,
        CoordKind::SearchArea,
        picker,
    );
}

fn coord_picker_row(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    coord: &mut CoordinateRef,
    kind: CoordKind,
    picker: &mut ActivePicker,
) {
    let display = if coord.is_empty() {
        "(unset)"
    } else {
        coord.as_str()
    };
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        ui.monospace(display);
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

fn combo_str(
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

fn text_field(ui: &mut egui::Ui, label: &str, help_text: &str, value: &mut String) {
    ui.horizontal(|ui| {
        help::label(ui, label, help_text);
        help::tip(
            ui.add(egui::TextEdit::singleline(value).desired_width(W_TEXT)),
            help_text,
        );
    });
}

/// Labeled DragValue (no `.prefix`); configure speed/range via `configure`.
fn drag_field<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
    value: &mut Num,
    configure: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
) {
    drag_field_enabled(ui, label, help_text, value, true, configure);
}

fn drag_field_enabled<Num: egui::emath::Numeric>(
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

fn scalar_field(
    ui: &mut egui::Ui,
    label: &str,
    help_text: &str,
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
                .desired_width(W_MULTILINE)
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
        W_MULTILINE,
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
        text_field(ui, "Name", h::NAME, &mut condition.name);
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
    tip_wrapped_section(ui, |ui| wait_editor(ui, &mut detection.wait));
    tip_wrapped_section(ui, |ui| {
        coords_editor(ui, &mut detection.coords, known_vars, is_dark);
    });
    tip_wrapped_section(ui, |ui| order_editor(ui, &mut detection.order));
    tip_wrapped_section(ui, |ui| {
        help::tip(
            ui.checkbox(
                &mut detection.run_branch_on_no_find,
                "Run branch on no find",
            ),
            h::RUN_ON_NO_FIND,
        );
    });
}

fn wait_editor(ui: &mut egui::Ui, wait: &mut WaitTilFoundConfig) {
    let mut mode = wait.repeat_mode.as_str().to_string();
    combo_str(
        ui,
        "Repeat mode",
        h::REPEAT_MODE,
        &mut mode,
        options::REPEAT_MODES,
    );
    wait.repeat_mode = RepeatMode::parse(&mode);
    // Once → all off; waituntilfound → timing only;
    // repeatwhilefound → timing + max iterations.
    let timing_enabled = wait.repeat_mode != RepeatMode::Once;
    let max_enabled = wait.is_repeat_while_found();
    drag_field_enabled(
        ui,
        "Wait seconds",
        h::WAIT_SECONDS,
        &mut wait.wait_til_found_seconds,
        timing_enabled,
        |d| d,
    );
    drag_field_enabled(
        ui,
        "Interval ms",
        h::WAIT_INTERVAL,
        &mut wait.wait_til_found_interval_ms,
        timing_enabled,
        |d| d,
    );
    drag_field_enabled(
        ui,
        "Max iterations",
        h::WAIT_MAX_ITER,
        &mut wait.max_iterations,
        max_enabled,
        |d| d,
    );
}

fn coords_editor(
    ui: &mut egui::Ui,
    coords: &mut CoordinateOutputs,
    known_vars: &HashSet<String>,
    is_dark: bool,
) {
    var_pills::var_name_text_edit(
        ui,
        "Output X",
        &mut coords.output_x_variable,
        known_vars,
        is_dark,
        W_VAR,
        h::OUT_X,
    );
    var_pills::var_name_text_edit(
        ui,
        "Output Y",
        &mut coords.output_y_variable,
        known_vars,
        is_dark,
        W_VAR,
        h::OUT_Y,
    );
}

fn order_editor(ui: &mut egui::Ui, order: &mut MatchOrder) {
    combo_str(
        ui,
        "Grouping",
        h::ORDER_GROUPING,
        &mut order.grouping,
        options::ORDER_GROUPING,
    );
    combo_str(
        ui,
        "Horizontal",
        h::ORDER_HORIZONTAL,
        &mut order.horizontal,
        options::ORDER_HORIZONTAL,
    );
    combo_str(
        ui,
        "Vertical",
        h::ORDER_VERTICAL,
        &mut order.vertical,
        options::ORDER_VERTICAL,
    );
}

/// Header row for repeatable list editors. Returns true when `+` was clicked.
fn list_header(ui: &mut egui::Ui, title: &str, add_help: &str) -> bool {
    let mut add = false;
    ui.horizontal(|ui| {
        ui.label(title);
        if ui.small_button("+").on_hover_text(add_help).clicked() {
            add = true;
        }
    });
    add
}

fn clauses_editor(
    ui: &mut egui::Ui,
    clauses: &mut Vec<ConditionClause>,
    known_vars: &HashSet<String>,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    if list_header(ui, "Clauses", h::CLAUSE_ADD) {
        clauses.push(ConditionClause::default());
    }
    let mut remove: Option<usize> = None;
    for (i, clause) in clauses.iter_mut().enumerate() {
        // Unique id so each clause's "op" ComboBox is distinct (same label salt).
        ui.push_id(i, |ui| {
            ui.horizontal(|ui| {
                scalar_field(
                    ui,
                    "L",
                    h::CLAUSE_LEFT,
                    &mut clause.left,
                    known_vars,
                    is_dark,
                    active_macro,
                );
                combo_str(
                    ui,
                    "op",
                    h::CLAUSE_OP,
                    &mut clause.operator,
                    options::CONDITIONAL_OPERATORS,
                );
                scalar_field(
                    ui,
                    "R",
                    h::CLAUSE_RIGHT,
                    &mut clause.right,
                    known_vars,
                    is_dark,
                    active_macro,
                );
                if ui
                    .small_button("−")
                    .on_hover_text(h::CLAUSE_REMOVE)
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove {
        clauses.remove(i);
    }
}

fn list_columns_editor(
    ui: &mut egui::Ui,
    sources: &mut Vec<ListColumn>,
    known_vars: &HashSet<String>,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    if list_header(ui, "Sources", h::FOREACH_ADD_SOURCE) {
        sources.push(ListColumn::default());
    }
    let mut remove: Option<usize> = None;
    for (i, col) in sources.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            if i > 0 {
                ui.separator();
            }
            theme::section_frame(ui.style()).show(ui, |ui| {
                var_ref_field(
                    ui,
                    "Source",
                    h::FOREACH_SOURCE,
                    &mut col.source,
                    known_vars,
                    is_dark,
                    W_TEXT,
                    active_macro,
                );
                var_pills::var_name_text_edit(
                    ui,
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
                if ui
                    .small_button("Remove")
                    .on_hover_text(h::FOREACH_REMOVE_SOURCE)
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove {
        sources.remove(i);
    }
}

fn assignments_editor(
    ui: &mut egui::Ui,
    assignments: &mut Vec<VariableAssignment>,
    known_vars: &HashSet<String>,
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
                        .small_button("Remove")
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
