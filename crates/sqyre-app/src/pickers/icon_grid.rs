use super::types::{
    EDIT_CELL, EDIT_CELL_MAX, EDIT_GAP, EDIT_THUMB, GRID_CELL, GRID_GAP, GRID_THUMB, REMOVE_BTN,
};
use crate::icon_cache::IconCache;
use crate::image_view;
use crate::theme::{
    picker_drop_stroke, picker_remove_hover, picker_selected_fill, picker_selected_stroke,
};
use eframe::egui::{self, Color32, Sense, Vec2};
use sqyre_domain::PROGRAM_DELIMITER;
use sqyre_persist::ProgramCatalog;

pub(crate) fn item_tooltip_parts(catalog: &ProgramCatalog, target: &str) -> (String, Vec<String>) {
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

/// Rich hover tooltip: bold name, vertical 12×12 variant rows, then italic tags.
pub fn attach_item_icon_tooltip(
    response: &egui::Response,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    target: &str,
) {
    if !response.hovered() {
        return;
    }
    let (name, tags) = item_tooltip_parts(catalog, target);
    response.clone().on_hover_ui(|ui| {
        paint_item_icon_tooltip(ui, catalog, icons, target, &name, &tags);
    });
}

const VARIANT_TIP_THUMB: f32 = 12.0;

fn paint_item_icon_tooltip(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    target: &str,
    name: &str,
    tags: &[String],
) {
    ui.set_max_width(280.0);
    ui.label(egui::RichText::new(name).strong().small());

    let paths = crate::demo_icons::merged_variant_paths(catalog, target);
    if !paths.is_empty() {
        let item_key = target
            .split_once(PROGRAM_DELIMITER)
            .map(|(_, rest)| {
                rest.split_once(PROGRAM_DELIMITER)
                    .map(|(base, _)| base)
                    .unwrap_or(rest)
            })
            .unwrap_or(target);
        ui.add_space(crate::theme::SPACE_4);
        for path in &paths {
            let Some(tex) = icons.for_path(ui.ctx(), path) else {
                continue;
            };
            let [tw, th] = tex.size();
            let size = image_view::fit_icon_thumb(
                tw as f32,
                th as f32,
                VARIANT_TIP_THUMB,
                VARIANT_TIP_THUMB,
            );
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                crate::icon_cache::paint_icon_thumb_at(
                    ui,
                    &tex,
                    rect.center(),
                    VARIANT_TIP_THUMB,
                    VARIANT_TIP_THUMB,
                    0.0,
                    None,
                );
                let variant = crate::data_editor_preview::variant_name_from_path(path, item_key);
                ui.label(
                    egui::RichText::new(format!(
                        "{}  {}",
                        crate::data_editor_preview::variant_display_label(&variant),
                        crate::data_editor_preview::pixel_size_text(tw as i32, th as i32)
                    ))
                    .small(),
                );
            });
        }
    }

    if tags.is_empty() {
        return;
    }
    ui.add_space(crate::theme::SPACE_4);
    let color = ui.visuals().hyperlink_color;
    for tag in tags {
        ui.label(egui::RichText::new(tag).small().italics().color(color));
    }
}

/// Minimum fraction of a cell that must fit before that column is counted.
/// Avoids a barely-visible trailing column when the pane is only a sliver wider
/// than N full cells (or a floating scrollbar covers most of the next cell).
const MIN_COLUMN_VISIBLE_FRAC: f32 = 0.8;

/// How many fixed-size grid columns fit in `avail_w`.
///
/// A column is counted only when at least [`MIN_COLUMN_VISIBLE_FRAC`] of its
/// cell width lies inside the budget (not merely a few pixels of overflow).
pub(crate) fn grid_column_count_for_width(avail_w: f32, cell: f32, gap: f32) -> usize {
    let cell = cell.max(1.0);
    let gap = gap.max(0.0);
    let avail = avail_w.max(0.0);
    let min_cell = cell * MIN_COLUMN_VISIBLE_FRAC;
    if avail < min_cell {
        return 1;
    }
    let stride = cell + gap;
    // (n - 1) * stride + min_cell ≤ avail  →  n ≤ (avail - min_cell) / stride + 1
    (((avail - min_cell) / stride).floor() as usize).saturating_add(1)
}

/// How an icon grid sizes cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconGridKind {
    /// Catalog / picker: fixed cell size.
    Picker,
    /// Image Search target list: larger cells when few items, shrinking toward
    /// the compact edit size as more are added. `removable` paints the × badge.
    Targets { removable: bool },
}

impl IconGridKind {
    fn show_remove(self) -> bool {
        matches!(self, Self::Targets { removable: true })
    }

    fn scale_with_count(self) -> bool {
        matches!(self, Self::Targets { .. })
    }
}

/// Cell edge that fills one row when possible, clamped to `[min_cell, max_cell]`.
pub(crate) fn adaptive_icon_cell(
    count: usize,
    avail_w: f32,
    min_cell: f32,
    max_cell: f32,
    gap: f32,
) -> f32 {
    if count == 0 {
        return max_cell;
    }
    let n = count as f32;
    let fitted = (avail_w - gap * (n - 1.0)) / n;
    fitted.clamp(min_cell, max_cell)
}

#[derive(Clone, Copy)]
struct IconCellStyle {
    cell: f32,
    thumb: f32,
    show_remove: bool,
}

fn metrics_for(kind: IconGridKind, count: usize, avail_w: f32) -> (IconCellStyle, f32) {
    let show_remove = kind.show_remove();
    if kind.scale_with_count() {
        let inset = EDIT_CELL - EDIT_THUMB;
        let cell = adaptive_icon_cell(count, avail_w, EDIT_CELL, EDIT_CELL_MAX, EDIT_GAP);
        (
            IconCellStyle {
                cell,
                thumb: (cell - inset).max(0.0),
                show_remove,
            },
            EDIT_GAP,
        )
    } else {
        (
            IconCellStyle {
                cell: GRID_CELL,
                thumb: GRID_THUMB,
                show_remove,
            },
            GRID_GAP,
        )
    }
}

/// Top-right × badge rect for a cell body (extends slightly past the cell edge).
fn remove_badge_rect(body: egui::Rect) -> egui::Rect {
    egui::Rect::from_center_size(
        egui::pos2(
            body.right() - REMOVE_BTN * 0.35,
            body.top() + REMOVE_BTN * 0.35,
        ),
        Vec2::splat(REMOVE_BTN),
    )
}

/// Paint and interact the remove × badge over `body`.
///
/// Uses [`Sense::click_and_drag`] so, when registered *after* a parent
/// [`Ui::dnd_drag_source`]'s drag sense, this badge wins both click and drag
/// hit-testing over its rect (pure `Sense::drag` would otherwise steal the
/// inner half of the badge and start a reorder drag).
fn paint_remove_badge(ui: &mut egui::Ui, body: egui::Rect, target: &str) -> egui::Response {
    let btn_rect = remove_badge_rect(body);
    let btn_id = ui.id().with(("icon_rm", target));
    let btn_resp = ui.interact(btn_rect, btn_id, Sense::click_and_drag());
    let btn_fill = if btn_resp.hovered() {
        picker_remove_hover()
    } else {
        Color32::from_gray(100)
    };
    ui.painter()
        .circle_filled(btn_rect.center(), REMOVE_BTN * 0.5, btn_fill);
    crate::theme::paint_text_centered(
        ui,
        btn_rect,
        "×",
        egui::FontId::proportional(REMOVE_BTN * 0.75),
        Color32::WHITE,
    );
    btn_resp
}

/// Paint a selectable icon cell (fixed square, no under-icon label).
/// Returns `(cell_clicked, remove_clicked, cell_response)`.
///
/// When `style.show_remove` is true the × is painted here only if
/// `paint_remove` is true; reorderable grids paint the badge after
/// [`Ui::dnd_drag_source`] so it sits above the drag sense.
fn icon_grid_cell_ex(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    target: &str,
    selected: bool,
    style: IconCellStyle,
    paint_remove: bool,
) -> (bool, bool, egui::Response) {
    let IconCellStyle {
        cell,
        thumb,
        show_remove,
    } = style;
    let rounding = if show_remove { 3.0 } else { 4.0 };

    let desired = Vec2::splat(cell);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click_and_drag());

    let fill = if selected {
        picker_selected_fill()
    } else if resp.hovered() {
        Color32::from_black_alpha(25)
    } else {
        Color32::TRANSPARENT
    };
    let body = rect;
    ui.painter().rect_filled(body, rounding, fill);
    if selected {
        ui.painter().rect_stroke(
            body,
            rounding,
            egui::Stroke::new(2.0, picker_selected_stroke()),
            egui::StrokeKind::Outside,
        );
    }

    let tex = icons.for_target_or_fallback(ui.ctx(), catalog, target);
    crate::icon_cache::paint_icon_thumb_at(ui, &tex, body.center(), thumb, thumb, 0.0, None);

    let remove_clicked =
        show_remove && paint_remove && paint_remove_badge(ui, body, target).clicked();

    attach_item_icon_tooltip(&resp, catalog, icons, target);

    (resp.clicked() && !remove_clicked, remove_clicked, resp)
}

/// Lay out `targets` in even rows (no column stretch, no staircase wrap).
///
/// When `on_reorder` is provided, dragging a cell onto another reorders the list
/// (`from_index`, `to_index` in the displayed `targets` slice).
///
/// `is_removable` further gates the × badge when [`IconGridKind::Targets`] has
/// `removable: true` (e.g. hide × on tag-filter-only Image Search matches).
#[allow(clippy::too_many_arguments)] // even grid: selection, kind, and click/remove/reorder callbacks
pub fn paint_even_icon_grid(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    targets: &[String],
    is_selected: impl Fn(&str) -> bool,
    kind: IconGridKind,
    mut on_cell: impl FnMut(usize, &str),
    mut on_remove: impl FnMut(usize),
    mut on_reorder: Option<&mut dyn FnMut(usize, usize)>,
    is_removable: impl Fn(&str) -> bool,
) {
    if targets.is_empty() {
        return;
    }
    // Visible clip ∩ max_rect, minus floating scrollbar overlay — not leftover
    // room toward Window max_size, and not width the bar will cover.
    let avail_raw = crate::widgets::visible_content_width(ui);
    let (style, gap) = metrics_for(kind, targets.len(), avail_raw);
    // Cap to visible width — do not inflate past the pane (would raise min_size).
    let avail = avail_raw;
    ui.set_max_width(avail);
    let cols = grid_column_count_for_width(avail, style.cell, gap);
    let old_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = Vec2::splat(gap);

    let reorderable = on_reorder.is_some();
    let mut pending_reorder: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < targets.len() {
        ui.allocate_ui_with_layout(
            egui::vec2(avail, style.cell),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(avail);
                ui.spacing_mut().item_spacing = Vec2::splat(gap);
                let end = (i + cols).min(targets.len());
                for (k, target) in targets.iter().enumerate().take(end).skip(i) {
                    let sel = is_selected(target);
                    let cell_removable = style.show_remove && is_removable(target);
                    let cell_id = ui.id().with(("icon_dnd", k, target));
                    if reorderable {
                        // Paint × after `dnd_drag_source`: that API registers
                        // Sense::drag over the whole cell *after* contents, which
                        // would otherwise steal the badge's inner hit area.
                        let drag = ui.dnd_drag_source(cell_id, k, |ui| {
                            let (clicked, _, cell) =
                                icon_grid_cell_ex(ui, catalog, icons, target, sel, style, false);
                            if clicked {
                                on_cell(k, target);
                            }
                            cell
                        });
                        if cell_removable
                            && !ui.ctx().is_being_dragged(cell_id)
                            && paint_remove_badge(ui, drag.inner.rect, target).clicked()
                        {
                            on_remove(k);
                        }
                        if let Some(payload) = drag.response.dnd_release_payload::<usize>() {
                            let from = *payload;
                            if from != k {
                                pending_reorder = Some((from, k));
                            }
                        } else if drag.response.dnd_hover_payload::<usize>().is_some() {
                            ui.painter().rect_stroke(
                                drag.response.rect,
                                3.0,
                                egui::Stroke::new(2.0, picker_drop_stroke()),
                                egui::StrokeKind::Outside,
                            );
                        }
                    } else {
                        let cell_style = IconCellStyle {
                            show_remove: cell_removable,
                            ..style
                        };
                        let (clicked, remove, _) =
                            icon_grid_cell_ex(ui, catalog, icons, target, sel, cell_style, true);
                        if clicked {
                            on_cell(k, target);
                        }
                        if remove {
                            on_remove(k);
                        }
                    }
                }
            },
        );
        i += cols;
    }

    if let (Some((from, to)), Some(cb)) = (pending_reorder, on_reorder.as_mut()) {
        cb(from, to);
    }

    ui.spacing_mut().item_spacing = old_spacing;
}
