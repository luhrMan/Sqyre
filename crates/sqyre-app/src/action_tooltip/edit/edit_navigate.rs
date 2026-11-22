//! Navigate Select action editor.

use super::edit_fields::chord_hotkey_field;
use crate::action_tooltip::sections::{tip_advanced, tip_section, tip_wrapped_section};
use crate::action_tooltip::{help, help as h};
use crate::hotkey_record::HotkeyRecordUi;
use crate::paint_ctx::{CatalogPaint, VarTheme};
use crate::recorded_action::{ChordRecordTarget, NavChordSlot};
use crate::var_pills;
use crate::widgets::{
    combo_enum, labeled_searchable_combo, text_field, SearchableComboOpts, W_VAR,
};
use eframe::egui;
use sqyre_domain::{MouseButton, NavPressMode, NavSelectDevice};
use sqyre_hotkeys::MacroHotkeyBridge;

pub(super) fn paint_navigate_select(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    theme: VarTheme<'_>,
    data: &mut sqyre_domain::NavigateSelectData,
    hotkey_record: &mut HotkeyRecordUi,
    macro_hotkeys: &MacroHotkeyBridge,
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
        let record_enabled = !hotkey_record.is_open();
        chord_hotkey_field(
            ui,
            egui_phosphor::regular::ARROW_UP,
            "Chord up",
            h::NAV_CHORD_UP,
            &mut data.chords.up,
            record_enabled,
            || {
                hotkey_record.open_for(
                    macro_hotkeys,
                    ChordRecordTarget::NavSelect(NavChordSlot::Up),
                );
            },
        );
        chord_hotkey_field(
            ui,
            egui_phosphor::regular::ARROW_DOWN,
            "Chord down",
            h::NAV_CHORD_DOWN,
            &mut data.chords.down,
            record_enabled,
            || {
                hotkey_record.open_for(
                    macro_hotkeys,
                    ChordRecordTarget::NavSelect(NavChordSlot::Down),
                );
            },
        );
        chord_hotkey_field(
            ui,
            egui_phosphor::regular::ARROW_LEFT,
            "Chord left",
            h::NAV_CHORD_LEFT,
            &mut data.chords.left,
            record_enabled,
            || {
                hotkey_record.open_for(
                    macro_hotkeys,
                    ChordRecordTarget::NavSelect(NavChordSlot::Left),
                );
            },
        );
        chord_hotkey_field(
            ui,
            egui_phosphor::regular::ARROW_RIGHT,
            "Chord right",
            h::NAV_CHORD_RIGHT,
            &mut data.chords.right,
            record_enabled,
            || {
                hotkey_record.open_for(
                    macro_hotkeys,
                    ChordRecordTarget::NavSelect(NavChordSlot::Right),
                );
            },
        );
        chord_hotkey_field(
            ui,
            egui_phosphor::regular::CHECK,
            "Chord select",
            h::NAV_CHORD_SELECT,
            &mut data.chords.select,
            record_enabled,
            || {
                hotkey_record.open_for(
                    macro_hotkeys,
                    ChordRecordTarget::NavSelect(NavChordSlot::Select),
                );
            },
        );
        chord_hotkey_field(
            ui,
            egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE,
            "Chord back",
            h::NAV_CHORD_BACK,
            &mut data.chords.back,
            record_enabled,
            || {
                hotkey_record.open_for(
                    macro_hotkeys,
                    ChordRecordTarget::NavSelect(NavChordSlot::Back),
                );
            },
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
            let VarTheme {
                known_vars,
                is_dark,
            } = theme;
            for (icon, label, value, tip) in [
                (
                    crate::model_icons::ATLAS,
                    "In atlas",
                    &mut data.inputs.atlas,
                    h::NAV_IN_ATLAS,
                ),
                (
                    egui_phosphor::regular::ROWS,
                    "In row",
                    &mut data.inputs.row,
                    h::NAV_IN_ROW,
                ),
                (
                    egui_phosphor::regular::COLUMNS,
                    "In col",
                    &mut data.inputs.col,
                    h::NAV_IN_COL,
                ),
                (
                    crate::model_icons::COLLECTION,
                    "In collection",
                    &mut data.inputs.collection,
                    h::NAV_IN_COLLECTION,
                ),
            ] {
                var_pills::var_name_text_edit(
                    ui, icon, label, value, known_vars, is_dark, W_VAR, tip,
                );
            }
        });
        tip_wrapped_section(ui, |ui| {
            let VarTheme {
                known_vars,
                is_dark,
            } = theme;
            for (icon, label, value, tip) in [
                (
                    egui_phosphor::regular::BRACKETS_CURLY,
                    "Output ref",
                    &mut data.outputs.output_ref,
                    h::NAV_OUT_REF,
                ),
                (
                    crate::model_icons::ATLAS,
                    "Output atlas",
                    &mut data.outputs.output_atlas,
                    h::NAV_OUT_ATLAS,
                ),
                (
                    egui_phosphor::regular::ROWS,
                    "Output row",
                    &mut data.outputs.output_row,
                    h::NAV_OUT_ROW,
                ),
                (
                    egui_phosphor::regular::COLUMNS,
                    "Output col",
                    &mut data.outputs.output_col,
                    h::NAV_OUT_COL,
                ),
                (
                    crate::model_icons::COLLECTION,
                    "Output collection",
                    &mut data.outputs.output_collection,
                    h::NAV_OUT_COLLECTION,
                ),
            ] {
                var_pills::var_name_text_edit(
                    ui, icon, label, value, known_vars, is_dark, W_VAR, tip,
                );
            }
        });
    });
}
