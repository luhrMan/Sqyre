use crate::icon_cache::IconCache;
use crate::image_view::{self, ImageViewTransform};
use crate::theme;
use crate::var_pills;
use eframe::egui;
use sqyre_domain::{KnownVariableNames, PROGRAM_DELIMITER};
use sqyre_validate::EntryValidation;

pub(crate) fn paint_preview_toolbar(
    ui: &mut egui::Ui,
    view: Option<&mut ImageViewTransform>,
) -> bool {
    ui.add_space(8.0);
    ui.separator();
    let mut force = false;
    let show_zoom_hint = view.is_some();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Preview").strong());
        if crate::theme::icon_button(ui, "↻")
            .on_hover_text("Refresh")
            .clicked()
        {
            force = true;
        }
        if let Some(view) = view {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(view.needs_reset_button(), egui::Button::new("Reset view"))
                    .on_hover_text("Fit image in viewport")
                    .clicked()
                {
                    view.reset();
                }
                if view.shows_zoom_label() {
                    ui.weak(format!("{:.0}%", view.zoom * 100.0));
                }
            });
        }
    });
    if show_zoom_hint {
        ui.weak("Scroll to zoom; drag to pan when zoomed.");
    }
    force
}

#[derive(Clone, Copy)]
pub(crate) enum CardinalEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Pure integer literal (no `${var}` / expressions) — enables drag + steppers.
fn pure_i32(s: &str) -> Option<i32> {
    let t = s.trim();
    if t.is_empty() || sqyre_varref::contains(t) {
        return None;
    }
    t.parse().ok()
}

/// Coord chip overlaid on a preview edge: sizes to text, Sqyre yellow border,
/// and when the value is a pure integer: drag-to-adjust + −/+ steppers.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_preview_coord_chip(
    ui: &mut egui::Ui,
    preview: egui::Rect,
    edge: CardinalEdge,
    placeholder: &str,
    value: &mut String,
    known: &KnownVariableNames,
    is_dark: bool,
    validation: &EntryValidation,
    help: &str,
) {
    const PAD_X: f32 = 8.0;
    const MIN_EDIT_W: f32 = 28.0;
    const CHIP_H: f32 = 22.0;
    const STEP_W: f32 = 18.0;
    const GAP: f32 = 2.0;
    const EDGE_PAD: f32 = 6.0;

    let id = ui.id().with(("preview_coord", placeholder));
    let focused = ui.memory(|m| m.has_focus(id));
    let pure = pure_i32(value);
    let has_steppers = pure.is_some();

    let font = egui::TextStyle::Body.resolve(ui.style());
    let measure = if value.is_empty() {
        placeholder
    } else {
        value.as_str()
    };
    let galley =
        ui.painter()
            .layout_no_wrap(measure.to_owned(), font.clone(), egui::Color32::WHITE);
    let edit_w = (galley.size().x + PAD_X * 2.0 + if focused { 8.0 } else { 0.0 }).max(MIN_EDIT_W);
    let total_w = if has_steppers {
        STEP_W + GAP + edit_w + GAP + STEP_W
    } else {
        edit_w
    };
    let size = egui::vec2(total_w, CHIP_H);
    let center = match edge {
        CardinalEdge::Top => {
            egui::pos2(preview.center().x, preview.top() + EDGE_PAD + CHIP_H * 0.5)
        }
        CardinalEdge::Bottom => egui::pos2(
            preview.center().x,
            preview.bottom() - EDGE_PAD - CHIP_H * 0.5,
        ),
        CardinalEdge::Left => egui::pos2(
            preview.left() + EDGE_PAD + total_w * 0.5,
            preview.center().y,
        ),
        CardinalEdge::Right => egui::pos2(
            preview.right() - EDGE_PAD - total_w * 0.5,
            preview.center().y,
        ),
    };
    let group = egui::Rect::from_center_size(center, size);

    let (minus_rect, edit_rect, plus_rect) = if has_steppers {
        let minus = egui::Rect::from_min_size(group.min, egui::vec2(STEP_W, CHIP_H));
        let edit = egui::Rect::from_min_size(
            egui::pos2(minus.right() + GAP, group.top()),
            egui::vec2(edit_w, CHIP_H),
        );
        let plus = egui::Rect::from_min_size(
            egui::pos2(edit.right() + GAP, group.top()),
            egui::vec2(STEP_W, CHIP_H),
        );
        (Some(minus), edit, Some(plus))
    } else {
        (None, group, None)
    };

    let fill = egui::Color32::from_rgba_unmultiplied(16, 16, 16, 170);
    let radius = 4.0;
    ui.painter().rect_filled(edit_rect, radius, fill);
    let border = var_pills::entry_validation_stroke(validation)
        .unwrap_or_else(|| egui::Stroke::new(1.5, theme::PRIMARY));
    ui.painter()
        .rect_stroke(edit_rect, radius, border, egui::StrokeKind::Outside);

    // Use `place` (not `put`) so overlay chips do not advance the form cursor
    // into the preview — widgets after the panel must stay below it.
    if let (Some(minus), Some(n)) = (minus_rect, pure) {
        let resp = ui.place(
            minus,
            egui::Button::new("−")
                .fill(fill)
                .stroke(egui::Stroke::new(1.0, theme::PRIMARY))
                .corner_radius(radius)
                .min_size(minus.size()),
        );
        if resp.clicked() {
            *value = (n.saturating_sub(1)).to_string();
        }
        resp.on_hover_text("Decrement");
    }
    if let (Some(plus), Some(n)) = (plus_rect, pure) {
        let resp = ui.place(
            plus,
            egui::Button::new("+")
                .fill(fill)
                .stroke(egui::Stroke::new(1.0, theme::PRIMARY))
                .corner_radius(radius)
                .min_size(plus.size()),
        );
        if resp.clicked() {
            *value = (n.saturating_add(1)).to_string();
        }
        resp.on_hover_text("Increment");
    }

    let show_overlay = !focused && !value.is_empty() && sqyre_varref::contains(value.as_str());
    let inner = edit_rect.shrink(3.0);

    let resp = if show_overlay {
        let plain_fg = egui::Color32::from_gray(230);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
        ));
        child.set_min_size(inner.size());
        var_pills::paint_var_ref_content(&mut child, value, known, is_dark, plain_fg);
        ui.interact(edit_rect, id.with("overlay"), egui::Sense::click())
    } else if let Some(n) = pure.filter(|_| !focused) {
        // Unfocused pure number: drag to adjust, click to edit.
        let resp = ui.interact(edit_rect, id.with("drag"), egui::Sense::click_and_drag());
        ui.painter().text(
            edit_rect.center(),
            egui::Align2::CENTER_CENTER,
            n.to_string(),
            font,
            egui::Color32::from_gray(230),
        );
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if resp.clicked() {
            ui.memory_mut(|m| m.request_focus(id));
        }
        if resp.dragged() {
            let delta = resp.drag_delta().x - resp.drag_delta().y;
            let drag_id = id.with("drag_accum");
            let precise = ui.data_mut(|d| {
                let p = d.get_temp::<f64>(drag_id).unwrap_or(n as f64) + delta as f64;
                d.insert_temp(drag_id, p);
                p
            });
            let next = precise.round() as i32;
            if next != n {
                *value = next.to_string();
            }
        }
        if resp.drag_stopped() {
            ui.data_mut(|d| d.remove_temp::<f64>(id.with("drag_accum")));
        }
        resp
    } else {
        ui.place(
            inner,
            egui::TextEdit::singleline(value)
                .id(id)
                .frame(egui::Frame::NONE)
                .hint_text(placeholder)
                .desired_width(inner.width()),
        )
    };

    if show_overlay && resp.clicked() {
        ui.memory_mut(|m| m.request_focus(id));
    }
    if let Some(tip) = var_pills::entry_validation_tip(validation) {
        resp.on_hover_text(tip);
    } else if !help.is_empty() {
        resp.on_hover_text(help);
    }
}

pub(crate) fn variant_name_from_path(path: &std::path::Path, item: &str) -> String {
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return String::new();
    };
    if stem == item {
        return String::new();
    }
    let prefix = format!("{item}{PROGRAM_DELIMITER}");
    stem.strip_prefix(&prefix).unwrap_or(stem).to_string()
}

pub(crate) fn variant_display_label(name: &str) -> &str {
    if name.is_empty() {
        "(default)"
    } else {
        name
    }
}

pub(crate) fn fit_thumbnail(w: f32, h: f32) -> egui::Vec2 {
    const MAX: f32 = 96.0;
    image_view::fit_in_box_no_upscale(w, h, MAX, MAX)
}

/// 1px dim gold border around a Data Editor image preview.
pub(crate) fn paint_preview_frame(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_stroke(rect, 0.0, theme::inner_stroke(), egui::StrokeKind::Inside);
}

pub(crate) fn paint_disk_preview(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    path: Option<&std::path::Path>,
    fallback: Option<egui::TextureHandle>,
    title: &str,
    grid: Option<(i32, i32)>,
    replace_clicked: Option<&mut bool>,
) {
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).strong());
        if let Some(path) = path {
            if crate::theme::icon_button(ui, "↻")
                .on_hover_text("Refresh")
                .clicked()
            {
                icons.invalidate_path(path);
            }
        }
        if let Some(flag) = replace_clicked {
            if ui.button("Replace Image").clicked() {
                *flag = true;
            }
        }
    });
    let tex = path.and_then(|p| icons.for_path(ui.ctx(), p)).or(fallback);
    match tex {
        Some(tex) => {
            let [tw, th] = tex.size();
            let size = fit_panel(tw as f32, th as f32);
            let resp = ui.add(egui::Image::new((tex.id(), size)));
            paint_preview_frame(ui.painter(), resp.rect);
            if let Some((rows, cols)) = grid {
                paint_grid_overlay(ui, resp.rect, rows, cols);
            }
            if let Some(path) = path {
                if path.is_file() {
                    ui.weak(path.display().to_string());
                }
            }
        }
        None => {
            ui.weak("No image on disk.");
        }
    }
}

/// Collection-tab preview with wheel zoom / drag pan.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_zoomable_collection_preview(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    path: &std::path::Path,
    rows: i32,
    cols: i32,
    view: &mut ImageViewTransform,
    replace_clicked: &mut bool,
    capturing: bool,
) {
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Collection image").strong());
        if crate::theme::icon_button(ui, "↻")
            .on_hover_text("Refresh")
            .clicked()
        {
            icons.invalidate_path(path);
        }
        if ui
            .add_enabled(
                !capturing,
                egui::Button::new(if capturing {
                    "Capturing…"
                } else {
                    "Replace Image"
                }),
            )
            .clicked()
        {
            *replace_clicked = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(view.needs_reset_button(), egui::Button::new("Reset view"))
                .on_hover_text("Fit image in viewport")
                .clicked()
            {
                view.reset();
            }
            if view.shows_zoom_label() {
                ui.weak(format!("{:.0}%", view.zoom * 100.0));
            }
        });
    });
    ui.weak("Scroll to zoom; drag to pan when zoomed.");

    let tex = icons.for_path(ui.ctx(), path);
    let avail_w = ui.available_width();
    let path_footer_h = if path.is_file() {
        ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y
    } else {
        0.0
    };
    // Fill remaining height; reserve path label; slack avoids ScrollArea overflow hysteresis.
    const MIN_H: f32 = 120.0;
    const FILL_SLACK: f32 = 1.0;
    let avail_h = ui.available_height() - path_footer_h;
    let desired_h = if avail_h < MIN_H {
        MIN_H
    } else {
        (avail_h - FILL_SLACK).max(MIN_H)
    };
    let image_size = match &tex {
        Some(t) => {
            let [tw, th] = t.size();
            egui::vec2(tw as f32, th as f32)
        }
        None => egui::vec2(avail_w, avail_w * 0.75),
    };
    let desired = egui::vec2(avail_w.max(160.0), desired_h);
    let (viewport, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

    image_view::handle_scroll_zoom(ui, viewport, image_size, view, resp.hovered());
    let content = image_view::image_content_rect(viewport, image_size, view.zoom, view.pan);
    let body_font = egui::TextStyle::Body.resolve(ui.style());

    {
        let painter = ui.painter_at(viewport);
        if let Some(tex) = &tex {
            painter.image(
                tex.id(),
                content,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            painter.rect_filled(viewport, 0.0, egui::Color32::from_gray(40));
            painter.text(
                viewport.center(),
                egui::Align2::CENTER_CENTER,
                "No image on disk",
                body_font,
                egui::Color32::LIGHT_GRAY,
            );
        }
        paint_grid_overlay_painter(&painter, content, rows, cols);
        paint_preview_frame(&painter, viewport);
    }
    let _ = image_view::handle_pan_drag(&resp, viewport, image_size, view);

    if path.is_file() {
        ui.weak(path.display().to_string());
    }
}

pub(crate) fn show_file_hover(
    ui: &mut egui::Ui,
    response: &egui::Response,
    icons: &mut IconCache,
    path: &std::path::Path,
    label: &str,
) {
    if !response.hovered() {
        return;
    }
    match icons.for_path(ui.ctx(), path) {
        Some(tex) => {
            response.clone().on_hover_ui(|ui| {
                let [tw, th] = tex.size();
                let size = fit_panel(tw as f32, th as f32);
                ui.add(egui::Image::new((tex.id(), size)));
                ui.label(label);
            });
        }
        None => {
            response.clone().on_hover_text(label);
        }
    }
}

pub(crate) fn fit_panel(w: f32, h: f32) -> egui::Vec2 {
    const MAX_W: f32 = 340.0;
    const MAX_H: f32 = 240.0;
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (MAX_W / w).min(MAX_H / h).min(1.0);
    egui::vec2(w * scale, h * scale)
}

pub(crate) fn paint_grid_overlay(ui: &mut egui::Ui, rect: egui::Rect, rows: i32, cols: i32) {
    paint_grid_overlay_painter(ui.painter(), rect, rows, cols);
}

pub(crate) fn paint_grid_overlay_painter(
    painter: &egui::Painter,
    rect: egui::Rect,
    rows: i32,
    cols: i32,
) {
    let rows = rows.max(1) as f32;
    let cols = cols.max(1) as f32;
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 80, 80));
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

/// Per-monitor `(x, y, w, h)` in virtual-desktop coordinates for the atlas plane.
/// Prefers real positions from the capturer; falls back to L→R layout from sizes.
fn atlas_monitor_rects(catalog: &sqyre_persist::ProgramCatalog) -> Vec<(i32, i32, i32, i32)> {
    #[cfg(feature = "native-runtime")]
    if let Ok(capturer) = sqyre_capture::shared_capturer_nonblocking() {
        if let Ok(rects) = capturer.monitor_rects_ref() {
            let out: Vec<_> = rects
                .into_iter()
                .filter(|r| r.w > 0 && r.h > 0)
                .map(|r| (r.x, r.y, r.w, r.h))
                .collect();
            if !out.is_empty() {
                return out;
            }
        }
        // Sizes only: place primary at virtual origin, others to the right.
        if let Ok(sizes) = capturer.monitor_sizes_ref() {
            let (ox, oy) = capturer
                .virtual_bounds_ref()
                .map(|vb| (vb.x, vb.y))
                .unwrap_or((0, 0));
            let laid = layout_monitors_ltr(ox, oy, &sizes);
            if !laid.is_empty() {
                return laid;
            }
        }
        if let Ok(vb) = capturer.virtual_bounds_ref() {
            if vb.w > 0 && vb.h > 0 {
                return vec![(vb.x, vb.y, vb.w, vb.h)];
            }
        }
    }
    // Catalog resolution key (`"{w}x{h}"`) as a single primary monitor at origin.
    let key = catalog.resolution_key();
    if let Some((w, h)) = key.split_once('x') {
        if let (Ok(w), Ok(h)) = (w.parse::<i32>(), h.parse::<i32>()) {
            if w > 0 && h > 0 {
                return vec![(0, 0, w, h)];
            }
        }
    }
    vec![(0, 0, 1920, 1080)]
}

fn layout_monitors_ltr(ox: i32, oy: i32, sizes: &[(i32, i32)]) -> Vec<(i32, i32, i32, i32)> {
    let mut x = ox;
    let mut out = Vec::new();
    for &(w, h) in sizes {
        if w > 0 && h > 0 {
            out.push((x, oy, w, h));
            x = x.saturating_add(w);
        }
    }
    out
}

/// Desktop extent: union of monitors and collection member bounds.
fn atlas_plane_desktop_bounds(
    monitors: &[(i32, i32, i32, i32)],
    member_bounds: &[(i32, i32, i32, i32)],
) -> (i32, i32, i32, i32) {
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for &(x, y, w, h) in monitors {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x.saturating_add(w));
        max_y = max_y.max(y.saturating_add(h));
    }
    for &(lx, ty, rx, by) in member_bounds {
        min_x = min_x.min(lx);
        min_y = min_y.min(ty);
        max_x = max_x.max(rx);
        max_y = max_y.max(by);
    }
    if min_x >= max_x || min_y >= max_y {
        (0, 0, 1920, 1080)
    } else {
        (min_x, min_y, max_x, max_y)
    }
}

/// Atlas-tab plane preview: monitor rectangles, collection capture images,
/// grids, and derived neighbor arrows.
pub(crate) fn paint_zoomable_atlas_preview(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    catalog: &sqyre_persist::ProgramCatalog,
    program: &str,
    members: &[String],
    view: &mut ImageViewTransform,
) {
    use sqyre_domain::{AtlasLayout, AtlasNode, CoordinateRef, Macro, NavDir};

    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Atlas plane").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(view.needs_reset_button(), egui::Button::new("Reset view"))
                .on_hover_text("Fit plane in viewport")
                .clicked()
            {
                view.reset();
            }
            if view.shows_zoom_label() {
                ui.weak(format!("{:.0}%", view.zoom * 100.0));
            }
        });
    });
    ui.weak("Monitors behind Collections; neighbors from search-area positions.");

    let empty_macro = Macro::new("", 0, vec![]);
    let mut nodes = Vec::new();
    let mut unresolved = Vec::new();
    for name in members {
        let Ok(col) = catalog.lookup_collection(&if program.is_empty() {
            CoordinateRef(name.clone())
        } else {
            CoordinateRef(format!("{program}~{name}"))
        }) else {
            unresolved.push(name.clone());
            continue;
        };
        if col.rows < 1 || col.cols < 1 {
            unresolved.push(name.clone());
            continue;
        }
        let cell = CoordinateRef::collection(program, name, 1, 1, col.rows, col.cols);
        match catalog.resolve_search_area(&cell, &empty_macro) {
            Ok(bounds) => nodes.push(AtlasNode {
                collection: name.clone(),
                bounds,
                rows: col.rows,
                cols: col.cols,
            }),
            Err(_) => unresolved.push(name.clone()),
        }
    }

    if !unresolved.is_empty() {
        ui.colored_label(
            egui::Color32::from_rgb(220, 160, 60),
            format!("Unresolved: {}", unresolved.join(", ")),
        );
    }
    if nodes.is_empty() {
        ui.weak("No resolvable Collections to preview.");
        return;
    }

    let monitors = atlas_monitor_rects(catalog);
    let member_bounds: Vec<_> = nodes.iter().map(|n| n.bounds).collect();
    let (min_x, min_y, max_x, max_y) = atlas_plane_desktop_bounds(&monitors, &member_bounds);
    let layout = AtlasLayout::new(nodes);
    let plane_w = (max_x - min_x).max(1) as f32;
    let plane_h = (max_y - min_y).max(1) as f32;
    let image_size = egui::vec2(plane_w, plane_h);

    // Prefetch collection capture textures so handles stay alive for painting.
    let textures: Vec<Option<egui::TextureHandle>> = layout
        .nodes()
        .iter()
        .map(|n| {
            let path = catalog.collection_image_path(program, &n.collection);
            icons.for_path(ui.ctx(), path.as_path())
        })
        .collect();

    let avail_w = ui.available_width();
    const MIN_H: f32 = 160.0;
    const FILL_SLACK: f32 = 1.0;
    let avail_h = ui.available_height();
    let desired_h = if avail_h < MIN_H {
        MIN_H
    } else {
        (avail_h - FILL_SLACK).max(MIN_H)
    };
    let desired = egui::vec2(avail_w.max(160.0), desired_h);
    let (viewport, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
    image_view::handle_scroll_zoom(ui, viewport, image_size, view, resp.hovered());
    let content = image_view::image_content_rect(viewport, image_size, view.zoom, view.pan);

    let to_screen = |x: i32, y: i32| -> egui::Pos2 {
        let nx = (x - min_x) as f32 / plane_w;
        let ny = (y - min_y) as f32 / plane_h;
        egui::pos2(
            content.left() + nx * content.width(),
            content.top() + ny * content.height(),
        )
    };

    let small = egui::TextStyle::Small.resolve(ui.style());
    {
        let painter = ui.painter_at(viewport);
        painter.rect_filled(viewport, 0.0, egui::Color32::from_gray(22));

        // One rectangle per monitor with identity + size label.
        let mon_fill = egui::Color32::from_gray(36);
        let mon_stroke = egui::Stroke::new(1.5, egui::Color32::from_gray(90));
        let label_color = egui::Color32::from_gray(200);
        for (i, &(mx, my, mw, mh)) in monitors.iter().enumerate() {
            let rect = egui::Rect::from_min_max(
                to_screen(mx, my),
                to_screen(mx.saturating_add(mw), my.saturating_add(mh)),
            );
            painter.rect_filled(rect, 0.0, mon_fill);
            painter.rect_stroke(rect, 0.0, mon_stroke, egui::StrokeKind::Outside);
            let label = format!("Monitor {} — {mw}×{mh}", i + 1);
            let galley = painter.layout_no_wrap(label, small.clone(), label_color);
            let chip =
                egui::Rect::from_center_size(rect.center(), galley.size() + egui::vec2(10.0, 4.0));
            painter.rect_filled(
                chip,
                3.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
            );
            painter.galley(chip.min + egui::vec2(5.0, 2.0), galley, label_color);
        }

        let fill = egui::Color32::from_rgba_unmultiplied(60, 100, 160, 60);
        let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 180, 255));
        let arrow = egui::Stroke::new(2.0, theme::PRIMARY);

        for (i, node) in layout.nodes().iter().enumerate() {
            let (lx, ty, rx, by) = node.bounds;
            let tl = to_screen(lx, ty);
            let br = to_screen(rx, by);
            let rect = egui::Rect::from_min_max(tl, br);
            if let Some(Some(tex)) = textures.get(i) {
                painter.image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                painter.rect_filled(rect, 2.0, fill);
            }
            painter.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Outside);
            paint_grid_overlay_painter(&painter, rect, node.rows, node.cols);
            // Label with a small dark chip so it stays readable over capture images.
            let label_pos = egui::pos2(rect.left() + 4.0, rect.top() + 4.0);
            let galley = painter.layout_no_wrap(
                node.collection.clone(),
                small.clone(),
                egui::Color32::WHITE,
            );
            let chip = egui::Rect::from_min_size(label_pos, galley.size() + egui::vec2(6.0, 2.0));
            painter.rect_filled(
                chip,
                2.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 160),
            );
            painter.galley(
                label_pos + egui::vec2(3.0, 1.0),
                galley,
                egui::Color32::WHITE,
            );

            for (dir, dest) in layout.neighbor_links(i) {
                let Some(dest_node) = layout.nodes().get(dest) else {
                    continue;
                };
                let (al, at, ar, ab) = node.bounds;
                let (bl, bt, br, bb) = dest_node.bounds;
                let (from, to) = match dir {
                    NavDir::Right => (to_screen(ar, (at + ab) / 2), to_screen(bl, (bt + bb) / 2)),
                    NavDir::Left => (to_screen(al, (at + ab) / 2), to_screen(br, (bt + bb) / 2)),
                    NavDir::Down => (to_screen((al + ar) / 2, ab), to_screen((bl + br) / 2, bt)),
                    NavDir::Up => (to_screen((al + ar) / 2, at), to_screen((bl + br) / 2, bb)),
                };
                painter.line_segment([from, to], arrow);
                let v = to - from;
                let len = v.length().max(1.0);
                let dir_v = v / len;
                let perp = egui::vec2(-dir_v.y, dir_v.x);
                let tip = to;
                let base = to - dir_v * 8.0;
                painter.line_segment([tip, base + perp * 4.0], arrow);
                painter.line_segment([tip, base - perp * 4.0], arrow);
            }
        }

        paint_preview_frame(&painter, viewport);
    }
    let _ = image_view::handle_pan_drag(&resp, viewport, image_size, view);
}
