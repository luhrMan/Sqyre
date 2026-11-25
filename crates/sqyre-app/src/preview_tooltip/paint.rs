//! Preview painting: hover tips, panel images, status placeholders, markers.

use super::geometry::{desktop_outline_rect, fit_display, panel_viewport_size, PreviewCoords};
use super::{PreviewKind, PreviewTooltipCache, CAPTURE_PENDING, DISPLAY_MAX_H, DISPLAY_MAX_W};
use crate::image_view::{self, ImageViewTransform};
use eframe::egui::{self, TextureHandle, Vec2};

/// Point-marker ring radius in capture pixels (the raster ring sat near r = 7–8).
const POINT_RING_RADIUS: f32 = 7.5;
/// Point-marker crosshair half-length in capture pixels.
const POINT_ARM: f32 = 15.0;
/// Crosshair thickness in capture pixels.
const POINT_STROKE_PX: f32 = 2.0;
/// Search-area outline and grid thickness in capture pixels.
const AREA_STROKE_PX: f32 = 1.0;
/// Screen-pixel floor so a fitted preview still shows a blinking mark.
const MARKER_MIN_STROKE: f32 = 1.5;

impl PreviewTooltipCache {
    pub(super) fn paint_no_saved_screen_inline(&mut self, ui: &mut egui::Ui, err: &str) {
        #[cfg(target_os = "android")]
        {
            let _ = err;
            self.paint_app_shot_inline(ui);
        }
        #[cfg(not(target_os = "android"))]
        crate::widgets::error_icon(ui, err);
    }

    pub(super) fn paint_no_saved_screen_panel(
        &mut self,
        ui: &mut egui::Ui,
        err: &str,
        view: &mut ImageViewTransform,
    ) -> egui::Rect {
        #[cfg(target_os = "android")]
        {
            let _ = err;
            self.paint_app_shot_panel(ui, view)
        }
        #[cfg(not(target_os = "android"))]
        paint_preview_panel_placeholder(ui, err, view)
    }

    #[cfg(target_os = "android")]
    fn paint_app_shot_inline(&mut self, ui: &mut egui::Ui) {
        self.prepare_app_shot(ui.ctx());
        self.paint_app_shot_status(ui);
        self.start_app_shot_if_chosen(ui, "inline_app_shot", None);
    }

    #[cfg(target_os = "android")]
    fn paint_app_shot_panel(
        &mut self,
        ui: &mut egui::Ui,
        _view: &mut ImageViewTransform,
    ) -> egui::Rect {
        let desired = panel_viewport_size(ui);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 4.0, crate::theme::preview_status_fill());
        let body = rect.shrink(12.0);
        let combo_w = (body.width() - 8.0).clamp(120.0, 360.0);
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            ui.set_max_width(combo_w);
            ui.add_space((body.height() * 0.3).min(72.0));
            if self.shot.status.is_empty() && self.shot.capture.is_none() {
                ui.label(crate::app_shot::CHOOSE_HEADING);
            } else {
                self.paint_app_shot_status(ui);
            }
            self.prepare_app_shot(ui.ctx());
            self.start_app_shot_if_chosen(ui, "panel_app_shot", Some(combo_w));
        });
        crate::data_editor_preview::paint_preview_frame(ui.painter(), rect);
        rect
    }

    #[cfg(target_os = "android")]
    fn prepare_app_shot(&mut self, ctx: &egui::Context) {
        self.shot.list.ensure_loading(ctx);
        self.shot.list.poll(ctx);
    }

    #[cfg(target_os = "android")]
    fn paint_app_shot_status(&self, ui: &mut egui::Ui) {
        if self.shot.capture.is_some() {
            ui.label(self.shot.status.as_str());
        } else if !self.shot.status.is_empty() {
            ui.colored_label(crate::theme::error_fg(), self.shot.status.as_str());
        }
    }

    #[cfg(target_os = "android")]
    fn start_app_shot_if_chosen(&mut self, ui: &mut egui::Ui, id_salt: &str, width: Option<f32>) {
        let Some(app) = self.shot.list.show(ui, id_salt, width) else {
            return;
        };
        if self.shot.capture.is_some() {
            return;
        }
        self.shot.status = crate::app_shot::taking_picture_of(app.shown());
        self.shot.capture = Some(crate::screen_pick::spawn_app_shot(
            ui.ctx(),
            app.package,
            app.label,
        ));
    }
}

/// Android: app dropdown and the in-flight screenshot it starts.
#[cfg(target_os = "android")]
#[derive(Default)]
pub(super) struct PreviewShot {
    pub(super) list: crate::app_shot::AppList,
    pub(super) capture: Option<crate::screen_pick::ShotRx>,
    pub(super) status: String,
}

/// Location mark in pre-downscale capture pixels. Painted over the image so
/// it can blink; the cached texture stays unmarked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PreviewMarker {
    Point {
        x: f32,
        y: f32,
        src_w: u32,
        src_h: u32,
    },
    Area {
        /// Exclusive rect. The outline is stroked just outside it.
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        grid: Option<(i32, i32)>,
        src_w: u32,
        src_h: u32,
    },
}

/// List/combo hover tips open to the **right** of the row so a tall collection
/// preview cannot cover the pointer and start an open/close flash loop.
pub(super) fn show_preview_hover_ui(
    response: &egui::Response,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let mut tip = egui::Tooltip::for_enabled(response);
    tip.popup = tip
        .popup
        .align(egui::RectAlign::RIGHT)
        .align_alternatives(&[
            egui::RectAlign::RIGHT_START,
            egui::RectAlign::RIGHT_END,
            egui::RectAlign::LEFT,
            egui::RectAlign::BOTTOM_START,
        ])
        .gap(10.0);
    tip.show(add_contents);
}

pub(super) fn paint_preview(
    ui: &mut egui::Ui,
    tex: &TextureHandle,
    caption: &str,
    show_image_size: bool,
    marker: PreviewMarker,
) {
    let [tw, th] = tex.size();
    let size = fit_display(ui, tw as f32, th as f32);
    // Click sense makes the tip "interactive" so egui keeps it open while the
    // pointer is over the image — needed when a tall tip still overlaps the row.
    let image = image_view::add_checkered_image(
        ui,
        tex,
        size,
        egui::Sense::click(),
        0.0,
        image_view::CheckerboardEdge::Outset,
    );
    paint_blinking_marker(&ui.painter_at(image.rect), image.rect, marker);
    ui.add(egui::Label::new(caption).wrap());
    if show_image_size {
        ui.label(
            egui::RichText::new(format!(
                "Preview {}",
                crate::data_editor_preview::pixel_size_text(tw as i32, th as i32)
            ))
            .weak()
            .small(),
        );
    }
}

/// Status line while capture is pending/failed. Reserves the display footprint so
/// the tip does not grow under the cursor when the texture arrives.
///
/// This lives inside a hover tip — a non-interactive surface where a nested
/// hover tooltip can never fire — so the error stays as centered visible text
/// rather than collapsing to an icon like interactive surfaces do.
pub(super) fn paint_preview_status(ui: &mut egui::Ui, caption: &str, err: &str) {
    paint_preview_status_waking(ui, caption, err, err == CAPTURE_PENDING);
}

fn paint_preview_status_waking(ui: &mut egui::Ui, caption: &str, err: &str, waking: bool) {
    let box_size = image_view::fit_in_box_no_upscale(
        DISPLAY_MAX_W,
        DISPLAY_MAX_H,
        crate::widgets::visible_content_width(ui),
        DISPLAY_MAX_H,
    );
    let (rect, _) = ui.allocate_exact_size(box_size, egui::Sense::click());
    ui.painter()
        .rect_filled(rect, 4.0, crate::theme::preview_status_fill());
    if waking {
        paint_wake_spinner(ui, rect);
    } else {
        paint_preview_message(ui, rect, err, crate::theme::error_fg(), 0.0);
    }
    ui.add(egui::Label::new(caption).wrap());
}

fn paint_wake_spinner(ui: &mut egui::Ui, rect: egui::Rect) {
    let side = ui.spacing().interact_size.y;
    let at = egui::Rect::from_center_size(rect.center(), egui::vec2(side, side));
    let resp = ui.interact(
        at,
        ui.id()
            .with(("pw_wake", at.left().to_bits(), at.top().to_bits())),
        egui::Sense::hover(),
    );
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::ProgressIndicator,
            true,
            crate::help_text::CAPTURE_WAKE,
        )
    });
    resp.on_hover_text(crate::help_text::CAPTURE_WAKE);
    if ui.is_rect_visible(at) {
        let glyph = egui::Rect::from_center_size(at.center(), at.size() * 0.92);
        sqyre_overlay::icons::paint_trace_spinner(ui, glyph, sqyre_ui_theme::accent());
        ui.ctx()
            .request_repaint_after(sqyre_overlay::icons::SPINNER_TICK);
    }
}

/// Centered status/error text inside a preview placeholder, word-wrapped to
/// `rect` minus `side_inset` per side (never narrower than a readable minimum).
fn paint_preview_message(
    ui: &egui::Ui,
    rect: egui::Rect,
    text: &str,
    color: egui::Color32,
    side_inset: f32,
) {
    const MIN_WRAP_W: f32 = 120.0;
    const EDGE_PAD: f32 = 8.0;
    let max_w = (rect.width() - EDGE_PAD * 2.0).max(1.0);
    let wrap_w = (rect.width() - side_inset * 2.0).max(MIN_WRAP_W).min(max_w);
    let mut job = egui::text::LayoutJob::simple(
        text.to_owned(),
        egui::TextStyle::Small.resolve(ui.style()),
        color,
        wrap_w,
    );
    job.halign = egui::Align::Center;
    let painter = ui.painter_at(rect);
    let galley = painter.layout_job(job);
    let pos = egui::pos2(rect.center().x, rect.center().y - galley.size().y * 0.5);
    painter.galley(pos, galley, color);
}

pub(super) fn log_hover_outline_transition(
    kind: PreviewKind,
    program: &str,
    name: &str,
    coords: PreviewCoords,
) {
    #[cfg(feature = "native-runtime")]
    {
        use std::sync::Mutex;
        type LastOutline = (String, (i32, i32, i32, i32));
        static LAST: Mutex<Option<LastOutline>> = Mutex::new(None);
        let rect = desktop_outline_rect(coords);
        let key = format!("{kind:?}:{program}~{name}");
        let mut guard = LAST.lock().unwrap_or_else(|e| e.into_inner());
        if guard.as_ref() == Some(&(key.clone(), rect)) {
            return;
        }
        *guard = Some((key.clone(), rect));
        sqyre_ports::event_log(
            "SQYRE_PREVIEW",
            &[
                ("op", "hover_outline"),
                ("entity", &key),
                (
                    "rect",
                    &format!("{},{}-{},{}", rect.0, rect.1, rect.2, rect.3),
                ),
            ],
        );
    }
    #[cfg(not(feature = "native-runtime"))]
    {
        let _ = (kind, program, name, coords);
    }
}

pub(super) fn paint_preview_panel_image(
    ui: &mut egui::Ui,
    tex: &TextureHandle,
    view: &mut ImageViewTransform,
    marker: PreviewMarker,
) -> egui::Rect {
    let [tw, th] = tex.size();
    let image_size = Vec2::new(tw as f32, th as f32);
    let desired = panel_viewport_size(ui);
    let (viewport, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
    image_view::handle_scroll_zoom(ui, viewport, image_size, view, resp.hovered());
    let content = image_view::image_content_rect(viewport, image_size, view.zoom, view.pan);
    {
        let painter = ui.painter_at(viewport);
        painter.rect_filled(viewport, 0.0, crate::theme::preview_viewport_fill());
        image_view::paint_checkerboard(
            &painter,
            content,
            0.0,
            image_view::CheckerboardEdge::Outset,
        );
        painter.image(
            tex.id(),
            content,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        paint_blinking_marker(&painter, content, marker);
        crate::data_editor_preview::paint_preview_frame(&painter, viewport);
    }
    let _ = image_view::handle_pan_drag(&resp, viewport, image_size, view);
    viewport
}

pub(super) fn paint_preview_panel_placeholder(
    ui: &mut egui::Ui,
    err: &str,
    _view: &mut ImageViewTransform,
) -> egui::Rect {
    let desired = panel_viewport_size(ui);
    // Hover-only on the panel so a drag still scrolls the form. The icon in
    // the middle is what opens the error (see `attach_preview_error_icon`).
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 4.0, crate::theme::preview_status_fill());
    if err == CAPTURE_PENDING {
        paint_wake_spinner(ui, rect);
    } else {
        crate::data_editor_preview::attach_preview_error_icon(ui, rect, err);
    }
    crate::data_editor_preview::paint_preview_frame(ui.painter(), rect);
    rect
}

/// Paint `marker` over `content` (the image rect, including zoom/pan).
///
/// The mark is not in the texture: it toggles on and off like a text caret so
/// it stays visible on a busy capture. `painter` should already be clipped to
/// the viewport.
fn paint_blinking_marker(painter: &egui::Painter, content: egui::Rect, marker: PreviewMarker) {
    if !content.is_positive() || !marker_in_frame(marker) {
        return;
    }
    if !crate::image_view::preview_marker_blink_on(painter.ctx()) {
        return;
    }
    let color = crate::theme::preview_grid_stroke();
    let (sx, sy) = marker_scale(content, marker);
    let scale = (sx + sy) * 0.5;
    let stroke_px = match marker {
        PreviewMarker::Point { .. } => POINT_STROKE_PX,
        PreviewMarker::Area { .. } => AREA_STROKE_PX,
    };
    let stroke = egui::Stroke::new((stroke_px * scale).max(MARKER_MIN_STROKE), color);
    match marker {
        PreviewMarker::Point { x, y, .. } => {
            let c = egui::pos2(content.left() + x * sx, content.top() + y * sy);
            painter.circle_stroke(c, POINT_RING_RADIUS * scale, stroke);
            painter.line_segment(
                [
                    egui::pos2(c.x - POINT_ARM * sx, c.y),
                    egui::pos2(c.x + POINT_ARM * sx, c.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x, c.y - POINT_ARM * sy),
                    egui::pos2(c.x, c.y + POINT_ARM * sy),
                ],
                stroke,
            );
        }
        PreviewMarker::Area {
            x0,
            y0,
            x1,
            y1,
            grid,
            ..
        } => {
            let rect = egui::Rect::from_min_max(
                egui::pos2(content.left() + x0 * sx, content.top() + y0 * sy),
                egui::pos2(content.left() + x1 * sx, content.top() + y1 * sy),
            );
            if !rect.is_positive() {
                return;
            }
            // Outside the exclusive region so searched pixels stay unpainted.
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
            if let Some((rows, cols)) = grid {
                let rows = rows.max(1);
                let cols = cols.max(1);
                for i in 1..rows {
                    let y = rect.top() + rect.height() * (i as f32) / (rows as f32);
                    painter.hline(rect.x_range(), y, stroke);
                }
                for i in 1..cols {
                    let x = rect.left() + rect.width() * (i as f32) / (cols as f32);
                    painter.vline(x, rect.y_range(), stroke);
                }
            }
        }
    }
}

fn marker_in_frame(marker: PreviewMarker) -> bool {
    match marker {
        PreviewMarker::Point { src_w, src_h, .. } => src_w > 0 && src_h > 0,
        PreviewMarker::Area {
            x0,
            y0,
            x1,
            y1,
            src_w,
            src_h,
            ..
        } => src_w > 0 && src_h > 0 && x1 > x0 && y1 > y0,
    }
}

fn marker_scale(content: egui::Rect, marker: PreviewMarker) -> (f32, f32) {
    let (src_w, src_h) = match marker {
        PreviewMarker::Point { src_w, src_h, .. } | PreviewMarker::Area { src_w, src_h, .. } => {
            (src_w, src_h)
        }
    };
    (
        content.width() / src_w.max(1) as f32,
        content.height() / src_h.max(1) as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_tooltip::LITERAL_COORDS_MSG;
    use eframe::egui::{ColorImage, TextureOptions};

    fn phone_screen() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(360.0, 640.0))
    }

    #[test]
    fn pending_capture_shows_a_spinner_while_the_stream_wakes() {
        use egui_kittest::kittest::Queryable;
        let mut h = egui_kittest::Harness::builder()
            .with_size([320.0, 240.0])
            .build_ui(|ui| {
                paint_preview_status_waking(ui, "point", CAPTURE_PENDING, true);
            });
        h.run_steps(1);
        h.get_by_label(crate::help_text::CAPTURE_WAKE);
    }

    fn input(screen: egui::Rect, time: f64, events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            predicted_dt: 0.1,
            events,
            ..Default::default()
        }
    }

    fn text_on_screen(
        shapes: &[egui::epaint::ClippedShape],
        screen: egui::Rect,
        needle: &str,
    ) -> bool {
        fn walk(shape: &egui::Shape, clip: egui::Rect, screen: egui::Rect, needle: &str) -> bool {
            match shape {
                egui::Shape::Text(text) if text.galley.text().contains(needle) => {
                    let painted =
                        egui::Rect::from_min_size(text.pos, text.galley.size()).intersect(clip);
                    let visible = painted.intersect(screen);
                    visible.width() > 8.0 && visible.height() > 8.0
                }
                egui::Shape::Vec(shapes) => shapes.iter().any(|s| walk(s, clip, screen, needle)),
                _ => false,
            }
        }
        shapes
            .iter()
            .any(|clipped| walk(&clipped.shape, clipped.clip_rect, screen, needle))
    }

    /// The preview fills a phone, so a tip hung off the panel itself sits under
    /// the screen. Hovering the icon must paint the message inside the screen.
    #[test]
    fn preview_error_is_on_screen_when_the_icon_is_hovered() {
        let ctx = egui::Context::default();
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = 0.0;
            style.interaction.show_tooltips_only_when_still = false;
        });
        let screen = phone_screen();
        let mut cache = PreviewTooltipCache::new();
        let mut view = ImageViewTransform::default();
        let mut center = egui::Pos2::ZERO;
        ctx.run_ui(input(screen, 0.0, vec![]), |ui| {
            center = cache
                .paint_point_panel(ui, None, None, false, &mut view)
                .center();
        })
        .drop_without_applying_deltas();

        let mut shown = false;
        for frame in 1..6 {
            let events = if frame == 1 {
                vec![egui::Event::PointerMoved(center)]
            } else {
                Vec::new()
            };
            let out = ctx.run_ui(input(screen, frame as f64 * 0.2, events), |ui| {
                let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
            });
            shown = text_on_screen(&out.shapes, screen, LITERAL_COORDS_MSG);
            out.drop_without_applying_deltas();
        }
        assert!(
            shown,
            "hovering the preview error icon did not paint the message on screen"
        );
    }

    /// Self-sizing tip Areas must not shrink wrapped text to one-character rows
    /// across frames (same ratchet class as item-icon tooltips).
    #[test]
    fn preview_error_tip_width_does_not_ratchet() {
        let ctx = egui::Context::default();
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = 0.0;
            style.interaction.show_tooltips_only_when_still = false;
        });
        let screen = phone_screen();
        let mut cache = PreviewTooltipCache::new();
        let mut view = ImageViewTransform::default();
        let mut center = egui::Pos2::ZERO;
        ctx.run_ui(input(screen, 0.0, vec![]), |ui| {
            center = cache
                .paint_point_panel(ui, None, None, false, &mut view)
                .center();
        })
        .drop_without_applying_deltas();

        let mut first_w = None;
        let mut last_w = None;
        for frame in 1..12 {
            let events = if frame == 1 {
                vec![egui::Event::PointerMoved(center)]
            } else {
                Vec::new()
            };
            let out = ctx.run_ui(input(screen, frame as f64 * 0.2, events), |ui| {
                let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
            });
            let tip_w = out.shapes.iter().find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.text().contains(LITERAL_COORDS_MSG) => {
                    Some(t.galley.size().x)
                }
                _ => None,
            });
            if let Some(w) = tip_w {
                first_w.get_or_insert(w);
                last_w = Some(w);
            }
            out.drop_without_applying_deltas();
        }
        let (first, last) = (
            first_w.expect("preview tip text never painted"),
            last_w.expect("preview tip text vanished"),
        );
        assert!(
            last + 1.0 >= first,
            "preview tip width ratcheted down: first={first:.1} last={last:.1}"
        );
    }

    /// A tap is not a hover: the message has to show while the finger is down
    /// and stay after it lifts, without waiting out the hover delay.
    #[test]
    fn preview_error_stays_on_screen_after_tap() {
        let ctx = egui::Context::default();
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = 30.0;
        });
        let screen = phone_screen();
        let mut cache = PreviewTooltipCache::new();
        let mut view = ImageViewTransform::default();
        let mut center = egui::Pos2::ZERO;
        ctx.run_ui(input(screen, 0.0, vec![]), |ui| {
            center = cache
                .paint_point_panel(ui, None, None, false, &mut view)
                .center();
        })
        .drop_without_applying_deltas();

        let press = pointer_button(center, true);
        ctx.run_ui(
            input(screen, 0.2, vec![egui::Event::PointerMoved(center), press]),
            |ui| {
                let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
            },
        )
        .drop_without_applying_deltas();
        // The opening frame is a sizing pass. The next frame, still held, paints.
        let held = ctx.run_ui(input(screen, 0.4, vec![]), |ui| {
            let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
        });
        let held_ok = text_on_screen(&held.shapes, screen, LITERAL_COORDS_MSG);
        held.drop_without_applying_deltas();
        assert!(
            held_ok,
            "holding the preview error icon did not paint the message"
        );

        let release = ctx.run_ui(
            input(
                screen,
                0.6,
                vec![pointer_button(center, false), egui::Event::PointerGone],
            ),
            |ui| {
                let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
            },
        );
        let release_ok = text_on_screen(&release.shapes, screen, LITERAL_COORDS_MSG);
        release.drop_without_applying_deltas();
        assert!(
            release_ok,
            "lifting off the preview error icon hid the message"
        );

        let stay = ctx.run_ui(input(screen, 0.8, vec![]), |ui| {
            let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
        });
        let stay_ok = text_on_screen(&stay.shapes, screen, LITERAL_COORDS_MSG);
        stay.drop_without_applying_deltas();
        assert!(stay_ok, "the preview error disappeared after the tap");
    }

    fn pointer_button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Tip bodies live in vertical-only scroll areas that never clip the X axis.
    /// Fixed-size preview content must shrink to the host bound instead of
    /// expanding the section frame — and the tip — past its right edge.
    #[test]
    fn preview_content_stays_within_narrow_tip_bound() {
        let ctx = egui::Context::default();
        let tex = ctx.load_texture(
            "narrow-bound",
            ColorImage::new([640, 360], vec![egui::Color32::RED; 640 * 360]),
            TextureOptions::LINEAR,
        );
        // FLOATER_MIN_COMPACT is the narrowest tip bound a user can reach.
        let bound = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(220.0, 600.0));
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.scope_builder(egui::UiBuilder::new().max_rect(bound), |ui| {
                crate::theme::framed_section(ui, |ui| {
                    paint_preview(
                        ui,
                        &tex,
                        "X: 1, Y: 2",
                        true,
                        PreviewMarker::Point {
                            x: 320.0,
                            y: 180.0,
                            src_w: 640,
                            src_h: 360,
                        },
                    );
                });
                crate::theme::framed_section(ui, |ui| {
                    paint_preview_status(ui, "cap", "failed");
                });
            });
        });
        for clipped in &out.shapes {
            let right = match &clipped.shape {
                egui::Shape::Mesh(mesh) => mesh
                    .vertices
                    .iter()
                    .map(|v| v.pos.x)
                    .fold(f32::NEG_INFINITY, f32::max),
                egui::Shape::Rect(rect) => rect.rect.right(),
                _ => continue,
            };
            assert!(
                right <= bound.right() + 1.0,
                "tooltip content paints past the tip bound: {right} > {}",
                bound.right()
            );
        }
        out.drop_without_applying_deltas();
    }

    #[test]
    fn search_area_outline_sits_outside_and_blinks_off() {
        let marker = PreviewMarker::Area {
            x0: 10.0,
            y0: 20.0,
            x1: 40.0,
            y1: 50.0,
            grid: Some((2, 2)),
            src_w: 100,
            src_h: 100,
        };
        let shapes = paint_marker_at(0.0, marker);
        let color = crate::theme::preview_grid_stroke();
        let outline = shapes.iter().find_map(|c| match &c.shape {
            egui::Shape::Rect(r) if r.stroke.color == color => Some(r),
            _ => None,
        });
        let outline = outline.expect("search-area outline");
        assert_eq!(outline.stroke_kind, egui::StrokeKind::Outside);
        let expected = egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(40.0, 50.0));
        assert!((outline.rect.min.x - expected.min.x).abs() < 0.01);
        assert!((outline.rect.min.y - expected.min.y).abs() < 0.01);
        assert!((outline.rect.max.x - expected.max.x).abs() < 0.01);
        assert!((outline.rect.max.y - expected.max.y).abs() < 0.01);
        // One outline plus the row and column lines.
        assert_eq!(marker_ink(&shapes, color), 3);
        let hidden = paint_marker_at(0.6, marker);
        assert_eq!(marker_ink(&hidden, color), 0);
    }

    #[test]
    fn point_marker_sits_on_the_point_and_blinks_off() {
        let marker = PreviewMarker::Point {
            x: 40.0,
            y: 50.0,
            src_w: 100,
            src_h: 100,
        };
        let content = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 100.0));
        let shapes = paint_marker_in(0.0, content, marker);
        let color = crate::theme::preview_grid_stroke();
        let circle = shapes.iter().find_map(|c| match &c.shape {
            egui::Shape::Circle(circle) if circle.stroke.color == color => Some(circle),
            _ => None,
        });
        let circle = circle.expect("point ring");
        assert!((circle.center.x - 50.0).abs() < 0.01, "{:?}", circle.center);
        assert!((circle.center.y - 70.0).abs() < 0.01, "{:?}", circle.center);
        assert!((circle.radius - POINT_RING_RADIUS).abs() < 0.01);
        // Ring plus the two crosshair arms.
        assert_eq!(marker_ink(&shapes, color), 3);
        let hidden = paint_marker_in(0.6, content, marker);
        assert_eq!(marker_ink(&hidden, color), 0);
    }

    fn paint_marker_at(time: f64, marker: PreviewMarker) -> Vec<egui::epaint::ClippedShape> {
        let content = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 100.0));
        paint_marker_in(time, content, marker)
    }

    fn paint_marker_in(
        time: f64,
        content: egui::Rect,
        marker: PreviewMarker,
    ) -> Vec<egui::epaint::ClippedShape> {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(240.0, 200.0));
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                paint_blinking_marker(&ui.painter_at(screen), content, marker);
            },
        );
        let shapes = out.shapes.clone();
        out.drop_without_applying_deltas();
        shapes
    }

    fn marker_ink(shapes: &[egui::epaint::ClippedShape], color: egui::Color32) -> usize {
        fn walk(shape: &egui::Shape, color: egui::Color32) -> usize {
            match shape {
                egui::Shape::Vec(v) => v.iter().map(|s| walk(s, color)).sum(),
                egui::Shape::LineSegment { stroke, .. } if stroke.color == color => 1,
                egui::Shape::Rect(r) if r.stroke.color == color && r.stroke.width > 0.0 => 1,
                egui::Shape::Circle(c) if c.stroke.color == color && c.stroke.width > 0.0 => 1,
                _ => 0,
            }
        }
        shapes.iter().map(|c| walk(&c.shape, color)).sum()
    }
}
