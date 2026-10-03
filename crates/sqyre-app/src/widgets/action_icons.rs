//! Vector action-type icons for types with no fitting font glyph.

use eframe::egui::{pos2, vec2, Color32, Painter, Rect, Shape, Stroke, StrokeKind};

/// Paints an icon centered in the given rect with the given color.
pub(crate) type VectorIcon = fn(&Painter, Rect, Color32);

/// Vector icon for `type_key`, or `None` when the type uses its font glyph.
pub(crate) fn vector_action_icon(type_key: &str) -> Option<VectorIcon> {
    match type_key {
        "move" => Some(paint_mouse_drag),
        "imagesearch" => Some(paint_image_search),
        _ => None,
    }
}

/// Picture frame (mountain + sun) with a magnifying glass over its bottom-right corner.
fn paint_image_search(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let p = |x: f32, y: f32| pos2(c.x + s * x, c.y + s * y);
    let stroke = Stroke::new((s * 0.09).max(1.0), color);

    let (left, top, right, bottom) = (-0.42, -0.38, 0.22, 0.18);
    let lens_r = 0.17;
    // Frame stops at the lens edge so its corner does not show through the glass.
    painter.add(Shape::line(
        vec![
            p(right, bottom - lens_r),
            p(right, top),
            p(left, top),
            p(left, bottom),
            p(right - lens_r, bottom),
        ],
        stroke,
    ));
    painter.add(Shape::line(
        vec![p(-0.36, 0.12), p(-0.2, -0.1), p(-0.06, 0.06)],
        stroke,
    ));
    painter.circle_filled(p(0.06, -0.22), s * 0.06, color);

    let lens = p(right, bottom);
    painter.circle_stroke(lens, s * lens_r, stroke);
    let d = std::f32::consts::FRAC_1_SQRT_2;
    painter.line_segment(
        [
            lens + vec2(d, d) * s * lens_r,
            lens + vec2(d, d) * s * (lens_r + 0.17),
        ],
        stroke,
    );
}

/// Mouse outline with motion lines trailing to the left.
fn paint_mouse_drag(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let stroke = Stroke::new((s * 0.09).max(1.0), color);
    let body = Rect::from_center_size(pos2(c.x + s * 0.18, c.y), vec2(s * 0.42, s * 0.64));
    painter.rect_stroke(body, body.width() * 0.5, stroke, StrokeKind::Middle);
    painter.line_segment(
        [
            pos2(body.center().x, body.top()),
            pos2(body.center().x, body.top() + body.height() * 0.32),
        ],
        stroke,
    );
    let tail_end = body.left() - s * 0.08;
    for (dy, len) in [(-0.16, 0.2), (0.0, 0.32), (0.16, 0.2)] {
        let y = c.y + s * dy;
        painter.line_segment([pos2(tail_end - s * len, y), pos2(tail_end, y)], stroke);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_icons_cover_move_and_image_search() {
        assert!(vector_action_icon("move").is_some());
        assert!(vector_action_icon("imagesearch").is_some());
        assert!(vector_action_icon("click").is_none());
    }
}
