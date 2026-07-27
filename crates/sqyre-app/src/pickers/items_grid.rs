use super::icon_grid::paint_even_icon_grid;
use super::query::{fuzzy_match_fold, query_matches_name_or_tags};
use crate::icon_cache::IconCache;
use eframe::egui;
use sqyre_domain::PROGRAM_DELIMITER;
use sqyre_persist::ProgramCatalog;

pub fn items_icon_grid_collapse_id(program: &str) -> egui::Id {
    egui::Id::new(("items_icon_grid", program))
}

/// Set open/closed for every program group in the items icon grid.
pub fn set_items_icon_grid_openness<'a>(
    ctx: &egui::Context,
    programs: impl IntoIterator<Item = &'a str>,
    open: bool,
) {
    set_collapsing_openness(
        ctx,
        programs.into_iter().map(items_icon_grid_collapse_id),
        open,
    );
}

/// Store open/closed for each collapsing id (expand/collapse-all).
pub fn set_collapsing_openness(
    ctx: &egui::Context,
    ids: impl IntoIterator<Item = egui::Id>,
    open: bool,
) {
    for id in ids {
        let mut state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id, true);
        state.set_open(open);
        state.store(ctx);
    }
}

/// Program accordion of item icon grids. Click toggles membership in `selected` when
/// `multi` is true; otherwise replaces selection with the clicked target.
/// When `multi`, each program header includes an All control over filtered targets
/// (items picker tri-state / All button).
///
/// When `selected_program` / `clicked_program` are used (data editor), program headers
/// are selectable and write the clicked program name into `clicked_program`.
#[allow(clippy::too_many_arguments)]
pub fn paint_items_icon_grid(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    search: &str,
    selected: &mut Vec<String>,
    multi: bool,
    selected_program: Option<&str>,
    clicked_program: &mut Option<String>,
) {
    let q = search.trim().to_ascii_lowercase();
    let pane_w = ui.available_width();
    ui.set_max_width(pane_w);
    for prog in catalog.program_names() {
        let Some(pdata) = catalog.get(prog) else {
            continue;
        };
        let items: Vec<_> = pdata
            .items
            .iter()
            .filter(|(name, item)| {
                if q.is_empty() {
                    return true;
                }
                fuzzy_match_fold(&q, prog) || query_matches_name_or_tags(&q, name, &item.tags)
            })
            .map(|(name, _)| name.clone())
            .collect();
        if items.is_empty() {
            continue;
        }
        let mut targets: Vec<(String, String)> = items
            .iter()
            .map(|item_key| {
                let display = pdata
                    .items
                    .get(item_key)
                    .map(|it| {
                        if it.name.trim().is_empty() {
                            item_key.clone()
                        } else {
                            it.name.clone()
                        }
                    })
                    .unwrap_or_else(|| item_key.clone());
                (format!("{prog}{PROGRAM_DELIMITER}{item_key}"), display)
            })
            .collect();
        sort_by_display_name(&mut targets);
        let targets: Vec<String> = targets.into_iter().map(|(t, _)| t).collect();

        let selected_in_group = targets
            .iter()
            .filter(|t| selected.iter().any(|s| s == *t))
            .count();
        let all_label = if selected_in_group == 0 {
            "All"
        } else if selected_in_group == targets.len() {
            "None"
        } else {
            "All"
        };

        // Absolute id so expand/collapse-all (outside this ui stack) can target the same state.
        let id = items_icon_grid_collapse_id(prog);
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                let prog_selected = selected_program == Some(prog.as_str());
                if crate::icon_cache::paint_program_label(
                    ui,
                    catalog,
                    icons,
                    prog,
                    crate::icon_cache::ProgramLabelStyle::Header {
                        selected: Some(prog_selected),
                        child_count: pdata.items.len(),
                    },
                )
                .clicked()
                {
                    *clicked_program = Some(prog.clone());
                }
            })
            .body(|ui| {
                ui.set_max_width(pane_w);
                if multi {
                    ui.horizontal(|ui| {
                        if ui
                            .small_button(all_label)
                            .on_hover_text(
                                "Select all visible items, or deselect all if all visible are selected",
                            )
                            .clicked()
                        {
                            toggle_select_all_filtered(selected, &targets);
                        }
                        if selected_in_group > 0 && selected_in_group < targets.len() {
                            ui.weak(format!("{selected_in_group}/{}", targets.len()));
                        }
                    });
                }
                let mut clicked: Option<String> = None;
                paint_even_icon_grid(
                    ui,
                    catalog,
                    icons,
                    &targets,
                    |t| selected.iter().any(|s| s == t),
                    false,
                    |_i, t| {
                        clicked = Some(t.to_string());
                    },
                    |_| {},
                );
                if let Some(target) = clicked {
                    let is_sel = selected.iter().any(|t| t == &target);
                    if multi {
                        if is_sel {
                            selected.retain(|t| t != &target);
                        } else {
                            selected.push(target);
                        }
                    } else {
                        *selected = vec![target];
                    }
                }
            });
    }
}

/// Toggle: if every `filtered` target is selected, remove them; otherwise add missing ones.
pub(crate) fn toggle_select_all_filtered(selected: &mut Vec<String>, filtered: &[String]) {
    if filtered.is_empty() {
        return;
    }
    let all_selected = filtered.iter().all(|t| selected.iter().any(|s| s == t));
    if all_selected {
        selected.retain(|s| !filtered.iter().any(|t| t == s));
    } else {
        for t in filtered {
            if !selected.iter().any(|s| s == t) {
                selected.push(t.clone());
            }
        }
    }
}

/// Sort `(key, display_name)` rows by display name (case-insensitive), then key.
pub(crate) fn sort_by_display_name(rows: &mut [(String, String)]) {
    rows.sort_by(|a, b| {
        a.1.to_ascii_lowercase()
            .cmp(&b.1.to_ascii_lowercase())
            .then_with(|| a.0.cmp(&b.0))
    });
}
