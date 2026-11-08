//! One framed control: an icon (or two) joined to the widget it labels or acts on.
//!
//! Resting icon buttons are bare glyphs. A loose glyph beside a field does not
//! read as part of that field, so [`pair_icon`] puts them in a single outline
//! with a hairline between the slots.

use eframe::egui::{self, Color32, Stroke};

use super::wrap::wrap_unit;

/// Space between the icon slot and the widget inside [`pair_icon`].
pub const PAIR_SLOT_GAP: f32 = crate::theme::SPACE_4;

/// Outline width reserved before layout. Hover only changes the color —
/// a wider stroke after [`egui::Frame::begin`] makes the row a couple of
/// pixels too wide, and a fill-width search field then flips in and out of
/// the overflow scroller every frame.
const PAIR_STROKE_W: f32 = 1.0;

/// Field icon and the control it names, in one [`pair_leading`] frame.
pub fn pair_labeled<R>(
    ui: &mut egui::Ui,
    icon: &str,
    name: &str,
    widget: impl FnMut(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    pair_leading(
        ui,
        |ui| {
            crate::action_tooltip::help::label(ui, icon, name);
        },
        widget,
    )
}

/// Icon on the left, widget on the right, one shared frame.
pub fn pair_leading<R>(
    ui: &mut egui::Ui,
    mut leading: impl FnMut(&mut egui::Ui),
    mut widget: impl FnMut(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    wrap_unit(ui, |ui| {
        let mut trailing: [&mut dyn FnMut(&mut egui::Ui); 0] = [];
        paint_pair(ui, Some(&mut leading), &mut widget, &mut trailing)
    })
}

/// Leading icon, widget, and a trailing icon (clear, pick, record) in one frame.
///
/// A single rounded frame (text-edit fill, widget stroke) wraps every slot.
/// A hairline sits in the gap between slots. Inner widgets keep their text and
/// focus stroke, but their own fill and border are suppressed so the pair is
/// one outline. The pair is a [`wrap_unit`]: a wrapping row never splits the
/// icon from its widget. Use [`pair_leading`] when there is no trailing icon.
pub fn pair_icon<R>(
    ui: &mut egui::Ui,
    mut leading: impl FnMut(&mut egui::Ui),
    mut widget: impl FnMut(&mut egui::Ui) -> R,
    mut trailing: impl FnMut(&mut egui::Ui),
) -> egui::InnerResponse<R> {
    wrap_unit(ui, |ui| {
        let mut slots: [&mut dyn FnMut(&mut egui::Ui); 1] = [&mut trailing];
        paint_pair(ui, Some(&mut leading), &mut widget, &mut slots)
    })
}

/// Leading icon, widget, and any number of trailing icon slots in one frame.
///
/// Each trailing closure is its own slot: a hairline sits in the gap before it,
/// same as [`pair_icon`]'s single trailing icon.
pub(crate) fn pair_slots<R>(
    ui: &mut egui::Ui,
    mut leading: impl FnMut(&mut egui::Ui),
    mut widget: impl FnMut(&mut egui::Ui) -> R,
    trailing: &mut [&mut dyn FnMut(&mut egui::Ui)],
) -> egui::InnerResponse<R> {
    wrap_unit(ui, |ui| {
        paint_pair(ui, Some(&mut leading), &mut widget, trailing)
    })
}

fn paint_pair<R>(
    ui: &mut egui::Ui,
    mut leading: Option<&mut dyn FnMut(&mut egui::Ui)>,
    widget: &mut dyn FnMut(&mut egui::Ui) -> R,
    trailing: &mut [&mut dyn FnMut(&mut egui::Ui)],
) -> R {
    let mut prepared = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(4, 2))
        .stroke(Stroke::new(PAIR_STROKE_W, Color32::TRANSPARENT))
        .begin(ui);
    strip_inner_chrome(&mut prepared.content_ui);

    let mut lead_rect = None;
    let mut widget_rect = egui::Rect::NOTHING;
    let mut trail_rects = Vec::with_capacity(trailing.len());
    prepared.content_ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
    let result = prepared
        .content_ui
        .horizontal(|ui| {
            if let Some(lead) = leading.as_mut() {
                lead_rect = Some(ui.scope(|ui| lead(ui)).response.rect);
                ui.add_space(PAIR_SLOT_GAP);
            }
            let scoped = ui.scope(|ui| widget(ui));
            widget_rect = scoped.response.rect;
            let result = scoped.inner;
            for slot in trailing.iter_mut() {
                ui.add_space(PAIR_SLOT_GAP);
                trail_rects.push(ui.scope(|ui| slot(ui)).response.rect);
            }
            result
        })
        .inner;

    let content_rect = prepared.content_ui.min_rect();
    let frame_rect = prepared.frame.widget_rect(content_rect);
    let enabled = ui.is_enabled();
    let hovered = enabled && ui.rect_contains_pointer(frame_rect);
    let visuals = if !enabled {
        &ui.visuals().widgets.noninteractive
    } else if hovered {
        &ui.visuals().widgets.hovered
    } else {
        &ui.visuals().widgets.inactive
    };
    let stroke = Stroke::new(PAIR_STROKE_W, visuals.bg_stroke.color);
    paint_dividers(ui, lead_rect, widget_rect, &trail_rects, stroke);
    prepared.frame = prepared
        .frame
        .corner_radius(visuals.corner_radius)
        .fill(ui.visuals().text_edit_bg_color())
        .stroke(stroke);
    prepared.end(ui);
    result
}

/// Drop the inner control's box so the pair frame is the only outline.
fn strip_inner_chrome(ui: &mut egui::Ui) {
    let visuals = ui.visuals_mut();
    visuals.text_edit_bg_color = Some(Color32::TRANSPARENT);
    for slot in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        slot.bg_fill = Color32::TRANSPARENT;
        slot.weak_bg_fill = Color32::TRANSPARENT;
        slot.bg_stroke = Stroke::NONE;
    }
}

fn paint_dividers(
    ui: &egui::Ui,
    lead: Option<egui::Rect>,
    widget: egui::Rect,
    trail: &[egui::Rect],
    stroke: Stroke,
) {
    let mut stroke = stroke;
    if stroke.width < 0.5 {
        stroke.width = 1.0;
    }
    if let Some(lead) = lead {
        vline_between(ui, lead, widget, stroke);
    }
    let mut prev = widget;
    for slot in trail {
        vline_between(ui, prev, *slot, stroke);
        prev = *slot;
    }
}

fn vline_between(ui: &egui::Ui, left: egui::Rect, right: egui::Rect, stroke: Stroke) {
    let band = left.union(right);
    let inset = 3.0;
    if band.height() <= inset * 2.0 {
        return;
    }
    let x = (left.right() + right.left()) * 0.5;
    ui.painter()
        .vline(x, (band.top() + inset)..=(band.bottom() - inset), stroke);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(
        ctx: &egui::Context,
        input: egui::RawInput,
        add: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput {
        ctx.run_ui(input, add)
    }

    fn filled_rects(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::epaint::RectShape> {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect) if rect.fill.a() > 0 => Some(rect.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn pair_frame_covers_icon_and_widget() {
        let ctx = egui::Context::default();
        let mut icon_rect = egui::Rect::NOTHING;
        let mut edit_rect = egui::Rect::NOTHING;
        let mut value = String::from("name");
        let out = run(&ctx, egui::RawInput::default(), |ui| {
            pair_leading(
                ui,
                |ui| {
                    icon_rect = ui.label("⏱").rect;
                },
                |ui| {
                    edit_rect = ui
                        .add(egui::TextEdit::singleline(&mut value).desired_width(120.0))
                        .rect;
                },
            );
        });
        let frame = filled_rects(&out.shapes)
            .into_iter()
            .find(|rect| {
                rect.rect.contains(icon_rect.center()) && rect.rect.contains(edit_rect.center())
            })
            .expect("pair frame");
        assert!(
            frame.rect.width() > icon_rect.width() + edit_rect.width() * 0.5,
            "frame should span both slots: {:?}",
            frame.rect
        );
        let divider = out.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, .. } => {
                let x = points[0].x;
                (points[0].x - points[1].x).abs() < 0.5
                    && x > icon_rect.right() - 1.0
                    && x < edit_rect.left() + 1.0
            }
            _ => false,
        });
        assert!(divider, "hairline between icon and widget");
        out.drop_without_applying_deltas();
    }

    #[test]
    fn paired_icon_widgets_in_a_row_share_a_baseline() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 200.0));
        for pass in 0..2 {
            let mut name = "Alpha".to_owned();
            let mut tags = "demo".to_owned();
            let out = run(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            pair_labeled(ui, "T", "Name", |ui| {
                                ui.add(egui::TextEdit::singleline(&mut name).desired_width(160.0));
                            });
                            pair_labeled(ui, "#", "Tags", |ui| {
                                ui.add(egui::TextEdit::singleline(&mut tags).desired_width(140.0));
                            });
                        });
                    });
                },
            );
            let mut frames: Vec<egui::Rect> = filled_rects(&out.shapes)
                .into_iter()
                .filter(|rect| rect.fill.a() > 200 && rect.rect.width() > 80.0)
                .map(|rect| rect.rect)
                .collect();
            out.drop_without_applying_deltas();
            frames.sort_by(|a, b| a.left().total_cmp(&b.left()));
            assert_eq!(frames.len(), 2, "expected one frame per pair: {frames:?}");
            assert!(
                (frames[0].top() - frames[1].top()).abs() < 1.0,
                "pair frames stepped on pass {pass}: {frames:?}"
            );
        }
    }

    #[test]
    fn pair_hover_reaches_the_inner_widget() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 200.0));
        let mut edit_rect = egui::Rect::NOTHING;
        let mut value = String::new();
        let first = run(
            &ctx,
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| {
                pair_leading(
                    ui,
                    |ui| {
                        ui.label("⏱");
                    },
                    |ui| {
                        edit_rect = ui
                            .add(egui::TextEdit::singleline(&mut value).desired_width(120.0))
                            .rect;
                    },
                );
            },
        );
        first.drop_without_applying_deltas();

        let mut hovered = false;
        let second = run(
            &ctx,
            egui::RawInput {
                screen_rect: Some(screen),
                events: vec![egui::Event::PointerMoved(edit_rect.center())],
                ..Default::default()
            },
            |ui| {
                pair_leading(
                    ui,
                    |ui| {
                        ui.label("⏱");
                    },
                    |ui| {
                        hovered = ui
                            .add(egui::TextEdit::singleline(&mut value).desired_width(120.0))
                            .hovered();
                    },
                );
            },
        );
        assert!(hovered, "pair frame must not eat the field's hover");
        let hovered_stroke = ctx.global_style().visuals.widgets.hovered.bg_stroke.color;
        let frame_hovered = filled_rects(&second.shapes).iter().any(|rect| {
            rect.rect.contains(edit_rect.center()) && rect.stroke.color == hovered_stroke
        });
        assert!(frame_hovered, "hovered pair uses the hovered widget stroke");
        second.drop_without_applying_deltas();
    }
}
