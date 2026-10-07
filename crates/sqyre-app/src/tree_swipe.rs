//! Macro-tree row swipe: right deletes, left opens the editor.
//!
//! The row slides a short way and reveals an action icon in the gap; release
//! eases it back (cancel / edit) or off the pane (delete) before the delete applies.

use crate::theme;
use eframe::egui::{self, emath::TSTransform, Vec2};
use sqyre_domain::ActionId;

/// A drag counts as a swipe when |dx| exceeds this multiple of |dy|.
const AXIS_RATIO: f32 = 2.0;
/// Finger travel (points) that arms the swipe action.
const COMMIT_DIST: f32 = 40.0;
/// Row travel (points) before the drag starts to resist.
const RESIST_AT: f32 = 56.0;
/// Share of finger travel applied past the resist point.
const RESIST_GAIN: f32 = 0.25;
/// Exponential approach rate (1/s) for snap-back and slide-out.
const SETTLE_RATE: f32 = 22.0;
/// Seconds for the armed icon pop.
const ARM_TIME: f32 = 0.12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwipeAction {
    Delete,
    Edit,
}

impl SwipeAction {
    fn for_offset(dx: f32) -> Self {
        if dx > 0.0 {
            Self::Delete
        } else {
            Self::Edit
        }
    }

    fn color(self) -> egui::Color32 {
        match self {
            Self::Delete => theme::MACRO_STOP,
            Self::Edit => theme::PRIMARY,
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Self::Delete => egui_phosphor::regular::TRASH,
            Self::Edit => egui_phosphor::regular::PENCIL_SIMPLE,
        }
    }
}

/// Whether drag travel since press is horizontal enough to be a row swipe.
pub(crate) fn is_swipe(travel: Vec2) -> bool {
    travel.x.abs() > AXIS_RATIO * travel.y.abs()
}

/// Right swipe deletes, left swipe edits; shorter drags cancel.
pub(crate) fn outcome(dx: f32) -> Option<SwipeAction> {
    (dx.abs() >= COMMIT_DIST).then(|| SwipeAction::for_offset(dx))
}

/// Row offset for finger travel `dx`: 1:1 up to the resist point, then damped.
pub(crate) fn rubber_band(dx: f32) -> f32 {
    let mag = dx.abs();
    let out = if mag <= RESIST_AT {
        mag
    } else {
        RESIST_AT + (mag - RESIST_AT) * RESIST_GAIN
    };
    out.copysign(dx)
}

/// Row easing toward `target` after release; deletes on arrival when `delete`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SwipeSettle {
    pub(crate) aid: ActionId,
    pub(crate) offset: f32,
    pub(crate) target: f32,
    pub(crate) delete: bool,
}

impl SwipeSettle {
    /// Advance by `dt` seconds; true once the row has arrived.
    pub(crate) fn step(&mut self, dt: f32) -> bool {
        let k = 1.0 - (-SETTLE_RATE * dt).exp();
        self.offset += (self.target - self.offset) * k;
        if (self.target - self.offset).abs() < 0.5 {
            self.offset = self.target;
            return true;
        }
        false
    }
}

/// Paint `add_row` shifted by `offset`, with the action icon in the revealed gap.
pub(crate) fn paint_row<R>(
    ui: &mut egui::Ui,
    aid: ActionId,
    row: egui::Rect,
    offset: f32,
    add_row: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if offset.abs() < 0.5 || !row.is_positive() {
        return add_row(ui);
    }
    paint_icon(ui, aid, row, offset);
    let clip = ui.clip_rect();
    let shift = egui::vec2(offset, 0.0);
    ui.with_visual_transform(TSTransform::from_translation(shift), |ui| {
        // Pre-shift the clip so the moved row stays inside the pane.
        ui.set_clip_rect(clip.translate(-shift));
        let out = add_row(ui);
        ui.set_clip_rect(clip);
        out
    })
    .inner
}

fn paint_icon(ui: &egui::Ui, aid: ActionId, row: egui::Rect, offset: f32) {
    let action = SwipeAction::for_offset(offset);
    let armed = ui.ctx().animate_bool_with_time_and_easing(
        egui::Id::new(("tree_swipe_armed", aid)),
        outcome(offset).is_some(),
        ARM_TIME,
        egui::emath::easing::cubic_out,
    );
    let gap = offset.abs().min(row.width());
    let reveal = (gap / COMMIT_DIST).clamp(0.0, 1.0);
    let side = row.height().min(gap);
    if side < 2.0 {
        return;
    }
    let center = match action {
        SwipeAction::Delete => egui::pos2(row.min.x + gap / 2.0, row.center().y),
        SwipeAction::Edit => egui::pos2(row.max.x - gap / 2.0, row.center().y),
    };
    let painter = ui.painter().with_clip_rect(row.intersect(ui.clip_rect()));
    let color = action.color();
    // Filled badge pops in once armed; before that only the tinted glyph shows.
    let radius = side * 0.45 * (0.8 + 0.2 * armed);
    if armed > 0.0 {
        painter.circle_filled(center, radius, color.gamma_multiply(armed));
    }
    let fg = theme::contrast_fg(color)
        .lerp_to_gamma(color, 1.0 - armed)
        .gamma_multiply(0.35 + 0.65 * reveal);
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        action.glyph(),
        egui::FontId::proportional(radius * 1.25),
        fg,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_deletes_left_edits_short_cancels() {
        assert_eq!(outcome(COMMIT_DIST), Some(SwipeAction::Delete));
        assert_eq!(outcome(-COMMIT_DIST), Some(SwipeAction::Edit));
        assert_eq!(outcome(COMMIT_DIST - 1.0), None);
        assert_eq!(outcome(-(COMMIT_DIST - 1.0)), None);
    }

    #[test]
    fn horizontal_drag_is_swipe_vertical_is_not() {
        assert!(is_swipe(egui::vec2(20.0, 5.0)));
        assert!(!is_swipe(egui::vec2(10.0, 8.0)));
        assert!(!is_swipe(egui::vec2(0.0, 0.0)));
    }

    #[test]
    fn rubber_band_is_linear_then_damped_and_keeps_sign() {
        assert_eq!(rubber_band(30.0), 30.0);
        assert_eq!(rubber_band(-30.0), -30.0);
        assert!(rubber_band(RESIST_AT + 100.0) < RESIST_AT + 100.0);
        assert!(rubber_band(-(RESIST_AT + 100.0)) > -(RESIST_AT + 100.0));
    }

    #[test]
    fn settle_converges_to_target() {
        let mut s = SwipeSettle {
            aid: ActionId::new(),
            offset: 60.0,
            target: 0.0,
            delete: false,
        };
        assert!((0..120).any(|_| s.step(1.0 / 60.0)));
        assert_eq!(s.offset, 0.0);
    }
}
