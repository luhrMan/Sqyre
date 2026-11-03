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

/// The scroll viewport expanded through the enclosing frame's padding, plus
/// the margin to re-apply inside the scroll content.
///
/// egui anchors floating bars at the scroll's outer rect, so a frame's
/// `inner_margin` insets the bar as well as the content. Walking the ui stack
/// to the nearest framed ancestor (same rule as
/// [`crate::widgets::sections::frame_outer_rect`]) gives its painted edge;
/// expanding the viewport right/bottom onto it puts the bars flush with the
/// container while [`egui::ScrollArea::content_margin`] re-pads the content —
/// the same move [`egui::Window`] makes for `Window::scroll`, which routes
/// `window_margin` into `content_margin`.
///
/// Returns `None` when there is no margin to reclaim, or when the gap to the
/// frame edge is wider than the margin — a column/row-bound scroll keeps its
/// own edge instead of detaching into a sibling's region. The clip is a
/// second bound so expansion never escapes a pane.
fn scroll_edge_expand(ui: &egui::Ui) -> Option<(egui::Rect, egui::Margin)> {
    let avail = ui.available_rect_before_wrap();
    let clip = ui.clip_rect();
    for node in ui.stack().iter() {
        let frame = node.frame();
        let m = frame.inner_margin;
        let framed = node.kind() == Some(egui::UiKind::Frame)
            && (m.left > 0 || m.right > 0 || !frame.stroke.is_empty());
        if !framed {
            continue;
        }
        // The innermost Ui reflects `set_max_width` narrowing; stack snapshots
        // do not (same caveat as `frame_outer_rect`).
        let inner = if node.id == ui.id() {
            ui.max_rect()
        } else {
            node.max_rect
        };
        let outer = egui::Rect::from_min_max(
            inner.min - egui::vec2(f32::from(m.left), f32::from(m.top)),
            inner.max + egui::vec2(f32::from(m.right), f32::from(m.bottom)),
        );
        // Expand an axis only when its whole gap is the frame's margin — a
        // wider gap means another bound is in the way and the bar would float
        // into a sibling's region instead of reaching the edge.
        let grow = egui::vec2(
            if outer.max.x - avail.max.x <= f32::from(m.right) + 0.5 {
                (outer.max.x - avail.max.x).max(0.0)
            } else {
                0.0
            },
            if outer.max.y - avail.max.y <= f32::from(m.bottom) + 0.5 {
                (outer.max.y - avail.max.y).max(0.0)
            } else {
                0.0
            },
        );
        let max = egui::pos2(
            (avail.max.x + grow.x).min(clip.max.x),
            (avail.max.y + grow.y).min(clip.max.y),
        );
        if max.x <= avail.max.x + 0.5 && max.y <= avail.max.y + 0.5 {
            return None;
        }
        return Some((
            egui::Rect::from_min_max(avail.min, max),
            egui::Margin {
                right: (max.x - avail.max.x).max(0.0).round() as i8,
                bottom: (max.y - avail.max.y).max(0.0).round() as i8,
                ..Default::default()
            },
        ));
    }
    None
}

/// Run `scroll` in a [`Ui::new_child`] bounded by `rect`, then report only the
/// claim that fits the original `bound` back to the parent. `new_child` does
/// not merge `min_rect`, so the expanded viewport can't ratchet auto-sized
/// windows open; siblings still advance past the scroll's real footprint.
///
/// `shrink_to_content` picks, per axis, whether the claim follows the painted
/// content (`content_size`) or the full inner rect. egui auto-shrinks the
/// painted viewport — and the floating bar anchor — to `content_size`, but
/// `ScrollAreaOutput::inner_rect` is captured before that shrink, so claiming
/// it verbatim leaves dead space between the bar and the frame edge when
/// content under-fills the bound.
fn show_expanded<R>(
    ui: &mut egui::Ui,
    bound: egui::Rect,
    rect: egui::Rect,
    scroll: egui::ScrollArea,
    shrink_to_content: egui::Vec2b,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::containers::scroll_area::ScrollAreaOutput<R> {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let out = scroll.show(&mut child, add_contents);
    let used = egui::pos2(
        if shrink_to_content.x {
            out.inner_rect.min.x + out.content_size.x
        } else {
            out.inner_rect.max.x
        }
        .clamp(bound.min.x, bound.max.x),
        if shrink_to_content.y {
            out.inner_rect.min.y + out.content_size.y
        } else {
            out.inner_rect.max.y
        }
        .clamp(bound.min.y, bound.max.y),
    );
    let claimed = egui::Rect::from_min_max(bound.min, used);
    ui.expand_to_include_rect(claimed);
    ui.advance_cursor_after_rect(claimed);
    out
}

/// Show `scroll` with its viewport expanded through the enclosing frame's
/// padding so floating bars paint flush with the container edge; see
/// [`scroll_edge_expand`]. Capped scrolls (`max_width`/`max_height` below the
/// bound) keep their own edge — use [`show_dialog_scroll`] for those.
pub fn show_scroll_edge<R>(
    ui: &mut egui::Ui,
    scroll: egui::ScrollArea,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::containers::scroll_area::ScrollAreaOutput<R> {
    let bound = ui.available_rect_before_wrap();
    let Some((rect, margin)) = scroll_edge_expand(ui) else {
        return scroll.show(ui, add_contents);
    };
    show_expanded(
        ui,
        bound,
        rect,
        scroll.content_margin(margin),
        egui::Vec2b::FALSE,
        add_contents,
    )
}

/// [`show_scroll_edge`] that claims the painted content size on each axis: a
/// floating bar hugs the shrunk viewport, so a scroll whose children do not
/// fill the bound keeps the bar flush with the frame edge instead of floating
/// in dead space.
pub fn show_scroll_edge_fit<R>(
    ui: &mut egui::Ui,
    scroll: egui::ScrollArea,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::containers::scroll_area::ScrollAreaOutput<R> {
    let bound = ui.available_rect_before_wrap();
    let Some((rect, margin)) = scroll_edge_expand(ui) else {
        return scroll.show(ui, add_contents);
    };
    show_expanded(
        ui,
        bound,
        rect,
        scroll.content_margin(margin),
        egui::Vec2b::TRUE,
        add_contents,
    )
}

/// [`dialog_scroll`] shown via [`show_scroll_edge`].
///
/// Viewport caps are widened by the reclaimed margin so the outer rect — and
/// the bar — reaches the frame edge. A cap already tighter than the bound
/// keeps its own edge instead of floating the bar into a gap.
pub fn show_dialog_scroll<R>(
    ui: &mut egui::Ui,
    max_w: f32,
    max_h: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::containers::scroll_area::ScrollAreaOutput<R> {
    show_dialog_scroll_mod(ui, max_w, max_h, |s| s, add_contents)
}

/// [`show_dialog_scroll`] plus a builder hook (`id_salt`, `stick_to_bottom`, …)
/// applied after the caps are grown.
pub fn show_dialog_scroll_mod<R>(
    ui: &mut egui::Ui,
    max_w: f32,
    max_h: f32,
    modify: impl FnOnce(egui::ScrollArea) -> egui::ScrollArea,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::containers::scroll_area::ScrollAreaOutput<R> {
    let avail = ui.available_size();
    let Some((rect, margin)) = scroll_edge_expand(ui) else {
        return modify(dialog_scroll(max_w, max_h)).show(ui, add_contents);
    };
    let grow = egui::vec2(
        if max_w >= avail.x - 0.5 {
            f32::from(margin.right)
        } else {
            0.0
        },
        if max_h >= avail.y - 0.5 {
            f32::from(margin.bottom)
        } else {
            0.0
        },
    );
    show_expanded(
        ui,
        ui.available_rect_before_wrap(),
        rect,
        modify(dialog_scroll(max_w + grow.x, max_h + grow.y)).content_margin(egui::Margin {
            right: grow.x as i8,
            bottom: grow.y as i8,
            ..Default::default()
        }),
        egui::Vec2b::FALSE,
        add_contents,
    )
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
    // `size_id` holds the last pass's measured size — the in-place measurement
    // while the strip paints unwrapped, and the scroll viewport's
    // `content_size` once scrolled. Both are a fresh "would it overflow" signal
    // every frame, so a measurement poisoned by a transient pane (egui's zoom
    // book-keeping can hand the strip a ~10_000pt pane for a single frame when
    // `set_zoom_factor` settles) or by content that shrank while scrolled heals
    // as soon as a scroll pass re-measures — but only if the stay check uses
    // the same slop as the entry check. A zero-slop stay check latches forever
    // on a `content_size` that lands a sub-pixel past the pane (`948.6 > 948`).
    let remembered = ui
        .ctx()
        .data(|d| d.get_temp::<egui::Vec2>(size_id))
        .unwrap_or(egui::Vec2::ZERO);
    let width_over = |size: f32| w.is_finite() && size > w + OVERFLOW_SLOP;
    let height_over = |size: f32| max_h.is_finite() && size > max_h + OVERFLOW_SLOP;
    let overflow = remembered.x > 0.0 && (width_over(remembered.x) || height_over(remembered.y));

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
        // Reclaim the enclosing frame's margin so the strip's bars reach the
        // pane edge ([`scroll_edge_expand`]); widen the caps by the same
        // amount and re-pad the content inside via `content_margin`.
        let avail = ui.available_size();
        let (rect, grow) = match scroll_edge_expand(ui) {
            Some((rect, margin)) => {
                let grow = egui::vec2(
                    if scroll_w >= avail.x - 0.5 {
                        f32::from(margin.right)
                    } else {
                        0.0
                    },
                    if scroll_h >= avail.y - 0.5 {
                        f32::from(margin.bottom)
                    } else {
                        0.0
                    },
                );
                (rect, grow)
            }
            None => (ui.available_rect_before_wrap(), egui::Vec2::ZERO),
        };
        let out = show_expanded(
            ui,
            ui.available_rect_before_wrap(),
            rect,
            scroll_both()
                .id_salt(id)
                .scroll_source(SCROLL_SOURCE_NO_DRAG)
                .auto_shrink([false, true])
                .min_scrolled_width(1.0)
                .min_scrolled_height(1.0)
                .max_width(scroll_w + grow.x)
                .max_height(scroll_h + grow.y)
                .content_margin(egui::Margin {
                    right: grow.x as i8,
                    bottom: grow.y as i8,
                    ..Default::default()
                }),
            egui::Vec2b::FALSE,
            |ui| {
                enable_dense_row_extend(ui);
                add_contents(ui)
            },
        );
        // `content_size` includes the content margin; subtract it so the
        // remembered "would it overflow" measurement stays on the content.
        let size = out.content_size - grow;
        ui.ctx().data_mut(|d| d.insert_temp(size_id, size));
        out.inner
    } else {
        // Cursor advance, not `min_rect`: a central panel expands `min_rect` to
        // its max rect before content, so that rect is the whole pane.
        // `x0` is the entry cursor, not `max_rect().left()`: the strip can share
        // a `ui.horizontal` with leading siblings, and `min_rect` spans them all.
        let x0 = ui.cursor().min.x;
        let y0 = ui.cursor().min.y;
        let spacing_y = ui.spacing().item_spacing.y;
        let inner = add_contents(ui);
        let size = egui::vec2(
            (ui.min_rect().right() - x0).max(0.0),
            (ui.cursor().min.y - y0 - spacing_y).max(0.0),
        );
        let overflows_now = width_over(size.x) || height_over(size.y);
        ui.ctx().data_mut(|d| {
            d.insert_temp(size_id, size);
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

    /// A measurement taken under a transient oversized pane must not keep the
    /// strip scrolled once the pane shrinks back. egui's `set_zoom_factor`
    /// book-keeping can produce such a pane for a single frame (`screen_rect`
    /// scaled from the 10_000pt `InputState` default); the scroll pass's
    /// `content_size` re-measures under the real pane and must clear it.
    #[test]
    fn transient_oversize_pane_does_not_latch_scroll() {
        let ctx = egui::Context::default();
        let mut flag_id = None;
        let mut run = |pane: egui::Vec2| {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let pane = egui::Rect::from_min_size(ui.max_rect().min, pane);
                let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                pane_ui.set_clip_rect(pane);
                // Panels expand `min_rect` to their rect before content runs.
                pane_ui.expand_to_include_rect(pane);
                show_overflow(&mut pane_ui, "row", 1.0, |ui| {
                    ui.horizontal(|ui| {
                        let _ = ui.button("Fits");
                    });
                });
                flag_id = Some(
                    pane_ui
                        .make_persistent_id("row")
                        .with("overflow_size")
                        .with("on"),
                );
            })
            .drop_without_applying_deltas();
        };
        // One giant transient pane (zoom book-keeping). Its tiny height
        // overflows the row vertically — the same poisoned measurement the
        // top bar records on a bogus frame — while the button fits fine at
        // the real pane below.
        run(egui::vec2(9_000.0, 10.0));
        for _ in 0..4 {
            run(egui::vec2(400.0, 100.0));
        }
        let flag = flag_id.and_then(|id| ctx.data(|d| d.get_temp::<bool>(id)));
        assert_eq!(flag, Some(false), "transient pane latched the strip");
    }

    /// The reverse: content that really does not fit must keep scrolling once
    /// the pane settles — the scroll pass keeps reporting an overflowing
    /// `content_size`.
    #[test]
    fn real_overflow_still_scrolls_when_pane_unchanged() {
        let ctx = egui::Context::default();
        let mut flag_id = None;
        for _ in 0..4 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(100.0, 400.0));
                let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                pane_ui.set_clip_rect(pane);
                show_overflow(&mut pane_ui, "row", 1.0, |ui| {
                    ui.horizontal(|ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        for label in ["Programs", "Items", "Coordinates", "Tools"] {
                            let _ = ui.button(label);
                        }
                    });
                });
                flag_id = Some(
                    pane_ui
                        .make_persistent_id("row")
                        .with("overflow_size")
                        .with("on"),
                );
            })
            .drop_without_applying_deltas();
        }
        let flag = flag_id.and_then(|id| ctx.data(|d| d.get_temp::<bool>(id)));
        assert_eq!(flag, Some(true), "real overflow did not keep scrolling");
    }

    /// A strip inside a fixed-height row (ui.horizontal bounds children to the
    /// row) must keep the measured content height in scroll mode instead of
    /// letting the capped viewport clip the buttons.
    #[test]
    fn h_bar_does_not_clip_row_content() {
        let ctx = egui::Context::default();
        ctx.all_styles_mut(|style| style.spacing.scroll = crate::theme::scroll_style());
        let mut clipped = Vec::new();
        for _ in 0..4 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(240.0, 400.0));
                let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                pane_ui.set_clip_rect(pane);
                pane_ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let _ = ui.button("X");
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            show_overflow(ui, "row", 1.0, |ui| {
                                ui.horizontal(|ui| {
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                    for label in [
                                        "Programs",
                                        "Items",
                                        "Coordinates",
                                        "Tools",
                                        "Extra1",
                                        "Extra2",
                                    ] {
                                        let resp = ui.button(label);
                                        // Buttons scrolled past the viewport
                                        // clip horizontally — expected. Only
                                        // the vertical extent (the H-bar lane)
                                        // is checked.
                                        if resp.interact_rect.is_positive()
                                            && resp.interact_rect.max.y < resp.rect.max.y
                                        {
                                            clipped.push((label, resp.rect, resp.interact_rect));
                                        }
                                    }
                                });
                            });
                        });
                    });
                });
            })
            .drop_without_applying_deltas();
        }
        assert!(clipped.is_empty(), "H-bar clipped buttons: {clipped:?}");
    }

    /// egui auto-shrinks the painted viewport — and the floating-bar anchor —
    /// to `content_size`, but `ScrollAreaOutput::inner_rect` is captured
    /// before that shrink. Claiming the pre-shrink rect would leave the bar
    /// floating left of the frame edge; `show_scroll_edge_fit` must claim the
    /// shrunk size instead.
    #[test]
    fn scroll_edge_fit_claims_shrunk_content_width() {
        let ctx = egui::Context::default();
        ctx.all_styles_mut(|style| style.spacing.scroll = crate::theme::scroll_style());
        let mut fit_w = f32::MAX;
        let mut edge_w = f32::MAX;
        for _ in 0..3 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(400.0, 600.0));
                let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                pane_ui.set_clip_rect(pane);
                egui::Frame::popup(pane_ui.style())
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(&mut pane_ui, |ui| {
                        ui.set_max_height(200.0);
                        show_scroll_edge_fit(ui, scroll_vertical().max_height(200.0), |ui| {
                            ui.label("narrow");
                            // Tall content so the v-bar paints.
                            ui.allocate_space(egui::vec2(80.0, 500.0));
                        });
                    });
                fit_w = pane_ui.min_rect().width();
            })
            .drop_without_applying_deltas();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(400.0, 600.0));
                let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                pane_ui.set_clip_rect(pane);
                egui::Frame::popup(pane_ui.style())
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(&mut pane_ui, |ui| {
                        ui.set_max_height(200.0);
                        show_scroll_edge(ui, scroll_vertical().max_height(200.0), |ui| {
                            ui.label("narrow");
                            ui.allocate_space(egui::vec2(80.0, 500.0));
                        });
                    });
                edge_w = pane_ui.min_rect().width();
            })
            .drop_without_applying_deltas();
        }
        // Fit hugs the shrunk viewport (~content + margins); the un-shrunk
        // claim keeps the full bound (~300 + margins), which is exactly the
        // dead space the bar used to float inside.
        assert!(fit_w < 200.0 && edge_w > 290.0, "fit={fit_w} edge={edge_w}");
    }

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

    /// Inside a framed container the scroll viewport reaches the frame's
    /// painted edge — per side, so asymmetric margins expand right/bottom by
    /// their own amounts — while `content_margin` keeps the content padded.
    #[test]
    fn scroll_viewport_expands_through_frame_margin() {
        let ctx = egui::Context::default();
        let margin = egui::Margin {
            left: 8,
            top: 2,
            right: 8,
            bottom: 2,
        };
        let mut viewport = egui::Rect::NOTHING;
        let mut content = egui::Rect::NOTHING;
        let mut frame_content = egui::Rect::NOTHING;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(300.0, 300.0));
            let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
            pane_ui.set_clip_rect(pane);
            egui::Frame::new()
                .inner_margin(margin)
                .show(&mut pane_ui, |ui| {
                    frame_content = ui.max_rect();
                    let out = show_scroll_edge(
                        ui,
                        egui::ScrollArea::vertical().auto_shrink([false, false]),
                        |ui| {
                            content = ui.allocate_space(egui::vec2(50.0, 500.0)).1;
                        },
                    );
                    viewport = out.inner_rect;
                });
        })
        .drop_without_applying_deltas();
        let eps = 0.5;
        assert!(
            (viewport.max.x - (frame_content.max.x + f32::from(margin.right))).abs() < eps,
            "right edge {viewport:?} should reach the painted frame edge {:?} + {margin:?}",
            frame_content,
        );
        assert!(
            (viewport.max.y - (frame_content.max.y + f32::from(margin.bottom))).abs() < eps,
            "bottom edge {viewport:?} should reach the painted frame edge",
        );
        assert!(
            content.max.x <= frame_content.max.x + eps,
            "content padding lost: {content:?} vs {frame_content:?}",
        );
    }

    /// A scroll deliberately narrower than the bound keeps its own edge —
    /// the cap binds before the reclaimed margin does.
    #[test]
    fn capped_scroll_keeps_its_own_edge() {
        let ctx = egui::Context::default();
        let mut viewport = egui::Rect::NOTHING;
        let mut frame_content = egui::Rect::NOTHING;
        let mut cursor_x = 0.0_f32;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(300.0, 300.0));
            let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
            pane_ui.set_clip_rect(pane);
            egui::Frame::new()
                .inner_margin(egui::Margin::same(8))
                .show(&mut pane_ui, |ui| {
                    frame_content = ui.max_rect();
                    cursor_x = ui.cursor().min.x;
                    let out = show_dialog_scroll(ui, 100.0, 100.0, |ui| {
                        ui.allocate_space(egui::vec2(50.0, 50.0));
                    });
                    viewport = out.inner_rect;
                });
        })
        .drop_without_applying_deltas();
        assert!(
            (viewport.max.x - (cursor_x + 100.0)).abs() < 1.0,
            "capped viewport {viewport:?} should end at its own cap, not the frame edge {frame_content:?}",
        );
    }

    /// The expanded viewport reaches into the frame's margin, but the claim
    /// reported to the parent stays inside the original bound — otherwise an
    /// auto-sized window grows every frame (the egui size ratchet).
    #[test]
    fn expanded_viewport_does_not_inflate_parent_size() {
        let ctx = egui::Context::default();
        let mut claimed = egui::Vec2::ZERO;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(300.0, 300.0));
            let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
            pane_ui.set_clip_rect(pane);
            let content = egui::Frame::new()
                .inner_margin(egui::Margin::same(8))
                .show(&mut pane_ui, |ui| {
                    show_scroll_edge(
                        ui,
                        egui::ScrollArea::vertical().auto_shrink([false, false]),
                        |ui| {
                            ui.allocate_space(egui::vec2(50.0, 500.0));
                        },
                    );
                    ui.min_rect().size()
                })
                .inner;
            let frame_w = pane_ui.min_rect().width();
            claimed = egui::vec2(content.x, frame_w);
        })
        .drop_without_applying_deltas();
        assert!(
            claimed.x <= 300.0 - 2.0 * 8.0 + 0.5,
            "reported content width {claimed:?} exceeded the frame's content lane",
        );
    }

    /// A scroll bound by a narrower in-between region (e.g. a column inside
    /// the frame) must not fling its bar across the gap into sibling space.
    #[test]
    fn column_bound_scroll_does_not_expand_past_a_gap() {
        let ctx = egui::Context::default();
        let mut viewport = egui::Rect::NOTHING;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(400.0, 300.0));
            let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
            pane_ui.set_clip_rect(pane);
            egui::Frame::new()
                .inner_margin(egui::Margin::same(8))
                .show(&mut pane_ui, |ui| {
                    // 40px gap to the frame edge, margin only 8: no expansion.
                    ui.set_max_width(200.0);
                    let avail_max = ui.available_rect_before_wrap().max;
                    let out = show_scroll_edge(
                        ui,
                        egui::ScrollArea::vertical().auto_shrink([false, false]),
                        |ui| {
                            ui.allocate_space(egui::vec2(50.0, 500.0));
                        },
                    );
                    viewport = out.inner_rect;
                    assert!(
                        (viewport.max.x - avail_max.x).abs() < 1.0,
                        "column-bound scroll expanded {viewport:?} past {avail_max:?}",
                    );
                });
        })
        .drop_without_applying_deltas();
    }
}
