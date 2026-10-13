//! Numeric entry box: an [`egui::DragValue`] with stacked increment/decrement
//! steppers inside its right end, painted with the upper and lower strokes of
//! the Sqyre "S". The steppers share the field's rect, so the row never grows
//! taller; a trailing spacer atom keeps the value text clear of the glyphs.

use crate::assets;
use crate::theme::{accent_dim, PRIMARY};
use eframe::egui::{
    self, Atom, Atoms, IntoAtoms, Key, Modifiers, Response, Sense, Stroke, TextureHandle,
    TextureOptions, Ui, Widget, WidgetInfo, WidgetType,
};
use std::ops::RangeInclusive;

/// Width of the stepper zone inside the entry box (points).
const STEPPER_W: f32 = 12.0;
/// Gap between the stepper zone and the field's right edge (points).
const STEPPER_INSET: f32 = 1.0;
/// Raster size of the cached stepper glyph textures.
const STEPPER_TEX_PX: u32 = 64;
/// Vertical inset around the glyph inside the stepper zone.
const GLYPH_PAD: f32 = 3.0;
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
        // Reserve the stepper column as an empty trailing atom so the value
        // text always ends left of the glyphs — the box keeps this width even
        // when the text is short.
        let mut suffix = suffix.unwrap_or_default();
        suffix.push_right(Atom::custom(
            egui::Id::new("sqyre_num_entry_stepper_zone"),
            egui::vec2(STEPPER_W + STEPPER_INSET, 0.0),
        ));
        dv = dv.suffix(suffix);
        dv = dv.min_decimals(min_decimals);
        if let Some(max_decimals) = max_decimals {
            dv = dv.max_decimals(max_decimals);
        }
        let mut response = ui.add(dv);
        // Steppers overlay the right end of the entry box itself.
        response |= steppers(ui, &mut v, step, &range, response.rect, response.id);
        if v != *value {
            *value = v;
            response.mark_changed();
        }
        response
    }
}

/// Pure integer literal (no `${var}` / expressions) — string fields can only
/// step while the text is a plain number.
pub(crate) fn pure_i32(s: &str) -> Option<i32> {
    let t = s.trim();
    if t.is_empty() || sqyre_varref::contains(t) {
        return None;
    }
    t.parse().ok()
}

/// Text margin reserving the stepper column inside a string field's
/// [`egui::TextEdit`] (egui's default is `Margin::symmetric(4, 2)`).
pub(crate) const TEXT_STEPPER_MARGIN: egui::Margin = egui::Margin {
    left: 4,
    right: 4 + STEPPER_W as i8 + STEPPER_INSET as i8,
    top: 2,
    bottom: 2,
};

/// Stacked S-stroke steppers inside `rect`'s right edge for a string field —
/// same column as the `DragValue` steppers. Active only while `value` is a
/// pure integer (see [`pure_i32`]); steps clamp to `range`. `field_id` is the
/// field's own id so sibling fields get distinct steppers.
pub(crate) fn string_steppers(
    ui: &mut Ui,
    field_id: egui::Id,
    rect: egui::Rect,
    value: &mut String,
    range: RangeInclusive<i32>,
) {
    let Some(n) = pure_i32(value) else {
        return;
    };
    let zone = egui::Rect::from_min_max(
        egui::pos2(rect.right() - STEPPER_W - STEPPER_INSET, rect.top()),
        egui::pos2(rect.right() - STEPPER_INSET, rect.bottom()),
    );
    let mid = zone.center().y;
    let up_rect = egui::Rect::from_min_max(zone.min, egui::pos2(zone.right(), mid));
    let down_rect = egui::Rect::from_min_max(egui::pos2(zone.left(), mid), zone.max);
    let up = ui
        .interact(up_rect, field_id.with("increment"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let down = ui
        .interact(down_rect, field_id.with("decrement"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let mut step = |mult: f64| {
        *value = n
            .saturating_add(mult as i32)
            .clamp(*range.start(), *range.end())
            .to_string();
    };
    let m = step_fires(ui, &up) + f64::from(arrow_step(ui, &up, true));
    if m > 0.0 {
        step(m);
    }
    let m = step_fires(ui, &down) + f64::from(arrow_step(ui, &down, false));
    if m > 0.0 {
        step(-m);
    }
    paint_steppers(ui, zone, up_rect, down_rect, &up, &down);
}

/// Rasterized S-stroke glyph for one stepper half, cached per `Context`.
pub(crate) fn stepper_texture(ui: &Ui, up: bool) -> Option<TextureHandle> {
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

/// Press → one step now; hold past [`REPEAT_DELAY`] → another step every
/// [`REPEAT_INTERVAL`], each tick's amount growing linearly (tick *k* → ×k).
/// Keyboard / AccessKit activation → one step. Returns the total step
/// multiplier to apply this frame (0 for none).
///
/// State is `(press_time, ticks_fired)` in ctx temp data. `now < press` can
/// only mean the clock restarted mid-hold; treat it as a fresh press.
pub(crate) fn step_fires(ui: &Ui, response: &Response) -> f64 {
    if !response.is_pointer_button_down_on() {
        ui.ctx().data_mut(|d| d.remove::<(f64, u64)>(response.id));
        // Space/Enter or AccessKit activation (pointer release is ignored —
        // the press already stepped once).
        return f64::from(response.clicked() && !response.clicked_by(egui::PointerButton::Primary));
    }
    let now = ui.input(|i| i.time);
    let Some((press, ticks)) = ui
        .ctx()
        .data(|d| d.get_temp::<(f64, u64)>(response.id))
        .filter(|(press, _)| *press <= now)
    else {
        // First down frame: take focus, step once, arm the repeat — and
        // schedule the frame it fires on, or a still hold never repeats.
        response.request_focus();
        ui.ctx()
            .data_mut(|d| d.insert_temp(response.id, (now, 0u64)));
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(REPEAT_DELAY));
        return 1.0;
    };
    let held = now - press;
    let due = if held < REPEAT_DELAY {
        0
    } else {
        1 + ((held - REPEAT_DELAY) / REPEAT_INTERVAL) as u64
    };
    if due <= ticks {
        // Wake for the next owed tick: press + DELAY + ticks * INTERVAL.
        let until = (REPEAT_DELAY + ticks as f64 * REPEAT_INTERVAL - held).max(0.0);
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(until));
        return 0.0;
    }
    // Each owed tick fires its own linearly growing amount (k = tick index).
    let total = (ticks + 1..=due).sum::<u64>() as f64;
    ui.ctx()
        .data_mut(|d| d.insert_temp(response.id, (press, due)));
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f64(REPEAT_INTERVAL));
    total
}

/// Arrow keys on a focused stepper half (mirrors the DragValue's own ↑/↓).
pub(crate) fn arrow_step(ui: &Ui, response: &Response, up: bool) -> bool {
    if !response.has_focus() {
        return false;
    }
    let key = if up { Key::ArrowUp } else { Key::ArrowDown };
    ui.input_mut(|i| i.consume_key(Modifiers::NONE, key))
}

/// Largest aspect-preserving rect for the glyph inside `zone`, centered.
pub(crate) fn glyph_rect(zone: egui::Rect, tex: &TextureHandle) -> egui::Rect {
    let [tw, th] = tex.size();
    let aspect = tw as f32 / th as f32;
    let zone = zone.shrink2(egui::vec2(0.0, GLYPH_PAD));
    let mut size = egui::vec2(zone.height() * aspect, zone.height());
    if size.x > zone.width() {
        size = egui::vec2(zone.width(), zone.width() / aspect);
    }
    egui::Rect::from_center_size(zone.center(), size)
}

/// Stacked increment/decrement buttons flush against the right edge of
/// `drag_rect` — inside the entry box, so the row never grows taller. The
/// zone is reserved by a trailing atom, so value text can't run under it.
/// `drag_id` is the DragValue's own id: per-field unique, so sibling steppers
/// don't collide (`ui.unique_id()` is constant within a Ui, so it can't be
/// used here).
fn steppers<Num: egui::emath::Numeric>(
    ui: &mut Ui,
    value: &mut Num,
    step: f64,
    range: &RangeInclusive<f64>,
    drag_rect: egui::Rect,
    drag_id: egui::Id,
) -> Response {
    let rect = egui::Rect::from_min_max(
        egui::pos2(
            drag_rect.right() - STEPPER_W - STEPPER_INSET,
            drag_rect.top(),
        ),
        egui::pos2(drag_rect.right() - STEPPER_INSET, drag_rect.bottom()),
    );
    let mid = rect.center().y;
    let up_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), mid));
    let down_rect = egui::Rect::from_min_max(egui::pos2(rect.left(), mid), rect.max);
    let up = ui
        .interact(up_rect, drag_id.with("increment"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let down = ui
        .interact(down_rect, drag_id.with("decrement"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let enabled = ui.is_enabled();
    let mut changed = false;
    if enabled {
        let m = step_fires(ui, &up) + f64::from(arrow_step(ui, &up, true));
        if m > 0.0 {
            changed |= apply_step(ui, value, m, step, range);
        }
        let m = step_fires(ui, &down) + f64::from(arrow_step(ui, &down, false));
        if m > 0.0 {
            changed |= apply_step(ui, value, -m, step, range);
        }
    }

    paint_steppers(ui, rect, up_rect, down_rect, &up, &down);

    let mut response = up | down;
    if changed {
        response.mark_changed();
    }
    response
}

/// Paint the stepper column: hover wash + S-stroke glyphs at full opacity.
/// Shared by [`num_entry`] fields and the image-preview coord chips.
pub(crate) fn paint_steppers(
    ui: &Ui,
    rect: egui::Rect,
    up_rect: egui::Rect,
    down_rect: egui::Rect,
    up: &Response,
    down: &Response,
) {
    let enabled = ui.is_enabled();
    let painter = ui.painter();

    for (zone, resp, up_flag) in [(up_rect, up, true), (down_rect, down, false)] {
        if enabled && (resp.hovered() || resp.is_pointer_button_down_on() || resp.has_focus()) {
            painter.rect_filled(zone, 2.0, accent_dim());
        }
        let glyph_color = if enabled {
            PRIMARY
        } else {
            ui.visuals().weak_text_color()
        };
        match (stepper_texture(ui, true), stepper_texture(ui, false)) {
            (Some(up_tex), Some(down_tex)) => {
                // Both SVGs share the whole-S viewBox: painted into one rect
                // they compose the icon's S, so the buttons look like it.
                let glyph = glyph_rect(rect, &up_tex);
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                let (tex, color) = if up_flag {
                    (&up_tex, glyph_color)
                } else {
                    (&down_tex, glyph_color)
                };
                painter.image(tex.id(), glyph, uv, color);
            }
            _ => {
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
            // Tight inside ring — the shared focus-ring helper expands and
            // strokes 2pt, which overwhelms the 12pt stepper column.
            painter.rect_stroke(
                zone.shrink(0.5),
                egui::CornerRadius::same(2),
                Stroke::new(1.0, PRIMARY),
                egui::StrokeKind::Inside,
            );
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

    /// Holding a stepper keeps stepping: after [`REPEAT_DELAY`] each
    /// [`REPEAT_INTERVAL`] tick fires a linearly growing amount, and every
    /// held frame schedules the next tick's repaint — a still hold otherwise
    /// produces no frames and never repeats.
    #[test]
    fn holding_a_stepper_repeats_with_growing_steps() {
        let mut h = harness(0..=1000);
        h.run_steps(2);
        let inc = h.get_by_label("Increment").rect().center();
        h.hover_at(inc);
        h.run_steps(2);
        // Press, no release.
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: inc,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        h.input_mut().time = Some(0.0);
        h.run_steps(1);
        assert_eq!(*h.state(), 6, "press steps once");
        let repaint = h.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        assert!(
            repaint < std::time::Duration::MAX,
            "a still hold must keep scheduling repaints or it never repeats"
        );
        // Still inside the delay: nothing owed yet.
        h.input_mut().time = Some(0.4);
        h.run_steps(1);
        assert_eq!(*h.state(), 6);
        // held 0.5 → tick 1 fires ×1.
        h.input_mut().time = Some(0.5);
        h.run_steps(1);
        assert_eq!(*h.state(), 7);
        // held 0.7 → ticks 2..=4 fire ×2+×3+×4.
        h.input_mut().time = Some(0.7);
        h.run_steps(1);
        assert_eq!(*h.state(), 16, "step amounts must grow linearly");
        // Release is not another step.
        h.input_mut().time = Some(0.8);
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: inc,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(2);
        assert_eq!(*h.state(), 16);
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

    /// Steppers hug the field's right edge, with the value text kept clear
    /// by the reserved trailing space inside the box.
    #[test]
    fn steppers_hug_the_entry_right_edge() {
        let mut h = harness(0..=10);
        h.run_steps(2);
        let field = h.get_by_role(egui::accesskit::Role::SpinButton).rect();
        let inc = h.get_by_label("Increment").rect();
        assert!(
            (field.right() - STEPPER_INSET - inc.right()).abs() < 0.5,
            "increment zone {inc:?} not flush with field right edge {field:?}"
        );
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
