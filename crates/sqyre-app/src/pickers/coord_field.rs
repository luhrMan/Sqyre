//! Catalog coordinate row: kind-glyph pick button, program icon, monospace value.

use super::types::{ActivePicker, CoordKind};
use crate::icon_cache::IconCache;
use eframe::egui;
use sqyre_persist::ProgramCatalog;

/// Display options for [`coord_ref_field`]. Empty copy, color, truncation, and
/// the clear eraser stay with the caller — those already differ per surface.
pub struct CoordRefFieldOpts<'a> {
    pub icon: &'a str,
    pub pick_tip: &'a str,
    pub display: &'a str,
    /// Program whose icon sits in front of the value. `None` skips it.
    pub program: Option<&'a str>,
    /// Label color. `None` uses the current text color.
    pub color: Option<egui::Color32>,
    /// Truncate to the row and reserve a trailing slot (validation icon).
    pub truncate: bool,
    /// Red eraser. `None` hides it. Pass `None` while the value is empty.
    pub clear_tip: Option<&'a str>,
}

/// Pick button and optional clear eraser for one coordinate reference.
#[expect(
    clippy::too_many_arguments,
    reason = "catalog paint + picker + opts; hover stays a trailing closure"
)]
pub fn coord_ref_field(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    picker: &mut ActivePicker,
    kind: CoordKind,
    value: &str,
    opts: CoordRefFieldOpts<'_>,
    hover: impl FnOnce(&mut egui::Ui, &egui::Response),
) -> bool {
    let mut cleared = false;
    let mut hover = Some(hover);
    crate::widgets::pair_leading(
        ui,
        |ui| {
            if crate::widgets::entity_pick_button(ui, opts.icon, opts.pick_tip).clicked() {
                *picker = ActivePicker::Coord {
                    kind,
                    search: String::new(),
                    value: value.to_string(),
                    cell_pick: None,
                    scroll_to_selection: true,
                };
            }
        },
        |ui| {
            if let Some(prog) = opts.program {
                crate::icon_cache::paint_program_icon(ui, catalog, icons, prog);
            }
            let mut text = egui::RichText::new(opts.display).monospace();
            if let Some(color) = opts.color {
                text = text.color(color);
            }
            let resp = if opts.truncate {
                let trail = ui.spacing().interact_size.x + ui.spacing().item_spacing.x;
                let label_width = (ui.available_width() - trail).max(0.0);
                ui.scope(|ui| {
                    ui.set_max_width(label_width);
                    ui.add(
                        egui::Label::new(text)
                            .truncate()
                            .show_tooltip_when_elided(false),
                    )
                })
                .inner
            } else {
                ui.label(text)
            };
            // wrap_unit probes invisibly first — keep hover for the visible pass.
            if ui.is_visible() {
                if let Some(hover) = hover.take() {
                    hover(ui, &resp);
                }
            }
            if let Some(tip) = opts.clear_tip {
                if crate::widgets::clear_icon_button(ui, tip).clicked() {
                    cleared = true;
                }
            }
        },
    );
    cleared
}
