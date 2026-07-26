//! Control-flow action editors (loop, while, if, for-each-row, break/continue).

use crate::action_tooltip::sections::{tip_section, tip_wrapped_section};
use super::{help as h, condition_editor, list_columns_editor, scalar_field};
use crate::pickers::options;
use crate::widgets::{combo_str_labeled, drag_field, text_field};
use eframe::egui;
use sqyre_domain::{
    ConditionBlock, KnownVariableNames, ListColumn, LoopJumpMode, Macro, ScalarValue,
};

pub(super) fn paint_loop_jump(ui: &mut egui::Ui, mode: &mut LoopJumpMode) {
    tip_wrapped_section(ui, |ui| {
        let mut mode_s = mode.as_str().to_string();
        combo_str_labeled(
            ui,
            "Mode",
            h::LOOP_JUMP_MODE,
            &mut mode_s,
            options::LOOP_JUMP_MODES,
            "break",
        );
        *mode = LoopJumpMode::parse(&mode_s);
    });
}

pub(super) fn paint_loop(
    ui: &mut egui::Ui,
    name: &mut String,
    count: &mut ScalarValue,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
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

pub(super) fn paint_while(
    ui: &mut egui::Ui,
    condition: &mut ConditionBlock,
    max_iterations: &mut i32,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
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

pub(super) fn paint_conditional(
    ui: &mut egui::Ui,
    condition: &mut ConditionBlock,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
    condition_editor(ui, condition, known_vars, is_dark, active_macro, |_| {});
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_foreach_row(
    ui: &mut egui::Ui,
    name: &mut String,
    sources: &mut Vec<ListColumn>,
    start_row: &mut ScalarValue,
    end_row: &mut ScalarValue,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    active_macro: Option<&Macro>,
) {
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
