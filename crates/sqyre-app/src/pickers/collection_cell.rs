use super::types::CollectionCellPick;
use crate::data_editor_preview::paint_grid_overlay_painter;
use crate::icon_cache::IconCache;
use crate::image_view::{self};
use eframe::egui::{self, Color32, Pos2, Sense, Vec2};
use sqyre_persist::ProgramCatalog;

fn fit_panel(w: f32, h: f32) -> Vec2 {
    crate::data_editor_preview::fit_panel(w, h)
}

/// Interactive collection image + rows×cols overlay; click/drag selects cells.
/// Wheel zooms at cursor; when zoomed, drag pans.
pub fn paint_collection_cell_picker(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    pick: &mut CollectionCellPick,
) {
    ui.horizontal(|ui| {
        crate::icon_cache::paint_program_icon(ui, catalog, icons, &pick.program);
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
            if pick.view.shows_zoom_label() {
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

    let content =
        image_view::image_content_rect(viewport, image_size, pick.view.zoom, pick.view.pan);
    let body_font = egui::TextStyle::Body.resolve(ui.style());

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
                body_font,
                Color32::LIGHT_GRAY,
            );
        }
        paint_grid_overlay_painter(&painter, content, pick.rows, pick.cols);
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
        egui::pos2(rect.left() + c2 as f32 * cw, rect.top() + r2 as f32 * ch),
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
