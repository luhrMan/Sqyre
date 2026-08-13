use super::edit_coord::{coord_picker_row, search_area_section};
use super::edit_fields::{scalar_field, var_ref_field};
use crate::icon_cache::IconCache;
use crate::paint_ctx::CatalogPaint;
use crate::pickers::{self, ActivePicker};
use crate::preview_tooltip::PreviewTooltipCache;
use crate::widgets::{combo_str, combo_str_labeled, drag_field, searchable_combo, text_field, W_TEXT, W_VAR};
use eframe::egui;
use sqyre_domain::{
    DetectionBranch, ListColumn, MatchOrder, RepeatMode, ScalarValue, WaitTilFoundConfig,
};
use sqyre_persist::ProgramCatalog;

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
            .button(egui::RichText::new("Add / edit…").color(theme::MACRO_START))
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
fn search_area_section(
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

fn detection_branch_editor(ui: &mut egui::Ui, detection: &mut DetectionBranch) {
    tip_wrapped_section(ui, |ui| wait_editor(ui, &mut detection.wait));
    tip_wrapped_section(ui, |ui| order_editor(ui, &mut detection.order));
}

/// Spaced repeat-mode label; *Until* italic, **While** bold.
fn repeat_mode_label(ui: &egui::Ui, mode: RepeatMode) -> egui::WidgetText {
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

fn wait_editor(ui: &mut egui::Ui, wait: &mut WaitTilFoundConfig) {
    ui.horizontal(|ui| {
        help::label(ui, "Repeat mode", h::REPEAT_MODE);
        help::tip(
            egui::ComboBox::from_id_salt("Repeat mode")
                .selected_text(repeat_mode_label(ui, wait.repeat_mode))
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
                .response,
            h::REPEAT_MODE,
        );
    });
    // Once → timing off; wait modes → timing only; repeat modes → timing + max iterations.
    let timing_enabled = wait.uses_timing();
    let max_enabled = wait.uses_max_iterations();
    drag_field_enabled(
        ui,
        "Wait seconds",
        h::WAIT_SECONDS,
        &mut wait.wait_til_found_seconds,
        timing_enabled,
        |d| d.speed(0.1).range(0.0..=1_000_000.0).max_decimals(3),
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
    known_vars: &KnownVariableNames,
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
    combo_str_labeled(
        ui,
        "Grouping",
        h::ORDER_GROUPING,
        &mut order.grouping,
        options::ORDER_GROUPING,
        "row",
    );
    combo_str_labeled(
        ui,
        "Horizontal",
        h::ORDER_HORIZONTAL,
        &mut order.horizontal,
        options::ORDER_HORIZONTAL,
        "left_to_right",
    );
    combo_str_labeled(
        ui,
        "Vertical",
        h::ORDER_VERTICAL,
        &mut order.vertical,
        options::ORDER_VERTICAL,
        "top_to_bottom",
    );
}
fn list_columns_editor(
    ui: &mut egui::Ui,
    sources: &mut Vec<ListColumn>,
    known_vars: &KnownVariableNames,
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
                    .add(
                        egui::Button::new(egui::RichText::new("Remove").color(theme::MACRO_STOP))
                            .small(),
                    )
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
