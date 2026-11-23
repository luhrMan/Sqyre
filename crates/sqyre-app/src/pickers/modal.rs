use super::collection_cell::paint_collection_cell_picker;
use super::coord_list::paint_coord_ref_list;
use super::items_grid::paint_items_icon_grid;
use super::query::{query_matches_name_or_tags, query_matches_window};
use super::scroll::{
    apply_list_nav, picker_searchable_scroll_ex, poll_list_nav, ListNavAction, PickerScrollOpts,
    HINT_LIST,
};
use super::types::{ActivePicker, CoordKind, PickerResult};
use super::window::{poll_window_picker_load, refresh_window_picker};
use crate::paint_ctx::CatalogPaint;
use crate::widgets::{list_vacancy, reset_focus_search};
use eframe::egui;
use sqyre_domain::CoordinateRef;

pub fn show_active_picker(
    ctx: &egui::Context,
    picker: &mut ActivePicker,
    paint: &mut CatalogPaint<'_>,
    // `(name, tags)` — tags are used by the macro search bar.
    macros: &[(String, Vec<String>)],
    compact_program_headers: bool,
    pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
) -> PickerResult {
    let mut result = PickerResult::None;
    let mut open = picker.is_open();
    if !open {
        return result;
    }

    poll_window_picker_load(picker, ctx);

    let in_cell_pick = matches!(
        picker,
        ActivePicker::Coord {
            cell_pick: Some(_),
            ..
        }
    );

    let title = match picker {
        ActivePicker::Items { .. } => "Pick items",
        ActivePicker::Coord {
            cell_pick: Some(_), ..
        } => "Select collection cells",
        ActivePicker::Coord {
            kind: CoordKind::Point,
            ..
        } => "Pick point",
        ActivePicker::Coord {
            kind: CoordKind::SearchArea,
            ..
        } => "Pick search area",
        ActivePicker::Macro { .. } => "Pick macro",
        ActivePicker::Window { .. } => "Pick window",
        ActivePicker::None => return result,
    };

    let focus_id = egui::Id::new(("sqyre_picker", title));
    let mut save = false;
    let mut cancel = false;
    let mut back = false;

    // Bounds only — pickers own their ScrollAreas. An outer Window scroll
    // (fit_dialog_window) nests with the item/list scroll and steals the wheel.
    crate::widgets::fit_dialog_popup(
        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .default_size([560.0, 460.0])
            .min_size(crate::widgets::FLOATER_MIN_PICKER)
            .order(egui::Order::Foreground)
            .open(&mut open),
        ctx,
        focus_id,
        pending_scale,
    )
    .show(ctx, |ui| {
        crate::widgets::fill_resize_body(ui, |ui| {
            let body_rect = ui.max_rect();
            // Local Esc/Enter/↑↓ before body widgets.
            let nav = poll_list_nav(ui);
            if nav == ListNavAction::Cancel {
                cancel = true;
            } else if nav == ListNavAction::Activate
                && !matches!(
                    picker,
                    ActivePicker::Macro { .. } | ActivePicker::Window { .. }
                )
            {
                // Macro/Window handle Activate after updating selection below.
                save = true;
            }

            let mut refresh_windows = false;
            match picker {
                ActivePicker::Items {
                    search,
                    staged,
                    staged_tags,
                } => {
                    let mut header_click = None;
                    let program_names: Vec<String> =
                        paint.catalog.program_names().cloned().collect();
                    let mut opts = PickerScrollOpts::list(ui).with_hint(Some("Search items…"));
                    // Count line (+ compact single-row tag chips when Image Search).
                    opts.footer_reserve = if staged_tags.is_some() { 72.0 } else { 36.0 };
                    let mut trailing = |ui: &mut egui::Ui| {
                        super::collapse_all_button(
                            ui,
                            program_names
                                .iter()
                                .map(|n| super::items_icon_grid_collapse_id(n)),
                            |ctx, open| {
                                super::set_items_icon_grid_openness(
                                    ctx,
                                    program_names.iter().map(|n| n.as_str()),
                                    open,
                                );
                            },
                        );
                    };
                    opts.trailing = Some(&mut trailing);
                    picker_searchable_scroll_ex(ui, search, opts, Some(focus_id), |ui, q| {
                        paint_items_icon_grid(
                            ui,
                            paint.catalog,
                            paint.icons,
                            q,
                            staged,
                            true,
                            None,
                            &mut header_click,
                            compact_program_headers,
                            false,
                            None,
                            sqyre_domain::CatalogItemSort::NameAsc,
                            &[],
                            None,
                        );
                        let visible: usize = paint
                            .catalog
                            .program_names()
                            .filter_map(|prog| {
                                let pdata = paint.catalog.get(prog)?;
                                Some(
                                    pdata
                                        .items
                                        .iter()
                                        .filter(|(name, item)| {
                                            q.is_empty()
                                                || crate::pickers::fuzzy_match_fold(q, prog)
                                                || query_matches_name_or_tags(q, name, &item.tags)
                                        })
                                        .count(),
                                )
                            })
                            .sum();
                        list_vacancy(ui, q, visible, "items");
                    });
                    if let Some(tags) = staged_tags.as_mut() {
                        let suggestions =
                            crate::data_editor::helpers::collect_all_item_tags(paint.catalog);
                        let draft_id = ui.id().with("items_picker_target_tags_draft");
                        let mut draft = ui
                            .ctx()
                            .data(|d| d.get_temp::<String>(draft_id))
                            .unwrap_or_default();
                        let program_icons = paint.icons.program_name_icons(ui.ctx(), paint.catalog);
                        let _ = crate::widgets::tag_chip_editor(
                            ui,
                            tags,
                            &mut draft,
                            &suggestions,
                            crate::widgets::TagChipOptions {
                                enabled: true,
                                show_add_button: true,
                                label_name: None,
                                draft_hover: Some(crate::action_tooltip::help::IS_TARGET_TAGS),
                                draft_hint: Some("Tag filters…"),
                                reorderable: false,
                                signed_filters: true,
                                single_row: true,
                                locked: &[],
                                locked_hover: None,
                                program_icons: Some(&program_icons),
                                draft_width: None,
                                draft_id: None,
                            },
                        );
                        ui.ctx().data_mut(|d| d.insert_temp(draft_id, draft));
                        let catalog_refs: Vec<sqyre_domain::CatalogItemRef> = paint
                            .catalog
                            .program_names()
                            .filter_map(|prog| {
                                paint.catalog.get(prog).map(|pdata| (prog.clone(), pdata))
                            })
                            .flat_map(|(prog, pdata)| {
                                pdata
                                    .items
                                    .iter()
                                    .map(move |(name, item)| sqyre_domain::CatalogItemRef {
                                        target: format!(
                                            "{prog}{}{name}",
                                            sqyre_domain::PROGRAM_DELIMITER
                                        ),
                                        tags: item.tags.clone(),
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect();
                        let expanded =
                            sqyre_domain::expand_image_search_targets(staged, tags, &catalog_refs);
                        let from_tags = expanded
                            .iter()
                            .filter(|t| !staged.iter().any(|s| s == *t))
                            .count();
                        ui.weak(format!(
                            "{} selected · {} from tags",
                            staged.len(),
                            from_tags
                        ));
                    } else {
                        ui.weak(format!("{} selected", staged.len()));
                    }
                }
                ActivePicker::Coord {
                    kind,
                    search,
                    value,
                    cell_pick,
                    scroll_to_selection,
                } => {
                    if let Some(pick) = cell_pick.as_mut() {
                        paint_collection_cell_picker(
                            ui,
                            paint.catalog,
                            paint.icons,
                            pick,
                            &mut back,
                        );
                    } else {
                        let kind = *kind;
                        let program_names: Vec<String> =
                            paint.catalog.program_names().cloned().collect();
                        let mut opts = PickerScrollOpts::list(ui).with_hint(Some(HINT_LIST));
                        let mut trailing = |ui: &mut egui::Ui| {
                            super::collapse_all_button(
                                ui,
                                program_names
                                    .iter()
                                    .map(|n| super::coord_list::coord_list_collapse_id(kind, n)),
                                |ctx, open| {
                                    super::set_coord_list_openness(
                                        ctx,
                                        kind,
                                        program_names.iter().map(|n| n.as_str()),
                                        open,
                                    );
                                },
                            );
                        };
                        opts.trailing = Some(&mut trailing);
                        if picker_searchable_scroll_ex(ui, search, opts, Some(focus_id), |ui, q| {
                            paint_coord_ref_list(
                                ui,
                                paint,
                                q,
                                value,
                                kind,
                                cell_pick,
                                scroll_to_selection,
                                compact_program_headers,
                            );
                        })
                        .search_changed
                        {
                            *scroll_to_selection = true;
                        }
                    }
                }
                ActivePicker::Macro {
                    search,
                    value,
                    scroll_to_selection,
                } => {
                    let mut did_scroll = false;
                    let q_nav = search.trim().to_ascii_lowercase();
                    let mut filtered: Vec<&str> = macros
                        .iter()
                        .filter(|(name, tags)| query_matches_name_or_tags(&q_nav, name, tags))
                        .map(|(n, _)| n.as_str())
                        .collect();
                    filtered.sort_by(|a, b| crate::macro_meta::cmp_display_name(a, b));
                    let mut sel = filtered
                        .iter()
                        .position(|n| *n == value.as_str())
                        .unwrap_or(0);
                    if apply_list_nav(&mut sel, filtered.len(), nav) {
                        if let Some(name) = filtered.get(sel) {
                            *value = (*name).to_string();
                            save = true;
                        }
                    } else if matches!(nav, ListNavAction::Up | ListNavAction::Down) {
                        if let Some(name) = filtered.get(sel) {
                            *value = (*name).to_string();
                            *scroll_to_selection = true;
                        }
                    } else if nav == ListNavAction::Activate && filtered.is_empty() {
                        // nothing
                    }
                    let search_changed = picker_searchable_scroll_ex(
                        ui,
                        search,
                        PickerScrollOpts::list(ui).with_hint(Some("Search macros…")),
                        Some(focus_id),
                        |ui, q| {
                            let rows: Vec<&String> = macros
                                .iter()
                                .filter(|(name, tags)| query_matches_name_or_tags(q, name, tags))
                                .map(|(name, _)| name)
                                .collect();
                            let selected = value.clone();
                            did_scroll = super::paint_nav_selectable_rows(
                                ui,
                                rows,
                                |name| selected == **name,
                                |name| egui::RichText::new(name.as_str()).small(),
                                |name| *value = (*name).clone(),
                                scroll_to_selection,
                                "macros",
                                q,
                            );
                        },
                    )
                    .search_changed;
                    if search_changed {
                        *scroll_to_selection = true;
                    } else if *scroll_to_selection && !did_scroll {
                        *scroll_to_selection = false;
                    }
                }
                ActivePicker::Window {
                    search,
                    process_path,
                    window_title,
                    windows,
                    load_error,
                    scroll_to_selection,
                    pending,
                } => {
                    let loading = pending.is_some();
                    let mut did_scroll = false;
                    let mut refresh_clicked = false;
                    let q_preview = search.trim().to_ascii_lowercase();
                    let filtered: Vec<usize> = windows
                        .iter()
                        .enumerate()
                        .filter(|(_, w)| query_matches_window(&q_preview, w))
                        .map(|(i, _)| i)
                        .collect();
                    let mut sel = filtered
                        .iter()
                        .position(|&i| {
                            let w = &windows[i];
                            window_title == &w.title
                                && (process_path == &w.process_path
                                    || (process_path == &w.title
                                        && w.process_path.trim().is_empty())
                                    || (process_path == &w.process_name
                                        && !w.process_name.is_empty()))
                        })
                        .unwrap_or(0);
                    if apply_list_nav(&mut sel, filtered.len(), nav) {
                        if let Some(&i) = filtered.get(sel) {
                            let w = &windows[i];
                            *window_title = w.title.clone();
                            *process_path = w.process_key();
                            save = true;
                        }
                    } else if matches!(nav, ListNavAction::Up | ListNavAction::Down) {
                        if let Some(&i) = filtered.get(sel) {
                            let w = &windows[i];
                            *window_title = w.title.clone();
                            *process_path = w.process_key();
                            *scroll_to_selection = true;
                        }
                    }
                    let mut opts = PickerScrollOpts::list(ui).with_hint(Some("Search windows…"));
                    let mut trailing = |ui: &mut egui::Ui| {
                        if let Some(err) = load_error.as_ref() {
                            crate::widgets::error_icon(ui, err.as_str());
                        }
                        let tip = if loading { "Refreshing…" } else { "Refresh" };
                        refresh_clicked = ui
                            .add_enabled(!loading, crate::widgets::IconButton::new("↻", tip, None))
                            .clicked();
                    };
                    opts.trailing = Some(&mut trailing);
                    let search_changed =
                        picker_searchable_scroll_ex(ui, search, opts, Some(focus_id), |ui, q| {
                            if loading {
                                crate::widgets::loading_line(ui, "Loading windows…");
                            }
                            let rows: Vec<&crate::window_types::WindowInfo> = windows
                                .iter()
                                .filter(|w| query_matches_window(q, w))
                                .collect();
                            let sel_title = window_title.clone();
                            let sel_path = process_path.clone();
                            did_scroll = super::paint_nav_rows(
                                ui,
                                rows,
                                |w| {
                                    sel_title == w.title
                                        && (sel_path == w.process_path
                                            || (sel_path == w.title
                                                && w.process_path.trim().is_empty())
                                            || (sel_path == w.process_name
                                                && !w.process_name.is_empty()))
                                },
                                |ui, w, selected| {
                                    let process_tex = match w.icon.as_ref() {
                                        Some(icon) => paint.icons.seed_process_icon(
                                            ui.ctx(),
                                            &w.process_path,
                                            &w.title,
                                            &w.process_name,
                                            icon,
                                        ),
                                        None => paint.icons.cached_process_for(
                                            &w.process_path,
                                            &w.title,
                                            &w.process_name,
                                        ),
                                    };
                                    let label = egui::RichText::new(w.label()).small();
                                    match process_tex.as_ref() {
                                        Some(tex) => {
                                            let icon = egui::Image::new((
                                                tex.id(),
                                                egui::Vec2::splat(
                                                    crate::icon_cache::PROCESS_ICON_SIDE,
                                                ),
                                            ))
                                            .fit_to_exact_size(egui::Vec2::splat(
                                                crate::icon_cache::PROCESS_ICON_SIDE,
                                            ))
                                            .maintain_aspect_ratio(true);
                                            ui.selectable_label(selected, (icon, label))
                                        }
                                        None => ui.selectable_label(selected, label),
                                    }
                                },
                                |w| {
                                    *window_title = w.title.clone();
                                    *process_path = w.process_key();
                                },
                                super::NavListChrome {
                                    scroll_to_selection,
                                    vacancy_noun: "windows",
                                    q,
                                    show_vacancy: !loading,
                                },
                            );
                        })
                        .search_changed;
                    if refresh_clicked && pending.is_none() {
                        refresh_windows = true;
                        *scroll_to_selection = true;
                        ui.ctx().request_repaint();
                    } else if search_changed {
                        *scroll_to_selection = true;
                    } else if *scroll_to_selection && !did_scroll {
                        *scroll_to_selection = false;
                    }
                }
                ActivePicker::None => {}
            }
            if refresh_windows {
                refresh_window_picker(picker);
            }

            // Save floats over the list's bottom-right corner; Esc (and ◂ Back
            // in the cell-picker header) covers cancel — no painted repeat.
            let cell_has_sel = picker
                .cell_pick_mut()
                .and_then(|c| c.as_ref())
                .and_then(|p| p.sel)
                .is_some();
            let save_enabled = if in_cell_pick {
                cell_has_sel
            } else if let ActivePicker::Window { process_path, .. } = picker {
                !process_path.trim().is_empty()
            } else {
                true
            };
            crate::widgets::float_bottom_right(ui, body_rect, |ui| {
                if crate::widgets::dirty_action_button(ui, "Save", save_enabled).clicked() {
                    save = true;
                }
            });
        });
    });

    if !open || cancel {
        reset_focus_search(ctx, focus_id);
        *picker = ActivePicker::None;
        return PickerResult::None;
    }
    if back {
        if let Some(slot) = picker.cell_pick_mut() {
            *slot = None;
        }
        return PickerResult::None;
    }
    if save {
        if in_cell_pick {
            let staged = picker
                .cell_pick_mut()
                .and_then(|c| c.as_ref())
                .and_then(|p| p.to_ref());
            if let Some(coord) = staged {
                result = match picker.coord_kind() {
                    Some(CoordKind::Point) => PickerResult::Point(coord),
                    Some(CoordKind::SearchArea) => PickerResult::SearchArea(coord),
                    None => PickerResult::None,
                };
                reset_focus_search(ctx, focus_id);
                *picker = ActivePicker::None;
            }
            return result;
        }
        // Window Save requires a process identity (same gate as the button).
        if let ActivePicker::Window { process_path, .. } = picker {
            if process_path.trim().is_empty() {
                return PickerResult::None;
            }
        }
        result = match picker {
            ActivePicker::Items {
                staged,
                staged_tags,
                ..
            } => PickerResult::Items {
                targets: staged.clone(),
                target_tags: staged_tags.clone(),
            },
            ActivePicker::Coord {
                kind: CoordKind::Point,
                value,
                ..
            } => PickerResult::Point(CoordinateRef(value.clone())),
            ActivePicker::Coord {
                kind: CoordKind::SearchArea,
                value,
                ..
            } => PickerResult::SearchArea(CoordinateRef(value.clone())),
            ActivePicker::Macro { value, .. } => PickerResult::MacroName(value.clone()),
            ActivePicker::Window {
                process_path,
                window_title,
                ..
            } => PickerResult::Window {
                process_path: process_path.clone(),
                window_title: window_title.clone(),
            },
            ActivePicker::None => PickerResult::None,
        };
        reset_focus_search(ctx, focus_id);
        *picker = ActivePicker::None;
    }
    result
}
