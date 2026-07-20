//! Framed tooltip sections with wrapping rows.
//!
//! # Conventions
//!
//! - [`tip_wrapped_section`] — compact labeled fields, combos, checkboxes, DragValues.
//! - [`tip_section`] — multiline editors, icon grids, previews, repeatable list editors.
//! - Field labels use `help::label` + control (not `DragValue.prefix`).
//! - Do not nest `ui.group` inside a tip section; list headers + rows (optional light
//!   per-item frame only for multi-line list items).

use crate::theme;
use eframe::egui::{self, Vec2};

/// Vertical gap between consecutive tip sections.
const SECTION_GAP: f32 = 4.0;

/// Sqyre-framed section (full-width vertical content).
pub fn tip_section(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    theme::framed_section(ui, SECTION_GAP, add_contents);
}

/// Framed section whose children flow left-to-right and wrap.
///
/// Prefer this for compact fields (pills, checkboxes, drag values, short labeled edits).
/// Multi-line editors and icon grids should use [`tip_section`] instead so they stay full-width.
pub fn tip_wrapped_section(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    tip_section(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
            add_contents(ui);
        });
    });
}
