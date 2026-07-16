//! Phosphor icon catalog for floating macro overlay buttons.

use eframe::egui::{self, Color32, FontFamily, FontId};
use egui_phosphor::regular::ICONS as PHOSPHOR_ICONS;
use std::sync::OnceLock;

/// One entry in the built-in overlay icon catalog.
#[derive(Debug, Clone, Copy)]
pub struct OverlayIcon {
    /// Stable kebab-case id stored in settings (e.g. `play`, `magnifying-glass`).
    pub id: &'static str,
    pub glyph: &'static str,
    pub label: &'static str,
}

/// Default icon when settings store an empty / unknown id.
pub const DEFAULT_ICON_ID: &str = "play";

const PHOSPHOR_FAMILY: &str = "phosphor";

fn phosphor_font_id(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(PHOSPHOR_FAMILY.into()))
}

fn to_kebab(screaming: &str) -> String {
    screaming.to_ascii_lowercase().replace('_', "-")
}

fn humanize(kebab: &str) -> String {
    let mut out = String::with_capacity(kebab.len() + 4);
    for (i, part) in kebab.split('-').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.extend(chars);
        }
    }
    out
}

fn build_catalog() -> Vec<OverlayIcon> {
    let mut out = Vec::with_capacity(PHOSPHOR_ICONS.len());
    for &(screaming, glyph) in PHOSPHOR_ICONS {
        let id = Box::leak(to_kebab(screaming).into_boxed_str());
        let label = Box::leak(humanize(id).into_boxed_str());
        out.push(OverlayIcon { id, glyph, label });
    }
    out
}

/// Full Phosphor Regular catalog (~1500 icons).
pub fn catalog() -> &'static [OverlayIcon] {
    static CATALOG: OnceLock<Vec<OverlayIcon>> = OnceLock::new();
    CATALOG.get_or_init(build_catalog).as_slice()
}

/// Resolve an icon id to a catalog entry (falls back to play).
pub fn resolve(id: &str) -> &'static OverlayIcon {
    let id = id.trim();
    let icons = catalog();
    let fallback = icons
        .iter()
        .find(|i| i.id == DEFAULT_ICON_ID)
        .or_else(|| icons.first());
    if id.is_empty() {
        return fallback.expect("phosphor overlay catalog is empty");
    }
    icons
        .iter()
        .find(|i| i.id == id)
        .or(fallback)
        .expect("phosphor overlay catalog is empty")
}

/// Ensure the Phosphor font is registered for overlay glyph rendering.
///
/// Call after [`egui_phosphor::add_to_fonts`] so `font_data["phosphor"]` exists.
pub fn register_phosphor_family(fonts: &mut egui::FontDefinitions) {
    if !fonts.font_data.contains_key(PHOSPHOR_FAMILY) {
        return;
    }
    fonts.families.insert(
        FontFamily::Name(PHOSPHOR_FAMILY.into()),
        vec![PHOSPHOR_FAMILY.to_owned()],
    );
}

/// Paint a Phosphor glyph with Sqyre yellow chrome (floating overlay buttons).
pub fn paint_glyph_bare(ui: &mut egui::Ui, icon: &OverlayIcon, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    paint_overlay_chrome(ui, rect, response.hovered());
    let color = if response.hovered() {
        crate::theme::PRIMARY
    } else {
        Color32::from_rgb(0xf5, 0xe6, 0xc0)
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.glyph,
        phosphor_font_id((size * 0.55).round()),
        color,
    );
    response
}

/// Compact preview button used in the editor / picker (with selection chrome).
pub fn icon_glyph_button(
    ui: &mut egui::Ui,
    icon: &OverlayIcon,
    selected: bool,
    size: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    paint_picker_chrome(ui, rect, selected, response.hovered());
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.glyph,
        phosphor_font_id((size * 0.48).round()),
        crate::theme::PRIMARY,
    );
    response.on_hover_text(icon.label)
}

fn paint_overlay_chrome(ui: &mut egui::Ui, rect: egui::Rect, hovered: bool) {
    // No fill — transparent viewport clear + gold stroke only (avoids a dark plate).
    let stroke = egui::Stroke::new(
        if hovered { 2.0 } else { 1.5 },
        crate::theme::PRIMARY,
    );
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        stroke,
        egui::StrokeKind::Outside,
    );
}

fn paint_picker_chrome(ui: &mut egui::Ui, rect: egui::Rect, selected: bool, hovered: bool) {
    let fill = if selected {
        crate::theme::accent_dim()
    } else if hovered {
        crate::theme::chip_fill()
    } else {
        crate::theme::frame_fill()
    };
    let stroke = if selected || hovered {
        egui::Stroke::new(1.5, crate::theme::PRIMARY)
    } else {
        egui::Stroke::new(1.0, crate::theme::PRIMARY)
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(8), fill);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        stroke,
        egui::StrokeKind::Outside,
    );
}

/// Searchable grid of Phosphor icons; returns the newly selected id when clicked.
pub fn show_icon_picker_grid(
    ui: &mut egui::Ui,
    current_id: &str,
    search: &mut String,
) -> Option<&'static str> {
    ui.horizontal(|ui| {
        ui.label("Search");
        ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(220.0)
                .hint_text("e.g. play, lightning, mouse"),
        );
        if !search.is_empty() && ui.small_button("Clear").clicked() {
            search.clear();
        }
    });
    ui.add_space(4.0);

    let query = search.trim().to_ascii_lowercase();
    let icons: Vec<&OverlayIcon> = if query.is_empty() {
        catalog().iter().collect()
    } else {
        catalog()
            .iter()
            .filter(|i| i.id.contains(&query) || i.label.to_ascii_lowercase().contains(&query))
            .collect()
    };

    ui.weak(format!("{} icons", icons.len()));
    ui.add_space(4.0);

    let mut picked = None;
    let cols = 10;
    egui::ScrollArea::vertical()
        .max_height(360.0)
        .show(ui, |ui| {
            egui::Grid::new("overlay_icon_picker_grid")
                .num_columns(cols)
                .spacing([6.0, 6.0])
                .show(ui, |ui| {
                    for (i, icon) in icons.iter().enumerate() {
                        let selected = icon.id == current_id
                            || (current_id.trim().is_empty() && icon.id == DEFAULT_ICON_ID);
                        if icon_glyph_button(ui, icon, selected, 32.0).clicked() {
                            picked = Some(icon.id);
                        }
                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_and_fallback() {
        assert_eq!(resolve("play").id, "play");
        assert_eq!(resolve("lightning").id, "lightning");
        assert_eq!(resolve("magnifying-glass").id, "magnifying-glass");
        assert_eq!(resolve("").id, DEFAULT_ICON_ID);
        assert_eq!(resolve("nope").id, DEFAULT_ICON_ID);
    }

    #[test]
    fn catalog_ids_unique_and_nonempty() {
        let icons = catalog();
        assert!(icons.len() > 1000);
        let mut seen = std::collections::BTreeSet::new();
        for icon in icons {
            assert!(!icon.id.is_empty());
            assert!(!icon.glyph.is_empty());
            assert!(seen.insert(icon.id), "duplicate icon id {}", icon.id);
        }
        assert!(seen.contains(DEFAULT_ICON_ID));
    }

    #[test]
    fn kebab_and_humanize() {
        assert_eq!(to_kebab("MAGNIFYING_GLASS"), "magnifying-glass");
        assert_eq!(humanize("magnifying-glass"), "Magnifying Glass");
    }
}
