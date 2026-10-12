//! Numeric entry box: an [`egui::DragValue`] with stacked increment/decrement
//! steppers at its end, painted with the upper and lower strokes of the Sqyre
//! "S". The steppers share the field's height, so the row never grows taller.

use crate::assets;
use crate::theme::{accent_dim, PRIMARY};
use eframe::egui::{
    self, Atoms, IntoAtoms, Key, Modifiers, Response, Sense, Stroke, TextureHandle, TextureOptions,
    Ui, Vec2, Widget, WidgetInfo, WidgetType,
};
use std::ops::RangeInclusive;

/// Column width for the stacked steppers (points).
const STEPPER_W: f32 = 16.0;
/// Raster size of the cached stepper glyph textures.
const STEPPER_TEX_PX: u32 = 64;
/// Inset around the glyph inside each stepper half.
const GLYPH_PAD: f32 = 2.0;
/// Hold-to-repeat: pause before auto-stepping, then delay between steps.
const REPEAT_DELAY: f64 = 0.45;
const REPEAT_INTERVAL: f64 = 0.08;

/// Numeric entry box: [`egui::DragValue`] + stacked S-stroke steppers.
///
/// Builder mirrors the `DragValue` options used across the app; the steppers
/// increment/decrement by [`Self::speed`] (1 when unset) clamped to
/// [`Self::range`].
pub struct NumEntry<'a, Num: egui::emath::Numeric> {
    value: &'a mut Num,
    speed: Option<f64>,
    range: Option<RangeInclusive<f64>>,
    prefix: Option<Atoms<'a>>,
    suffix: Option<Atoms<'a>>,
    min_decimals: usize,
    max_decimals: Option<usize>,
}

/// Numeric entry box with trailing increment/decrement steppers.
pub fn num_entry<Num: egui::emath::Numeric>(value: &mut Num) -> NumEntry<'_, Num> {
    NumEntry {
        value,
        speed: None,
        range: None,
        prefix: None,
        suffix: None,
        min_decimals: 0,
        max_decimals: None,
    }
}

impl<'a, Num: egui::emath::Numeric> NumEntry<'a, Num> {
    /// Drag speed and stepper increment (see [`egui::DragValue::speed`]).
    pub fn speed(mut self, speed: impl Into<f64>) -> Self {
        self.speed = Some(speed.into());
        self
    }

    /// Clamp range for the entry and its steppers.
    pub fn range<Num2: egui::emath::Numeric>(mut self, range: RangeInclusive<Num2>) -> Self {
        self.range = Some(range.start().to_f64()..=range.end().to_f64());
        self
    }

    /// Prefix inside the entry box (see [`egui::DragValue::prefix`]).
    pub fn prefix(mut self, prefix: impl IntoAtoms<'a>) -> Self {
        self.prefix = Some(prefix.into_atoms());
        self
    }

    /// Suffix inside the entry box (see [`egui::DragValue::suffix`]).
    pub fn suffix(mut self, suffix: impl IntoAtoms<'a>) -> Self {
        self.suffix = Some(suffix.into_atoms());
        self
    }

    /// Maximum decimals shown (see [`egui::DragValue::max_decimals`]).
    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.max_decimals = Some(max_decimals);
        self
    }

    /// Fixed decimals shown (see [`egui::DragValue::fixed_decimals`]).
    pub fn fixed_decimals(mut self, num_decimals: usize) -> Self {
        self.min_decimals = num_decimals;
        self.max_decimals = Some(num_decimals);
        self
    }
}

impl<Num: egui::emath::Numeric> Widget for NumEntry<'_, Num> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            value,
            speed,
            range: range_opt,
            prefix,
            suffix,
            min_decimals,
            max_decimals,
        } = self;
        // Whole-unit button steps by default: egui's integral drag speed (0.25)
        // is a drag nicety, useless as a click step.
        let step = speed.unwrap_or(1.0);
        let range = range_opt
            .clone()
            .unwrap_or_else(|| Num::MIN.to_f64()..=Num::MAX.to_f64());
        // The DragValue and the steppers share `v`; it commits to `value` at the
        // end so both borrow phases stay inside the row closure.
        let mut v = *value;
        let mut response = ui
            .horizontal(|ui| {
                let mut dv = egui::DragValue::new(&mut v);
                if let Some(speed) = speed {
                    dv = dv.speed(speed);
                }
                if let Some(range) = &range_opt {
                    dv = dv.range(range.clone());
                }
                if let Some(prefix) = prefix {
                    dv = dv.prefix(prefix);
                }
                if let Some(suffix) = suffix {
                    dv = dv.suffix(suffix);
                }
                dv = dv.min_decimals(min_decimals);
                if let Some(max_decimals) = max_decimals {
                    dv = dv.max_decimals(max_decimals);
                }
                let drag = ui.add(dv);
                // Attach the steppers flush against the entry box.
                ui.spacing_mut().item_spacing.x = 0.0;
                let steppers = steppers(ui, &mut v, step, &range, drag.rect.height());
                drag | steppers
            })
            .inner;
        if v != *value {
            *value = v;
            response.mark_changed();
        }
        response
    }
}

/// Rasterized S-stroke glyph for one stepper half, cached per `Context`.
fn stepper_texture(ui: &Ui, up: bool) -> Option<TextureHandle> {
    let id = egui::Id::new(("sqyre_num_entry_stepper", up));
    if let Some(tex) = ui.ctx().data_mut(|d| d.get_persisted::<TextureHandle>(id)) {
        return Some(tex);
    }
    let svg = if up {
        assets::STEPPER_UP_SVG
    } else {
        assets::STEPPER_DOWN_SVG
    };
    let (rgba, w, h) = assets::svg_rgba(svg, STEPPER_TEX_PX)?;
    let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    let tex = ui.ctx().load_texture(
        if up {
            "sqyre-stepper-up"
        } else {
            "sqyre-stepper-down"
        },
        image,
        TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear)),
    );
    ui.ctx().data_mut(|d| d.insert_persisted(id, tex.clone()));
    Some(tex)
}

/// `value += delta * step` clamped to `range`, rounded like [`egui::DragValue`]
/// does for arrow-key steps. Returns `true` when the value changed.
fn apply_step<Num: egui::emath::Numeric>(
    ui: &Ui,
    value: &mut Num,
    delta: f64,
    step: f64,
    range: &RangeInclusive<f64>,
) -> bool {
    let old = value.to_f64();
    // Mirror egui's arrow-key step resolution (DragValue::ui).
    let aim_rad = ui.input(|i| i.aim_radius() as f64);
    let auto_decimals = (aim_rad / step.abs()).log10().ceil().clamp(0.0, 15.0) as usize;
    let next = egui::emath::round_to_decimals(old + step * delta, auto_decimals)
        .clamp(*range.start(), *range.end());
    if next == old {
        return false;
    }
    *value = Num::from_f64(next);
    true
}

/// Press → one step now, then repeat after [`REPEAT_DELAY`]; keyboard /
/// AccessKit activation → one step. Returns `true` when a step should be
/// applied this frame.
fn step_fires(ui: &Ui, response: &Response) -> bool {
    if response.is_pointer_button_down_on() {
        let now = ui.input(|i| i.time);
        return match ui.ctx().data(|d| d.get_temp::<f64>(response.id)) {
            // First down frame: take focus, step immediately, arm the repeat delay.
            None => {
                response.request_focus();
                ui.ctx()
                    .data_mut(|d| d.insert_temp(response.id, now + REPEAT_DELAY));
                true
            }
            Some(next) if now >= next => {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(response.id, now + REPEAT_INTERVAL));
                true
            }
            Some(next) => {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(
                        (next - now).max(0.0),
                    ));
                false
            }
        };
    }
    ui.ctx().data_mut(|d| d.remove::<f64>(response.id));
    // Space/Enter or AccessKit activation (pointer release is ignored — the
    // press already stepped once).
    response.clicked() && !response.clicked_by(egui::PointerButton::Primary)
}

/// Arrow keys on a focused stepper half (mirrors the DragValue's own ↑/↓).
fn arrow_step(ui: &Ui, response: &Response, up: bool) -> bool {
    if !response.has_focus() {
        return false;
    }
    let key = if up { Key::ArrowUp } else { Key::ArrowDown };
    ui.input_mut(|i| i.consume_key(Modifiers::NONE, key))
}

/// Largest aspect-preserving rect for the glyph inside `zone`.
fn glyph_rect(zone: egui::Rect, tex: &TextureHandle) -> egui::Rect {
    let [tw, th] = tex.size();
    let aspect = tw as f32 / th as f32;
    let zone = zone.shrink(GLYPH_PAD);
    let mut size = egui::vec2(zone.height() * aspect, zone.height());
    if size.x > zone.width() {
        size = egui::vec2(zone.width(), zone.width() / aspect);
    }
    egui::Rect::from_center_size(zone.center(), size)
}

/// Stacked increment/decrement buttons, `height` = the entry box height.
fn steppers<Num: egui::emath::Numeric>(
    ui: &mut Ui,
    value: &mut Num,
    step: f64,
    range: &RangeInclusive<f64>,
    height: f32,
) -> Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(STEPPER_W, height), Sense::hover());
    let mid = rect.center().y;
    let up_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), mid));
    let down_rect = egui::Rect::from_min_max(egui::pos2(rect.left(), mid), rect.max);
    // unique_id, not id(): child Uis of `ui.horizontal` share the same `id`
    // (a stable per-parent id), so sibling num_entries would collide.
    let up = ui.interact(up_rect, ui.unique_id().with("increment"), Sense::click());
    let down = ui.interact(down_rect, ui.unique_id().with("decrement"), Sense::click());

    let enabled = ui.is_enabled();
    let mut changed = false;
    if enabled {
        if step_fires(ui, &up) || arrow_step(ui, &up, true) {
            changed |= apply_step(ui, value, 1.0, step, range);
        }
        if step_fires(ui, &down) || arrow_step(ui, &down, false) {
            changed |= apply_step(ui, value, -1.0, step, range);
        }
    }

    paint_steppers(ui, rect, up_rect, down_rect, &up, &down);

    let mut response = up | down;
    if changed {
        response.mark_changed();
    }
    response
}

/// Paint the stepper column: attached button chrome + S-stroke glyphs.
fn paint_steppers(
    ui: &Ui,
    rect: egui::Rect,
    up_rect: egui::Rect,
    down_rect: egui::Rect,
    up: &Response,
    down: &Response,
) {
    let enabled = ui.is_enabled();
    let painter = ui.painter();
    // Attached to the field's right edge: only the outer corners are rounded,
    // matching the theme's widget rounding.
    let mut rounding = ui.visuals().widgets.inactive.corner_radius;
    rounding.nw = 0;
    rounding.sw = 0;
    let border = Stroke::new(1.0, ui.visuals().widgets.inactive.bg_stroke.color);
    painter.rect(
        rect,
        rounding,
        ui.visuals().widgets.inactive.bg_fill,
        border,
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            egui::pos2(up_rect.left() + 1.0, up_rect.bottom()),
            egui::pos2(up_rect.right() - 1.0, up_rect.bottom()),
        ],
        border,
    );

    for (zone, resp, up_flag) in [(up_rect, up, true), (down_rect, down, false)] {
        let visuals = ui.style().interact(resp);
        if enabled && (resp.hovered() || resp.is_pointer_button_down_on()) {
            painter.rect_filled(zone.intersect(rect), rounding, visuals.bg_fill);
        }
        let glyph_color = if !enabled {
            ui.visuals().weak_text_color()
        } else if resp.hovered() || resp.is_pointer_button_down_on() || resp.has_focus() {
            PRIMARY
        } else {
            accent_dim()
        };
        match stepper_texture(ui, up_flag) {
            Some(tex) => {
                painter.image(
                    tex.id(),
                    glyph_rect(zone, &tex),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    glyph_color,
                );
            }
            None => {
                // Rasterize fallback: plain chevron strokes.
                let c = zone.shrink(GLYPH_PAD * 2.0).center();
                let s = 3.0;
                let dy = if up_flag { -s } else { s };
                painter.line_segment(
                    [egui::pos2(c.x - s, c.y - dy), egui::pos2(c.x, c.y + dy)],
                    Stroke::new(1.5, glyph_color),
                );
                painter.line_segment(
                    [egui::pos2(c.x, c.y + dy), egui::pos2(c.x + s, c.y - dy)],
                    Stroke::new(1.5, glyph_color),
                );
            }
        }
        if resp.has_focus() {
            crate::widgets::controls::paint_keyboard_focus_ring(ui, zone, resp);
        }
        resp.clone()
            .on_hover_text(if up_flag { "Increment" } else { "Decrement" });
        resp.widget_info(|| {
            WidgetInfo::labeled(
                WidgetType::Button,
                enabled,
                if up_flag { "Increment" } else { "Decrement" },
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    fn pointer_click<T: 'static>(harness: &mut egui_kittest::Harness<'_, T>, at: egui::Pos2) {
        harness.hover_at(at);
        harness.run_steps(2);
        for pressed in [true, false] {
            harness.input_mut().events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.run_steps(1);
        }
        harness.run_steps(2);
    }

    fn harness(range: RangeInclusive<i32>) -> egui_kittest::Harness<'static, i32> {
        egui_kittest::Harness::builder()
            .with_size([300.0, 120.0])
            .build_ui_state(
                move |ui, v: &mut i32| {
                    ui.add(num_entry(v).speed(1).range(range.clone()));
                },
                5,
            )
    }

    #[test]
    fn increment_and_decrement_step() {
        let mut h = harness(0..=10);
        h.run_steps(2);
        let inc = h.get_by_label("Increment").rect().center();
        pointer_click(&mut h, inc);
        assert_eq!(*h.state(), 6);
        let dec = h.get_by_label("Decrement").rect().center();
        pointer_click(&mut h, dec);
        pointer_click(&mut h, dec);
        assert_eq!(*h.state(), 4);
    }

    #[test]
    fn sibling_steppers_have_unique_ids() {
        // Sibling num_entries in one `Ui` must get distinct stepper ids —
        // otherwise presses on one field can step another (child Uis share a
        // stable `id`; only `unique_id` differs).
        let mut h = egui_kittest::Harness::builder()
            .with_size([300.0, 160.0])
            .build_ui_state(
                move |ui, v: &mut [i32; 2]| {
                    ui.add(num_entry(&mut v[0]).speed(1).range(0..=10));
                    ui.add(num_entry(&mut v[1]).speed(1).range(0..=10));
                },
                [5, 5],
            );
        h.run_steps(2);
        let incs: Vec<egui::Rect> = h
            .get_all_by_label("Increment")
            .map(|node| node.rect())
            .collect();
        assert_eq!(incs.len(), 2);
        assert_ne!(incs[0], incs[1]);
        pointer_click(&mut h, incs[1].center());
        assert_eq!(*h.state(), [5, 6]);
        pointer_click(&mut h, incs[0].center());
        assert_eq!(*h.state(), [6, 6]);
    }

    #[test]
    fn steppers_clamp_to_range() {
        let mut h = harness(0..=6);
        h.run_steps(2);
        let inc = h.get_by_label("Increment").rect().center();
        for _ in 0..5 {
            pointer_click(&mut h, inc);
        }
        assert_eq!(*h.state(), 6);
        let dec = h.get_by_label("Decrement").rect().center();
        for _ in 0..10 {
            pointer_click(&mut h, dec);
        }
        assert_eq!(*h.state(), 0);
    }

    #[test]
    fn steppers_do_not_grow_row_height() {
        // Steppers reuse the DragValue's height: the whole row is exactly as
        // tall as a plain DragValue.
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let plain = ui.add(egui::DragValue::new(&mut 5i32).speed(1)).rect;
            let mut v = 5i32;
            let entry = ui
                .horizontal(|ui| ui.add(num_entry(&mut v).speed(1)).rect)
                .inner;
            assert!(
                (entry.height() - plain.height()).abs() < 0.5,
                "entry {entry:?} taller than plain {plain:?}"
            );
        })
        .drop_without_applying_deltas();
    }
}
