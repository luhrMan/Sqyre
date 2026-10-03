//! Vector action-type icons for types with no fitting font glyph.

use eframe::egui::{
    pos2, vec2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind,
};
use sqyre_domain::{Action, ActionKind, MouseButton, PressState};
use sqyre_ui_model::looks_like_var_ref;

/// Segments per quarter circle on the mouse body outline.
const ARC_STEPS: usize = 6;

/// Edge of an action icon (vector or glyph) shown beside `font_size` text.
pub(crate) fn action_icon_side(font_size: f32) -> f32 {
    font_size * 1.6
}

/// Font for a glyph icon filling an `action_icon_side` square.
pub(crate) fn action_glyph_font(side: f32, family: FontFamily) -> FontId {
    FontId::new(side * 0.85, family)
}

/// Painted action icon; per-action variants carry the button / key they show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VectorIcon {
    MouseDrag,
    ImageSearch,
    Grid,
    /// Mouse with the pressed button highlighted.
    Mouse(MouseButton),
    /// Keycap showing `label`; filled when the key goes down.
    Key {
        label: String,
        pressed: bool,
    },
}

impl VectorIcon {
    /// Icon shared by every action of `type_key`, or `None` when it uses a font glyph
    /// (or needs the action itself, see [`Self::for_action`]).
    pub(crate) fn for_type(type_key: &str) -> Option<Self> {
        match type_key {
            "move" => Some(Self::MouseDrag),
            "imagesearch" => Some(Self::ImageSearch),
            "foreachcell" => Some(Self::Grid),
            _ => None,
        }
    }

    /// Icon for `action`, reflecting its mouse button or key.
    pub(crate) fn for_action(action: &Action) -> Option<Self> {
        match &action.kind {
            ActionKind::Click { button, .. } => Some(Self::Mouse(*button)),
            ActionKind::Key { key, state } => Some(Self::Key {
                label: key_label(key),
                pressed: !matches!(state, PressState::Up),
            }),
            _ => Self::for_type(action.type_key()),
        }
    }

    /// Paints the icon centered in `rect` with `color`.
    pub(crate) fn paint(&self, painter: &Painter, rect: Rect, color: Color32) {
        match self {
            Self::MouseDrag => paint_mouse_drag(painter, rect, color),
            Self::ImageSearch => paint_image_search(painter, rect, color),
            Self::Grid => paint_grid(painter, rect, color),
            Self::Mouse(button) => paint_mouse_button(painter, rect, color, *button),
            Self::Key { label, pressed } => paint_keycap(painter, rect, color, label, *pressed),
        }
    }
}

/// Short keycap label for a key name.
fn key_label(key: &str) -> String {
    let key = key.trim();
    if looks_like_var_ref(key) {
        return "$".into();
    }
    let lower = key.to_ascii_lowercase();
    let named = match lower.as_str() {
        "ctrl" | "control" | "rctrl" | "rcontrol" => Some("Ctrl"),
        "shift" | "rshift" => Some("Shift"),
        "alt" | "ralt" => Some("Alt"),
        "cmd" | "rcmd" | "command" | "super" | "meta" | "win" | "windows" => Some("Super"),
        "pagedown" | "page_down" => Some("PgDn"),
        "pageup" | "page_up" => Some("PgUp"),
        "backspace" => Some("Bksp"),
        "delete" | "del" => Some("Del"),
        "escape" | "esc" => Some("Esc"),
        "enter" | "return" => Some("Enter"),
        "space" | "spacebar" => Some("Spc"),
        "tab" => Some("Tab"),
        "insert" => Some("Ins"),
        "capslock" => Some("Caps"),
        "minus" => Some("-"),
        "equal" => Some("="),
        "up" => Some("↑"),
        "down" => Some("↓"),
        "left" => Some("←"),
        "right" => Some("→"),
        _ => None,
    };
    if let Some(named) = named {
        return named.into();
    }
    if let Some(n) = lower
        .strip_prefix('f')
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    {
        return format!("F{n}");
    }
    let mut chars = lower.chars();
    match chars.next() {
        None => "?".into(),
        Some(first) => first.to_uppercase().chain(chars.take(3)).collect(),
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

/// 3×3 cell grid.
fn paint_grid(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let stroke = Stroke::new((s * 0.09).max(1.0), color);
    let grid = Rect::from_center_size(rect.center(), vec2(s * 0.8, s * 0.8));
    painter.rect_stroke(grid, s * 0.08, stroke, StrokeKind::Middle);
    for t in [1.0 / 3.0, 2.0 / 3.0] {
        let x = grid.left() + grid.width() * t;
        let y = grid.top() + grid.height() * t;
        painter.line_segment([pos2(x, grid.top()), pos2(x, grid.bottom())], stroke);
        painter.line_segment([pos2(grid.left(), y), pos2(grid.right(), y)], stroke);
    }
}

/// Points on the circle of `radius` around `center` from angle `from` to `to` (screen space).
fn arc(center: Pos2, radius: f32, from: f32, to: f32) -> impl Iterator<Item = Pos2> {
    (0..=ARC_STEPS).map(move |i| {
        let a = from + (to - from) * i as f32 / ARC_STEPS as f32;
        center + vec2(a.cos(), a.sin()) * radius
    })
}

/// Upright mouse with `button` filled: a top quarter for Left / Right, the wheel for
/// Middle, the lower body for Scroll (matching the mouse button picker).
fn paint_mouse_button(painter: &Painter, rect: Rect, color: Color32, button: MouseButton) {
    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    let s = rect.width().min(rect.height());
    let c = rect.center();
    let stroke = Stroke::new((s * 0.09).max(1.0), color);
    let body = Rect::from_center_size(c, vec2(s * 0.6, s * 0.92));
    let r = body.width() * 0.5;
    let top_c = pos2(c.x, body.top() + r);
    let bottom_c = pos2(c.x, body.bottom() - r);
    let split = body.top() + body.height() * 0.42;

    let fill: Option<Vec<Pos2>> = match button {
        MouseButton::Left => Some(
            arc(top_c, r, PI, PI + FRAC_PI_2)
                .chain([pos2(c.x, split), pos2(body.left(), split)])
                .collect(),
        ),
        MouseButton::Right => Some(
            arc(top_c, r, PI + FRAC_PI_2, TAU)
                .chain([pos2(body.right(), split), pos2(c.x, split)])
                .collect(),
        ),
        MouseButton::Scroll => Some(
            [pos2(body.left(), split), pos2(body.right(), split)]
                .into_iter()
                .chain(arc(bottom_c, r, 0.0, PI))
                .collect(),
        ),
        MouseButton::Middle => None,
    };
    if let Some(points) = fill {
        painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    }

    let outline = arc(top_c, r, PI, TAU)
        .chain(arc(bottom_c, r, 0.0, PI))
        .collect();
    painter.add(Shape::closed_line(outline, stroke));
    painter.line_segment(
        [pos2(body.left(), split), pos2(body.right(), split)],
        stroke,
    );
    painter.line_segment([pos2(c.x, body.top()), pos2(c.x, split)], stroke);
    if button == MouseButton::Middle {
        let wheel = Rect::from_center_size(
            pos2(c.x, (body.top() + split) * 0.5 + s * 0.02),
            vec2(s * 0.18, s * 0.24),
        );
        painter.rect_filled(wheel, wheel.width() * 0.5, color);
    }
}

/// Keycap with `label`; filled with knocked-out text when `pressed`, else outlined.
fn paint_keycap(painter: &Painter, rect: Rect, color: Color32, label: &str, pressed: bool) {
    let s = rect.width().min(rect.height());
    let cap = Rect::from_center_size(rect.center(), vec2(s * 0.96, s * 0.84));
    let rounding = s * 0.18;
    let text_color = if pressed {
        painter.rect_filled(cap, rounding, color);
        crate::theme::contrast_fg(color)
    } else {
        let stroke = Stroke::new((s * 0.09).max(1.0), color);
        painter.rect_stroke(cap, rounding, stroke, StrokeKind::Inside);
        color
    };

    let max_w = cap.width() - s * 0.2;
    let layout = |size: f32| {
        painter.layout_no_wrap(label.to_owned(), FontId::proportional(size), text_color)
    };
    let size = s * 0.6;
    let mut galley = layout(size);
    if galley.size().x > max_w {
        galley = layout(size * max_w / galley.size().x);
    }
    // Optical center: baseline metrics make the layout box look top-heavy.
    let pos = if galley.mesh_bounds.is_positive() {
        cap.center() - galley.mesh_bounds.center().to_vec2()
    } else {
        cap.center() - galley.size() * 0.5
    };
    painter.galley(pos, galley, text_color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{blank_action, ActionId};

    fn key(key: &str, state: PressState) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: key.into(),
                state,
            },
        }
    }

    #[test]
    fn type_icons_cover_move_image_search_and_grid() {
        assert_eq!(VectorIcon::for_type("move"), Some(VectorIcon::MouseDrag));
        assert_eq!(
            VectorIcon::for_type("imagesearch"),
            Some(VectorIcon::ImageSearch)
        );
        assert_eq!(VectorIcon::for_type("foreachcell"), Some(VectorIcon::Grid));
        assert_eq!(VectorIcon::for_type("click"), None);
        assert_eq!(VectorIcon::for_type("wait"), None);
    }

    #[test]
    fn click_icon_shows_its_button() {
        for button in MouseButton::ALL {
            let action = Action {
                id: ActionId::new(),
                kind: ActionKind::Click {
                    button: *button,
                    state: PressState::Tap,
                },
            };
            assert_eq!(
                VectorIcon::for_action(&action),
                Some(VectorIcon::Mouse(*button))
            );
        }
    }

    #[test]
    fn key_icon_fills_unless_released() {
        let icon = |state| VectorIcon::for_action(&key("a", state));
        let cap = |pressed| {
            Some(VectorIcon::Key {
                label: "A".into(),
                pressed,
            })
        };
        assert_eq!(icon(PressState::Down), cap(true));
        assert_eq!(icon(PressState::Tap), cap(true));
        assert_eq!(icon(PressState::Up), cap(false));
    }

    #[test]
    fn action_icon_falls_back_to_type_icon() {
        let cell = blank_action("foreachcell").expect("foreachcell blank");
        assert_eq!(VectorIcon::for_action(&cell), Some(VectorIcon::Grid));
        let wait = blank_action("wait").expect("wait blank");
        assert_eq!(VectorIcon::for_action(&wait), None);
    }

    #[test]
    fn key_labels_are_short() {
        for (key, want) in [
            ("a", "A"),
            (" Control ", "Ctrl"),
            ("shift", "Shift"),
            ("PageDown", "PgDn"),
            ("backspace", "Bksp"),
            ("escape", "Esc"),
            ("return", "Enter"),
            ("space", "Spc"),
            ("up", "↑"),
            ("right", "→"),
            ("F12", "F12"),
            ("${hotkey}", "$"),
            ("home", "Home"),
            ("numpad5", "Nump"),
            ("", "?"),
        ] {
            assert_eq!(key_label(key), want, "{key:?}");
        }
    }
}
