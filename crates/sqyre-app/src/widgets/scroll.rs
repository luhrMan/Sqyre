//! Shared [`egui::ScrollArea`] builders for panels, floaters, and pickers.
//!
//! Prefer these over ad-hoc `ScrollArea::vertical()` / `::both()` so drag-scroll,
//! viewport caps, and horizontal overflow stay consistent.
//!
//! # When to use which
//!
//! | Helper | Use |
//! |--------|-----|
//! | [`dialog_scroll`] / [`scroll_both`] | Default for lists, forms, floater bodies — both axes; H-bar when content is wider |
//! | [`scroll_vertical`] | Intentional V-only: wrapping grids, tip bodies capped with wrap, surfaces that clip rows |
//! | [`show_overflow`] | Chrome painted directly when it fits; both-axis scroll only after a measured overflow |
//!
//! For dense single-line rows that must not wrap, call [`enable_dense_row_extend`]
//! inside a [`dialog_scroll`] body and **do not** `set_max_width(viewport)` (that
//! caps content and hides H-overflow). Grids that should reflow use
//! `horizontal_wrapped` instead. Macro tree stays V-only (egui_ltreeview width ratchet).

use egui::containers::scroll_area::{DragScroll, ScrollSource};

/// Wheel + scrollbar + click-drag. egui's default drag is touch-only (`OnTouch`).
pub const SCROLL_SOURCE: ScrollSource = ScrollSource::ALL;

/// Scroll source for areas that implement their own drag-scroll (e.g. macro tree).
pub const SCROLL_SOURCE_NO_DRAG: ScrollSource = ScrollSource {
    scroll_bar: true,
    drag: DragScroll::Never,
    mouse_wheel: true,
};

/// Vertical [`egui::ScrollArea`] with click-drag scrolling enabled.
pub fn scroll_vertical() -> egui::ScrollArea {
    egui::ScrollArea::vertical().scroll_source(SCROLL_SOURCE)
}

/// Bidirectional [`egui::ScrollArea`] with click-drag scrolling enabled.
pub fn scroll_both() -> egui::ScrollArea {
    egui::ScrollArea::both().scroll_source(SCROLL_SOURCE)
}

/// Bidirectional scroll that fills a capped viewport without expanding the parent.
///
/// Vertical-only `ScrollArea` + `auto_shrink([false, false])` expands to content
/// width and ratchets windows off-screen; enabling both axes keeps width at the
/// viewport (`(true, false) => inner_size` in egui).
pub fn dialog_scroll(max_w: f32, max_h: f32) -> egui::ScrollArea {
    scroll_both()
        .auto_shrink([false, false])
        .max_width(max_w.max(1.0))
        .max_height(max_h.max(1.0))
}

/// Prefer Extend so dense horizontal rows report their intrinsic width to a parent
/// [`dialog_scroll`] (H-bar when the pane is narrower).
pub fn enable_dense_row_extend(ui: &mut egui::Ui) {
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
}

/// Pixels past the viewport that still count as "fits", so a rounding error does
/// not flip a full-width row into a scrollbar.
const OVERFLOW_SLOP: f32 = 4.0;

/// Width the strip may occupy, from the current clip and max rect.
///
/// Not [`super::visible_width`]: that helper also caps to the dialog constrain
/// rect, which is inset from the root window, so a full-width toolbar would
/// always look wider than the budget.
fn overflow_width(ui: &egui::Ui) -> f32 {
    axis_limit(
        ui.available_width(),
        ui.clip_rect().width(),
        ui.max_rect().width(),
    )
}

/// Height the strip may occupy before it scrolls.
///
/// Top and bottom panels shrink-wrap, so their current height is the previous
/// frame's content, not the viewport. A row taller than that placeholder is not
/// off-screen; the panel grows. Those bars scroll vertically only past the
/// screen. Every other parent is a fixed pane, so the cap is that pane.
fn overflow_max_height(ui: &egui::Ui, frac: f32) -> f32 {
    let local = axis_limit(
        ui.available_height(),
        ui.clip_rect().height(),
        ui.max_rect().height(),
    );
    let shrink_wrap = !ui.stack().contained_in(egui::UiKind::ScrollArea)
        && (ui.stack().contained_in(egui::UiKind::TopPanel)
            || ui.stack().contained_in(egui::UiKind::BottomPanel));
    let room = if shrink_wrap {
        ui.ctx().content_rect().height().max(local)
    } else {
        local
    };
    let row = ui.spacing().interact_size.y.max(1.0);
    let floor = row.min(room).max(1.0);
    (room * frac).clamp(floor, room.max(floor))
}

fn axis_limit(avail: f32, clip: f32, max_rect: f32) -> f32 {
    let current = clip.min(max_rect);
    if current.is_finite() && current > 1.0 {
        // `avail` is infinite inside a parent scroll. A finite clip is the viewport.
        if avail.is_finite() {
            current.min(avail).max(1.0)
        } else {
            current
        }
    } else if avail.is_finite() && avail > 1.0 {
        avail
    } else {
        // No finite pane on this axis: the parent grows or already scrolls.
        // A 1px stand-in becomes a viewport that shows a sliver of the text.
        f32::INFINITY
    }
}

/// Paint `add_contents` in place when it fits. Scroll it when it does not.
///
/// The first overflowing pass is discarded, so the clipped frame is not shown.
/// Later passes use a both-axis strip capped to the visible pane (`min_scrolled_*`
/// is 1 so egui's 64px floor cannot stretch a one-row bar). Drag-scroll is off
/// so buttons in the strip still receive clicks.
///
/// `max_height_frac` is the share of the visible height the strip may use before
/// it scrolls vertically (`1.0` for a single row: only overflow past the whole
/// pane). Width always scrolls past the visible width. `id` is scoped to the
/// parent `Ui`.
pub fn show_overflow<R>(
    ui: &mut egui::Ui,
    id: &str,
    max_height_frac: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let frac = max_height_frac.clamp(0.15, 1.0);
    let w = overflow_width(ui);
    let max_h = overflow_max_height(ui, frac);

    let size_id = ui.make_persistent_id(id).with("overflow_size");
    let flag_id = size_id.with("on");
    // `natural_id` holds the size content measured without the scroll wrapper.
    // The scroll branch reports `content_size` under scroll constraints, which
    // can be smaller than the natural size (content re-wraps); comparing that
    // against the pane clears the flag while the unwrapped content still
    // overflows — a scroll/no-scroll oscillation that discards every pass.
    let natural_id = size_id.with("natural");
    let remembered = ui
        .ctx()
        .data(|d| d.get_temp::<egui::Vec2>(size_id))
        .unwrap_or(egui::Vec2::ZERO);
    let natural = ui
        .ctx()
        .data(|d| d.get_temp::<egui::Vec2>(natural_id))
        .unwrap_or(remembered);
    let was_overflow = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(flag_id))
        .unwrap_or(false);
    // Enter only past the slop so a full-width row does not flicker into a bar.
    // Leave when the natural size fits the pane again.
    let width_over = |size: f32, slop: f32| w.is_finite() && size > w + slop;
    let height_over = |size: f32, slop: f32| max_h.is_finite() && size > max_h + slop;
    let overflow = remembered.x > 0.0
        && if was_overflow {
            width_over(natural.x, 0.0) || height_over(natural.y, 0.0)
        } else {
            width_over(remembered.x, OVERFLOW_SLOP) || height_over(remembered.y, OVERFLOW_SLOP)
        };

    if overflow {
        let row = ui.spacing().interact_size.y.max(1.0);
        // An unbounded axis must not be passed to `max_height` as infinity: the
        // viewport collapses to a sliver. Use the measured content size instead.
        let scroll_w = if w.is_finite() {
            w
        } else {
            remembered.x.max(1.0)
        };
        let scroll_h = if max_h.is_finite() {
            max_h
        } else {
            remembered.y.max(row)
        };
        let out = scroll_both()
            .id_salt(id)
            .scroll_source(SCROLL_SOURCE_NO_DRAG)
            .auto_shrink([false, true])
            .min_scrolled_width(1.0)
            .min_scrolled_height(1.0)
            .max_width(scroll_w)
            .max_height(scroll_h)
            .show(ui, |ui| {
                enable_dense_row_extend(ui);
                add_contents(ui)
            });
        let size = out.content_size;
        ui.ctx().data_mut(|d| d.insert_temp(size_id, size));
        out.inner
    } else {
        // Cursor advance, not `min_rect`: a central panel expands `min_rect` to
        // its max rect before content, so that rect is the whole pane.
        let y0 = ui.cursor().min.y;
        let spacing_y = ui.spacing().item_spacing.y;
        let inner = add_contents(ui);
        let size = egui::vec2(
            (ui.min_rect().right() - ui.max_rect().left()).max(0.0),
            (ui.cursor().min.y - y0 - spacing_y).max(0.0),
        );
        let overflows_now = width_over(size.x, OVERFLOW_SLOP) || height_over(size.y, OVERFLOW_SLOP);
        ui.ctx().data_mut(|d| {
            d.insert_temp(size_id, size);
            d.insert_temp(natural_id, size);
            d.insert_temp(flag_id, overflows_now);
        });
        if overflows_now {
            ui.ctx().request_discard("overflow strip");
        }
        inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_strip_stays_inside_narrow_pane() {
        let ctx = egui::Context::default();
        let mut last_w = f32::MAX;
        for _ in 0..3 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(100.0, 400.0));
                let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                pane_ui.set_clip_rect(pane);
                show_overflow(&mut pane_ui, "row", 0.5, |ui| {
                    ui.horizontal(|ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        for label in ["Programs", "Items", "Coordinates", "Tools"] {
                            let _ = ui.button(label);
                        }
                    });
                });
                last_w = pane_ui.min_rect().width();
            })
            .drop_without_applying_deltas();
        }
        assert!(
            last_w <= 100.5,
            "overflow strip widened the pane to {last_w}"
        );
    }

    /// A parent with no finite height (settings scroll content) must not collapse
    /// a one-line header into a 1px viewport.
    #[test]
    fn unbounded_height_does_not_collapse_the_row() {
        let ctx = egui::Context::default();
        let mut last_h = 0.0_f32;
        for _ in 0..3 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let origin = ui.max_rect().min;
                let pane =
                    egui::Rect::from_min_max(origin, egui::pos2(origin.x + 240.0, f32::INFINITY));
                let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                pane_ui.set_clip_rect(egui::Rect::from_min_max(
                    origin,
                    egui::pos2(f32::INFINITY, f32::INFINITY),
                ));
                show_overflow(&mut pane_ui, "Screen recording", 1.0, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Granted");
                        ui.label("Screen recording");
                    });
                });
                last_h = pane_ui.min_rect().height();
            })
            .drop_without_applying_deltas();
        }
        assert!(last_h > 12.0, "header collapsed to a sliver ({last_h}px)");
    }
}
