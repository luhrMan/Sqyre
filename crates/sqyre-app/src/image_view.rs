//! Fit-relative zoom/pan for image previews.

use eframe::egui::{self, Pos2, Vec2};

const IMAGE_ZOOM_MIN: f32 = 0.5;
const IMAGE_ZOOM_MAX: f32 = 16.0;
/// Relative zoom change per mouse-wheel notch (~50px of `smooth_scroll_delta`).
const IMAGE_ZOOM_WHEEL_STEP: f32 = 0.035;
/// Typical egui wheel notch size in points of `smooth_scroll_delta.y`.
const IMAGE_ZOOM_SCROLL_NOTCH: f32 = 50.0;
const IMAGE_PAN_EDGE_PAD: f32 = 32.0;

/// Zoom 1.0 = fit in viewport; values above enlarge. Pan is in viewport pixels.
#[derive(Debug, Clone)]
pub struct ImageViewTransform {
    pub zoom: f32,
    pub pan: Vec2,
    /// Pointer start + pan at drag start while panning.
    pub pan_drag: Option<(Pos2, Vec2)>,
}

impl Default for ImageViewTransform {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            pan_drag: None,
        }
    }
}

impl ImageViewTransform {
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.pan_drag = None;
    }

    pub fn is_zoomed(&self) -> bool {
        self.zoom > 1.01
    }

    pub fn needs_reset_button(&self) -> bool {
        self.zoom != 1.0 || self.pan != Vec2::ZERO
    }
}

pub fn clamp_image_zoom(z: f32) -> f32 {
    z.clamp(IMAGE_ZOOM_MIN, IMAGE_ZOOM_MAX)
}

pub fn scroll_zoom_factor(delta_y: f32) -> f32 {
    if delta_y == 0.0 {
        return 1.0;
    }
    // Proportional to delta so trackpads zoom smoothly; one notch ≈ WHEEL_STEP.
    (1.0 + IMAGE_ZOOM_WHEEL_STEP).powf(delta_y / IMAGE_ZOOM_SCROLL_NOTCH)
}

/// Displayed image rect inside the viewport.
pub fn image_content_rect(
    viewport: egui::Rect,
    image_size: Vec2,
    zoom: f32,
    pan: Vec2,
) -> egui::Rect {
    if viewport.width() <= 0.0 || viewport.height() <= 0.0 {
        return egui::Rect::NOTHING;
    }
    if image_size.x <= 0.0 || image_size.y <= 0.0 {
        return viewport;
    }
    let fit = (viewport.width() / image_size.x).min(viewport.height() / image_size.y);
    let scale = fit * zoom;
    let w = image_size.x * scale;
    let h = image_size.y * scale;
    let x = viewport.left() + (viewport.width() - w) * 0.5 + pan.x;
    let y = viewport.top() + (viewport.height() - h) * 0.5 + pan.y;
    egui::Rect::from_min_size(egui::pos2(x, y), Vec2::new(w, h))
}

pub fn clamp_image_pan(viewport: egui::Rect, image_size: Vec2, zoom: f32, mut pan: Vec2) -> Vec2 {
    let content = image_content_rect(viewport, image_size, zoom, pan);
    let pad = IMAGE_PAN_EDGE_PAD;
    if content.width() <= viewport.width() {
        pan.x = 0.0;
    } else {
        let min_x = viewport.right() - content.width() - pad;
        let max_x = viewport.left() + pad;
        if content.left() < min_x {
            pan.x += min_x - content.left();
        }
        if content.left() > max_x {
            pan.x += max_x - content.left();
        }
    }
    if content.height() <= viewport.height() {
        pan.y = 0.0;
    } else {
        let min_y = viewport.bottom() - content.height() - pad;
        let max_y = viewport.top() + pad;
        if content.top() < min_y {
            pan.y += min_y - content.top();
        }
        if content.top() > max_y {
            pan.y += max_y - content.top();
        }
    }
    pan
}

pub fn zoom_image_at_cursor(
    viewport: egui::Rect,
    image_size: Vec2,
    zoom: f32,
    pan: Vec2,
    cursor: Pos2,
    factor: f32,
) -> (f32, Vec2) {
    if factor <= 0.0 || image_size.x <= 0.0 || image_size.y <= 0.0 {
        return (zoom, pan);
    }
    let content = image_content_rect(viewport, image_size, zoom, pan);
    if content.width() <= 0.0 || content.height() <= 0.0 {
        return (zoom, pan);
    }
    let u = (cursor.x - content.left()) / content.width();
    let v = (cursor.y - content.top()) / content.height();
    let new_zoom = clamp_image_zoom(zoom * factor);
    if (new_zoom - zoom).abs() < f32::EPSILON {
        return (zoom, pan);
    }
    let mut pan = pan;
    let after = image_content_rect(viewport, image_size, new_zoom, pan);
    pan.x += (cursor.x - u * after.width()) - after.left();
    pan.y += (cursor.y - v * after.height()) - after.top();
    let pan = clamp_image_pan(viewport, image_size, new_zoom, pan);
    (new_zoom, pan)
}

/// Apply wheel zoom while the pointer hovers `viewport`; updates `view` in place.
pub fn handle_scroll_zoom(
    ui: &egui::Ui,
    viewport: egui::Rect,
    image_size: Vec2,
    view: &mut ImageViewTransform,
    hovered: bool,
) {
    if !hovered {
        return;
    }
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll == 0.0 {
        return;
    }
    let cursor = ui
        .input(|i| i.pointer.hover_pos())
        .unwrap_or(viewport.center());
    let factor = scroll_zoom_factor(scroll);
    let (z, p) = zoom_image_at_cursor(viewport, image_size, view.zoom, view.pan, cursor, factor);
    view.zoom = z;
    view.pan = p;
}

/// Primary-drag pans when zoomed. Returns true if this interaction consumed the drag.
pub fn handle_pan_drag(
    resp: &egui::Response,
    viewport: egui::Rect,
    image_size: Vec2,
    view: &mut ImageViewTransform,
) -> bool {
    if !view.is_zoomed() {
        view.pan_drag = None;
        return false;
    }
    if resp.drag_started() {
        if let Some(pos) = resp.interact_pointer_pos() {
            view.pan_drag = Some((pos, view.pan));
        }
    }
    if resp.dragged() {
        if let (Some((start, base)), Some(pos)) = (view.pan_drag, resp.interact_pointer_pos()) {
            view.pan = clamp_image_pan(viewport, image_size, view.zoom, base + (pos - start));
        }
    }
    if resp.drag_stopped() {
        view.pan_drag = None;
    }
    if resp.hovered() {
        resp.ctx.set_cursor_icon(egui::CursorIcon::Grab);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_clamps_and_scroll_factor_is_symmetric() {
        assert_eq!(clamp_image_zoom(0.1), IMAGE_ZOOM_MIN);
        assert_eq!(clamp_image_zoom(100.0), IMAGE_ZOOM_MAX);
        let up = scroll_zoom_factor(IMAGE_ZOOM_SCROLL_NOTCH);
        let down = scroll_zoom_factor(-IMAGE_ZOOM_SCROLL_NOTCH);
        assert!((up - (1.0 + IMAGE_ZOOM_WHEEL_STEP)).abs() < 1e-5);
        assert!((up * down - 1.0).abs() < 1e-5);
        let half = scroll_zoom_factor(IMAGE_ZOOM_SCROLL_NOTCH * 0.5);
        assert!(half > 1.0 && half < up);
    }

    #[test]
    fn content_rect_centers_at_zoom_one() {
        let vp = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(200.0, 100.0));
        let img = Vec2::new(100.0, 100.0);
        let r = image_content_rect(vp, img, 1.0, Vec2::ZERO);
        assert!((r.height() - 100.0).abs() < 0.01);
        assert!((r.center().x - 100.0).abs() < 0.01);
    }
}
