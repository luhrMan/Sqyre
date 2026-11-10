use super::types::{
    EDIT_CELL, EDIT_CELL_MAX, EDIT_GAP, EDIT_THUMB, GRID_CELL, GRID_GAP, GRID_THUMB,
};
use crate::icon_cache::IconCache;
use crate::image_view;
use crate::theme::{picker_drop_stroke, picker_selected_fill, picker_selected_stroke};
use eframe::egui::{self, Sense, Vec2};
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
    ui.set_max_width(280.0_f32.min(crate::widgets::visible_content_width(ui)));
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
    let color = ui.visuals().hyperlink_color;
    let program_icons = icons.program_name_icons(ui.ctx(), catalog);
    for tag in tags {
        let icon = crate::widgets::tags::program_icon_for_tag(Some(&program_icons), tag);
        if let Some(tex) = icon {
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::splat(VARIANT_TIP_THUMB), egui::Sense::hover());
                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                ui.label(egui::RichText::new(tag).small().italics().color(color));
            });
        } else {
            ui.label(egui::RichText::new(tag).small().italics().color(color));
        }
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
    /// Height of one grid row: tallest stable thumb plus the cell inset, so wide
    /// icons do not leave a mostly empty square cell and rotating variants cannot
    /// resize the row each second.
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
                crate::icon_cache::stable_icon_thumb_size(
                    ctx, catalog, icons, target, self.thumb, self.thumb,
                )
                .y
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

    // `contains_pointer`: when wrapped in `dnd_drag_source`, the outer drag
    // interact steals exclusive `hovered()`, but the pointer is still here.
    let fill = if selected {
        picker_selected_fill()
    } else if resp.contains_pointer() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().widgets.inactive.weak_bg_fill
    };
    let body = rect;
    ui.painter().rect_filled(body, rounding, fill);
    if selected {
        ui.painter().rect_stroke(
            body,
            rounding,
            egui::Stroke::new(2.0, picker_selected_stroke()),
            egui::StrokeKind::Inside,
        );
    } else {
        // Slight button look: the cell is a control, not a static thumbnail.
        ui.painter().rect_stroke(
            body,
            rounding,
            ui.visuals().widgets.inactive.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    crate::widgets::controls::paint_keyboard_focus_ring(ui, body, &resp);

    let (name, _) = item_tooltip_parts(catalog, target);
    let enabled = ui.is_enabled();
    resp.clone()
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name.clone()));

    let tex = icons.for_target_or_fallback(ui.ctx(), catalog, target);
    crate::icon_cache::paint_icon_thumb_at(ui, &tex, body.center(), thumb, thumb, 0.0, None);

    // Caller attaches the tip: under `dnd_drag_source` it must use the outer
    // drag response (that owns hover + Grab cursor), not this inner cell.
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
    // A pane narrower than one cell used to clip the cell. Size the row to the
    // cell so the parent scroll can reach it; wide panes keep the old cap.
    let row_w = if avail + 0.5 < style.cell {
        style.cell
    } else {
        ui.set_max_width(avail);
        avail
    };
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
            egui::vec2(row_w, row_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(row_w);
                ui.spacing_mut().item_spacing = Vec2::splat(gap);
                for (k, target) in targets.iter().enumerate().take(end).skip(i) {
                    let sel = is_selected(target);
                    let cell_removable = kind.show_remove()
                        && ops.on_remove.is_some()
                        && ops.is_removable.is_none_or(|pred| pred(target));
                    let cell_id = ui.id().with(("icon_dnd", k, target));
                    let (cell_rect, cell_anchor) = if reorderable {
                        // `dnd_drag_source` adds a later Sense::drag interact that
                        // owns hover (Grab hand). Tip must attach to that outer
                        // response — same pattern as reorderable tag chips.
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
                        attach_item_icon_tooltip(&drag.response, catalog, icons, target);
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
                        (drag.inner.rect, Some(drag.response))
                    } else {
                        let (clicked, cell) =
                            icon_grid_cell(ui, catalog, icons, target, sel, style, row_h);
                        attach_item_icon_tooltip(&cell, catalog, icons, target);
                        if clicked {
                            if let Some(on_cell) = ops.on_cell.as_mut() {
                                on_cell(k, target);
                            }
                        }
                        (cell.rect, Some(cell))
                    };
                    if cell_removable || cell_menu.is_some() {
                        let menu_id = cell_id.with("menu");
                        crate::widgets::rect_context_menu(
                            ui,
                            menu_id,
                            cell_rect,
                            cell_anchor.as_ref(),
                            |ui| {
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
                            },
                        );
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The selection ring must not spill past the cell into the grid gap or the
    /// enclosing pane's padding — paint it inside the cell bounds.
    #[test]
    fn selected_cell_stroke_stays_inside_cell() {
        let ctx = egui::Context::default();
        let catalog = ProgramCatalog::default();
        let mut icons = IconCache::new();
        let targets = vec!["p~item".to_string()];
        for _ in 0..3 {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                paint_even_icon_grid(
                    ui,
                    &catalog,
                    &mut icons,
                    &targets,
                    |_| true,
                    IconGridKind::Picker,
                    IconGridOps::default(),
                    None,
                );
            });
            let outside = out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect) => {
                    rect.stroke == egui::Stroke::new(2.0, picker_selected_stroke())
                        && rect.stroke_kind == egui::StrokeKind::Outside
                }
                _ => false,
            });
            assert!(!outside, "selection stroke painted outside the cell");
            out.drop_without_applying_deltas();
        }
    }

    /// Hover tips are self-sizing egui `Area`s: a width bound derived from the
    /// tip's own painted size ratchets narrower every frame until text wraps
    /// into single-character rows. The name galley must keep the width it
    /// measured on the first open frame.
    #[test]
    fn item_tooltip_width_does_not_ratchet() {
        let ctx = egui::Context::default();
        let catalog = ProgramCatalog::default();
        let mut icons = IconCache::new();
        let target = "prog~a_fairly_long_item_name_to_wrap";
        let cell = egui::Rect::from_min_size(egui::pos2(40.0, 40.0), egui::vec2(24.0, 24.0));
        let mut widths = Vec::new();
        for frame in 0..10 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(1024.0, 768.0),
                )),
                time: Some(frame as f64 * 0.25),
                predicted_dt: 0.25,
                events: if frame == 0 {
                    vec![egui::Event::PointerMoved(cell.center())]
                } else {
                    Vec::new()
                },
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| {
                let resp = ui.put(cell, egui::Button::new(""));
                attach_item_icon_tooltip(&resp, &catalog, &mut icons, target);
            });
            let tip_w = out.shapes.iter().find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.text().contains("a_fairly") => {
                    Some(t.galley.size().x)
                }
                _ => None,
            });
            if let Some(w) = tip_w {
                widths.push(w);
            }
            out.drop_without_applying_deltas();
        }
        assert!(widths.len() >= 2, "item tooltip never opened");
        let (first, last) = (widths[0], *widths.last().unwrap());
        assert!(
            (last - first).abs() <= 1.0,
            "item tooltip width ratcheted across frames: {first:.1} -> {last:.1} ({widths:?})"
        );
    }

    /// Reorderable grids wrap each cell in `dnd_drag_source`, whose later
    /// Sense::drag interact steals exclusive `hovered()` (Grab hand). The tip
    /// must open via the outer drag response — not the inner cell alone.
    #[test]
    fn item_tooltip_opens_on_reorderable_grid() {
        let ctx = egui::Context::default();
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = 0.0;
            style.interaction.tooltip_grace_time = 0.0;
        });
        let catalog = ProgramCatalog::default();
        let mut icons = IconCache::new();
        let targets = vec!["prog~dnd_tip_item".to_owned()];
        let needle = "dnd_tip_item";
        // First cell sits near the origin inside the grid layout.
        let hover = egui::pos2(36.0, 36.0);
        let mut saw_tip = false;
        let mut reorder = |_from: usize, _to: usize| {};
        for frame in 0..12 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(1024.0, 768.0),
                )),
                time: Some(frame as f64 * 0.25),
                predicted_dt: 0.25,
                events: if frame == 0 {
                    vec![egui::Event::PointerMoved(hover)]
                } else {
                    Vec::new()
                },
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| {
                paint_even_icon_grid(
                    ui,
                    &catalog,
                    &mut icons,
                    &targets,
                    |_| false,
                    IconGridKind::Targets { removable: false },
                    IconGridOps {
                        on_reorder: Some(&mut reorder),
                        ..Default::default()
                    },
                    None,
                );
            });
            let tip = out.shapes.iter().any(|clipped| {
                matches!(
                    &clipped.shape,
                    egui::Shape::Text(t) if t.galley.text().contains(needle)
                )
            });
            if tip {
                saw_tip = true;
                break;
            }
            out.drop_without_applying_deltas();
        }
        assert!(
            saw_tip,
            "item tooltip never opened on reorderable icon grid (Grab hand steals hover)"
        );
    }

    /// Target grids (overlay gate items, Image Search tips) used the current
    /// rotating variant's aspect for row height — wide then tall icons made the
    /// section jump every second. Slot height must stay fixed across ticks.
    #[test]
    fn target_grid_row_height_stable_across_variant_rotation() {
        crate::demo_icons::with_exclusive(|| {
            crate::demo_icons::clear();
            crate::demo_icons::register_item_variant_solid(
                "Prog",
                "Gate",
                "wide",
                64,
                16,
                [255, 0, 0, 255],
            );
            crate::demo_icons::register_item_variant_solid(
                "Prog",
                "Gate",
                "tall",
                16,
                64,
                [0, 255, 0, 255],
            );
            let target = "Prog~Gate".to_string();
            let catalog = ProgramCatalog::default();
            let ctx = egui::Context::default();
            let mut icons = IconCache::new();
            for path in crate::demo_icons::merged_variant_paths(&catalog, &target) {
                assert!(
                    icons.for_path(&ctx, &path).is_some(),
                    "demo variant failed to load: {}",
                    path.display()
                );
            }
            // Prove the fitted heights actually differ — otherwise the test is inert.
            let wide = image_view::fit_icon_thumb(64.0, 16.0, EDIT_THUMB, EDIT_THUMB);
            let tall = image_view::fit_icon_thumb(16.0, 64.0, EDIT_THUMB, EDIT_THUMB);
            assert!(
                (wide.y - tall.y).abs() > 1.0,
                "fixture aspect ratios must differ: wide={wide:?} tall={tall:?}"
            );

            let mut heights = Vec::new();
            let mut reorder = |_from: usize, _to: usize| {};
            for tick in 0..5 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::pos2(0.0, 0.0),
                        egui::vec2(640.0, 480.0),
                    )),
                    time: Some(tick as f64),
                    predicted_dt: 1.0,
                    ..Default::default()
                };
                let out = ctx.run_ui(input, |ui| {
                    let y0 = ui.cursor().min.y;
                    paint_even_icon_grid(
                        ui,
                        &catalog,
                        &mut icons,
                        std::slice::from_ref(&target),
                        |_| false,
                        IconGridKind::Targets { removable: false },
                        IconGridOps {
                            on_reorder: Some(&mut reorder),
                            ..Default::default()
                        },
                        None,
                    );
                    heights.push(ui.cursor().min.y - y0);
                });
                out.drop_without_applying_deltas();
            }
            let first = heights[0];
            assert!(
                heights.iter().all(|h| (h - first).abs() <= 0.5),
                "target grid row height jumped across variant ticks: {heights:?}"
            );
        });
    }
}
