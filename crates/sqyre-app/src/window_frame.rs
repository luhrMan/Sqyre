//! Client-side move/resize when the main window has OS decorations off.

use eframe::egui::{self, CursorIcon, ResizeDirection, ViewportCommand};
use std::sync::atomic::{AtomicBool, Ordering};

/// Thickness of the invisible edge/corner resize grips (logical points).
pub const RESIZE_EDGE: f32 = 6.0;

const BORDERLESS_RESIZE_ACTIVE_ID: &str = "sqyre_borderless_resize_active";
const BORDERLESS_RESIZE_DIR_ID: &str = "sqyre_borderless_resize_dir";

/// Process-wide flag so [`eframe::App::clear_color`] can switch to an opaque
/// clear for the duration of a client-side resize (transparent clears flicker
/// while wgpu recreates the swapchain on every configure).
static RESIZE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// True while a client-side `BeginResize` gesture is in progress.
pub fn is_borderless_resize_active() -> bool {
    RESIZE_ACTIVE.load(Ordering::Relaxed)
}

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

/// Top-bar move hit target with window-edge resize grips carved out.
///
/// Shrinks top/left/right by [`RESIZE_EDGE`] so north/corner grips never share
/// an interact rect with [`top_bar_window_drag`]'s `StartDrag`.
pub fn top_bar_drag_rect(bar_rect: egui::Rect, edge: f32) -> egui::Rect {
    if !bar_rect.is_positive() || edge <= 0.0 {
        return bar_rect;
    }
    let mut r = bar_rect;
    r.min.x += edge;
    r.max.x -= edge;
    r.min.y += edge;
    if r.max.x <= r.min.x || r.max.y <= r.min.y {
        egui::Rect::NOTHING
    } else {
        r
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

fn resize_active_id() -> egui::Id {
    egui::Id::new(BORDERLESS_RESIZE_ACTIVE_ID)
}

fn resize_dir_id() -> egui::Id {
    egui::Id::new(BORDERLESS_RESIZE_DIR_ID)
}

fn set_borderless_resize_active(ctx: &egui::Context, active: bool, dir: Option<ResizeDirection>) {
    RESIZE_ACTIVE.store(active, Ordering::Relaxed);
    ctx.data_mut(|d| {
        if active {
            d.insert_temp(resize_active_id(), true);
            d.insert_temp(resize_dir_id(), dir);
        } else {
            d.remove_temp::<bool>(resize_active_id());
            // `ResizeDirection` is not Default; clear via Option.
            d.insert_temp(resize_dir_id(), None::<ResizeDirection>);
        }
    });
}

fn borderless_resize_active_ctx(ctx: &egui::Context) -> bool {
    is_borderless_resize_active()
        || ctx.data(|d| d.get_temp::<bool>(resize_active_id()).unwrap_or(false))
}

fn active_resize_dir(ctx: &egui::Context) -> Option<ResizeDirection> {
    ctx.data(|d| {
        d.get_temp::<Option<ResizeDirection>>(resize_dir_id())
            .flatten()
    })
}

/// Start a client-side resize as soon as the edge is pressed.
///
/// Call from [`eframe::App::logic`] (before any paint) so Wayland gets
/// `xdg_toplevel.resize` with a fresh button serial. Sets the resize-active
/// flag so [`eframe::App::clear_color`] stays opaque for the gesture.
#[cfg(not(target_arch = "wasm32"))]
pub fn poll_borderless_resize_start(ctx: &egui::Context) {
    if borderless_resize_active_ctx(ctx) {
        if !ctx.input(|i| i.pointer.primary_down()) {
            set_borderless_resize_active(ctx, false, None);
        }
        return;
    }
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if maximized {
        return;
    }
    if !ctx.input(|i| i.pointer.primary_pressed()) {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) else {
        return;
    };
    let Some(dir) = resize_direction_at(ctx.content_rect(), pos, RESIZE_EDGE) else {
        return;
    };
    ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    set_borderless_resize_active(ctx, true, Some(dir));
    ctx.set_cursor_icon(cursor_for(dir));
}

/// Cursor + release cleanup while OS decorations are off.
///
/// Skipped while maximized. Call after UI so the cursor icon wins over widget
/// hover cursors on the thin edge strip. [`poll_borderless_resize_start`] owns
/// the press → `BeginResize` transition.
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_borderless_resize(ctx: &egui::Context) {
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if maximized {
        set_borderless_resize_active(ctx, false, None);
        return;
    }

    let primary_down = ctx.input(|i| i.pointer.primary_down());
    if borderless_resize_active_ctx(ctx) {
        if !primary_down {
            set_borderless_resize_active(ctx, false, None);
            return;
        }
        // Re-assert the same cursor every frame — egui resets to Default when
        // nothing sets an icon, which reads as flicker during the gesture.
        if let Some(dir) = active_resize_dir(ctx) {
            ctx.set_cursor_icon(cursor_for(dir));
        }
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
}

/// Top-bar drag-to-move and double-click maximize for an undecorated window.
///
/// Interact the bar rect *before* children so later widgets keep click priority.
/// The interact rect excludes window-edge resize grips so `StartDrag` cannot
/// fight [`poll_borderless_resize_start`]'s `BeginResize` on the top edge.
#[cfg(not(target_arch = "wasm32"))]
pub fn top_bar_window_drag(ui: &mut egui::Ui, bar_rect: egui::Rect) {
    // Never compete with an in-flight BeginResize (move+resize = window lurch).
    if borderless_resize_active_ctx(ui.ctx()) {
        return;
    }
    let drag_rect = top_bar_drag_rect(bar_rect, RESIZE_EDGE);
    if !drag_rect.is_positive() {
        return;
    }
    let resp = ui.interact(
        drag_rect,
        ui.id().with("undecorated_window_drag"),
        egui::Sense::click_and_drag(),
    );
    if resp.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        return;
    }
    if !resp.drag_started_by(egui::PointerButton::Primary) {
        return;
    }
    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
        if resize_direction_at(ui.ctx().content_rect(), pos, RESIZE_EDGE).is_some() {
            return;
        }
    }
    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
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

    #[test]
    fn top_bar_drag_rect_excludes_resize_grips() {
        let bar = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(200.0, 40.0));
        let drag = top_bar_drag_rect(bar, RESIZE_EDGE);
        assert!(drag.is_positive());
        assert_eq!(drag.min.x, RESIZE_EDGE);
        assert_eq!(drag.min.y, RESIZE_EDGE);
        assert_eq!(drag.max.x, 200.0 - RESIZE_EDGE);
        assert_eq!(drag.max.y, 40.0);
        // North mid-edge stays a resize grip, not a drag target.
        assert_eq!(
            resize_direction_at(bar, pos2(100.0, 2.0), RESIZE_EDGE),
            Some(ResizeDirection::North)
        );
        assert!(!drag.contains(pos2(100.0, 2.0)));
        // Interior of the carved bar is free for window move.
        assert!(drag.contains(pos2(100.0, 20.0)));
        assert_eq!(
            resize_direction_at(bar, pos2(100.0, 20.0), RESIZE_EDGE),
            None
        );
    }

    #[test]
    fn top_bar_drag_rect_collapses_when_too_thin() {
        let bar = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(10.0, 4.0));
        assert!(!top_bar_drag_rect(bar, RESIZE_EDGE).is_positive());
    }

    #[test]
    fn resize_active_flag_round_trips() {
        assert!(!is_borderless_resize_active());
        RESIZE_ACTIVE.store(true, Ordering::Relaxed);
        assert!(is_borderless_resize_active());
        RESIZE_ACTIVE.store(false, Ordering::Relaxed);
        assert!(!is_borderless_resize_active());
    }
}
