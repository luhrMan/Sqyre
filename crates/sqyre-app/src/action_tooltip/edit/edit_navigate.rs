//! Navigate Select action editor.

use super::edit_fields::string_list_field;
use crate::action_tooltip::sections::{tip_advanced, tip_section, tip_wrapped_section};
use crate::action_tooltip::{help, help as h};
use crate::paint_ctx::{CatalogPaint, VarTheme};
use crate::widgets::{
    combo_enum, labeled_searchable_combo, num_or_var_field, text_field, NumOrVarOpts,
    SearchableComboOpts, W_TEXT,
};
use eframe::egui;
use sqyre_domain::{MouseButton, NavPressMode, NavSelectDevice};
use sqyre_validate::EntryValidation;

pub(super) fn paint_navigate_select(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    theme: VarTheme<'_>,
    data: &mut sqyre_domain::NavigateSelectData,
) {
    tip_wrapped_section(ui, |ui| {
        {
            let programs: Vec<String> = paint.catalog.program_names().cloned().collect();
            let mut option_icon =
                |ctx: &egui::Context, name: &str| paint.icons.for_program(ctx, paint.catalog, name);
            let _ = labeled_searchable_combo(
                ui,
                crate::model_icons::PROGRAM,
                "Program",
                h::NAV_PROGRAM,
                "nav_program",
                &mut data.program,
                SearchableComboOpts {
                    options: &programs,
                    empty_text: sqyre_domain::EMPTY_NONE,
                    none_label: Some(sqyre_domain::EMPTY_NONE),
                    width: None,
                    on_option_hover: None,
                    option_icon: Some(&mut option_icon),
                    closed_icon: true,
                    option_label: None,
                },
                None,
            );
        }
        {
            let atlases: Vec<String> = if data.program.trim().is_empty() {
                Vec::new()
            } else {
                paint
                    .catalog
                    .get(data.program.trim())
                    .map(|p| p.atlases.keys().cloned().collect())
                    .unwrap_or_default()
            };
            let _ = labeled_searchable_combo(
                ui,
                crate::model_icons::ATLAS,
                "Atlas",
                h::NAV_ATLAS,
                "nav_atlas",
                &mut data.atlas,
                SearchableComboOpts {
                    options: &atlases,
                    empty_text: sqyre_domain::EMPTY_NONE,
                    none_label: None,
                    width: None,
                    on_option_hover: None,
                    option_icon: None,
                    closed_icon: true,
                    option_label: None,
                },
                None,
            );
        }
    });
    tip_section(ui, |ui| {
        string_list_field(
            ui,
            egui_phosphor::regular::ARROW_UP,
            "Chord up",
            &mut data.chords.up,
            h::NAV_CHORD_UP,
        );
        string_list_field(
            ui,
            egui_phosphor::regular::ARROW_DOWN,
            "Chord down",
            &mut data.chords.down,
            h::NAV_CHORD_DOWN,
        );
        string_list_field(
            ui,
            egui_phosphor::regular::ARROW_LEFT,
            "Chord left",
            &mut data.chords.left,
            h::NAV_CHORD_LEFT,
        );
        string_list_field(
            ui,
            egui_phosphor::regular::ARROW_RIGHT,
            "Chord right",
            &mut data.chords.right,
            h::NAV_CHORD_RIGHT,
        );
        string_list_field(
            ui,
            egui_phosphor::regular::CHECK,
            "Chord select",
            &mut data.chords.select,
            h::NAV_CHORD_SELECT,
        );
        string_list_field(
            ui,
            egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE,
            "Chord back",
            &mut data.chords.back,
            h::NAV_CHORD_BACK,
        );
    });
    tip_section(ui, |ui| {
        help::tip(
            help::label(
                ui,
                egui_phosphor::regular::TREE_STRUCTURE,
                "Nav Key children",
            ),
            h::NAV_KEY_CHILDREN,
        );
    });
    tip_advanced(ui, |ui| {
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
            combo_enum(
                ui,
                egui_phosphor::regular::MOUSE,
                "Select device",
                h::NAV_SELECT_DEVICE,
                &mut data.select.device,
                NavSelectDevice::ALL,
                NavSelectDevice::label,
            );
            combo_enum(
                ui,
                egui_phosphor::regular::MOUSE,
                "Select button",
                h::NAV_SELECT_BUTTON,
                &mut data.select.button,
                MouseButton::ALL,
                MouseButton::label,
            );
            text_field(
                ui,
                egui_phosphor::regular::KEY,
                "Select key",
                h::NAV_SELECT_KEY,
                &mut data.select.key,
            );
            combo_enum(
                ui,
                egui_phosphor::regular::HAND_POINTING,
                "Select press mode",
                h::NAV_SELECT_PRESS,
                &mut data.select.press_mode,
                NavPressMode::ALL,
                NavPressMode::label,
            );
        });
        tip_wrapped_section(ui, |ui| {
            text_field(
                ui,
                crate::model_icons::ATLAS,
                "In atlas",
                h::NAV_IN_ATLAS,
                &mut data.inputs.atlas,
            );
            let no_block = EntryValidation::default();
            num_or_var_field(
                ui,
                egui_phosphor::regular::ROWS,
                "In row",
                &mut data.inputs.row,
                NumOrVarOpts {
                    width: W_TEXT,
                    range: Some(1.0..=i32::MAX as f64),
                    theme,
                    validation: &no_block,
                    help: h::NAV_IN_ROW,
                },
            );
            num_or_var_field(
                ui,
                egui_phosphor::regular::COLUMNS,
                "In col",
                &mut data.inputs.col,
                NumOrVarOpts {
                    width: W_TEXT,
                    range: Some(1.0..=i32::MAX as f64),
                    theme,
                    validation: &no_block,
                    help: h::NAV_IN_COL,
                },
            );
            text_field(
                ui,
                crate::model_icons::COLLECTION,
                "In collection",
                h::NAV_IN_COLLECTION,
                &mut data.inputs.collection,
            );
        });
        tip_wrapped_section(ui, |ui| {
            text_field(
                ui,
                egui_phosphor::regular::BRACKETS_CURLY,
                "Output ref",
                h::NAV_OUT_REF,
                &mut data.outputs.output_ref,
            );
            text_field(
                ui,
                crate::model_icons::ATLAS,
                "Output atlas",
                h::NAV_OUT_ATLAS,
                &mut data.outputs.output_atlas,
            );
            text_field(
                ui,
                egui_phosphor::regular::ROWS,
                "Output row",
                h::NAV_OUT_ROW,
                &mut data.outputs.output_row,
            );
            text_field(
                ui,
                egui_phosphor::regular::COLUMNS,
                "Output col",
                h::NAV_OUT_COL,
                &mut data.outputs.output_col,
            );
            text_field(
                ui,
                crate::model_icons::COLLECTION,
                "Output collection",
                h::NAV_OUT_COLLECTION,
                &mut data.outputs.output_collection,
            );
        });
    });
}
