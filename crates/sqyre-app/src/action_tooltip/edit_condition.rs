use crate::widgets::{combo_condition_operator, combo_str, text_field, W_TEXT};
use eframe::egui;
use sqyre_domain::{ConditionBlock, ConditionClause, MatchMode, ScalarValue};

fn condition_editor(
    ui: &mut egui::Ui,
    condition: &mut ConditionBlock,
    known_vars: &KnownVariableNames,
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
/// Header row for repeatable list editors. Returns true when `+` was clicked.
fn list_header(ui: &mut egui::Ui, title: &str, add_help: &str) -> bool {
    let mut add = false;
    ui.horizontal(|ui| {
        ui.label(title);
        if ui
            .add(egui::Button::new(egui::RichText::new("+").color(theme::MACRO_START)).small())
            .on_hover_text(add_help)
            .clicked()
        {
            add = true;
        }
    });
    add
}

fn clauses_editor(
    ui: &mut egui::Ui,
    clauses: &mut Vec<ConditionClause>,
    known_vars: &KnownVariableNames,
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
                combo_condition_operator(ui, "op", h::CLAUSE_OP, &mut clause.operator);
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
                    .add(
                        egui::Button::new(egui::RichText::new("−").color(theme::MACRO_STOP))
                            .small(),
                    )
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
