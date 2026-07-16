//! Shared entity pickers: item icon grids, point / search-area lists, collection cells,
//! macros, and Focus Window live-window lists.

use crate::icon_cache::IconCache;
use crate::image_view::{self, ImageViewTransform};
use crate::preview_tooltip::{PreviewKind, PreviewTooltipCache};
use eframe::egui::{self, Color32, Pos2, Sense, Vec2};
use sqyre_capture::WindowInfo;
use sqyre_domain::{CoordinateRef, PROGRAM_DELIMITER};
use sqyre_persist::ProgramCatalog;

/// Fixed cell size (thumb + padding; no under-icon label).
const GRID_CELL: f32 = 64.0;
const GRID_THUMB: f32 = 52.0;
const GRID_GAP: f32 = 6.0;
const HEADER_SIZE: f32 = 16.0;

/// In-progress collection cell selection (1-based inclusive).
#[derive(Debug, Clone)]
pub struct CollectionCellPick {
    pub program: String,
    pub collection: String,
    pub rows: i32,
    pub cols: i32,
    /// Current selection; `None` until the user clicks.
    pub sel: Option<(i32, i32, i32, i32)>,
    /// Drag start cell while pointer is down (selection mode).
    drag_anchor: Option<(i32, i32)>,
    view: ImageViewTransform,
}

impl CollectionCellPick {
    pub fn new(program: impl Into<String>, collection: impl Into<String>, rows: i32, cols: i32) -> Self {
        Self {
            program: program.into(),
            collection: collection.into(),
            rows: rows.max(1),
            cols: cols.max(1),
            sel: None,
            drag_anchor: None,
            view: ImageViewTransform::default(),
        }
    }

    pub fn with_initial_sel(mut self, sel: Option<(i32, i32, i32, i32)>) -> Self {
        self.sel = sel;
        self
    }

    pub fn reset_view(&mut self) {
        self.view.reset();
    }

    pub fn to_ref(&self) -> Option<CoordinateRef> {
        let (r1, c1, r2, c2) = self.sel?;
        Some(CoordinateRef::collection(
            &self.program,
            &self.collection,
            r1,
            c1,
            r2,
            c2,
        ))
    }
}

/// Which modal picker is open from an action edit tip (or similar).
#[derive(Debug, Clone, Default)]
pub enum ActivePicker {
    #[default]
    None,
    /// Multi-select item targets (`program~item`).
    Items {
        search: String,
        staged: Vec<String>,
    },
    Point {
        search: String,
        /// Working value shown/edited in the picker.
        value: String,
        /// When set, list is replaced by the collection cell grid.
        cell_pick: Option<CollectionCellPick>,
        /// Scroll selected row into view once on open.
        scroll_to_selection: bool,
    },
    SearchArea {
        search: String,
        value: String,
        cell_pick: Option<CollectionCellPick>,
        scroll_to_selection: bool,
    },
    Macro {
        search: String,
        value: String,
        scroll_to_selection: bool,
    },
    /// Live OS windows for Focus Window (`process_path` + `window_title`).
    Window {
        search: String,
        process_path: String,
        window_title: String,
        windows: Vec<WindowInfo>,
        load_error: Option<String>,
        scroll_to_selection: bool,
    },
}

/// Reload the live window list into an open `ActivePicker::Window`.
pub fn refresh_window_picker(picker: &mut ActivePicker) {
    let ActivePicker::Window {
        windows,
        load_error,
        ..
    } = picker
    else {
        return;
    };
    match sqyre_capture::list_open_windows() {
        Ok(list) => {
            *windows = list;
            *load_error = None;
        }
        Err(e) => {
            windows.clear();
            *load_error = Some(e);
        }
    }
}

/// Open a Focus Window picker preloaded with current fields + live windows.
pub fn open_window_picker(process_path: &str, window_title: &str) -> ActivePicker {
    let mut picker = ActivePicker::Window {
        search: String::new(),
        process_path: process_path.to_string(),
        window_title: window_title.to_string(),
        windows: Vec::new(),
        load_error: None,
        scroll_to_selection: true,
    };
    refresh_window_picker(&mut picker);
    picker
}

impl ActivePicker {
    pub fn is_open(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn cell_pick_mut(&mut self) -> Option<&mut Option<CollectionCellPick>> {
        match self {
            Self::Point { cell_pick, .. } | Self::SearchArea { cell_pick, .. } => Some(cell_pick),
            _ => None,
        }
    }
}

fn header_text(label: &str) -> egui::RichText {
    egui::RichText::new(label).size(HEADER_SIZE).strong()
}

/// Fuzzy match on `name` or any tag. Empty `q` matches everything.
pub fn query_matches_name_or_tags(q: &str, name: &str, tags: &[String]) -> bool {
    if q.is_empty() {
        return true;
    }
    fuzzy_match_fold(q, name)
        || tags
            .iter()
            .any(|t| fuzzy_match_fold(q, t))
}

/// Subsequence fuzzy match: each needle char appears in order in haystack.
pub fn fuzzy_match_fold(needle: &str, haystack: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let mut hay = haystack.chars().flat_map(|c| c.to_lowercase());
    'outer: for nc in needle.chars().flat_map(|c| c.to_lowercase()) {
        for hc in hay.by_ref() {
            if hc == nc {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

fn query_matches_window(q: &str, w: &WindowInfo) -> bool {
    if q.is_empty() {
        return true;
    }
    fuzzy_match_fold(q, &w.title)
        || fuzzy_match_fold(q, &w.process_name)
        || fuzzy_match_fold(q, &w.process_path)
}

/// Display name + tags for an item target (`program~item` or `program~item~variant`).
fn item_tooltip_parts(catalog: &ProgramCatalog, target: &str) -> (String, Vec<String>) {
    let Some((program, rest)) = target.split_once(PROGRAM_DELIMITER) else {
        return (target.to_string(), Vec::new());
    };
    let item_key = rest
        .split_once(PROGRAM_DELIMITER)
        .map(|(base, _)| base)
        .unwrap_or(rest);
    if let Some(item) = catalog.get(program).and_then(|p| p.items.get(item_key)) {
        let name = if item.name.is_empty() {
            item_key.to_string()
        } else {
            item.name.clone()
        };
        return (name, item.tags.clone());
    }
    (item_key.to_string(), Vec::new())
}

/// Rich hover tooltip: bold name, then italic primary-colored tags.
pub fn attach_item_icon_tooltip(
    response: &egui::Response,
    catalog: &ProgramCatalog,
    target: &str,
) {
    if !response.hovered() {
        return;
    }
    let (name, tags) = item_tooltip_parts(catalog, target);
    response.clone().on_hover_ui(|ui| {
        paint_item_icon_tooltip(ui, &name, &tags);
    });
}

fn paint_item_icon_tooltip(ui: &mut egui::Ui, name: &str, tags: &[String]) {
    ui.set_max_width(280.0);
    ui.label(egui::RichText::new(name).strong().size(13.0));
    if tags.is_empty() {
        return;
    }
    ui.add_space(4.0);
    let color = ui.visuals().hyperlink_color;
    for tag in tags {
        ui.label(
            egui::RichText::new(tag)
                .size(11.0)
                .italics()
                .color(color),
        );
    }
}

/// How many fixed-size columns fit in `avail_w`.
pub fn grid_column_count_for_width(avail_w: f32) -> usize {
    let avail = avail_w.max(GRID_CELL);
    let cols = ((avail + GRID_GAP) / (GRID_CELL + GRID_GAP)).floor() as usize;
    cols.max(1)
}

/// Paint a selectable icon cell (fixed square, no under-icon label).
/// Returns `(cell_clicked, remove_clicked)`.
pub fn icon_grid_cell_ex(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    target: &str,
    selected: bool,
    show_remove: bool,
) -> (bool, bool) {
    let cell_h = if show_remove {
        GRID_CELL + 18.0
    } else {
        GRID_CELL
    };

    let mut remove_clicked = false;
    let desired = Vec2::new(GRID_CELL, cell_h);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());

    let fill = if selected {
        Color32::from_rgba_unmultiplied(80, 160, 100, 60)
    } else if resp.hovered() {
        Color32::from_black_alpha(25)
    } else {
        Color32::TRANSPARENT
    };
    let body = egui::Rect::from_min_size(rect.min, Vec2::splat(GRID_CELL));
    ui.painter().rect_filled(body, 4.0, fill);
    if selected {
        ui.painter().rect_stroke(
            body,
            4.0,
            egui::Stroke::new(2.0, Color32::from_rgb(60, 140, 80)),
            egui::StrokeKind::Outside,
        );
    }

    let tex = icons.for_target_or_fallback(ui.ctx(), catalog, target);
    let [tw, th] = tex.size();
    let size = fit_thumb(tw as f32, th as f32, GRID_THUMB);
    let img_rect = egui::Rect::from_center_size(body.center(), size);
    // Paint directly — avoid `ui.put(Image)` which can advance the wrap cursor.
    ui.painter().image(
        tex.id(),
        img_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );

    if show_remove {
        let btn_center = egui::pos2(rect.center().x, body.bottom() + 9.0);
        let btn_rect = egui::Rect::from_center_size(btn_center, Vec2::new(22.0, 16.0));
        let btn_id = ui.id().with(("icon_rm", target));
        let btn_resp = ui.interact(btn_rect, btn_id, Sense::click());
        let btn_fill = if btn_resp.hovered() {
            Color32::from_rgb(180, 60, 60)
        } else {
            Color32::from_gray(100)
        };
        ui.painter().rect_filled(btn_rect, 3.0, btn_fill);
        ui.painter().text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );
        remove_clicked = btn_resp.clicked();
    }

    attach_item_icon_tooltip(&resp, catalog, target);

    (resp.clicked() && !remove_clicked, remove_clicked)
}

fn fit_thumb(w: f32, h: f32, max: f32) -> Vec2 {
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (max / w).min(max / h);
    Vec2::new(w * scale, h * scale)
}

/// Lay out `targets` in fixed-size rows (no column stretch, no staircase wrap).
pub fn paint_even_icon_grid(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    targets: &[String],
    is_selected: impl Fn(&str) -> bool,
    show_remove: bool,
    mut on_cell: impl FnMut(usize, &str),
    mut on_remove: impl FnMut(usize),
) {
    if targets.is_empty() {
        return;
    }
    let avail = ui.available_width().max(GRID_CELL);
    ui.set_max_width(avail);
    let cols = grid_column_count_for_width(avail);
    let cell_h = if show_remove {
        GRID_CELL + 18.0
    } else {
        GRID_CELL
    };
    let old_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = Vec2::splat(GRID_GAP);

    let mut i = 0;
    while i < targets.len() {
        ui.allocate_ui_with_layout(
            egui::vec2(avail, cell_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(avail);
                ui.spacing_mut().item_spacing = Vec2::splat(GRID_GAP);
                let end = (i + cols).min(targets.len());
                for k in i..end {
                    let target = &targets[k];
                    let sel = is_selected(target);
                    let (clicked, remove) =
                        icon_grid_cell_ex(ui, catalog, icons, target, sel, show_remove);
                    if clicked {
                        on_cell(k, target);
                    }
                    if remove {
                        on_remove(k);
                    }
                }
            },
        );
        i += cols;
    }

    ui.spacing_mut().item_spacing = old_spacing;
}

/// Program accordion of item icon grids. Click toggles membership in `selected` when
/// `multi` is true; otherwise replaces selection with the clicked target.
/// When `multi`, each program header includes an All control over filtered targets
/// (items picker tri-state / All button).
pub fn paint_items_icon_grid(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    search: &str,
    selected: &mut Vec<String>,
    multi: bool,
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
                fuzzy_match_fold(&q, prog)
                    || query_matches_name_or_tags(&q, name, &item.tags)
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
                (
                    format!("{prog}{PROGRAM_DELIMITER}{item_key}"),
                    display,
                )
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

        egui::CollapsingHeader::new(header_text(prog))
            .default_open(true)
            .show(ui, |ui| {
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
fn toggle_select_all_filtered(selected: &mut Vec<String>, filtered: &[String]) {
    if filtered.is_empty() {
        return;
    }
    let all_selected = filtered
        .iter()
        .all(|t| selected.iter().any(|s| s == t));
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
fn sort_by_display_name(rows: &mut [(String, String)]) {
    rows.sort_by(|a, b| {
        a.1.to_ascii_lowercase()
            .cmp(&b.1.to_ascii_lowercase())
            .then_with(|| a.0.cmp(&b.0))
    });
}

fn maybe_scroll_to(ui: &mut egui::Ui, resp: &egui::Response, scroll: &mut bool) {
    if *scroll {
        ui.scroll_to_rect(resp.rect, Some(egui::Align::Center));
        *scroll = false;
    }
}

/// Finite height for scroll panes inside content-sized popup windows.
/// Without this, `ScrollArea` + `auto_shrink([false, false])` grows the window forever.
///
/// `footer_reserve` is space still to be laid out below the scroll (buttons, status).
pub fn popup_scroll_max_height(ui: &egui::Ui, footer_reserve: f32) -> f32 {
    const FALLBACK: f32 = 360.0;
    let screen_cap = (ui.ctx().content_rect().height() * 0.65).max(100.0);
    let h = ui.available_height() - footer_reserve;
    let capped = if h.is_finite() {
        h.max(100.0)
    } else {
        FALLBACK
    };
    capped.min(screen_cap)
}

fn picker_list_max_height(ui: &egui::Ui) -> f32 {
    // Separator + Cancel/Save row still laid out below the list.
    popup_scroll_max_height(ui, 52.0)
}

/// Flat searchable list of `program~name` refs from points or search areas,
/// plus program collections (opens cell picker on click).
/// Rows are sorted by display name within each program.
pub fn paint_coord_ref_list(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    search: &str,
    current: &mut String,
    kind: CoordKind,
    previews: &mut PreviewTooltipCache,
    cell_pick: &mut Option<CollectionCellPick>,
    scroll_to_selection: &mut bool,
) {
    let q = search.trim().to_ascii_lowercase();
    let res = catalog.resolution_key().to_string();
    let preview_kind = match kind {
        CoordKind::Point => PreviewKind::Point,
        CoordKind::SearchArea => PreviewKind::SearchArea,
    };
    let current_ref = CoordinateRef(current.clone());
    let mut did_scroll = false;
    let list_h = picker_list_max_height(ui);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(list_h)
        .show(ui, |ui| {
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
                match kind {
                    CoordKind::Point => {
                        if let Some(m) = pdata
                            .points
                            .get(&res)
                            .or_else(|| pdata.points.values().next())
                        {
                            for (key, pt) in m {
                                let display = if pt.name.trim().is_empty() {
                                    key.clone()
                                } else {
                                    pt.name.clone()
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
                        }
                    }
                    CoordKind::SearchArea => {
                        if let Some(m) = pdata
                            .search_areas
                            .get(&res)
                            .or_else(|| pdata.search_areas.values().next())
                        {
                            for (key, sa) in m {
                                let display = if sa.name.trim().is_empty() {
                                    key.clone()
                                } else {
                                    sa.name.clone()
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
                        }
                    }
                }
                for col in pdata.collections.values() {
                    if q.is_empty()
                        || fuzzy_match_fold(&q, &col.name)
                        || fuzzy_match_fold(&q, prog)
                    {
                        rows.push((col.name.clone(), Row::Collection(col.clone())));
                    }
                }
                if rows.is_empty() {
                    continue;
                }
                rows.sort_by(|a, b| {
                    a.0.to_ascii_lowercase()
                        .cmp(&b.0.to_ascii_lowercase())
                        .then_with(|| match (&a.1, &b.1) {
                            (Row::Coord { key: ka, .. }, Row::Coord { key: kb, .. }) => {
                                ka.cmp(kb)
                            }
                            (Row::Collection(ca), Row::Collection(cb)) => {
                                ca.name.cmp(&cb.name)
                            }
                            (Row::Coord { .. }, Row::Collection(_)) => std::cmp::Ordering::Less,
                            (Row::Collection(_), Row::Coord { .. }) => {
                                std::cmp::Ordering::Greater
                            }
                        })
                });

                ui.label(header_text(prog));
                for (_, row) in rows {
                    match row {
                        Row::Coord { key, display } => {
                            let target = format!("{prog}{PROGRAM_DELIMITER}{key}");
                            let selected = current == &target;
                            let label = if display == key {
                                format!("  {key}")
                            } else {
                                format!("  {display}")
                            };
                            let resp = ui.selectable_label(
                                selected,
                                egui::RichText::new(label).size(13.0),
                            );
                            previews.show_for_entity(ui, &resp, catalog, prog, &key, preview_kind);
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
                            let resp = ui.selectable_label(
                                selected,
                                egui::RichText::new(label).size(13.0),
                            );
                            let path = catalog.collection_image_path(prog, &col.name);
                            if resp.hovered() {
                                if let Some(tex) = icons.for_path(ui.ctx(), &path) {
                                    resp.clone().on_hover_ui(|ui| {
                                        let [tw, th] = tex.size();
                                        let size = fit_panel(tw as f32, th as f32);
                                        ui.add(egui::Image::new((tex.id(), size)));
                                        ui.label(format!("{prog}~{}", col.name));
                                    });
                                } else {
                                    resp.clone().on_hover_text(format!(
                                        "{prog}~{} (no image)",
                                        col.name
                                    ));
                                }
                            }
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
                ui.add_space(6.0);
            }
        });
    if *scroll_to_selection && !did_scroll {
        // Selection not visible under current filter — don't keep retrying every frame.
        *scroll_to_selection = false;
    }
}

fn fit_panel(w: f32, h: f32) -> Vec2 {
    const MAX_W: f32 = 320.0;
    const MAX_H: f32 = 240.0;
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (MAX_W / w).min(MAX_H / h).min(1.0);
    Vec2::new(w * scale, h * scale)
}

/// Interactive collection image + rows×cols overlay; click/drag selects cells.
/// Wheel zooms at cursor; when zoomed, drag pans.
fn paint_collection_cell_picker(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    pick: &mut CollectionCellPick,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "Select cells — {}~{}",
                pick.program, pick.collection
            ))
            .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    pick.view.needs_reset_button(),
                    egui::Button::new("Reset view"),
                )
                .on_hover_text("Fit image in viewport")
                .clicked()
            {
                pick.reset_view();
            }
            if pick.view.zoom != 1.0 {
                ui.weak(format!("{:.0}%", pick.view.zoom * 100.0));
            }
        });
    });
    ui.weak("Scroll to zoom; drag to pan when zoomed; click/drag selects cells at 100%.");

    let path = catalog.collection_image_path(&pick.program, &pick.collection);
    let tex = icons.for_path(ui.ctx(), &path);
    let avail = ui.available_width().min(520.0);
    let image_size = match &tex {
        Some(t) => {
            let [tw, th] = t.size();
            Vec2::new(tw as f32, th as f32)
        }
        None => Vec2::new(avail, avail * 0.75),
    };
    let fit = fit_panel(image_size.x, image_size.y);
    let scale = (avail / fit.x).min(1.0);
    let desired = Vec2::new((fit.x * scale).max(160.0), (fit.y * scale).max(120.0));
    let (viewport, resp) = ui.allocate_exact_size(desired, Sense::click_and_drag());

    image_view::handle_scroll_zoom(ui, viewport, image_size, &mut pick.view, resp.hovered());

    let content = image_view::image_content_rect(
        viewport,
        image_size,
        pick.view.zoom,
        pick.view.pan,
    );

    {
        let painter = ui.painter_at(viewport);
        if let Some(tex) = &tex {
            painter.image(
                tex.id(),
                content,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.rect_filled(viewport, 0.0, Color32::from_gray(40));
            painter.text(
                viewport.center(),
                egui::Align2::CENTER_CENTER,
                "No collection image",
                egui::FontId::proportional(14.0),
                Color32::LIGHT_GRAY,
            );
        }
        paint_cell_grid_lines_painter(&painter, content, pick.rows, pick.cols);
        if let Some(sel) = pick.sel {
            paint_cell_selection_painter(&painter, content, pick.rows, pick.cols, sel);
        }
    }

    if image_view::handle_pan_drag(&resp, viewport, image_size, &mut pick.view) {
        if resp.drag_started() {
            pick.drag_anchor = None;
        }
    } else if let Some(pos) = resp.interact_pointer_pos() {
        if let Some((r, c)) = cell_at(content, pick.rows, pick.cols, pos) {
            if resp.drag_started() || (resp.clicked() && !resp.dragged()) {
                pick.drag_anchor = Some((r, c));
                pick.sel = Some((r, c, r, c));
            } else if resp.dragged() {
                if let Some((ar, ac)) = pick.drag_anchor {
                    pick.sel = Some((ar, ac, r, c));
                }
            }
        }
        if resp.drag_stopped() {
            pick.drag_anchor = None;
            if let Some((r1, c1, r2, c2)) = pick.sel {
                let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
                let (c1, c2) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
                pick.sel = Some((r1, c1, r2, c2));
            }
        }
    }

    let status = match pick.sel {
        Some((r1, c1, r2, c2)) if r1 == r2 && c1 == c2 => {
            format!("Selected cell R{r1} C{c1}")
        }
        Some((r1, c1, r2, c2)) => format!("Selected R{r1}–{r2} × C{c1}–{c2}"),
        None => "Click or drag to select cell(s)".into(),
    };
    ui.weak(status);
}

fn paint_cell_grid_lines_painter(painter: &egui::Painter, rect: egui::Rect, rows: i32, cols: i32) {
    let rows = rows.max(1) as f32;
    let cols = cols.max(1) as f32;
    let stroke = egui::Stroke::new(1.0, Color32::from_rgb(255, 80, 80));
    for i in 1..rows as i32 {
        let y = rect.top() + rect.height() * (i as f32) / rows;
        painter.hline(rect.x_range(), y, stroke);
    }
    for i in 1..cols as i32 {
        let x = rect.left() + rect.width() * (i as f32) / cols;
        painter.vline(x, rect.y_range(), stroke);
    }
    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
}

fn paint_cell_selection_painter(
    painter: &egui::Painter,
    rect: egui::Rect,
    rows: i32,
    cols: i32,
    sel: (i32, i32, i32, i32),
) {
    let (r1, c1, r2, c2) = sel;
    let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
    let (c1, c2) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
    let rows = rows.max(1) as f32;
    let cols = cols.max(1) as f32;
    let cw = rect.width() / cols;
    let ch = rect.height() / rows;
    let sel_rect = egui::Rect::from_min_max(
        egui::pos2(
            rect.left() + (c1 as f32 - 1.0) * cw,
            rect.top() + (r1 as f32 - 1.0) * ch,
        ),
        egui::pos2(
            rect.left() + c2 as f32 * cw,
            rect.top() + r2 as f32 * ch,
        ),
    );
    painter.rect_filled(
        sel_rect,
        0.0,
        Color32::from_rgba_unmultiplied(60, 160, 255, 70),
    );
    painter.rect_stroke(
        sel_rect,
        0.0,
        egui::Stroke::new(2.0, Color32::from_rgb(40, 140, 255)),
        egui::StrokeKind::Outside,
    );
}

fn cell_at(rect: egui::Rect, rows: i32, cols: i32, pos: Pos2) -> Option<(i32, i32)> {
    if !rect.contains(pos) {
        return None;
    }
    let rows = rows.max(1) as f32;
    let cols = cols.max(1) as f32;
    let c = (((pos.x - rect.left()) / rect.width()) * cols).floor() as i32 + 1;
    let r = (((pos.y - rect.top()) / rect.height()) * rows).floor() as i32 + 1;
    Some((r.clamp(1, rows as i32), c.clamp(1, cols as i32)))
}

#[derive(Debug, Clone, Copy)]
pub enum CoordKind {
    Point,
    SearchArea,
}

/// Draw open picker window; returns committed value on Save (and clears picker).
pub fn show_active_picker(
    ctx: &egui::Context,
    picker: &mut ActivePicker,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    previews: &mut PreviewTooltipCache,
    // `(name, tags)` — tags are used by the macro search bar.
    macros: &[(String, Vec<String>)],
) -> PickerResult {
    let mut result = PickerResult::None;
    let mut open = picker.is_open();
    if !open {
        return result;
    }

    let in_cell_pick = matches!(
        picker,
        ActivePicker::Point {
            cell_pick: Some(_),
            ..
        } | ActivePicker::SearchArea {
            cell_pick: Some(_),
            ..
        }
    );

    let title = match picker {
        ActivePicker::Items { .. } => "Pick items",
        ActivePicker::Point {
            cell_pick: Some(_), ..
        } => "Select collection cells",
        ActivePicker::Point { .. } => "Pick point",
        ActivePicker::SearchArea {
            cell_pick: Some(_), ..
        } => "Select collection cells",
        ActivePicker::SearchArea { .. } => "Pick search area",
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
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Search").size(HEADER_SIZE));
                        ui.text_edit_singleline(search);
                    });
                    ui.separator();
                    let list_h = picker_list_max_height(ui);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(list_h)
                        .show(ui, |ui| {
                            paint_items_icon_grid(ui, catalog, icons, search, staged, true);
                        });
                    ui.separator();
                    ui.label(format!("{} selected", staged.len()));
                }
                ActivePicker::Point {
                    search,
                    value,
                    cell_pick,
                    scroll_to_selection,
                } => {
                    if let Some(pick) = cell_pick.as_mut() {
                        paint_collection_cell_picker(ui, catalog, icons, pick);
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Search").size(HEADER_SIZE));
                            if ui.text_edit_singleline(search).changed() {
                                *scroll_to_selection = true;
                            }
                        });
                        ui.separator();
                        paint_coord_ref_list(
                            ui,
                            catalog,
                            icons,
                            search,
                            value,
                            CoordKind::Point,
                            previews,
                            cell_pick,
                            scroll_to_selection,
                        );
                    }
                }
                ActivePicker::SearchArea {
                    search,
                    value,
                    cell_pick,
                    scroll_to_selection,
                } => {
                    if let Some(pick) = cell_pick.as_mut() {
                        paint_collection_cell_picker(ui, catalog, icons, pick);
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Search").size(HEADER_SIZE));
                            if ui.text_edit_singleline(search).changed() {
                                *scroll_to_selection = true;
                            }
                        });
                        ui.separator();
                        paint_coord_ref_list(
                            ui,
                            catalog,
                            icons,
                            search,
                            value,
                            CoordKind::SearchArea,
                            previews,
                            cell_pick,
                            scroll_to_selection,
                        );
                    }
                }
                ActivePicker::Macro {
                    search,
                    value,
                    scroll_to_selection,
                } => {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Search").size(HEADER_SIZE));
                        if ui.text_edit_singleline(search).changed() {
                            *scroll_to_selection = true;
                        }
                    });
                    ui.separator();
                    let q = search.trim().to_ascii_lowercase();
                    let mut did_scroll = false;
                    let list_h = picker_list_max_height(ui);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(list_h)
                        .show(ui, |ui| {
                            for (name, tags) in macros {
                                if !query_matches_name_or_tags(&q, name, tags) {
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
                        });
                    if *scroll_to_selection && !did_scroll {
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
                } => {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Search").size(HEADER_SIZE));
                        if ui.text_edit_singleline(search).changed() {
                            *scroll_to_selection = true;
                        }
                        if ui
                            .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
                            .on_hover_text("Refresh")
                            .clicked()
                        {
                            match sqyre_capture::list_open_windows() {
                                Ok(list) => {
                                    *windows = list;
                                    *load_error = None;
                                }
                                Err(e) => {
                                    windows.clear();
                                    *load_error = Some(e);
                                }
                            }
                            *scroll_to_selection = true;
                        }
                    });
                    ui.separator();
                    if let Some(err) = load_error.as_ref() {
                        ui.colored_label(Color32::RED, err.as_str());
                    }
                    let q = search.trim().to_ascii_lowercase();
                    let mut did_scroll = false;
                    let list_h = picker_list_max_height(ui);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(list_h)
                        .show(ui, |ui| {
                            for w in windows.iter() {
                                if !query_matches_window(&q, w) {
                                    continue;
                                }
                                let selected = window_title == &w.title
                                    && process_path == &w.process_path;
                                let resp = ui.selectable_label(
                                    selected,
                                    egui::RichText::new(w.label()).size(13.0),
                                );
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
                    if *scroll_to_selection && !did_scroll {
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
                if in_cell_pick {
                    if ui.button("Back").clicked() {
                        back = true;
                    }
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
                result = match picker {
                    ActivePicker::Point { .. } => PickerResult::Point(coord),
                    ActivePicker::SearchArea { .. } => PickerResult::SearchArea(coord),
                    _ => PickerResult::None,
                };
                *picker = ActivePicker::None;
            }
            return result;
        }
        result = match picker {
            ActivePicker::Items { staged, .. } => PickerResult::Items(staged.clone()),
            ActivePicker::Point { value, .. } => PickerResult::Point(CoordinateRef(value.clone())),
            ActivePicker::SearchArea { value, .. } => {
                PickerResult::SearchArea(CoordinateRef(value.clone()))
            }
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

#[derive(Debug, Clone)]
pub enum PickerResult {
    None,
    Items(Vec<String>),
    Point(CoordinateRef),
    SearchArea(CoordinateRef),
    MacroName(String),
    Window {
        process_path: String,
        window_title: String,
    },
}

/// Static option lists for ComboBox fields (>2 options).
pub mod options {
    use sqyre_domain::{OP_EQUALS, REPEAT_ONCE, REPEAT_WAIT_UNTIL_FOUND, REPEAT_WHILE_FOUND};

    pub const CLICK_BUTTONS: &[&str] = &["left", "right", "center", "scroll"];

    pub const CONDITIONAL_OPERATORS: &[&str] = &[
        OP_EQUALS,
        "!=",
        "<",
        "<=",
        ">",
        ">=",
        "contains",
        "starts with",
        "ends with",
        "is set",
        "is empty",
    ];

    pub const REPEAT_MODES: &[&str] = &[
        REPEAT_ONCE,
        REPEAT_WAIT_UNTIL_FOUND,
        REPEAT_WHILE_FOUND,
    ];

    /// Match-order grouping (empty allowed as unset).
    pub const ORDER_GROUPING: &[&str] = &["", "row", "column", "none"];
    pub const ORDER_HORIZONTAL: &[&str] = &["", "left_to_right", "right_to_left"];
    pub const ORDER_VERTICAL: &[&str] = &["", "top_to_bottom", "bottom_to_top"];

    pub const SELECT_DEVICES: &[&str] = &["", "mouse", "keyboard"];
    pub const SELECT_PRESS_MODES: &[&str] = &["", "click", "down", "up", "hold"];
    pub const MOUSE_BUTTONS: &[&str] = &["", "left", "right", "center"];
}

#[cfg(test)]
mod tests {
    use super::{
        item_tooltip_parts, query_matches_name_or_tags, query_matches_window,
        sort_by_display_name, toggle_select_all_filtered,
    };
    use sqyre_capture::WindowInfo;
    use sqyre_persist::{ProgramCatalog, ProgramData, ProgramItem};
    use std::collections::BTreeMap;

    #[test]
    fn sort_by_display_name_orders_case_insensitive() {
        let mut rows = vec![
            ("b".into(), "Zebra".into()),
            ("a".into(), "apple".into()),
            ("c".into(), "Banana".into()),
        ];
        sort_by_display_name(&mut rows);
        assert_eq!(
            rows.iter().map(|(_, d)| d.as_str()).collect::<Vec<_>>(),
            vec!["apple", "Banana", "Zebra"]
        );
    }

    #[test]
    fn toggle_select_all_adds_then_clears_filtered() {
        let filtered = vec!["A~1".into(), "A~2".into()];
        let mut selected = vec!["B~9".into()];
        toggle_select_all_filtered(&mut selected, &filtered);
        assert!(selected.contains(&"A~1".into()));
        assert!(selected.contains(&"A~2".into()));
        assert!(selected.contains(&"B~9".into()));
        toggle_select_all_filtered(&mut selected, &filtered);
        assert_eq!(selected, vec!["B~9".to_string()]);
    }

    #[test]
    fn empty_query_matches_anything() {
        assert!(query_matches_name_or_tags("", "Potion", &[]));
        assert!(query_matches_name_or_tags(
            "",
            "x",
            &["healing".into()]
        ));
    }

    #[test]
    fn matches_name_substring() {
        assert!(query_matches_name_or_tags("pot", "HealthPotion", &[]));
        assert!(!query_matches_name_or_tags("sword", "HealthPotion", &[]));
    }

    #[test]
    fn matches_name_fuzzy_subsequence() {
        assert!(query_matches_name_or_tags("hlt", "HealthPotion", &[]));
        assert!(query_matches_name_or_tags("HPT", "HealthPotion", &[]));
        assert!(!query_matches_name_or_tags("thl", "HealthPotion", &[])); // wrong order
    }

    #[test]
    fn matches_tag_substring() {
        let tags = vec!["consumable".into(), "healing".into()];
        assert!(query_matches_name_or_tags("heal", "Minor Flask", &tags));
        assert!(query_matches_name_or_tags("CONSUM", "Minor Flask", &tags));
        assert!(!query_matches_name_or_tags("weapon", "Minor Flask", &tags));
    }

    #[test]
    fn item_tooltip_parts_resolves_name_and_tags() {
        let mut cat = ProgramCatalog::default();
        cat.programs_mut().insert(
            "Game".into(),
            ProgramData {
                name: "Game".into(),
                items: BTreeMap::from([(
                    "Flask".into(),
                    ProgramItem {
                        name: "Health Flask".into(),
                        tags: vec!["healing".into(), "consumable".into()],
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        );

        let (name, tags) = item_tooltip_parts(&cat, "Game~Flask");
        assert_eq!(name, "Health Flask");
        assert_eq!(tags, vec!["healing", "consumable"]);

        let (name, tags) = item_tooltip_parts(&cat, "Game~Flask~v2");
        assert_eq!(name, "Health Flask");
        assert_eq!(tags, vec!["healing", "consumable"]);

        let (name, tags) = item_tooltip_parts(&cat, "Missing~Item");
        assert_eq!(name, "Item");
        assert!(tags.is_empty());
    }

    #[test]
    fn window_query_matches_title_name_or_path() {
        let w = WindowInfo {
            title: "Inbox — Mail".into(),
            process_name: "thunderbird".into(),
            process_path: "/usr/lib/thunderbird/thunderbird".into(),
        };
        assert!(query_matches_window("", &w));
        assert!(query_matches_window("inbox", &w));
        assert!(query_matches_window("THUNDER", &w));
        assert!(query_matches_window("/usr/lib/thunder", &w));
        assert!(!query_matches_window("firefox", &w));
    }
}
