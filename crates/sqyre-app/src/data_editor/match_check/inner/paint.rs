//! MatchCheck overlay paint helpers (boxes, heatmap texture).

use super::heatmap::{closeness_percent, heatmap_hover_allowed, hover_at_image};
use super::{
    image_to_content, no_match_box_caption, paint_mouse_follow_tooltip, score_passes,
    MatchCheckCache, MatchCheckHover, MatchSummary, ToleranceMatch,
};
use crate::image_view::{self, ImageViewTransform};
use crate::theme::{match_fail_fg, match_pass_fg, match_within_fg};
use eframe::egui::{self, Color32};
use sqyre_domain::MatchMethod;

pub(super) fn paint_match_box_caption(
    painter: &egui::Painter,
    content: egui::Rect,
    box_rect: egui::Rect,
    text: &str,
    color: Color32,
) {
    let font = egui::FontId::proportional(12.0);
    let galley = painter.layout_no_wrap(text.to_owned(), font, color);
    let pad = egui::vec2(5.0, 2.0);
    let size = galley.size() + pad * 2.0;
    // Prefer above the box; fall inside the top edge if that would clip.
    let mut origin = egui::pos2(
        box_rect.center().x - size.x * 0.5,
        box_rect.min.y - size.y - 4.0,
    );
    if origin.y < content.min.y {
        origin.y = box_rect.min.y + 2.0;
    }
    origin.x = origin
        .x
        .clamp(content.min.x, (content.max.x - size.x).max(content.min.x));
    let badge = egui::Rect::from_min_size(origin, size);
    painter.rect_filled(badge, 3.0, crate::theme::preview_scrim());
    painter.galley(badge.min + pad, galley, color);
}

pub(crate) fn paint_tolerance_match_boxes(
    painter: &egui::Painter,
    content: egui::Rect,
    image_size: egui::Vec2,
    summary: &MatchSummary,
    tolerance_matches: &[ToleranceMatch],
    tmpl_w: usize,
    tmpl_h: usize,
    method: MatchMethod,
    tolerance: f32,
    show_match_boxes: bool,
) {
    if !show_match_boxes {
        return;
    }
    if tmpl_w == 0 || tmpl_h == 0 || !summary.best_score.is_finite() {
        return;
    }
    let scale_x = content.width() / image_size.x.max(1.0);
    let scale_y = content.height() / image_size.y.max(1.0);
    let box_size = egui::vec2(tmpl_w as f32 * scale_x, tmpl_h as f32 * scale_y);
    if tolerance_matches.is_empty() {
        if score_passes(summary.best_score, method, tolerance) {
            return;
        }
        let top_left = image_to_content(
            egui::pos2(summary.best_x as f32, summary.best_y as f32),
            content,
            image_size,
        );
        let rect = egui::Rect::from_min_size(top_left, box_size);
        let color = match_fail_fg();
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, color),
            egui::StrokeKind::Outside,
        );
        let pct = closeness_percent(summary.best_score, method, summary);
        paint_match_box_caption(painter, content, rect, &no_match_box_caption(pct), color);
        return;
    }
    for m in tolerance_matches {
        if !score_passes(m.score, method, tolerance) {
            continue;
        }
        let is_best = m.x == summary.best_x && m.y == summary.best_y;
        let top_left = image_to_content(egui::pos2(m.x as f32, m.y as f32), content, image_size);
        let rect = egui::Rect::from_min_size(top_left, box_size);
        let color = if is_best {
            match_pass_fg()
        } else {
            match_within_fg()
        };
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(if is_best { 2.0 } else { 1.5 }, color),
            egui::StrokeKind::Outside,
        );
        if is_best {
            let pct = closeness_percent(m.score, method, summary);
            paint_match_box_caption(painter, content, rect, &format!("{pct:.1}%"), color);
        }
    }
}

pub(crate) fn paint_heatmap_overlay(
    ui: &mut egui::Ui,
    viewport: egui::Rect,
    image_size: egui::Vec2,
    view: &ImageViewTransform,
    cache: &MatchCheckCache,
    hover: &mut Option<MatchCheckHover>,
    method: MatchMethod,
    tolerance: f32,
    show_match_boxes: bool,
) {
    let content = image_view::image_content_rect(viewport, image_size, view.zoom, view.pan);
    let painter = ui.painter_at(viewport);
    painter.image(
        cache.heatmap.id(),
        content,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    paint_tolerance_match_boxes(
        &painter,
        content,
        image_size,
        &cache.data.summary,
        &cache.data.tolerance_matches,
        cache.data.tmpl_w,
        cache.data.tmpl_h,
        method,
        tolerance,
        show_match_boxes,
    );

    *hover = None;
    // Read the pointer without `ui.interact` — a second interactable over the
    // panel's click_and_drag rect steals the drag and interrupts pan/scroll.
    let pointer_down = ui.input(|i| i.pointer.any_down());
    let hover_pos = heatmap_hover_allowed(pointer_down, view.pan_drag.is_some())
        .then(|| ui.input(|i| i.pointer.hover_pos()))
        .flatten()
        .filter(|p| content.contains(*p));
    if let Some(pos) = hover_pos {
        let scale_x = image_size.x / content.width().max(1.0);
        let scale_y = image_size.y / content.height().max(1.0);
        let ix = ((pos.x - content.min.x) * scale_x).floor() as i32;
        let iy = ((pos.y - content.min.y) * scale_y).floor() as i32;
        *hover = hover_at_image(
            &cache.data.match_map,
            &cache.data.summary,
            ix,
            iy,
            cache.data.tmpl_w,
            cache.data.tmpl_h,
            method,
            tolerance,
        );
    }
    if let (Some(h), Some(pointer)) = (hover.as_ref(), hover_pos) {
        let tol_pct = if method.is_normed() {
            Some(tolerance * 100.0)
        } else {
            None
        };
        paint_mouse_follow_tooltip(ui.ctx(), pointer, viewport, h, tol_pct);
        ui.ctx().request_repaint();
    }
}
