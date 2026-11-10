use super::items_grid::set_collapsing_openness;
use super::query::fuzzy_match_fold;
use super::scroll::maybe_scroll_to;
use super::types::{CollectionCellPick, CoordKind};
use crate::paint_ctx::CatalogPaint;
use crate::preview_tooltip::PreviewKind;
use eframe::egui;
use sqyre_domain::{CoordinateRef, PROGRAM_DELIMITER};

pub fn coord_list_collapse_id(kind: CoordKind, program: &str) -> egui::Id {
    let kind_key = match kind {
        CoordKind::Point => "point",
        CoordKind::SearchArea => "search_area",
    };
    egui::Id::new(("coord_ref_list", kind_key, program))
}

/// Set open/closed for every program group in a coord picker list.
pub fn set_coord_list_openness<'a>(
    ctx: &egui::Context,
    kind: CoordKind,
    programs: impl IntoIterator<Item = &'a str>,
    open: bool,
) {
    set_collapsing_openness(
        ctx,
        programs
            .into_iter()
            .map(|p| coord_list_collapse_id(kind, p)),
        open,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "picker list: query, selection, kind, and scroll in one paint"
)]
pub fn paint_coord_ref_list(
    ui: &mut egui::Ui,
    paint: &mut CatalogPaint<'_>,
    search: &str,
    current: &mut String,
    kind: CoordKind,
    cell_pick: &mut Option<CollectionCellPick>,
    scroll_to_selection: &mut bool,
    compact_program_headers: bool,
) {
    let CatalogPaint {
        catalog,
        icons,
        previews,
        ..
    } = paint;
    let q = search.trim().to_ascii_lowercase();
    let res = catalog.resolution_key().to_string();
    let preview_kind = match kind {
        CoordKind::Point => PreviewKind::Point,
        CoordKind::SearchArea => PreviewKind::SearchArea,
    };
    let current_ref = CoordinateRef(current.clone());
    let mut did_scroll = false;
    // Caller owns the scroll (picker_searchable_scroll / fill_resize body).
    let list_w = crate::widgets::visible_width(ui);
    let entity_plural = match kind {
        CoordKind::Point => "points",
        CoordKind::SearchArea => "search areas",
    };
    ui.set_max_width(list_w);
    let mut visible = 0usize;
    for prog in catalog.program_names() {
        let Some(pdata) = catalog.get(prog) else {
            continue;
        };
        #[derive(Clone)]
        enum Row {
            Coord { key: String, display: String },
            Collection(sqyre_persist::ProgramCollection),
        }
        let mut rows: Vec<(String, Row)> = Vec::new();
        let entries: Vec<(&String, &str)> = match kind {
            CoordKind::Point => pdata
                .points
                .get(&res)
                .or_else(|| pdata.points.values().next())
                .map(|m| m.iter().map(|(k, pt)| (k, pt.name.as_str())).collect())
                .unwrap_or_default(),
            CoordKind::SearchArea => pdata
                .search_areas
                .get(&res)
                .or_else(|| pdata.search_areas.values().next())
                .map(|m| m.iter().map(|(k, sa)| (k, sa.name.as_str())).collect())
                .unwrap_or_default(),
        };
        for (key, name) in entries {
            let display = if name.trim().is_empty() {
                key.clone()
            } else {
                name.to_string()
            };
            if q.is_empty()
                || fuzzy_match_fold(&q, key)
                || fuzzy_match_fold(&q, &display)
                || fuzzy_match_fold(&q, prog)
            {
                rows.push((
                    display.clone(),
                    Row::Coord {
                        key: key.clone(),
                        display,
                    },
                ));
            }
        }
        for col in pdata.collections.values() {
            if q.is_empty() || fuzzy_match_fold(&q, &col.name) || fuzzy_match_fold(&q, prog) {
                rows.push((col.name.clone(), Row::Collection(col.clone())));
            }
        }
        if rows.is_empty() {
            continue;
        }
        visible += rows.len();
        rows.sort_by(|a, b| {
            a.0.to_ascii_lowercase()
                .cmp(&b.0.to_ascii_lowercase())
                .then_with(|| match (&a.1, &b.1) {
                    (Row::Coord { key: ka, .. }, Row::Coord { key: kb, .. }) => ka.cmp(kb),
                    (Row::Collection(ca), Row::Collection(cb)) => ca.name.cmp(&cb.name),
                    (Row::Coord { .. }, Row::Collection(_)) => std::cmp::Ordering::Less,
                    (Row::Collection(_), Row::Coord { .. }) => std::cmp::Ordering::Greater,
                })
        });

        let contains_selection = rows.iter().any(|(_, row)| match row {
            Row::Coord { key, .. } => {
                current
                    .strip_prefix(prog)
                    .and_then(|rest| rest.strip_prefix(PROGRAM_DELIMITER))
                    == Some(key.as_str())
            }
            Row::Collection(col) => {
                current_ref.is_collection()
                    && current_ref.program() == Some(prog.as_str())
                    && current_ref.name() == col.name
            }
        });

        // Absolute id so expand/collapse-all (outside this ui stack) can target the same state.
        let id = coord_list_collapse_id(kind, prog);
        let mut state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
        // Open the group that holds the current selection so scroll-to can find it.
        if *scroll_to_selection && contains_selection && !state.is_open() {
            state.set_open(true);
            state.store(ui.ctx());
        }
        state
            .show_header(ui, |ui| {
                crate::icon_cache::paint_program_label(
                    ui,
                    catalog,
                    icons,
                    prog,
                    crate::icon_cache::ProgramLabelStyle::Header {
                        selected: None,
                        child_count: rows.len(),
                    },
                    compact_program_headers,
                );
            })
            .body(|ui| {
                for (_, row) in &rows {
                    match row {
                        Row::Coord { key, display } => {
                            let target = format!("{prog}{PROGRAM_DELIMITER}{key}");
                            let selected = current == &target;
                            let label = if display == key {
                                format!("  {key}")
                            } else {
                                format!("  {display}")
                            };
                            let resp =
                                ui.selectable_label(selected, egui::RichText::new(label).small());
                            previews.show_for_entity(ui, &resp, catalog, prog, key, preview_kind);
                            if selected && *scroll_to_selection && !did_scroll {
                                maybe_scroll_to(ui, &resp, scroll_to_selection);
                                did_scroll = true;
                            }
                            if resp.clicked() {
                                *current = target;
                            }
                        }
                        Row::Collection(col) => {
                            let selected = current_ref.is_collection()
                                && current_ref.program() == Some(prog.as_str())
                                && current_ref.name() == col.name;
                            let label = format!("  {} (collection)", col.name);
                            let resp =
                                ui.selectable_label(selected, egui::RichText::new(label).small());
                            previews.show_for_entity(
                                ui,
                                &resp,
                                catalog,
                                prog,
                                &col.name,
                                PreviewKind::Collection,
                            );
                            if selected && *scroll_to_selection && !did_scroll {
                                maybe_scroll_to(ui, &resp, scroll_to_selection);
                                did_scroll = true;
                            }
                            if resp.clicked() {
                                let initial = if selected {
                                    current_ref.cell_range()
                                } else {
                                    None
                                };
                                *cell_pick = Some(
                                    CollectionCellPick::new(prog, &col.name, col.rows, col.cols)
                                        .with_initial_sel(initial),
                                );
                            }
                        }
                    }
                }
            });
    }
    crate::widgets::list_vacancy(ui, &q, visible, entity_plural);
    if *scroll_to_selection && !did_scroll {
        // Selection not visible under current filter — don't keep retrying every frame.
        *scroll_to_selection = false;
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{self, RawInput};

    /// The coord list must fill the remaining body — the old 52pt reserved
    /// footer under a floating Save left a large empty band in pickers.
    #[test]
    fn coord_list_height_fills_remaining_body() {
        let ctx = egui::Context::default();
        let body_h = 460.0_f32;
        let mut list_h = 0.0_f32;
        let mut after_search = 0.0_f32;
        ctx.run_ui(
            RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(560.0, body_h),
                )),
                ..Default::default()
            },
            |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(560.0, body_h), egui::Sense::hover());
                let mut body = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                body.set_max_size(rect.size());
                // Search row (same placement as the coord picker header).
                let _ = body.label("Search…");
                after_search = body.available_height();
                // Mirror paint_coord_ref_list's height choice.
                list_h = body.available_height().max(40.0);
                let list_w = body.available_width();
                crate::widgets::show_dialog_scroll(&mut body, list_w, list_h, |ui| {
                    ui.label("row");
                });
            },
        )
        .drop_without_applying_deltas();

        assert!(
            (list_h - after_search).abs() < 1.0,
            "list height must equal remaining body ({list_h} vs {after_search})"
        );
        assert!(
            list_h > body_h - 52.0 - 40.0,
            "list must not leave a 52pt footer dead band (list_h={list_h}, body_h={body_h})"
        );
    }
}
