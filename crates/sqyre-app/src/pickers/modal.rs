use super::collection_cell::paint_collection_cell_picker;
use super::coord_list::paint_coord_ref_list;
use super::items_grid::paint_items_icon_grid;
use super::query::{query_matches_name_or_tags, query_matches_window};
use super::scroll::{maybe_scroll_to, picker_searchable_scroll, PickerScrollOpts};
use super::types::{ActivePicker, CoordKind, PickerResult, HEADER_SIZE};
#[cfg(feature = "native-runtime")]
use super::window::fetch_open_windows;
use super::window::poll_window_picker_load;
use crate::paint_ctx::CatalogPaint;
use eframe::egui;
use sqyre_domain::CoordinateRef;
use std::sync::mpsc;
#[cfg(feature = "native-runtime")]
use std::thread;

pub fn show_active_picker(
    ctx: &egui::Context,
    picker: &mut ActivePicker,
    paint: &mut CatalogPaint<'_>,
    // `(name, tags)` — tags are used by the macro search bar.
    macros: &[(String, Vec<String>)],
    compact_program_headers: bool,
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

    let mut save = false;
    let mut cancel = false;
    let mut back = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(true)
        .default_size([560.0, 460.0])
        .min_size([400.0, 280.0])
        .order(egui::Order::Foreground)
        .open(&mut open)
        .show(ctx, |ui| {
            match picker {
                ActivePicker::Items { search, staged } => {
                    let mut header_click = None;
                    let program_names: Vec<String> =
                        paint.catalog.program_names().cloned().collect();
                    let mut opts = PickerScrollOpts::list(ui);
                    let mut trailing = |ui: &mut egui::Ui| {
                        super::collapse_all_buttons(ui, |ctx, open| {
                            super::set_items_icon_grid_openness(
                                ctx,
                                program_names.iter().map(|n| n.as_str()),
                                open,
                            );
                        });
                    };
                    opts.trailing = Some(&mut trailing);
                    picker_searchable_scroll(ui, search, opts, |ui, q| {
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
                        );
                    });
                    ui.separator();
                    ui.label(format!("{} selected", staged.len()));
                }
                ActivePicker::Coord {
                    kind,
                    search,
                    value,
                    cell_pick,
                    scroll_to_selection,
                } => {
                    if let Some(pick) = cell_pick.as_mut() {
                        paint_collection_cell_picker(ui, paint.catalog, paint.icons, pick);
                    } else {
                        let kind = *kind;
                        let program_names: Vec<String> =
                            paint.catalog.program_names().cloned().collect();
                        // Search chrome only — list owns its own ScrollArea (program groups).
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS)
                                    .size(HEADER_SIZE),
                            )
                            .on_hover_text("Search");
                            if ui.text_edit_singleline(search).changed() {
                                *scroll_to_selection = true;
                            }
                            super::collapse_all_buttons(ui, |ctx, open| {
                                super::set_coord_list_openness(
                                    ctx,
                                    kind,
                                    program_names.iter().map(|n| n.as_str()),
                                    open,
                                );
                            });
                        });
                        ui.separator();
                        paint_coord_ref_list(
                            ui,
                            paint,
                            search,
                            value,
                            kind,
                            cell_pick,
                            scroll_to_selection,
                            compact_program_headers,
                        );
                    }
                }
                ActivePicker::Macro {
                    search,
                    value,
                    scroll_to_selection,
                } => {
                    let mut did_scroll = false;
                    let search_changed = picker_searchable_scroll(
                        ui,
                        search,
                        PickerScrollOpts::list(ui),
                        |ui, q| {
                            for (name, tags) in macros {
                                if !query_matches_name_or_tags(q, name, tags) {
                                    continue;
                                }
                                let selected = value == name;
                                let resp = ui.selectable_label(
                                    selected,
                                    egui::RichText::new(name.as_str()).size(13.0),
                                );
                                if selected && *scroll_to_selection && !did_scroll {
                                    maybe_scroll_to(ui, &resp, scroll_to_selection);
                                    did_scroll = true;
                                }
                                if resp.clicked() {
                                    *value = name.clone();
                                }
                            }
                        },
                    );
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
                    let mut opts = PickerScrollOpts::list(ui);
                    let mut trailing = |ui: &mut egui::Ui| {
                        refresh_clicked = ui
                            .add_enabled_ui(!loading, |ui| crate::theme::icon_button(ui, "↻"))
                            .inner
                            .on_hover_text(if loading { "Refreshing…" } else { "Refresh" })
                            .clicked();
                    };
                    opts.trailing = Some(&mut trailing);
                    let search_changed = picker_searchable_scroll(ui, search, opts, |ui, q| {
                        if loading {
                            ui.label("Loading windows…");
                        }
                        if let Some(err) = load_error.as_ref() {
                            ui.colored_label(crate::theme::error_fg(), err.as_str());
                        }
                        for w in windows.iter() {
                            if !query_matches_window(q, w) {
                                continue;
                            }
                            let selected =
                                window_title == &w.title && process_path == &w.process_path;
                            // Prefer icon bytes from the list fetch; avoid per-row OS re-scan.
                            let process_tex = match w.icon.as_ref() {
                                Some(icon) => {
                                    paint
                                        .icons
                                        .seed_process_icon(ui.ctx(), &w.process_path, icon)
                                }
                                None => paint.icons.cached_process(&w.process_path),
                            };
                            let resp = ui
                                .horizontal(|ui| {
                                    if let Some(tex) = process_tex.as_ref() {
                                        crate::icon_cache::paint_process_icon(
                                            ui,
                                            tex,
                                            crate::icon_cache::PROCESS_ICON_SIDE,
                                        );
                                    }
                                    ui.selectable_label(
                                        selected,
                                        egui::RichText::new(w.label()).size(13.0),
                                    )
                                })
                                .inner;
                            if selected && *scroll_to_selection && !did_scroll {
                                maybe_scroll_to(ui, &resp, scroll_to_selection);
                                did_scroll = true;
                            }
                            if resp.clicked() {
                                *window_title = w.title.clone();
                                *process_path = w.process_path.clone();
                            }
                        }
                    });
                    if refresh_clicked && pending.is_none() {
                        *load_error = None;
                        let (tx, rx) = mpsc::channel();
                        #[cfg(feature = "native-runtime")]
                        thread::spawn(move || {
                            let _ = tx.send(fetch_open_windows());
                        });
                        #[cfg(not(feature = "native-runtime"))]
                        {
                            let _ = tx.send(Ok(Vec::new()));
                        }
                        *pending = Some(rx);
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

            ui.separator();
            let cell_has_sel = picker
                .cell_pick_mut()
                .and_then(|c| c.as_ref())
                .and_then(|p| p.sel)
                .is_some();
            ui.horizontal(|ui| {
                if in_cell_pick && ui.button("Back").clicked() {
                    back = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                let save_enabled = !in_cell_pick || cell_has_sel;
                if ui
                    .add_enabled(save_enabled, egui::Button::new("Save"))
                    .clicked()
                {
                    save = true;
                }
            });
        });

    if !open || cancel {
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
            // Collection cell Save commits immediately.
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
                *picker = ActivePicker::None;
            }
            return result;
        }
        result = match picker {
            ActivePicker::Items { staged, .. } => PickerResult::Items(staged.clone()),
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
        *picker = ActivePicker::None;
    }
    result
}
