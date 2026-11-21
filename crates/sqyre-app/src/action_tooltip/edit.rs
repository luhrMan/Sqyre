//! Staged field editors and draft apply (preserve `subactions`).

mod edit_control_flow;
mod edit_detect;
mod edit_fields;
mod edit_navigate;
mod edit_pickers;

use self::edit_fields::{
    assignments_editor, scalar_field, string_list_field, string_list_field_opts, var_ref_field,
};
use self::edit_pickers::{
    cells_picker_row, paint_coord_preview, picker_display_row, point_picker_row,
};
use super::sections::{tip_advanced, tip_section, tip_wrapped_section};
use super::{help, help as h};
use crate::icon_cache::IconCache;
use crate::paint_ctx::{EditFieldsCtx, RecordBridges};
use crate::pickers::{self, ActivePicker};
use crate::preview_tooltip::PreviewKind;
use crate::tree_chrome;
use crate::var_pills::{self};
use crate::widgets::{combo_enum, drag_field, text_field, wrap_unit, wrapped_row, W_TEXT, W_VAR};
use eframe::egui;
use sqyre_domain::{
    Action, ActionKind, ItemSortBy, ItemSortThen, MAX_SMOOTH_DELAY_MS, MAX_TYPE_DELAY_MS,
    MIN_SMOOTH_DELAY_MS, MIN_TYPE_DELAY_MS,
};
use sqyre_persist::CatalogView;

/// Copy draft fields onto `live`, keeping `live`'s then/else children.
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
    let preserved_then = live.children().to_vec();
    let preserved_else = live.else_children().map(|c| c.to_vec()).unwrap_or_default();
    live.kind = draft.kind;
    if let Some(kids) = live.children_mut() {
        *kids = preserved_then;
    }
    if let Some(kids) = live.else_children_mut() {
        *kids = preserved_else;
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
    let RecordBridges {
        key_record,
        hotkey_record,
        macro_hotkeys,
        screen_click,
    } = bridges;
    let active_macro = *active_macro;
    match &mut draft.kind {
        ActionKind::LoopJump { mode } => edit_control_flow::paint_loop_jump(ui, mode),
        ActionKind::Wait { time } => {
            tip_wrapped_section(ui, |ui| {
                scalar_field(
                    ui,
                    egui_phosphor::regular::TIMER,
                    "Time (ms)",
                    h::WAIT_TIME,
                    time,
                    theme.known_vars,
                    theme.is_dark,
                    active_macro,
                );
            });
        }
        ActionKind::Click { button, state } => {
            tip_section(ui, |ui| {
                // `ui.horizontal` starts at interact_size.y and grows to the
                // mouse/slider — not `with_layout(… Align::Center)`, which fills
                // ScrollArea's unbounded height and ratchets the tip to the screen.
                ui.horizontal(|ui| {
                    help::tip(
                        crate::widgets::mouse_button_picker(ui, button),
                        h::CLICK_BUTTON,
                    );
                    crate::widgets::press_state_column(ui, state, h::CLICK_STATE);
                });
            });
        }
        ActionKind::Key { key, state } => {
            tip_wrapped_section(ui, |ui| {
                var_ref_field(
                    ui,
                    egui_phosphor::regular::KEY,
                    "Key",
                    key,
                    *theme,
                    h::KEY,
                    W_VAR,
                    active_macro,
                );
                // Own unit so the button wraps below the field instead of
                // overflowing the tip's clip on narrow panels.
                wrap_unit(ui, |ui| {
                    if crate::widgets::record_icon_button(ui, "Record a key", !key_record.is_open())
                        .clicked()
                    {
                        key_record.open(macro_hotkeys);
                    }
                });
                wrap_unit(ui, |ui| {
                    crate::widgets::press_state_column(ui, state, h::KEY_STATE);
                });
            });
        }
        ActionKind::Type { text, delay_ms } => {
            tip_wrapped_section(ui, |ui| {
                var_ref_field(
                    ui,
                    egui_phosphor::regular::KEYBOARD,
                    "Text",
                    text,
                    *theme,
                    h::TYPE_TEXT,
                    W_VAR,
                    active_macro,
                );
                drag_field(
                    ui,
                    egui_phosphor::regular::TIMER,
                    "Delay ms",
                    h::TYPE_DELAY,
                    delay_ms,
                    |d| d.speed(1).range(MIN_TYPE_DELAY_MS..=MAX_TYPE_DELAY_MS),
                );
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
                point_picker_row(ui, &mut *paint, point, picker);
                paint_coord_preview(ui, paint.catalog, paint.previews, point, PreviewKind::Point);
            });
            tip_wrapped_section(ui, |ui| {
                help::tip(ui.checkbox(smooth, "Smooth"), h::MOVE_SMOOTH);
            });
            tip_advanced(ui, |ui| {
                tip_wrapped_section(ui, |ui| {
                    drag_field(
                        ui,
                        egui_phosphor::regular::TIMER,
                        "Smooth low",
                        h::MOVE_SMOOTH_LOW,
                        smooth_low,
                        |d| d.speed(0.01).range(0.0..=1.0),
                    );
                    drag_field(
                        ui,
                        egui_phosphor::regular::TIMER,
                        "Smooth high",
                        h::MOVE_SMOOTH_HIGH,
                        smooth_high,
                        |d| d.speed(0.01).range(0.0..=1.0),
                    );
                    drag_field(
                        ui,
                        egui_phosphor::regular::TIMER,
                        "Smooth delay ms",
                        h::MOVE_SMOOTH_DELAY,
                        smooth_delay_ms,
                        |d| d.speed(1).range(MIN_SMOOTH_DELAY_MS..=MAX_SMOOTH_DELAY_MS),
                    );
                });
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
                    egui_phosphor::regular::CHAT_TEXT,
                    "Message",
                    message,
                    *theme,
                    h::PAUSE_MESSAGE,
                    W_TEXT,
                    active_macro,
                );
            });
            tip_section(ui, |ui| {
                let record_enabled = !hotkey_record.is_open() && !key_record.is_open();
                let mut record_clicked = false;
                let mut trailing = |ui: &mut egui::Ui| {
                    if crate::widgets::record_icon_button(
                        ui,
                        "Record continue chord",
                        record_enabled,
                    )
                    .clicked()
                    {
                        record_clicked = true;
                    }
                };
                string_list_field_opts(
                    ui,
                    egui_phosphor::regular::KEYBOARD,
                    "Continue keys (one per line)",
                    continue_key,
                    h::PAUSE_CONTINUE,
                    Some(&mut trailing),
                );
                if record_clicked {
                    hotkey_record.open(macro_hotkeys);
                }
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
            previous,
        } => {
            tip_wrapped_section(ui, |ui| {
                help::tip(ui.checkbox(previous, "Previous window"), h::FOCUS_PREVIOUS);
            });
            // Stack vertically; path wraps so long executables stay inside the tip.
            if !*previous {
                tip_section(ui, |ui| {
                    crate::widgets::text_field_opts(
                        ui,
                        crate::model_icons::WINDOW,
                        "Window title",
                        h::FOCUS_TITLE,
                        window_title,
                        crate::widgets::TextFieldOpts {
                            width: f32::INFINITY,
                        },
                    );
                    let mut pick = false;
                    crate::widgets::pair_leading(
                        ui,
                        |ui| {
                            pick = crate::widgets::entity_pick_button(
                                ui,
                                egui_phosphor::regular::TERMINAL_WINDOW,
                                "Pick process window…",
                            )
                            .clicked();
                        },
                        |ui| {
                            crate::icon_cache::paint_leading_process_icon(
                                ui,
                                paint.icons,
                                process_path,
                                window_title,
                            );
                            // Paths never contain newlines; disable Return so Enter
                            // doesn't insert one. Multiline still wraps soft-breaks.
                            if process_path.contains(['\n', '\r']) {
                                process_path.retain(|c| c != '\n' && c != '\r');
                            }
                            let width = crate::widgets::fields::finite_room(ui).max(24.0);
                            let path_rows = path_wrap_rows(width, process_path);
                            help::tip(
                                ui.add(
                                    egui::TextEdit::multiline(process_path)
                                        .desired_width(width)
                                        .desired_rows(path_rows)
                                        .return_key(None)
                                        .hint_text("/path/to/app"),
                                ),
                                h::FOCUS_PROCESS,
                            );
                        },
                    );
                    if pick {
                        *picker = pickers::open_window_picker(process_path, window_title);
                    }
                });
            }
        }
        ActionKind::RunMacro { macro_name } => {
            tip_wrapped_section(ui, |ui| {
                if picker_display_row(
                    ui,
                    crate::model_icons::MACRO,
                    "Macro",
                    h::RUN_MACRO,
                    macro_name.as_str(),
                ) {
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
                assignments_editor(
                    ui,
                    assignments,
                    theme.known_vars,
                    theme.is_dark,
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
                    egui_phosphor::regular::BRACKETS_CURLY,
                    "Variable",
                    variable_name,
                    theme.known_vars,
                    theme.is_dark,
                    W_VAR,
                    h::SAVE_VAR,
                );
                var_ref_field(
                    ui,
                    egui_phosphor::regular::FOLDER,
                    "Destination",
                    destination,
                    *theme,
                    h::SAVE_DEST,
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
            edit_control_flow::paint_loop(
                ui,
                name,
                count,
                theme.known_vars,
                theme.is_dark,
                active_macro,
            );
        }
        ActionKind::While {
            condition,
            max_iterations,
            ..
        } => {
            edit_control_flow::paint_while(
                ui,
                condition,
                max_iterations,
                theme.known_vars,
                theme.is_dark,
                active_macro,
            );
        }
        ActionKind::Conditional { condition, .. } => {
            edit_control_flow::paint_conditional(
                ui,
                condition,
                theme.known_vars,
                theme.is_dark,
                active_macro,
            );
        }
        ActionKind::ForEachRow {
            name,
            sources,
            start_row,
            end_row,
            ..
        } => {
            edit_control_flow::paint_foreach_row(
                ui,
                edit_control_flow::ForEachRowFields {
                    name,
                    sources,
                    start_row,
                    end_row,
                },
                *theme,
                active_macro,
            );
        }
        ActionKind::ForEachCell { name, cells, .. } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, egui_phosphor::regular::TEXT_T, "Name", h::NAME, name);
            });
            tip_section(ui, |ui| {
                cells_picker_row(ui, &mut *paint, cells, picker);
                paint_coord_preview(
                    ui,
                    paint.catalog,
                    paint.previews,
                    cells,
                    PreviewKind::SearchArea,
                );
            });
        }
        ActionKind::ImageSearch(d) => {
            edit_detect::paint_image_search_fields(ui, &mut *paint, picker, *theme, d);
        }
        ActionKind::Ocr(d) => {
            edit_detect::paint_ocr_fields(ui, &mut *paint, picker, *theme, active_macro, d);
        }
        ActionKind::FindPixel(d) => {
            edit_detect::paint_find_pixel_fields(
                ui,
                &mut *paint,
                picker,
                *theme,
                active_macro,
                screen_click,
                d,
            );
        }
        ActionKind::NavigateSelect(data) => {
            edit_navigate::paint_navigate_select(ui, &mut *paint, *theme, data);
        }
        ActionKind::NavigateKey {
            name,
            chord,
            exit,
            subactions: _,
        } => {
            tip_wrapped_section(ui, |ui| {
                text_field(ui, egui_phosphor::regular::TEXT_T, "Name", h::NAME, name);
                help::tip(
                    ui.checkbox(exit, "Exit Navigate Select after branch"),
                    h::NAV_KEY_EXIT,
                );
            });
            tip_section(ui, |ui| {
                string_list_field(
                    ui,
                    egui_phosphor::regular::KEYBOARD,
                    "Chord",
                    chord,
                    h::NAV_KEY_CHORD,
                );
            });
        }
    }
}

fn targets_editor(
    ui: &mut egui::Ui,
    catalog: CatalogView<'_>,
    icons: &mut IconCache,
    edit: TargetsSortEdit<'_>,
    picker: &mut ActivePicker,
) {
    let TargetsSortEdit {
        targets,
        target_tags,
        sort_by,
        sort_then,
        tag_priority,
    } = edit;
    wrapped_row(ui, |ui| {
        wrap_unit(ui, |ui| {
            help::tip(ui.label(egui::RichText::new("Items").strong()), h::IS_ITEMS);
            crate::widgets::count_suffix(ui, targets.len());
            if !target_tags.is_empty() {
                help::tip(
                    ui.label(
                        egui::RichText::new(format!("· {} tag filters", target_tags.len())).weak(),
                    ),
                    h::IS_TARGET_TAGS,
                );
            }
        });
        if crate::widgets::entity_pick_button(ui, crate::model_icons::ITEM, h::IS_ITEMS).clicked() {
            *picker = ActivePicker::Items {
                search: String::new(),
                staged: targets.clone(),
                staged_tags: Some(target_tags.clone()),
            };
        }
        combo_enum(
            ui,
            egui_phosphor::regular::SORT_ASCENDING,
            "Sort by",
            h::IS_SEARCH_SORTING,
            sort_by,
            ItemSortBy::ALL,
            ItemSortBy::label,
        );
        if *sort_by != ItemSortBy::Manual {
            combo_enum(
                ui,
                egui_phosphor::regular::ARROWS_DOWN_UP,
                "Then",
                h::IS_SORT_THEN,
                sort_then,
                ItemSortThen::ALL,
                ItemSortThen::label,
            );
        }
    });
    if *sort_by == ItemSortBy::Tags {
        let suggestions = item_tag_suggestions(ui.ctx(), catalog, targets, target_tags);
        let draft_id = ui.id().with("image_search_tag_priority_draft");
        let mut draft = ui
            .ctx()
            .data(|d| d.get_temp::<String>(draft_id))
            .unwrap_or_default();
        help::tip(
            ui.label(egui::RichText::new("Tag priority").strong()),
            h::IS_TAG_PRIORITY,
        );
        let program_icons = icons.program_name_icons(ui.ctx(), catalog);
        let edit = crate::widgets::tag_chip_editor(
            ui,
            tag_priority,
            &mut draft,
            &suggestions,
            crate::widgets::TagChipOptions {
                enabled: true,
                show_add_button: true,
                label_name: None,
                draft_hover: Some(h::IS_TAG_PRIORITY),
                draft_hint: None,
                reorderable: true,
                signed_filters: false,
                single_row: false,
                locked: &[],
                locked_hover: None,
                program_icons: Some(&program_icons),
                draft_width: None,
                draft_id: None,
            },
        );
        let _ = edit;
        ui.ctx().data_mut(|d| d.insert_temp(draft_id, draft));
    }
    let display = tree_chrome::cached_sorted_image_search_targets(
        ui.ctx(),
        catalog,
        targets,
        target_tags,
        *sort_by,
        *sort_then,
        tag_priority,
    );
    if display.is_empty() && targets.is_empty() && target_tags.is_empty() {
        ui.label(sqyre_domain::EMPTY_NONE);
        return;
    }
    if display.is_empty() {
        ui.label(egui::RichText::new("(no items match these tag filters yet)").weak());
        return;
    }
    let infos = target_sort_infos(catalog, &display);
    let mut remove: Option<usize> = None;
    let mut reorder: Option<(usize, usize)> = None;
    let mut on_remove = |i| {
        remove = Some(i);
    };
    let mut on_reorder = |from: usize, to: usize| {
        reorder = Some((from, to));
    };
    // Drag-reorder only when the grid is exactly the explicit target list
    // (tag expansion would make display indices disagree with stored order).
    let allow_reorder = target_tags.is_empty();
    // Tag-filter matches are not stored in `targets`; Remove would be a no-op.
    let is_removable = |t: &str| targets.iter().any(|explicit| explicit == t);
    pickers::paint_even_icon_grid(
        ui,
        catalog,
        icons,
        &display,
        |_| false,
        pickers::IconGridKind::Targets { removable: true },
        pickers::IconGridOps {
            on_remove: Some(&mut on_remove),
            on_reorder: allow_reorder.then_some(&mut on_reorder),
            is_removable: Some(&is_removable),
            ..Default::default()
        },
        None,
    );
    if let Some((from, to)) = reorder {
        let _ = sqyre_domain::apply_display_reorder(
            targets,
            sort_by,
            *sort_then,
            tag_priority,
            &infos,
            from,
            to,
        );
    }
    if let Some(i) = remove {
        if let Some(target) = display.get(i) {
            targets.retain(|t| t != target);
        }
    }
}

struct TargetsSortEdit<'a> {
    targets: &'a mut Vec<String>,
    target_tags: &'a mut Vec<String>,
    sort_by: &'a mut ItemSortBy,
    sort_then: &'a mut ItemSortThen,
    tag_priority: &'a mut Vec<String>,
}

fn target_sort_infos(
    catalog: CatalogView<'_>,
    targets: &[String],
) -> Vec<sqyre_domain::ItemSortInfo> {
    targets
        .iter()
        .map(|target| {
            let (name, tags) = pickers::item_tooltip_parts(catalog, target);
            let (rows, cols) = catalog
                .item_meta(target)
                .map(|m| (m.rows, m.cols))
                .unwrap_or((1, 1));
            sqyre_domain::ItemSortInfo::from_parts(target.clone(), name, rows, cols, tags)
        })
        .collect()
}

fn item_tag_suggestions(
    ctx: &egui::Context,
    catalog: CatalogView<'_>,
    targets: &[String],
    target_tags: &[String],
) -> Vec<String> {
    let mut tags = Vec::new();
    for target in tree_chrome::cached_sorted_image_search_targets(
        ctx,
        catalog,
        targets,
        target_tags,
        ItemSortBy::Manual,
        ItemSortThen::ListOrder,
        &[],
    ) {
        let (_, item_tags) = pickers::item_tooltip_parts(catalog, &target);
        for tag in item_tags {
            if !tags.iter().any(|t| t == &tag) {
                tags.push(tag);
            }
        }
    }
    for tag in target_tags {
        if let Some(name) = sqyre_domain::tag_filter_name(tag) {
            if !tags.iter().any(|t| t == &name) {
                tags.push(name);
            }
        }
    }
    tags.sort_by_key(|t| t.to_ascii_lowercase());
    tags
}

/// Soft-wrap row count for a process path in the Focus Window tip.
fn path_wrap_rows(available_width: f32, path: &str) -> usize {
    // ~7px per monospace-ish char at default tip font size; clamp so the tip
    // stays compact for short paths and readable for Steam/Proton-length ones.
    let chars_per_line = (available_width / 7.0).floor().max(24.0) as usize;
    let len = path.chars().count().max(1);
    len.div_ceil(chars_per_line).clamp(1, 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{PressState, ScalarValue};

    #[test]
    fn apply_rejects_type_change() {
        let mut live = Action::new(ActionKind::Wait {
            time: ScalarValue::Int(1),
        });
        let draft = Action {
            id: live.id,
            kind: ActionKind::Click {
                button: "left".into(),
                state: PressState::Down,
            },
        };
        assert!(apply_draft_preserving_children(&mut live, draft).is_err());
    }
}
