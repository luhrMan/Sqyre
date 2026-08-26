use super::types::{
    EDIT_CELL, EDIT_CELL_MAX, EDIT_GAP, EDIT_THUMB, GRID_CELL, GRID_GAP, GRID_THUMB, REMOVE_BTN,
};
use crate::icon_cache::IconCache;
use crate::image_view;
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

/// Rich hover tooltip: bold name, 12×12 variant icons, then italic primary-colored tags.
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
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(2.0);
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
                let (rect, _) =
                    ui.allocate_exact_size(size, egui::Sense::hover());
                crate::icon_cache::paint_icon_thumb_at(
                    ui,
                    &tex,
                    rect.center(),
                    VARIANT_TIP_THUMB,
                    VARIANT_TIP_THUMB,
                    0.0,
                    None,
                );
            }
        });
    }

    if tags.is_empty() {
        return;
    }
    ui.add_space(4.0);
    let color = ui.visuals().hyperlink_color;
    for tag in tags {
        ui.label(egui::RichText::new(tag).small().italics().color(color));
    }
}

pub(crate) fn grid_column_count_for_width(avail_w: f32, cell: f32, gap: f32) -> usize {
    let avail = avail_w.max(cell);
    let cols = ((avail + gap) / (cell + gap)).floor() as usize;
    cols.max(1)
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

/// Paint a selectable icon cell (fixed square, no under-icon label).
/// Returns `(cell_clicked, remove_clicked)`.
fn icon_grid_cell_ex(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    target: &str,
    selected: bool,
    style: IconCellStyle,
) -> (bool, bool) {
    let IconCellStyle {
        cell,
        thumb,
        show_remove,
    } = style;
    let rounding = if show_remove { 3.0 } else { 4.0 };

    let mut remove_clicked = false;
    let desired = Vec2::splat(cell);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());

    let fill = if selected {
        Color32::from_rgba_unmultiplied(80, 160, 100, 60)
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
            egui::Stroke::new(2.0, Color32::from_rgb(60, 140, 80)),
            egui::StrokeKind::Outside,
        );
    }

    let tex = icons.for_target_or_fallback(ui.ctx(), catalog, target);
    crate::icon_cache::paint_icon_thumb_at(ui, &tex, body.center(), thumb, thumb, 0.0, None);

    if show_remove {
        let btn_rect = egui::Rect::from_center_size(
            egui::pos2(
                body.right() - REMOVE_BTN * 0.35,
                body.top() + REMOVE_BTN * 0.35,
            ),
            Vec2::splat(REMOVE_BTN),
        );
        let btn_id = ui.id().with(("icon_rm", target));
        let btn_resp = ui.interact(btn_rect, btn_id, Sense::click());
        let btn_fill = if btn_resp.hovered() {
            Color32::from_rgb(180, 60, 60)
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
        remove_clicked = btn_resp.clicked();
    }

    attach_item_icon_tooltip(&resp, catalog, icons, target);

    (resp.clicked() && !remove_clicked, remove_clicked)
}

/// Lay out `targets` in even rows (no column stretch, no staircase wrap).
#[allow(clippy::too_many_arguments)] // even grid: selection, kind, and click/remove callbacks
pub fn paint_even_icon_grid(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    targets: &[String],
    is_selected: impl Fn(&str) -> bool,
    kind: IconGridKind,
    mut on_cell: impl FnMut(usize, &str),
    mut on_remove: impl FnMut(usize),
) {
    if targets.is_empty() {
        return;
    }
    let avail_raw = ui.available_width();
    let (style, gap) = metrics_for(kind, targets.len(), avail_raw);
    let avail = avail_raw.max(style.cell);
    ui.set_max_width(avail);
    let cols = grid_column_count_for_width(avail, style.cell, gap);
    let old_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = Vec2::splat(gap);

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
                    let (clicked, remove) =
                        icon_grid_cell_ex(ui, catalog, icons, target, sel, style);
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
