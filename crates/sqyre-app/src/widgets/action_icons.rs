//! Vector action-type icons for types with no fitting font glyph.

use eframe::egui::{pos2, vec2, Color32, Painter, Rect, Stroke, StrokeKind};

/// Paints an icon centered in the given rect with the given color.
pub(crate) type VectorIcon = fn(&Painter, Rect, Color32);

/// Vector icon for `type_key`, or `None` when the type uses its font glyph.
pub(crate) fn vector_action_icon(type_key: &str) -> Option<VectorIcon> {
    match type_key {
        "move" => Some(paint_mouse_drag),
        _ => None,
    }
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
    fn only_move_has_a_vector_icon() {
        assert!(vector_action_icon("move").is_some());
        assert!(vector_action_icon("click").is_none());
    }
}
