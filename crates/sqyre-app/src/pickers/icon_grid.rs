use super::types::{
    EDIT_CELL, EDIT_CELL_MAX, EDIT_GAP, EDIT_THUMB, GRID_CELL, GRID_GAP, GRID_THUMB,
};
use crate::icon_cache::IconCache;
use crate::image_view;
use crate::theme::{picker_drop_stroke, picker_selected_fill, picker_selected_stroke};
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
    /// the compact edit size as more are added. `removable` adds a Remove menu entry.
    Targets { removable: bool },
    /// Read-only target list sized like a [`Self::Targets`] grid of `count` items,
    /// so sibling grids share one cell size.
    TargetsSizedAs { count: usize },
}

impl IconGridKind {
    fn show_remove(self) -> bool {
        matches!(self, Self::Targets { removable: true })
    }

    fn scale_with_count(self) -> bool {
        matches!(self, Self::Targets { .. } | Self::TargetsSizedAs { .. })
    }

    fn sizing_count(self, count: usize) -> usize {
        match self {
            Self::TargetsSizedAs { count } => count,
            _ => count,
        }
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
}

impl IconCellStyle {
    /// Height of one grid row: tallest fitted thumb plus the cell inset, so wide
    /// icons do not leave a mostly empty square cell.
    fn row_height(
        self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        row: &[String],
    ) -> f32 {
        let inset = self.cell - self.thumb;
        let tallest = row
            .iter()
            .map(|target| {
                let [tw, th] = icons.for_target_or_fallback(ctx, catalog, target).size();
                image_view::fit_icon_thumb(tw as f32, th as f32, self.thumb, self.thumb).y
            })
            .fold(0.0, f32::max);
        (tallest + inset).clamp(inset.max(1.0), self.cell)
    }
}

fn metrics_for(kind: IconGridKind, count: usize, avail_w: f32) -> (IconCellStyle, f32) {
    if kind.scale_with_count() {
        let inset = EDIT_CELL - EDIT_THUMB;
        let cell = adaptive_icon_cell(
            kind.sizing_count(count),
            avail_w,
            EDIT_CELL,
            EDIT_CELL_MAX,
            EDIT_GAP,
        );
        (
            IconCellStyle {
                cell,
                thumb: (cell - inset).max(0.0),
            },
            EDIT_GAP,
        )
    } else {
        (
            IconCellStyle {
                cell: GRID_CELL,
                thumb: GRID_THUMB,
            },
            GRID_GAP,
        )
    }
}

/// Paint a selectable `cell_w`×`row_h` icon cell (no under-icon label).
/// Returns `(cell_clicked, cell_response)`.
fn icon_grid_cell(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    target: &str,
    selected: bool,
    style: IconCellStyle,
    row_h: f32,
) -> (bool, egui::Response) {
    let IconCellStyle { cell, thumb } = style;
    let rounding = 4.0;

    let desired = Vec2::new(cell, row_h);
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

    attach_item_icon_tooltip(&resp, catalog, icons, target);

    (resp.clicked(), resp)
}

/// Extra right-click menu entries for a grid cell, given its target.
pub type IconCellMenu<'a> = dyn FnMut(&mut egui::Ui, &str) + 'a;
/// Primary click on a cell: `(index, target)`.
pub type IconCellClick<'a> = dyn FnMut(usize, &str) + 'a;
/// Remove (or similar) for one displayed index.
pub type IconIndexClick<'a> = dyn FnMut(usize) + 'a;
/// Drag-reorder `(from_index, to_index)` in the displayed slice.
pub type IconReorder<'a> = dyn FnMut(usize, usize) + 'a;
/// Whether a target may show Remove.
pub type IconTargetPred<'a> = dyn Fn(&str) -> bool + 'a;

/// Click, remove, and reorder behavior for [`paint_even_icon_grid`].
///
/// [`Default`] paints a display-only grid. `is_removable: None` allows Remove on
/// every target (still gated by [`IconGridKind::Targets`] `{ removable: true }`).
/// Extra menu entries stay a separate argument so their lifetime is not tied to
/// these callbacks.
#[derive(Default)]
pub struct IconGridOps<'a> {
    pub on_cell: Option<&'a mut IconCellClick<'a>>,
    pub on_remove: Option<&'a mut IconIndexClick<'a>>,
    pub on_reorder: Option<&'a mut IconReorder<'a>>,
    pub is_removable: Option<&'a IconTargetPred<'a>>,
}

/// Lay out `targets` in even rows (no column stretch, no staircase wrap).
///
/// When `ops.on_reorder` is set, dragging a cell onto another reorders the list
/// (`from_index`, `to_index` in the displayed `targets` slice).
///
/// Right-clicking a cell opens a menu with `cell_menu` entries, then Remove when
/// [`IconGridKind::Targets`] has `removable: true`, `ops.on_remove` is set, and
/// `ops.is_removable` allows it (e.g. no Remove on tag-filter-only Image Search matches).
#[expect(
    clippy::too_many_arguments,
    reason = "grid inputs plus ops; menu lifetime stays separate"
)]
pub fn paint_even_icon_grid(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    targets: &[String],
    is_selected: impl Fn(&str) -> bool,
    kind: IconGridKind,
    mut ops: IconGridOps<'_>,
    mut cell_menu: Option<&mut IconCellMenu<'_>>,
) {
    if targets.is_empty() {
        return;
    }
    // Visible clip ∩ max_rect, minus floating scrollbar overlay — not leftover
    // room toward Window max_size, and not width the bar will cover.
    let avail = crate::widgets::visible_content_width(ui);
    let (style, gap) = metrics_for(kind, targets.len(), avail);
    ui.set_max_width(avail);
    let cols = grid_column_count_for_width(avail, style.cell, gap);
    let old_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = Vec2::splat(gap);

    let reorderable = ops.on_reorder.is_some();
    let mut pending_reorder: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < targets.len() {
        let end = (i + cols).min(targets.len());
        let row_h = if kind.scale_with_count() {
            style.row_height(ui.ctx(), catalog, icons, &targets[i..end])
        } else {
            style.cell
        };
        ui.allocate_ui_with_layout(
            egui::vec2(avail, row_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(avail);
                ui.spacing_mut().item_spacing = Vec2::splat(gap);
                for (k, target) in targets.iter().enumerate().take(end).skip(i) {
                    let sel = is_selected(target);
                    let cell_removable = kind.show_remove()
                        && ops.on_remove.is_some()
                        && ops.is_removable.is_none_or(|pred| pred(target));
                    let cell_id = ui.id().with(("icon_dnd", k, target));
                    let cell_rect = if reorderable {
                        let drag = ui.dnd_drag_source(cell_id, k, |ui| {
                            let (clicked, cell) =
                                icon_grid_cell(ui, catalog, icons, target, sel, style, row_h);
                            if clicked {
                                if let Some(on_cell) = ops.on_cell.as_mut() {
                                    on_cell(k, target);
                                }
                            }
                            cell
                        });
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
                        drag.inner.rect
                    } else {
                        let (clicked, cell) =
                            icon_grid_cell(ui, catalog, icons, target, sel, style, row_h);
                        if clicked {
                            if let Some(on_cell) = ops.on_cell.as_mut() {
                                on_cell(k, target);
                            }
                        }
                        cell.rect
                    };
                    if cell_removable || cell_menu.is_some() {
                        let menu_id = cell_id.with("menu");
                        crate::widgets::rect_context_menu(ui, menu_id, cell_rect, |ui| {
                            if let Some(menu) = cell_menu.as_mut() {
                                menu(ui, target);
                            }
                            if cell_removable
                                && crate::widgets::menu_item_danger(ui, "Remove", true)
                            {
                                if let Some(on_remove) = ops.on_remove.as_mut() {
                                    on_remove(k);
                                }
                            }
                        });
                    }
                }
            },
        );
        i += cols;
    }

    if let (Some((from, to)), Some(cb)) = (pending_reorder, ops.on_reorder.as_mut()) {
        cb(from, to);
    }

    ui.spacing_mut().item_spacing = old_spacing;
}
