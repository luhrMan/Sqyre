//! Right-click menus for list rows, chips, and thumbnails.
//!
//! Per-element actions (edit, delete, remove, toggle) live in these menus rather
//! than as inline buttons beside the element.

use eframe::egui::{self, containers::menu::menu_style, Popup, PopupAnchor, PopupKind};

/// True when the secondary button was clicked inside `rect` on this `ui`'s layer.
///
/// Geometric (not `Response::secondary_clicked`) so rows built from hover-only
/// labels or with child buttons still open a menu without stealing primary clicks.
fn secondary_clicked_in(ui: &egui::Ui, rect: egui::Rect) -> bool {
    let secondary = ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary));
    if !secondary && !long_pressing(ui) {
        return false;
    }
    let hit = rect.intersect(ui.clip_rect());
    ui.input(|i| i.pointer.interact_pos())
        .is_some_and(|p| hit.contains(p) && ui.ctx().layer_id_at(p) == Some(ui.layer_id()))
}

/// A finger held still past `max_click_duration` — egui's long-touch, read
/// geometrically since hover-only rows get no `Response` flag for it.
fn long_pressing(ui: &egui::Ui) -> bool {
    let (max_dist, max_dur) = ui.ctx().options(|o| {
        (
            o.input_options.max_click_dist,
            o.input_options.max_click_duration,
        )
    });
    ui.input(|i| {
        i.any_touches()
            && i.pointer.primary_down()
            && i.pointer
                .press_start_time()
                .is_some_and(|t| i.time - t > max_dur)
            && i.pointer
                .press_origin()
                .zip(i.pointer.interact_pos())
                .is_some_and(|(origin, pos)| origin.distance(pos) <= max_dist)
    })
}

/// Show the menu `id` at the pointer, opening it this frame when `open_now`.
/// Closes on any click; returns the contents' result while open.
pub fn context_menu_popup<R>(
    ui: &egui::Ui,
    id: egui::Id,
    open_now: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    Popup::new(
        id,
        ui.ctx().clone(),
        PopupAnchor::PointerFixed,
        ui.layer_id(),
    )
    .kind(PopupKind::Menu)
    .layout(egui::Layout::top_down_justified(egui::Align::Min))
    .style(menu_style)
    .open_memory(open_now.then_some(egui::SetOpenCommand::Bool(true)))
    .show(add_contents)
    .map(|r| r.inner)
}

/// Right-click menu for whatever was painted in `rect`.
pub fn rect_context_menu<R>(
    ui: &egui::Ui,
    id: egui::Id,
    rect: egui::Rect,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    context_menu_popup(ui, id, secondary_clicked_in(ui, rect), add_contents)
}

/// Right-click menu for a widget, using its rect (works for hover-only rows).
pub fn response_context_menu<R>(
    ui: &egui::Ui,
    resp: &egui::Response,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    rect_context_menu(ui, resp.id.with("context_menu"), resp.rect, add_contents)
}

/// Right-click menu stretched to the current row's right edge.
pub fn row_context_menu<R>(
    ui: &egui::Ui,
    id: egui::Id,
    rect: egui::Rect,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    rect_context_menu(ui, id, rect.with_max_x(ui.max_rect().right()), add_contents)
}

/// Right-click `rect` to open a menu with one destructive entry.
pub fn rect_danger_menu(
    ui: &egui::Ui,
    id: egui::Id,
    rect: egui::Rect,
    label: &str,
    enabled: bool,
) -> bool {
    rect_context_menu(ui, id, rect, |ui| menu_item_danger(ui, label, enabled)).unwrap_or(false)
}

/// [`rect_danger_menu`] on a widget response.
pub fn response_danger_menu(ui: &egui::Ui, resp: &egui::Response, label: &str) -> bool {
    rect_danger_menu(ui, resp.id.with("danger_menu"), resp.rect, label, true)
}

/// [`rect_danger_menu`] stretched across the current row.
pub fn row_danger_menu(ui: &egui::Ui, id: egui::Id, rect: egui::Rect, label: &str) -> bool {
    row_context_menu(ui, id, rect, |ui| menu_item_danger(ui, label, true)).unwrap_or(false)
}

fn menu_button(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>, enabled: bool) -> bool {
    ui.add_enabled(enabled, egui::Button::new(text)).clicked()
}

/// Plain menu entry.
pub fn menu_item(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    menu_button(ui, label, enabled)
}

/// Destructive menu entry (delete / remove), tinted like inline delete buttons were.
pub fn menu_item_danger(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    menu_button(
        ui,
        egui::RichText::new(label).color(crate::theme::MACRO_STOP),
        enabled,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(200.0, 40.0))
    }

    fn harness() -> egui_kittest::Harness<'static, bool> {
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(400.0, 300.0))
            .build_ui_state(
                |ui, open: &mut bool| {
                    *open = rect_context_menu(ui, ui.id().with("menu"), row_rect(), |ui| {
                        menu_item(ui, "Delete", true)
                    })
                    .is_some();
                },
                false,
            )
    }

    /// Finger down at `pos`: the touch event plus its emulated primary press,
    /// the pair egui-winit sends for the first finger.
    fn touch_down(harness: &mut egui_kittest::Harness<'_, bool>, pos: egui::Pos2) {
        let input = harness.input_mut();
        input.events.push(egui::Event::PointerMoved(pos));
        input.events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        input.events.push(egui::Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(0),
            phase: egui::TouchPhase::Start,
            pos,
            force: None,
        });
    }

    #[test]
    fn touch_long_press_opens_the_menu() {
        let mut harness = harness();
        harness.run_steps(2);
        touch_down(&mut harness, row_rect().center());
        harness.run_steps(1);
        assert!(!*harness.state(), "tap must not open the menu");
        // ~0.25 s per step: past the 0.8 s max_click_duration the menu opens.
        harness.run_steps(6);
        assert!(*harness.state(), "long-press must open the menu");
    }

    #[test]
    fn moved_finger_never_opens_the_menu() {
        let mut harness = harness();
        harness.run_steps(2);
        touch_down(&mut harness, row_rect().center());
        harness.run_steps(1);
        let to = row_rect().center() + egui::vec2(40.0, 0.0);
        let input = harness.input_mut();
        input.events.push(egui::Event::PointerMoved(to));
        input.events.push(egui::Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(0),
            phase: egui::TouchPhase::Move,
            pos: to,
            force: None,
        });
        harness.run_steps(6);
        assert!(!*harness.state(), "a scroll drag must not open the menu");
    }
}
