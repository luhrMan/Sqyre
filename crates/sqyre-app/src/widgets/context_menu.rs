//! Right-click menus for list rows, chips, and thumbnails.
//!
//! Per-element actions (edit, delete, remove, toggle) live in these menus rather
//! than as inline buttons beside the element.

use eframe::egui::{
    self, containers::menu::menu_style, InnerResponse, Popup, PopupAnchor, PopupCloseBehavior,
    PopupKind,
};

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

/// Shift+F10 while `anchor` holds keyboard focus — the keyboard equivalent of
/// a right-click (egui delivers no dedicated Menu-key event).
fn context_menu_key_pressed(ui: &egui::Ui, anchor: Option<&egui::Response>) -> bool {
    anchor.is_some_and(|r| r.has_focus())
        && ui.input(|i| {
            i.key_pressed(egui::Key::F10)
                && i.modifiers.shift
                && !i.modifiers.alt
                && !i.modifiers.command
        })
}

/// Show the menu `id`, opening it this frame when `open_now`: at the pointer
/// for clicks, or beside `open_at` (the anchor widget's rect) for keyboard.
/// Closes on any click; returns the contents' result while open.
pub fn context_menu_popup<R>(
    ui: &egui::Ui,
    id: egui::Id,
    open_now: bool,
    open_at: Option<egui::Rect>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let anchor = open_at.map_or(PopupAnchor::PointerFixed, PopupAnchor::ParentRect);
    let inner = Popup::new(id, ui.ctx().clone(), anchor, ui.layer_id())
        .kind(PopupKind::Menu)
        .layout(egui::Layout::top_down_justified(egui::Align::Min))
        .style(menu_style)
        .open_memory(open_now.then_some(egui::SetOpenCommand::Bool(true)))
        .show(add_contents);
    if let Some(inner) = &inner {
        // Only once the menu is already open: the click that opened it is the
        // ancestor's own outside click to evaluate — a menu opening at the
        // pointer would otherwise shield that click from a `parent_popup`.
        if !open_now {
            register_nested_popup(ui.ctx(), inner.response.rect);
        }
    }
    inner.map(|r| r.inner)
}

/// Rects of popups shown inside another popup this frame. See [`parent_popup`].
fn nested_rects_id() -> egui::Id {
    egui::Id::new("sqyre_nested_popup_rects")
}

/// Record the rect of a popup shown inside another popup's content.
///
/// Every egui `Popup`/`Area` paints on its own layer, so a click inside a
/// nested popup (context menu, suggestion dropdown) is `clicked_elsewhere` for
/// the ancestor that spawned it: `PopupCloseBehavior::CloseOnClickOutside`
/// would dismiss the whole popup family. Popups nested inside a [`parent_popup`]
/// call this each frame so the parent's outside-click check treats clicks
/// inside them as inside the family.
pub fn register_nested_popup(ctx: &egui::Context, rect: egui::Rect) {
    ctx.data_mut(|d| {
        let mut rects = d
            .get_temp::<Vec<egui::Rect>>(nested_rects_id())
            .unwrap_or_default();
        rects.push(rect);
        d.insert_temp(nested_rects_id(), rects);
    });
}

/// Show `popup` with `open_bool` state and a nested-popup-aware outside-click
/// close.
///
/// Use for a `PopupKind::Popup` whose content opens further popups (context
/// menus via [`context_menu_popup`], tag suggestion dropdowns). egui's
/// `CloseOnClickOutside` cannot express this: nested popups live on other
/// layers, so clicks inside them read as "outside" and wrongly dismiss the
/// parent. This shows the popup with `PopupCloseBehavior::IgnoreClicks` and
/// instead writes `open = false` only when a click lands outside both the
/// popup's own rect and every rect registered via [`register_nested_popup`]
/// this frame. Escape / `ui.close()` still close it through egui's own
/// handling.
///
/// Open state lives in `open` (a temp-data bool at the call site), not
/// `Memory::popups`: that slot holds a single popup per viewport, so a nested
/// `open_memory` context menu would evict the parent popup it was opened from.
/// [`Popup::is_id_open`] / [`Popup::close_all`] therefore do not see it.
pub fn parent_popup<'a, R>(
    popup: egui::Popup<'a>,
    open: &'a mut bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<InnerResponse<R>> {
    let ctx = popup.ctx().clone();
    let popup_id = popup.get_id();
    // Same guard egui applies for CloseOnClickOutside: the click that opened
    // the popup must not immediately close it again.
    let was_open_last_frame = ctx.read_response(popup_id).is_some();
    ctx.data_mut(|d| d.remove_temp::<Vec<egui::Rect>>(nested_rects_id()));
    let inner = popup
        .open_bool(open)
        .close_behavior(PopupCloseBehavior::IgnoreClicks)
        .show(add_contents)?;
    let click_pos = ctx.input(|i| {
        if i.pointer.any_click() {
            i.pointer.interact_pos()
        } else {
            None
        }
    });
    if let (true, Some(pos)) = (was_open_last_frame, click_pos) {
        let in_nested = ctx.data(|d| {
            d.get_temp::<Vec<egui::Rect>>(nested_rects_id())
                .unwrap_or_default()
                .iter()
                .any(|r| r.contains(pos))
        });
        if !inner.response.rect.contains(pos) && !in_nested {
            *open = false;
        }
    }
    Some(inner)
}

/// Right-click menu for whatever was painted in `rect`.
///
/// `anchor` is the focusable response the rect stands for: Shift+F10 while it
/// has focus opens the menu beside it. Pass `None` when nothing in `rect` can
/// hold keyboard focus.
pub fn rect_context_menu<R>(
    ui: &egui::Ui,
    id: egui::Id,
    rect: egui::Rect,
    anchor: Option<&egui::Response>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let keyed = context_menu_key_pressed(ui, anchor);
    let open = secondary_clicked_in(ui, rect) || keyed;
    let open_at = anchor.filter(|_| keyed).map(|r| r.rect);
    context_menu_popup(ui, id, open, open_at, add_contents)
}

/// Right-click menu for a widget, using its rect (works for hover-only rows).
pub fn response_context_menu<R>(
    ui: &egui::Ui,
    resp: &egui::Response,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    rect_context_menu(
        ui,
        resp.id.with("context_menu"),
        resp.rect,
        Some(resp),
        add_contents,
    )
}

/// Right-click menu stretched to the current row's right edge.
pub fn row_context_menu<R>(
    ui: &egui::Ui,
    id: egui::Id,
    rect: egui::Rect,
    anchor: Option<&egui::Response>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    rect_context_menu(
        ui,
        id,
        rect.with_max_x(ui.max_rect().right()),
        anchor,
        add_contents,
    )
}

/// Right-click `rect` to open a menu with one destructive entry.
pub fn rect_danger_menu(
    ui: &egui::Ui,
    id: egui::Id,
    rect: egui::Rect,
    anchor: Option<&egui::Response>,
    label: &str,
    enabled: bool,
) -> bool {
    rect_context_menu(ui, id, rect, anchor, |ui| {
        menu_item_danger(ui, label, enabled)
    })
    .unwrap_or(false)
}

/// [`rect_danger_menu`] on a widget response.
pub fn response_danger_menu(ui: &egui::Ui, resp: &egui::Response, label: &str) -> bool {
    rect_danger_menu(
        ui,
        resp.id.with("danger_menu"),
        resp.rect,
        Some(resp),
        label,
        true,
    )
}

/// [`rect_danger_menu`] stretched across the current row.
pub fn row_danger_menu(
    ui: &egui::Ui,
    id: egui::Id,
    rect: egui::Rect,
    anchor: Option<&egui::Response>,
    label: &str,
) -> bool {
    row_context_menu(ui, id, rect, anchor, |ui| menu_item_danger(ui, label, true)).unwrap_or(false)
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
                    let rect = row_rect();
                    let anchor = ui.interact(rect, ui.id().with("row"), egui::Sense::click());
                    *open =
                        rect_context_menu(ui, ui.id().with("menu"), rect, Some(&anchor), |ui| {
                            menu_item(ui, "Delete", true)
                        })
                        .is_some();
                },
                false,
            )
    }

    fn key_press(
        harness: &mut egui_kittest::Harness<'_, bool>,
        key: egui::Key,
        modifiers: egui::Modifiers,
    ) {
        let input = harness.input_mut();
        // egui tracks InputState::modifiers via ModifiersChanged events —
        // the modifiers field on the Key event is not enough.
        input.events.push(egui::Event::ModifiersChanged(modifiers));
        input.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        });
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
    fn shift_f10_opens_the_menu_for_the_focused_row() {
        let mut harness = harness();
        harness.run_steps(2);
        // Shift+F10 with nothing focused does not open the menu.
        key_press(&mut harness, egui::Key::F10, egui::Modifiers::SHIFT);
        harness.run_steps(1);
        assert!(!*harness.state(), "unfocused Shift+F10 must not open");
        // Tab focuses the row (the only focusable widget), then Shift+F10.
        key_press(&mut harness, egui::Key::Tab, egui::Modifiers::NONE);
        harness.run_steps(1);
        assert!(
            harness.ctx.memory(|m| m.focused()).is_some(),
            "Tab should focus the row widget"
        );
        key_press(&mut harness, egui::Key::F10, egui::Modifiers::SHIFT);
        harness.run_steps(1);
        assert!(*harness.state(), "Shift+F10 on the focused row must open");
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

    /// `open` flag | tag chips | draft — a `parent_popup` hosting a tag editor
    /// like the data editor's sort menu does.
    type FamilyState = (bool, Vec<String>, String);

    fn family_harness(state: FamilyState) -> egui_kittest::Harness<'static, FamilyState> {
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(480.0, 360.0))
            .build_ui_state(
                |ui, (open, tags, draft): &mut FamilyState| {
                    let btn = ui.button("Sort");
                    if btn.clicked() {
                        *open = !*open;
                    }
                    parent_popup(
                        egui::Popup::new(
                            btn.id.with("sort_menu"),
                            ui.ctx().clone(),
                            egui::PopupAnchor::ParentRect(btn.rect),
                            ui.layer_id(),
                        )
                        .kind(egui::PopupKind::Popup)
                        .layout(egui::Layout::top_down_justified(egui::Align::Min)),
                        open,
                        |ui| {
                            let _ = crate::widgets::tag_chip_editor(
                                ui,
                                tags,
                                draft,
                                &["beta".to_owned()],
                                crate::widgets::TagChipOptions::default(),
                            );
                        },
                    );
                },
                state,
            )
    }

    fn pointer_click(
        harness: &mut egui_kittest::Harness<'_, FamilyState>,
        at: egui::Pos2,
        button: egui::PointerButton,
    ) {
        harness.hover_at(at);
        harness.run_steps(2);
        for pressed in [true, false] {
            harness.input_mut().events.push(egui::Event::PointerButton {
                pos: at,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
            harness.run_steps(1);
        }
        harness.run_steps(2);
    }

    /// Top-left of the painted galley for `text` — tag chips are painter-drawn
    /// pills, not widgets, so their position comes from the frame output.
    fn painted_text_pos(
        harness: &egui_kittest::Harness<'_, FamilyState>,
        text: &str,
    ) -> egui::Pos2 {
        harness
            .output()
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.job.text == text => Some(t.pos),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no painted text {text:?}"))
    }

    #[test]
    fn chip_menu_inside_parent_popup_stays_open_and_removes() {
        use egui_kittest::kittest::Queryable;
        let mut harness = family_harness((true, vec!["alpha".to_string()], String::new()));
        harness.run_steps(3);
        let chip = painted_text_pos(&harness, "alpha") + egui::vec2(4.0, 4.0);
        pointer_click(&mut harness, chip, egui::PointerButton::Secondary);
        let remove = harness.get_by_label("Remove").rect().center();
        pointer_click(&mut harness, remove, egui::PointerButton::Primary);
        harness.run_steps(2);
        assert!(harness.state().1.is_empty(), "Remove should delete the tag");
        assert!(harness.state().0, "parent popup must stay open");
        harness.get_by_role(egui::accesskit::Role::TextInput);
    }

    #[test]
    fn suggestion_click_inside_parent_popup_stays_open() {
        use egui_kittest::kittest::Queryable;
        let mut harness = family_harness((true, Vec::new(), String::new()));
        harness.run_steps(3);
        let field = harness
            .get_by_role(egui::accesskit::Role::TextInput)
            .rect()
            .center();
        pointer_click(&mut harness, field, egui::PointerButton::Primary);
        let beta = harness.get_by_label("beta").rect().center();
        pointer_click(&mut harness, beta, egui::PointerButton::Primary);
        assert_eq!(harness.state().1, vec!["beta".to_string()]);
        assert!(harness.state().0, "parent popup must stay open");
    }

    #[test]
    fn outside_click_closes_parent_popup() {
        let mut harness = family_harness((true, Vec::new(), String::new()));
        harness.run_steps(3);
        pointer_click(
            &mut harness,
            egui::pos2(430.0, 330.0),
            egui::PointerButton::Primary,
        );
        assert!(!harness.state().0, "click outside must close the popup");
    }
}
