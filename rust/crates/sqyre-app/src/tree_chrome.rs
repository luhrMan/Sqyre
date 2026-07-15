//! Macro tree row chrome: icon badge, pastel pills, swatches.

use crate::icon_cache::IconCache;
use crate::var_pills;
use eframe::egui::{self, Color32, FontId, Sense, Stroke, Vec2};
use sqyre_domain::{
    action_icon_glyph, action_pastel_color, parse_hex_color, Action, ActionKind, SummaryPill,
};
use sqyre_persist::ProgramCatalog;
use std::collections::HashSet;

/// Icon badge edge length.
const ICON_SIZE: f32 = 20.0;
/// Glyph inside the type badge.
const ICON_GLYPH_SIZE: f32 = 14.0;
/// Pill label font (1px smaller than prior 13).
const PILL_FONT_SIZE: f32 = 12.0;
/// Pill inner padding (1px tighter on X).
const PILL_MARGIN_X: i8 = 3;
const PILL_MARGIN_Y: i8 = 0;
const PILL_RADIUS: f32 = 5.0;
/// Image Search target thumbnail: max height (width follows original aspect).
const TARGET_THUMB_MAX_H: f32 = 36.0;
/// Cap very wide icons so a row cannot grow unboundedly.
const TARGET_THUMB_MAX_W: f32 = 64.0;
const MAX_TARGET_THUMBS: usize = 8;

/// Clickable chrome on a tree row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowAction {
    #[default]
    None,
    Logs,
    Delete,
}

/// Execution highlight overlay for a tree row (Go `highlightSimple` / `highlightFill`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RowHighlight {
    #[default]
    None,
    Cursor,
    Fill(f32),
}

/// Row hover / click signals for the action tooltip state machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowInteraction {
    pub action: RowAction,
    pub hovered: bool,
    /// Pointer geometrically over the row (ignores tooltip layers covering it).
    /// Used to keep the view tooltip open when edge-constrain flips the tip over the row.
    pub pointer_in_row: bool,
    pub secondary_clicked: bool,
    pub double_clicked: bool,
    pub primary_clicked: bool,
    /// Label content rect (for execution highlight follow / scroll).
    pub row_rect: egui::Rect,
}

impl Default for RowInteraction {
    fn default() -> Self {
        Self {
            action: RowAction::None,
            hovered: false,
            pointer_in_row: false,
            secondary_clicked: false,
            double_clicked: false,
            primary_clicked: false,
            row_rect: egui::Rect::NOTHING,
        }
    }
}

pub(crate) fn rgba_pub(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn rgba(c: [u8; 4]) -> Color32 {
    rgba_pub(c)
}

/// Foreground that contrasts with a pastel/solid fill (relative luminance).
pub(crate) fn contrast_fg(bg: Color32) -> Color32 {
    let lum = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if lum > 140.0 {
        Color32::from_rgb(30, 30, 30)
    } else {
        Color32::from_rgb(240, 240, 240)
    }
}

/// How many overflow targets to show as a `+N` pill (0 = none).
pub(crate) fn image_search_overflow_count(total: usize) -> usize {
    total.saturating_sub(MAX_TARGET_THUMBS)
}

/// Short label for `program~item` targets (item segment only).
pub(crate) fn target_short_name(target: &str) -> &str {
    target.rsplit('~').next().unwrap_or(target)
}

fn icon_btn(ui: &mut egui::Ui, glyph: &str, tip: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(glyph).size(14.0))
            .small()
            .frame(false),
    )
    .on_hover_text(tip)
}

/// Paint the pastel type badge with a glyph.
pub fn paint_action_icon(ui: &mut egui::Ui, action: &Action, is_dark: bool) {
    let pastel = rgba(action_pastel_color(action.type_key(), is_dark));
    let size = Vec2::splat(ICON_SIZE);
    let (rect, _resp) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        5.0,
        pastel,
        Stroke::new(1.0, pastel.gamma_multiply(0.7)),
        egui::StrokeKind::Outside,
    );
    let glyph = action_icon_glyph(action);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(ICON_GLYPH_SIZE),
        contrast_fg(pastel),
    );
}

pub(crate) fn paint_pill_pub(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    paint_pill(ui, text, fill)
}

fn paint_pill(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    let fg = contrast_fg(fill);
    let inner = egui::Frame::new()
        .fill(fill)
        .corner_radius(PILL_RADIUS)
        .inner_margin(egui::Margin::symmetric(PILL_MARGIN_X, PILL_MARGIN_Y))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .size(PILL_FONT_SIZE)
                    .color(fg),
            );
        });
    // Frame responses are not always hover-active; interact so tip tracks pills.
    ui.interact(
        inner.response.rect,
        ui.id().with(("action_pill", text)),
        Sense::hover(),
    )
}

fn paint_summary_pill(
    ui: &mut egui::Ui,
    action: &Action,
    pill: &SummaryPill,
    known: &HashSet<String>,
    is_dark: bool,
) -> egui::Response {
    var_pills::paint_summary_pill(ui, action.type_key(), pill, known, is_dark)
}

/// Fit texture into a max height/width box while keeping the source aspect ratio.
fn thumb_display_size(tex_w: f32, tex_h: f32) -> Vec2 {
    let h = tex_h.max(1.0);
    let w = tex_w.max(1.0);
    let mut out_h = TARGET_THUMB_MAX_H;
    let mut out_w = out_h * (w / h);
    if out_w > TARGET_THUMB_MAX_W {
        out_w = TARGET_THUMB_MAX_W;
        out_h = out_w * (h / w);
    }
    Vec2::new(out_w, out_h)
}

fn paint_color_swatch(ui: &mut egui::Ui, hex: &str) {
    let Some(c) = parse_hex_color(hex) else {
        return;
    };
    let fill = rgba(c);
    let size = Vec2::splat(16.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        3.0,
        fill,
        Stroke::new(1.0, Color32::from_gray(80)),
        egui::StrokeKind::Outside,
    );
}

fn paint_target_thumb(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    target: &str,
) -> egui::Response {
    if let Some(tex) = icons.for_target(ui.ctx(), catalog, target) {
        let [tw, th] = tex.size();
        let size = thumb_display_size(tw as f32, th as f32);
        ui.add(
            egui::Image::new((tex.id(), size))
                .fit_to_exact_size(size)
                .maintain_aspect_ratio(true)
                .corner_radius(3.0)
                .bg_fill(Color32::from_black_alpha(20)),
        )
        .on_hover_text(target_short_name(target))
    } else {
        // Placeholder when the PNG is missing (still aspect-neutral square).
        let size = Vec2::splat(TARGET_THUMB_MAX_H);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        ui.painter().rect(
            rect,
            3.0,
            Color32::from_gray(80),
            Stroke::new(1.0, Color32::from_gray(120)),
            egui::StrokeKind::Outside,
        );
        resp.on_hover_text(target_short_name(target))
    }
}

fn paint_image_search_extras(
    ui: &mut egui::Ui,
    action: &Action,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    is_dark: bool,
) -> bool {
    let ActionKind::ImageSearch { targets, .. } = &action.kind else {
        return false;
    };
    let mut tip_hovered = false;
    let pastel = rgba(action_pastel_color(action.type_key(), is_dark));
    if paint_pill(ui, &format!("🔍 {}", targets.len()), pastel).hovered() {
        tip_hovered = true;
    }
    for target in targets.iter().take(MAX_TARGET_THUMBS) {
        if paint_target_thumb(ui, catalog, icons, target).hovered() {
            tip_hovered = true;
        }
    }
    if targets.len() > MAX_TARGET_THUMBS {
        if paint_pill(
            ui,
            &format!("+{}", image_search_overflow_count(targets.len())),
            pastel,
        )
        .hovered()
        {
            tip_hovered = true;
        }
    }
    tip_hovered
}

pub(crate) fn paint_image_search_tooltip_thumbs_pub(
    ui: &mut egui::Ui,
    action: &Action,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
) {
    let ActionKind::ImageSearch { targets, .. } = &action.kind else {
        return;
    };
    if targets.is_empty() {
        return;
    }
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Items").size(PILL_FONT_SIZE).strong());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(4.0);
        for target in targets {
            paint_target_thumb(ui, catalog, icons, target);
        }
    });
}

/// Full tree-row label content. Tooltip show/hide is handled by `action_tooltip`.
pub fn paint_action_row(
    ui: &mut egui::Ui,
    action: &Action,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    known_vars: &HashSet<String>,
    is_dark: bool,
    highlight: RowHighlight,
) -> RowInteraction {
    let mut action_click = RowAction::None;
    let mut chrome_hovered = false;
    let mut tip_hovered = false;
    let row = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        paint_action_icon(ui, action, is_dark);

        for pill in action.tree_summary_pills() {
            if paint_summary_pill(ui, action, &pill, known_vars, is_dark).hovered() {
                tip_hovered = true;
            }
        }

        if let ActionKind::FindPixel { target_color, .. } = &action.kind {
            paint_color_swatch(ui, target_color);
        }
        if matches!(action.kind, ActionKind::ImageSearch { .. })
            && paint_image_search_extras(ui, action, catalog, icons, is_dark)
        {
            tip_hovered = true;
        }

        ui.add_space(4.0);

        let logs = icon_btn(ui, "📋", "Logs");
        // contains_pointer: the full-row sense below steals `.hovered()` over these buttons.
        if logs.contains_pointer() {
            chrome_hovered = true;
        }
        if logs.clicked() {
            action_click = RowAction::Logs;
        }

        let del = icon_btn(ui, "🗑", "Delete");
        if del.contains_pointer() {
            chrome_hovered = true;
        }
        if del.clicked() {
            action_click = RowAction::Delete;
        }

        logs.rect.union(del.rect)
    });

    paint_row_highlight(ui, row.response.rect, highlight);

    // Keep row click/hover sense off the logs/delete chrome so they win interaction.
    let mut sense_rect = row.response.rect;
    let chrome_rect = row.inner;
    if chrome_rect.width() > 0.0 {
        sense_rect.max.x = sense_rect.max.x.min(chrome_rect.min.x);
    }

    let sense = ui.interact(
        sense_rect,
        ui.id().with(("action_row_sense", action.id.as_str())),
        Sense::click(),
    );

    // Geometric hit: tooltip Order layers steal `.hovered()` when edge-constrain
    // slides the tip over the row (classic right-edge flicker).
    let pointer_in_row = ui
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| sense_rect.contains(p) && !chrome_rect.contains(p));

    let hovered = (row.response.hovered() || tip_hovered || sense.hovered()) && !chrome_hovered;
    RowInteraction {
        action: action_click,
        hovered: hovered && action_click == RowAction::None,
        pointer_in_row: pointer_in_row && action_click == RowAction::None,
        secondary_clicked: sense.secondary_clicked() && action_click == RowAction::None,
        double_clicked: sense.double_clicked() && action_click == RowAction::None,
        primary_clicked: sense.clicked() && action_click == RowAction::None,
        row_rect: row.response.rect,
    }
}

/// Go `highlightSimpleColor` / `highlightFillColor`.
fn highlight_cursor_color() -> Color32 {
    Color32::from_rgba_unmultiplied(90, 160, 240, 70)
}
fn highlight_fill_color() -> Color32 {
    Color32::from_rgba_unmultiplied(90, 200, 130, 90)
}

fn paint_row_highlight(ui: &mut egui::Ui, rect: egui::Rect, highlight: RowHighlight) {
    match highlight {
        RowHighlight::None => {}
        RowHighlight::Cursor => {
            ui.painter()
                .rect_filled(rect, 0.0, highlight_cursor_color());
        }
        RowHighlight::Fill(frac) => {
            let frac = frac.clamp(0.0, 1.0);
            let mut fill_rect = rect;
            fill_rect.max.x = fill_rect.min.x + fill_rect.width() * frac;
            ui.painter()
                .rect_filled(fill_rect, 0.0, highlight_fill_color());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{
        ActionId, ActionKind, CoordinateOutputs, CoordinateRef, ScalarValue, WaitTilFoundConfig,
    };

    fn with_ui(mut f: impl FnMut(&mut egui::Ui)) {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| f(ui));
    }

    #[test]
    fn contrast_fg_picks_dark_on_light_fill() {
        let light = Color32::from_rgb(200, 200, 200);
        let dark = Color32::from_rgb(20, 20, 20);
        assert_eq!(contrast_fg(light), Color32::from_rgb(30, 30, 30));
        assert_eq!(contrast_fg(dark), Color32::from_rgb(240, 240, 240));
    }

    #[test]
    fn overflow_and_short_name() {
        assert_eq!(image_search_overflow_count(3), 0);
        assert_eq!(image_search_overflow_count(MAX_TARGET_THUMBS), 0);
        assert_eq!(image_search_overflow_count(MAX_TARGET_THUMBS + 3), 3);
        assert_eq!(target_short_name("Prog~Sword"), "Sword");
        assert_eq!(target_short_name("noscale"), "noscale");
    }

    #[test]
    fn thumb_display_size_preserves_aspect() {
        let wide = thumb_display_size(64.0, 32.0);
        assert!((wide.x / wide.y - 2.0).abs() < 0.01);
        assert!(wide.y <= TARGET_THUMB_MAX_H + 0.01);

        let tall = thumb_display_size(16.0, 32.0);
        assert!((tall.x / tall.y - 0.5).abs() < 0.01);
        assert!((tall.y - TARGET_THUMB_MAX_H).abs() < 0.01);
    }

    #[test]
    fn paint_wait_row_smoke() {
        with_ui(|ui| {
            let action = Action {
                id: ActionId::new(),
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(100),
                },
            };
            let catalog = ProgramCatalog::default();
            let mut icons = IconCache::new();
            let result = paint_action_row(
                ui,
                &action,
                &catalog,
                &mut icons,
                &HashSet::new(),
                false,
                RowHighlight::None,
            );
            assert_eq!(result.action, RowAction::None);
        });
    }

    #[test]
    fn paint_find_pixel_and_image_search_smoke() {
        with_ui(|ui| {
            let catalog = ProgramCatalog::default();
            let mut icons = IconCache::new();
            let find = Action {
                id: ActionId::new(),
                kind: ActionKind::FindPixel {
                    name: "red".into(),
                    search_area: CoordinateRef("P~A".into()),
                    target_color: "#ff0000".into(),
                    color_tolerance: 5,
                    wait: WaitTilFoundConfig::default(),
                    coords: CoordinateOutputs::defaults(),
                    run_branch_on_no_find: false,
                    order: Default::default(),
                    subactions: vec![],
                },
            };
            assert_eq!(
                paint_action_row(
                    ui,
                    &find,
                    &catalog,
                    &mut icons,
                    &HashSet::new(),
                    true,
                    RowHighlight::None
                )
                .action,
                RowAction::None
            );

            let mut targets = Vec::new();
            for i in 0..(MAX_TARGET_THUMBS + 2) {
                targets.push(format!("Prog~Item{i}"));
            }
            let search = Action {
                id: ActionId::new(),
                kind: ActionKind::ImageSearch {
                    name: "find".into(),
                    targets,
                    search_area: CoordinateRef("P~Box".into()),
                    row_split: 0,
                    col_split: 0,
                    tolerance: 0.9,
                    blur: 0,
                    wait: WaitTilFoundConfig::default(),
                    coords: CoordinateOutputs::defaults(),
                    run_branch_on_no_find: false,
                    order: Default::default(),
                    subactions: vec![],
                },
            };
            assert_eq!(
                paint_action_row(
                    ui,
                    &search,
                    &catalog,
                    &mut icons,
                    &HashSet::new(),
                    false,
                    RowHighlight::None
                )
                .action,
                RowAction::None
            );
        });
    }

    #[test]
    fn paint_action_icon_smoke() {
        with_ui(|ui| {
            let action = Action {
                id: ActionId::new(),
                kind: ActionKind::Click {
                    button: "left".into(),
                    state: true,
                },
            };
            paint_action_icon(ui, &action, false);
        });
    }
}
