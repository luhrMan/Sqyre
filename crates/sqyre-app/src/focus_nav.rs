//! Keep arrow-key focus navigation inside the top window.
//!
//! egui's directional focus search spans every layer, so an arrow press can
//! jump into whichever overlapping window happens to be nearest. The topmost
//! expanded window (or popup) owns keyboard navigation: when an arrow press
//! moves focus anywhere else, it is redirected to the nearest widget inside
//! that window. Collapsed windows are skipped, and with no window open the
//! main panels own navigation. The same top surface owns Esc ([`is_top_layer`]).

use eframe::egui::{self, collapsing_header::CollapsingState, Id, Key, LayerId, Order, Rect, Vec2};

#[derive(Clone, Copy)]
struct ArrowMove {
    from: Id,
    dir: Vec2,
}

fn state_id() -> Id {
    Id::new("sqyre_focus_nav_arrow_move")
}

/// Call once per frame before any UI is shown.
///
/// egui moves focus at the end of the pass, so the arrow press seen this frame
/// is checked against the focus egui picked by the start of the next frame.
pub fn lock_to_window(ctx: &egui::Context) {
    let prev = ctx.data_mut(|d| {
        let prev = d.get_temp::<ArrowMove>(state_id());
        d.remove::<ArrowMove>(state_id());
        prev
    });
    if let Some(prev) = prev {
        redirect_escape(ctx, prev);
    }
    let pending = ctx
        .memory(|m| m.focused())
        .zip(arrow_dir(ctx))
        .map(|(from, dir)| ArrowMove { from, dir });
    if let Some(pending) = pending {
        ctx.data_mut(|d| d.insert_temp(state_id(), pending));
    }
}

/// Direction of the last unmodified arrow press, matching egui's focus rules.
fn arrow_dir(ctx: &egui::Context) -> Option<Vec2> {
    ctx.input(|i| {
        i.events.iter().rev().find_map(|e| match e {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if !modifiers.any() => match key {
                Key::ArrowUp => Some(Vec2::UP),
                Key::ArrowDown => Some(Vec2::DOWN),
                Key::ArrowLeft => Some(Vec2::LEFT),
                Key::ArrowRight => Some(Vec2::RIGHT),
                _ => None,
            },
            _ => None,
        })
    })
}

fn redirect_escape(ctx: &egui::Context, prev: ArrowMove) {
    let Some(to) = ctx.memory(|m| m.focused()) else {
        return;
    };
    if to == prev.from {
        return;
    }
    let home = home_layer(ctx);
    let Some((from_layer, from_rect, to_layer)) = ctx.viewport(|v| {
        let widgets = &v.prev_pass.widgets;
        let from = widgets.get(prev.from)?;
        let to_layer = widgets.get(to)?.layer_id;
        Some((from.layer_id, from.rect, to_layer))
    }) else {
        return;
    };
    // In-window arrow moves are the common case: skip the candidate scan.
    if layer_in_home(ctx, home, to_layer) || matches!(to_layer.order, Order::Tooltip | Order::Debug)
    {
        return;
    }
    let home_layers = layers_in_home(ctx, home);
    let candidates: Vec<(Id, Rect)> = ctx.viewport(|v| {
        let widgets = &v.prev_pass.widgets;
        home_layers
            .iter()
            .flat_map(|&l| widgets.get_layer(l))
            .filter(|w| w.enabled && w.sense.is_focusable())
            .map(|w| (w.id, w.rect))
            .collect()
    });
    let fallback = if layer_in_home(ctx, home, from_layer) {
        Some(prev.from)
    } else {
        nearest_by_distance(from_rect, &candidates)
    };
    if let Some(target) =
        nearest_in_direction(prev.from, from_rect, prev.dir, candidates).or(fallback)
    {
        ctx.memory_mut(|m| m.request_focus(target));
    }
}

fn layer_in_home(ctx: &egui::Context, home: LayerId, layer: LayerId) -> bool {
    layer == home || ctx.memory(|m| m.areas().parent_layer(layer) == Some(home))
}

fn layers_in_home(ctx: &egui::Context, home: LayerId) -> Vec<LayerId> {
    ctx.memory(|m| {
        m.layer_ids()
            .filter(|&l| l == home || m.areas().parent_layer(l) == Some(home))
            .collect()
    })
}

/// Closest candidate when focus arrived from outside the home window.
fn nearest_by_distance(from: Rect, candidates: &[(Id, Rect)]) -> Option<Id> {
    candidates
        .iter()
        .min_by(|a, b| {
            let origin = from.center();
            a.1.center()
                .distance_sq(origin)
                .total_cmp(&b.1.center().distance_sq(origin))
        })
        .map(|(id, _)| *id)
}

/// Whether `layer` is the top surface that should handle Esc.
///
/// Tooltip-order layers float above every window, so they always qualify;
/// sublayers (e.g. a palette window over its dismiss area) count as their parent.
pub fn is_top_layer(ctx: &egui::Context, layer: LayerId) -> bool {
    if matches!(layer.order, Order::Tooltip | Order::Debug) {
        return true;
    }
    let layer = ctx
        .memory(|m| m.areas().parent_layer(layer))
        .unwrap_or(layer);
    layer == home_layer(ctx)
}

/// Topmost visible, expanded window or popup with focusable widgets; the main
/// panels when there is none.
fn home_layer(ctx: &egui::Context) -> LayerId {
    let stacked: Vec<LayerId> = ctx.memory(|m| {
        m.layer_ids()
            .filter(|l| matches!(l.order, Order::Middle | Order::Foreground))
            .filter(|l| m.areas().visible_last_frame(l) && m.areas().parent_layer(*l).is_none())
            .collect()
    });
    stacked
        .into_iter()
        .rev()
        .find(|&l| is_expanded(ctx, l) && has_focusable(ctx, l))
        .unwrap_or_else(LayerId::background)
}

fn is_expanded(ctx: &egui::Context, layer: LayerId) -> bool {
    CollapsingState::load(ctx, layer.id.with("collapsing")).is_none_or(|s| s.is_open())
}

fn has_focusable(ctx: &egui::Context, layer: LayerId) -> bool {
    ctx.viewport(|v| {
        v.prev_pass
            .widgets
            .get_layer(layer)
            .any(|w| w.enabled && w.sense.is_focusable())
    })
}

/// egui's directional pick (±45° cone, distance weighted by alignment),
/// restricted to `candidates`.
fn nearest_in_direction(
    from: Id,
    from_rect: Rect,
    dir: Vec2,
    candidates: impl IntoIterator<Item = (Id, Rect)>,
) -> Option<Id> {
    fn range_diff(a: egui::Rangef, b: egui::Rangef) -> f32 {
        let overlaps = a.intersection(b).span() >= 0.5 * b.span().min(a.span());
        if overlaps {
            0.0
        } else {
            a.center() - b.center()
        }
    }

    candidates
        .into_iter()
        .filter(|(id, _)| *id != from)
        .filter_map(|(id, rect)| {
            let to = egui::vec2(
                range_diff(rect.x_range(), from_rect.x_range()),
                range_diff(rect.y_range(), from_rect.y_range()),
            );
            let cos = to.normalized().dot(dir);
            (cos >= 0.5_f32.sqrt()).then(|| (id, to.length() / (cos * cos)))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32) -> Rect {
        Rect::from_min_size(egui::pos2(x, y), egui::vec2(40.0, 20.0))
    }

    #[test]
    fn picks_closest_aligned_widget() {
        let from = Id::new("from");
        let below = Id::new("below");
        let far_below = Id::new("far_below");
        let right = Id::new("right");
        let candidates = [
            (from, rect(0.0, 0.0)),
            (below, rect(0.0, 30.0)),
            (far_below, rect(0.0, 90.0)),
            (right, rect(60.0, 0.0)),
        ];
        assert_eq!(
            nearest_in_direction(from, rect(0.0, 0.0), Vec2::DOWN, candidates),
            Some(below)
        );
        assert_eq!(
            nearest_in_direction(from, rect(0.0, 0.0), Vec2::RIGHT, candidates),
            Some(right)
        );
    }

    #[test]
    fn none_when_nothing_in_direction() {
        let from = Id::new("from");
        let candidates = [(from, rect(0.0, 0.0)), (Id::new("below"), rect(0.0, 30.0))];
        assert_eq!(
            nearest_in_direction(from, rect(0.0, 0.0), Vec2::UP, candidates),
            None
        );
    }

    #[test]
    fn fallback_picks_nearest_widget() {
        let near = Id::new("near");
        let far = Id::new("far");
        let candidates = [(far, rect(200.0, 0.0)), (near, rect(10.0, 40.0))];
        assert_eq!(nearest_by_distance(rect(0.0, 0.0), &candidates), Some(near));
        assert_eq!(nearest_by_distance(rect(0.0, 0.0), &[]), None);
    }
}
