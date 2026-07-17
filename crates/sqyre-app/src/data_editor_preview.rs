use crate::icon_cache::IconCache;
use crate::image_view::{self, ImageViewTransform};
use crate::var_pills;
use eframe::egui;
use sqyre_domain::PROGRAM_DELIMITER;
use sqyre_validate::EntryValidation;
use std::collections::HashSet;

pub(crate) fn paint_preview_toolbar(ui: &mut egui::Ui) -> bool {
    ui.add_space(8.0);
    ui.separator();
    let mut force = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Preview").strong());
        if ui
            .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
            .on_hover_text("Refresh")
            .clicked()
        {
            force = true;
        }
    });
    force
}

#[derive(Clone, Copy)]
pub(crate) enum CardinalEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Borderless coord chip overlaid on a preview edge.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_preview_coord_chip(
    ui: &mut egui::Ui,
    preview: egui::Rect,
    edge: CardinalEdge,
    placeholder: &str,
    value: &mut String,
    known: &HashSet<String>,
    is_dark: bool,
    validation: &EntryValidation,
) {
    const CHIP_W: f32 = 76.0;
    const CHIP_H: f32 = 24.0;
    const PAD: f32 = 6.0;
    let size = egui::vec2(CHIP_W, CHIP_H);
    let center = match edge {
        CardinalEdge::Top => egui::pos2(preview.center().x, preview.top() + PAD + CHIP_H * 0.5),
        CardinalEdge::Bottom => {
            egui::pos2(preview.center().x, preview.bottom() - PAD - CHIP_H * 0.5)
        }
        CardinalEdge::Left => egui::pos2(preview.left() + PAD + CHIP_W * 0.5, preview.center().y),
        CardinalEdge::Right => egui::pos2(preview.right() - PAD - CHIP_W * 0.5, preview.center().y),
    };
    let chip = egui::Rect::from_center_size(center, size);
    ui.painter().rect_filled(
        chip,
        4.0,
        egui::Color32::from_rgba_unmultiplied(16, 16, 16, 170),
    );
    if let Some(stroke) = var_pills::entry_validation_stroke(validation) {
        ui.painter()
            .rect_stroke(chip, 4.0, stroke, egui::StrokeKind::Outside);
    }
    let edit_rect = chip.shrink(3.0);
    let id = ui.id().with(("preview_coord", placeholder));
    let focused = ui.memory(|m| m.has_focus(id));
    let show_overlay = !focused && !value.is_empty() && sqyre_varref::contains(value.as_str());
    let resp = if show_overlay {
        let plain_fg = egui::Color32::from_gray(230);
        ui.scope_builder(egui::UiBuilder::new().max_rect(edit_rect), |ui| {
            ui.set_min_size(edit_rect.size());
            ui.centered_and_justified(|ui| {
                var_pills::paint_var_ref_content(ui, value, known, is_dark, plain_fg);
            });
        })
        .response
        .interact(egui::Sense::click())
    } else {
        ui.put(
            edit_rect,
            egui::TextEdit::singleline(value)
                .id(id)
                .frame(egui::Frame::NONE)
                .hint_text(placeholder)
                .desired_width(edit_rect.width()),
        )
    };
    if show_overlay && resp.clicked() {
        ui.memory_mut(|m| m.request_focus(id));
    }
    if let Some(tip) = var_pills::entry_validation_tip(validation) {
        resp.on_hover_text(tip);
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
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (MAX / w).min(MAX / h).min(1.0);
    egui::vec2(w * scale, h * scale)
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
            if ui
                .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
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
pub(crate) fn paint_zoomable_collection_preview(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    path: &std::path::Path,
    rows: i32,
    cols: i32,
    view: &mut ImageViewTransform,
    replace_clicked: &mut bool,
) {
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Collection image").strong());
        if ui
            .add(egui::Button::new(egui::RichText::new("↻").size(14.0)).small())
            .on_hover_text("Refresh")
            .clicked()
        {
            icons.invalidate_path(path);
        }
        if ui.button("Replace Image").clicked() {
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
            if view.zoom != 1.0 {
                ui.weak(format!("{:.0}%", view.zoom * 100.0));
            }
        });
    });
    ui.weak("Scroll to zoom; drag to pan when zoomed.");

    let tex = icons.for_path(ui.ctx(), path);
    let avail = ui.available_width().min(520.0);
    let image_size = match &tex {
        Some(t) => {
            let [tw, th] = t.size();
            egui::vec2(tw as f32, th as f32)
        }
        None => egui::vec2(avail, avail * 0.75),
    };
    let fit = fit_panel(image_size.x, image_size.y);
    let scale = (avail / fit.x).min(1.0);
    let desired = egui::vec2((fit.x * scale).max(160.0), (fit.y * scale).max(120.0));
    let (viewport, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

    image_view::handle_scroll_zoom(ui, viewport, image_size, view, resp.hovered());
    let content = image_view::image_content_rect(viewport, image_size, view.zoom, view.pan);

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
                egui::FontId::proportional(14.0),
                egui::Color32::LIGHT_GRAY,
            );
        }
        paint_grid_overlay_painter(&painter, content, rows, cols);
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
