//! Client-side move/resize when the main window has OS decorations off.

use eframe::egui::{self, CursorIcon, ResizeDirection, ViewportCommand};

/// Thickness of the invisible edge/corner resize grips (logical points).
pub const RESIZE_EDGE: f32 = 6.0;

/// Which edge/corner grip contains `pos`, if any.
pub fn resize_direction_at(
    rect: egui::Rect,
    pos: egui::Pos2,
    edge: f32,
) -> Option<ResizeDirection> {
    if !rect.is_positive() || edge <= 0.0 || !rect.expand(edge).contains(pos) {
        return None;
    }
    let on_left = pos.x - rect.left() <= edge;
    let on_right = rect.right() - pos.x <= edge;
    let on_top = pos.y - rect.top() <= edge;
    let on_bottom = rect.bottom() - pos.y <= edge;
    match (on_left, on_right, on_top, on_bottom) {
        (true, false, true, false) => Some(ResizeDirection::NorthWest),
        (false, true, true, false) => Some(ResizeDirection::NorthEast),
        (true, false, false, true) => Some(ResizeDirection::SouthWest),
        (false, true, false, true) => Some(ResizeDirection::SouthEast),
        (true, false, false, false) => Some(ResizeDirection::West),
        (false, true, false, false) => Some(ResizeDirection::East),
        (false, false, true, false) => Some(ResizeDirection::North),
        (false, false, false, true) => Some(ResizeDirection::South),
        _ => None,
    }
}

fn cursor_for(dir: ResizeDirection) -> CursorIcon {
    match dir {
        ResizeDirection::North => CursorIcon::ResizeNorth,
        ResizeDirection::South => CursorIcon::ResizeSouth,
        ResizeDirection::East => CursorIcon::ResizeEast,
        ResizeDirection::West => CursorIcon::ResizeWest,
        ResizeDirection::NorthEast => CursorIcon::ResizeNorthEast,
        ResizeDirection::SouthWest => CursorIcon::ResizeSouthWest,
        ResizeDirection::NorthWest => CursorIcon::ResizeNorthWest,
        ResizeDirection::SouthEast => CursorIcon::ResizeSouthEast,
    }
}

/// When OS decorations are off, grab window edges/corners to resize.
///
/// Skipped while maximized — there is nothing to resize. Call after UI so the
/// cursor icon wins over widget hover cursors on the thin edge strip.
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_borderless_resize(ctx: &egui::Context) {
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if maximized {
        return;
    }
    let rect = ctx.content_rect();
    let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) else {
        return;
    };
    let Some(dir) = resize_direction_at(rect, pos, RESIZE_EDGE) else {
        return;
    };
    ctx.set_cursor_icon(cursor_for(dir));
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    }
}

/// Top-bar drag-to-move and double-click maximize for an undecorated window.
///
/// Interact the bar rect *before* children so later widgets keep click priority.
#[cfg(not(target_arch = "wasm32"))]
pub fn top_bar_window_drag(ui: &mut egui::Ui, bar_rect: egui::Rect) {
    let resp = ui.interact(
        bar_rect,
        ui.id().with("undecorated_window_drag"),
        egui::Sense::click_and_drag(),
    );
    if resp.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if resp.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

/// Minimize / maximize / close when the OS title bar is hidden.
///
/// Call inside a right-to-left layout so Close lands rightmost (OS order).
#[cfg(not(target_arch = "wasm32"))]
pub fn window_control_buttons(ui: &mut egui::Ui) {
    if crate::widgets::icon_button(ui, egui_phosphor::regular::X, "Close").clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let (glyph, tip) = if maximized {
        (egui_phosphor::regular::ARROWS_IN_SIMPLE, "Restore")
    } else {
        (egui_phosphor::regular::ARROWS_OUT_SIMPLE, "Maximize")
    };
    if crate::widgets::icon_button(ui, glyph, tip).clicked() {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    if crate::widgets::icon_button(ui, egui_phosphor::regular::MINUS, "Minimize").clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, Rect, Vec2};

    fn rect() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(200.0, 100.0))
    }

    #[test]
    fn corners_win_over_edges() {
        let r = rect();
        assert_eq!(
            resize_direction_at(r, pos2(2.0, 2.0), RESIZE_EDGE),
            Some(ResizeDirection::NorthWest)
        );
        assert_eq!(
            resize_direction_at(r, pos2(198.0, 2.0), RESIZE_EDGE),
            Some(ResizeDirection::NorthEast)
        );
        assert_eq!(
            resize_direction_at(r, pos2(2.0, 98.0), RESIZE_EDGE),
            Some(ResizeDirection::SouthWest)
        );
        assert_eq!(
            resize_direction_at(r, pos2(198.0, 98.0), RESIZE_EDGE),
            Some(ResizeDirection::SouthEast)
        );
    }

    #[test]
    fn mid_edges_resolve() {
        let r = rect();
        assert_eq!(
            resize_direction_at(r, pos2(2.0, 50.0), RESIZE_EDGE),
            Some(ResizeDirection::West)
        );
        assert_eq!(
            resize_direction_at(r, pos2(198.0, 50.0), RESIZE_EDGE),
            Some(ResizeDirection::East)
        );
        assert_eq!(
            resize_direction_at(r, pos2(100.0, 2.0), RESIZE_EDGE),
            Some(ResizeDirection::North)
        );
        assert_eq!(
            resize_direction_at(r, pos2(100.0, 98.0), RESIZE_EDGE),
            Some(ResizeDirection::South)
        );
    }

    #[test]
    fn interior_is_not_a_grip() {
        let r = rect();
        assert_eq!(resize_direction_at(r, pos2(100.0, 50.0), RESIZE_EDGE), None);
    }
}
