//! One framed control: an icon (or two) joined to the widget it labels or acts on.
//!
//! Resting icon buttons are bare glyphs. A loose glyph beside a field does not
//! read as part of that field, so [`pair_icon`] puts them in a single outline
//! with a hairline between the slots. Pressing the leading icon focuses the
//! paired control and, for a dropdown, leaves the menu open the way a click
//! on the menu itself does.

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
        .inner_margin(egui::Margin::symmetric(
            crate::theme::SPACE_4 as i8,
            crate::theme::SPACE_2 as i8,
        ))
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
            let mut gesture = IconGesture::default();
            if let Some(lead) = leading.as_mut() {
                let rect = ui.scope(|ui| lead(ui)).response.rect;
                gesture = icon_gesture(ui, rect);
                // The glyph is only a label. Focus the control before it is
                // built, and keep that focus when the button releases over the
                // icon. Otherwise egui treats the release as a click outside
                // and blurs the control, which closes a dropdown at once.
                if gesture.pressed {
                    focus_following_widget(ui);
                }
                lead_rect = Some(rect);
                ui.add_space(PAIR_SLOT_GAP);
            }
            let saved_surrender = ui.ctx().options(|o| o.input_options.surrender_focus_on);
            if gesture.pressed || gesture.clicked {
                ui.ctx().options_mut(|o| {
                    o.input_options.surrender_focus_on = egui::SurrenderFocusOn::Never;
                });
            }
            if gesture.clicked {
                activate_icon_target(ui);
            }
            let events_before = ui.ctx().output(|o| o.events.len());
            let scoped = ui.scope(|ui| widget(ui));
            if gesture.pressed || gesture.clicked {
                ui.ctx().options_mut(|o| {
                    o.input_options.surrender_focus_on = saved_surrender;
                });
            }
            if gesture.pressed {
                remember_icon_target(ui, events_before);
            }
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

#[derive(Clone, Copy, Default)]
struct IconGesture {
    pressed: bool,
    clicked: bool,
}

/// The control an icon press focused, and whether that control is a combo.
#[derive(Clone, Copy)]
struct IconFocusTarget {
    id: egui::Id,
    combo: bool,
}

fn icon_target_id(ui: &egui::Ui) -> egui::Id {
    ui.id().with("pair_icon_target")
}

/// Pointer went down, or finished a click, on the leading glyph.
fn icon_gesture(ui: &egui::Ui, rect: egui::Rect) -> IconGesture {
    if !ui.is_enabled() {
        return IconGesture::default();
    }
    ui.input(|i| {
        let on_icon = i
            .pointer
            .interact_pos()
            .is_some_and(|pos| rect.contains(pos));
        IconGesture {
            pressed: on_icon && i.pointer.primary_pressed(),
            clicked: on_icon && i.pointer.primary_clicked(),
        }
    })
}

/// Move keyboard focus to the next focusable widget painted after this call.
fn focus_following_widget(ui: &egui::Ui) {
    ui.memory_mut(|mem| {
        if let Some(id) = mem.focused() {
            mem.surrender_focus(id);
        }
        mem.move_focus(egui::FocusDirection::Next);
    });
}

/// Re-focus the control from the icon press, and toggle its menu when it is a combo.
fn activate_icon_target(ui: &egui::Ui) {
    let Some(target) = ui
        .ctx()
        .data(|data| data.get_temp::<IconFocusTarget>(icon_target_id(ui)))
    else {
        return;
    };
    ui.memory_mut(|mem| mem.request_focus(target.id));
    if target.combo {
        egui::Popup::toggle_id(ui.ctx(), target.id.with("popup"));
    }
}

/// Remember which control the icon press focused, including combo menus.
fn remember_icon_target(ui: &egui::Ui, events_before: usize) {
    let Some(id) = ui.memory(|mem| mem.focused()) else {
        return;
    };
    let gained_combo = ui.ctx().output(|output| {
        output.events.iter().skip(events_before).any(|event| {
            matches!(
                event,
                egui::output::OutputEvent::FocusGained(info)
                    if info.typ == egui::WidgetType::ComboBox
            )
        })
    });
    let key = icon_target_id(ui);
    let combo = gained_combo
        || ui.ctx().data(|data| {
            data.get_temp::<IconFocusTarget>(key)
                .is_some_and(|target| target.combo && target.id == id)
        });
    ui.ctx()
        .data_mut(|data| data.insert_temp(key, IconFocusTarget { id, combo }));
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

    fn glyph_rect(shapes: &[egui::epaint::ClippedShape], glyph: &str) -> egui::Rect {
        shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text() == glyph => {
                    Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
                }
                _ => None,
            })
            .unwrap_or(egui::Rect::NOTHING)
    }

    fn press(pos: egui::Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn release(pos: egui::Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn galley_has(shapes: &[egui::epaint::ClippedShape], needle: &str) -> bool {
        shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => text.galley.text().contains(needle),
            _ => false,
        })
    }

    /// Settle layout, then primary-press `glyph` and return whether `focused` became true.
    fn focus_after_icon_press(
        glyph: &str,
        mut paint: impl FnMut(&mut egui::Ui, &mut bool),
    ) -> bool {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 240.0));
        let mut icon = egui::Rect::NOTHING;
        for pass in 0..2 {
            let mut focused = false;
            let out = run(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(pass as f64 * 0.1),
                    ..Default::default()
                },
                |ui| paint(ui, &mut focused),
            );
            icon = glyph_rect(&out.shapes, glyph);
            out.drop_without_applying_deltas();
            assert!(!focused, "the paired widget started focused");
            assert!(icon.is_positive(), "missing icon glyph {glyph:?}");
        }
        let mut focused = false;
        let pressed = run(
            &ctx,
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(0.3),
                events: vec![press(icon.center())],
                ..Default::default()
            },
            |ui| paint(ui, &mut focused),
        );
        pressed.drop_without_applying_deltas();
        assert!(focused, "pressing {glyph:?} did not focus its widget");
        focused = false;
        let held = run(
            &ctx,
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(0.35),
                ..Default::default()
            },
            |ui| paint(ui, &mut focused),
        );
        held.drop_without_applying_deltas();
        assert!(focused, "holding {glyph:?} blurred its widget");
        focused = false;
        let released = run(
            &ctx,
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(0.45),
                events: vec![release(icon.center())],
                ..Default::default()
            },
            |ui| paint(ui, &mut focused),
        );
        released.drop_without_applying_deltas();
        assert!(focused, "releasing {glyph:?} blurred its widget");
        focused = false;
        let idle = run(
            &ctx,
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(0.55),
                ..Default::default()
            },
            |ui| paint(ui, &mut focused),
        );
        idle.drop_without_applying_deltas();
        focused
    }

    #[test]
    fn pressing_the_pair_icon_focuses_the_widget() {
        let mut name = "Alpha".to_owned();
        let text_focused = focus_after_icon_press("T", |ui, focused| {
            pair_labeled(ui, "T", "Name", |ui| {
                *focused = ui
                    .add(egui::TextEdit::singleline(&mut name).desired_width(140.0))
                    .has_focus();
            });
        });
        assert!(text_focused, "the name field lost focus on the next frame");

        let mut number = 4.0;
        let drag_focused = focus_after_icon_press("N", |ui, focused| {
            pair_labeled(ui, "N", "Count", |ui| {
                *focused = ui
                    .add(egui::DragValue::new(&mut number).speed(1.0))
                    .has_focus();
            });
        });
        assert!(drag_focused, "the drag value lost focus on the next frame");

        let mut choice = 0;
        let combo_focused = focus_after_icon_press("C", |ui, focused| {
            pair_labeled(ui, "C", "Choice", |ui| {
                *focused = egui::ComboBox::from_id_salt("choice")
                    .width(120.0)
                    .show_index(ui, &mut choice, 2, |i| format!("Item {i}"))
                    .has_focus();
            });
        });
        assert!(combo_focused, "the combo lost focus on the next frame");
    }

    #[test]
    fn pressing_one_pair_icon_focuses_that_pair() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 240.0));
        let mut name = "Alpha".to_owned();
        let mut tags = "demo".to_owned();
        let mut name_icon = egui::Rect::NOTHING;
        let mut tag_icon = egui::Rect::NOTHING;
        for pass in 0..2 {
            let out = run(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(pass as f64 * 0.1),
                    ..Default::default()
                },
                |ui| {
                    ui.horizontal(|ui| {
                        pair_labeled(ui, "T", "Name", |ui| {
                            ui.add(egui::TextEdit::singleline(&mut name).desired_width(120.0));
                        });
                        pair_labeled(ui, "#", "Tags", |ui| {
                            ui.add(egui::TextEdit::singleline(&mut tags).desired_width(120.0));
                        });
                    });
                },
            );
            name_icon = glyph_rect(&out.shapes, "T");
            tag_icon = glyph_rect(&out.shapes, "#");
            out.drop_without_applying_deltas();
        }

        let mut name_focus = false;
        let mut tag_focus = false;
        let pressed = run(
            &ctx,
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(0.3),
                events: vec![press(tag_icon.center())],
                ..Default::default()
            },
            |ui| {
                ui.horizontal(|ui| {
                    pair_labeled(ui, "T", "Name", |ui| {
                        name_focus = ui
                            .add(egui::TextEdit::singleline(&mut name).desired_width(120.0))
                            .has_focus();
                    });
                    pair_labeled(ui, "#", "Tags", |ui| {
                        tag_focus = ui
                            .add(egui::TextEdit::singleline(&mut tags).desired_width(120.0))
                            .has_focus();
                    });
                });
            },
        );
        pressed.drop_without_applying_deltas();
        assert!(
            tag_focus && !name_focus,
            "tag icon press focused name={name_focus} tags={tag_focus}, icons {name_icon:?} {tag_icon:?}"
        );
    }

    #[test]
    fn trailing_icon_press_does_not_focus_the_field() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 240.0));
        let mut value = "query".to_owned();
        let mut trail = egui::Rect::NOTHING;
        for pass in 0..2 {
            let out = run(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(pass as f64 * 0.1),
                    ..Default::default()
                },
                |ui| {
                    pair_icon(
                        ui,
                        |ui| {
                            ui.label("S");
                        },
                        |ui| {
                            ui.add(egui::TextEdit::singleline(&mut value).desired_width(140.0));
                        },
                        |ui| {
                            trail = ui
                                .add(egui::Button::new("X").min_size(egui::vec2(18.0, 18.0)))
                                .rect;
                        },
                    );
                },
            );
            out.drop_without_applying_deltas();
        }
        assert!(trail.is_positive(), "missing trailing button");

        let mut focused = false;
        let mut clicked = false;
        let down = run(
            &ctx,
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(0.3),
                events: vec![press(trail.center())],
                ..Default::default()
            },
            |ui| {
                pair_icon(
                    ui,
                    |ui| {
                        ui.label("S");
                    },
                    |ui| {
                        focused = ui
                            .add(egui::TextEdit::singleline(&mut value).desired_width(140.0))
                            .has_focus();
                    },
                    |ui| {
                        clicked = ui
                            .add(egui::Button::new("X").min_size(egui::vec2(18.0, 18.0)))
                            .clicked();
                    },
                );
            },
        );
        down.drop_without_applying_deltas();
        assert!(!focused, "pressing the trailing icon focused the field");
        assert!(!clicked, "a press is not a click");

        let up = run(
            &ctx,
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(0.4),
                events: vec![egui::Event::PointerButton {
                    pos: trail.center(),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| {
                pair_icon(
                    ui,
                    |ui| {
                        ui.label("S");
                    },
                    |ui| {
                        focused = ui
                            .add(egui::TextEdit::singleline(&mut value).desired_width(140.0))
                            .has_focus();
                    },
                    |ui| {
                        clicked = ui
                            .add(egui::Button::new("X").min_size(egui::vec2(18.0, 18.0)))
                            .clicked();
                    },
                );
            },
        );
        up.drop_without_applying_deltas();
        assert!(clicked, "releasing the trailing icon did not click it");
        assert!(!focused, "the trailing click focused the field");
    }

    #[test]
    fn pressing_a_combo_icon_leaves_the_menu_open() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 240.0));
        let mut choice = 0usize;
        let mut icon = egui::Rect::NOTHING;
        let mut paint = |ui: &mut egui::Ui| {
            pair_labeled(ui, "C", "Choice", |ui| {
                egui::ComboBox::from_id_salt("choice")
                    .width(160.0)
                    .show_index(ui, &mut choice, 3, |i| format!("Item {i}"));
            });
        };
        for pass in 0..2 {
            let out = run(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(pass as f64 * 0.1),
                    ..Default::default()
                },
                &mut paint,
            );
            icon = glyph_rect(&out.shapes, "C");
            out.drop_without_applying_deltas();
        }
        assert!(icon.is_positive(), "missing combo icon");
        let point = icon.center();
        for (time, events) in [
            (0.3, vec![press(point)]),
            (0.35, vec![]),
            (0.45, vec![release(point)]),
            (0.55, vec![]),
        ] {
            let out = run(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                &mut paint,
            );
            let open = egui::Popup::is_any_open(&ctx);
            let menu = galley_has(&out.shapes, "Item 1");
            let texts: Vec<String> = out
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_string()),
                    _ => None,
                })
                .collect();
            out.drop_without_applying_deltas();
            if time >= 0.45 {
                assert!(open, "combo menu closed at t={time} texts={texts:?}");
            }
            // A popup's first open frame is an invisible sizing pass.
            if time >= 0.55 {
                assert!(menu, "combo menu not painted at t={time} texts={texts:?}");
            }
        }
    }

    #[test]
    fn pressing_a_tag_icon_leaves_suggestions_open() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 320.0));
        let mut tags = Vec::new();
        let mut draft = String::new();
        let suggestions = vec!["alpha".to_owned(), "beta".to_owned()];
        let glyph = egui_phosphor::regular::TAG;
        let mut icon = egui::Rect::NOTHING;
        let mut paint = |ui: &mut egui::Ui| {
            super::super::tag_chip_editor(
                ui,
                &mut tags,
                &mut draft,
                &suggestions,
                super::super::TagChipOptions {
                    label_name: Some("Tags"),
                    ..Default::default()
                },
            );
        };
        for pass in 0..2 {
            let out = run(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(pass as f64 * 0.1),
                    ..Default::default()
                },
                &mut paint,
            );
            icon = glyph_rect(&out.shapes, glyph);
            out.drop_without_applying_deltas();
        }
        assert!(icon.is_positive(), "missing tag icon");
        let point = icon.center();
        for (time, events) in [
            (0.3, vec![press(point)]),
            (0.35, vec![]),
            (0.45, vec![release(point)]),
            (0.55, vec![]),
        ] {
            let out = run(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                &mut paint,
            );
            let visible = galley_has(&out.shapes, "alpha");
            out.drop_without_applying_deltas();
            if time >= 0.45 {
                assert!(visible, "tag suggestions missing at t={time}");
            }
        }
    }
}
