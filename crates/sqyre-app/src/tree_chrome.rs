//! Macro tree row chrome: icon badge, pastel pills, swatches.

use crate::icon_cache::IconCache;
use crate::pickers::attach_item_icon_tooltip;
use crate::var_pills;
use eframe::egui::{self, Color32, FontId, Sense, Stroke, Vec2};
use sqyre_domain::{parse_hex_color, Action, ActionKind};
use sqyre_persist::ProgramCatalog;
use sqyre_ui_model::{action_icon_glyph, action_pastel_color, ActionDisplay, SummaryPill};
use std::collections::HashSet;

/// Icon badge edge length.
const ICON_SIZE: f32 = 20.0;
/// Glyph inside the type badge.
const ICON_GLYPH_SIZE: f32 = 14.0;
/// Pill label font (1px smaller than prior 13).
const PILL_FONT_SIZE: f32 = 12.0;
/// Pill inner padding (4×2).
const PILL_MARGIN_X: i8 = 4;
const PILL_MARGIN_Y: i8 = 2;
const PILL_RADIUS: f32 = 5.0;
/// Image Search target thumbnail: max height (width follows original aspect).
const TARGET_THUMB_MAX_H: f32 = 24.0;
/// Cap very wide icons so a row cannot grow unboundedly.
const TARGET_THUMB_MAX_W: f32 = 40.0;
const MAX_TARGET_THUMBS: usize = 8;

/// Default tree-row height (icon column + chrome), excluding image-search thumbs.
pub fn default_row_height(interact_y: f32) -> f32 {
    interact_y.max(ICON_SIZE)
}

/// Row height for a painted tree label (taller when image-search thumbs are shown).
pub fn action_row_height(action: &Action, interact_y: f32) -> f32 {
    let base = default_row_height(interact_y);
    if matches!(
        &action.kind,
        ActionKind::ImageSearch { targets, .. } if !targets.is_empty()
    ) {
        base.max(TARGET_THUMB_MAX_H + 4.0)
    } else {
        base
    }
}

/// Clickable chrome on a tree row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowAction {
    #[default]
    None,
    Logs,
    Delete,
}

/// Execution highlight overlay for a tree row.
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
    /// Pointer over the row, treating non-interactable tooltip layers as transparent
    /// (view tip clickthrough) but not other windows covering the tree.
    pub pointer_in_row: bool,
    pub secondary_clicked: bool,
    pub double_clicked: bool,
    pub primary_clicked: bool,
    /// Label content rect (for execution highlight follow / scroll).
    pub row_rect: egui::Rect,
    /// Union of the type icon + summary pills (and image-search thumbs). Tree DnD
    /// should only start when the press origin is inside this rect; the rest of
    /// the row is reserved for drag-to-scroll.
    pub drag_handle_rect: egui::Rect,
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
            drag_handle_rect: egui::Rect::NOTHING,
        }
    }
}

fn extend_drag_handle(acc: &mut egui::Rect, part: egui::Rect) {
    if part.width() <= 0.0 || part.height() <= 0.0 {
        return;
    }
    *acc = if *acc == egui::Rect::NOTHING {
        part
    } else {
        acc.union(part)
    };
}

pub(crate) fn rgba_pub(c: [u8; 4]) -> Color32 {
    crate::theme::rgba(c)
}

fn rgba(c: [u8; 4]) -> Color32 {
    crate::theme::rgba(c)
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

fn icon_btn(ui: &mut egui::Ui, glyph: &str, tip: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(glyph).size(14.0))
            .small()
            .frame(false),
    )
    .on_hover_text(tip)
}

/// Paint the pastel type badge with a glyph.
pub fn paint_action_icon(ui: &mut egui::Ui, action: &Action, is_dark: bool) -> egui::Response {
    let pastel = rgba(action_pastel_color(action.type_key(), is_dark));
    let size = Vec2::splat(ICON_SIZE);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
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
    resp
}

pub(crate) fn paint_pill_pub(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    paint_pill(ui, text, fill)
}

fn paint_pill(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    // Allocate through the parent layout (unlike Frame::show, which top-aligns in
    // available_rect and ignores Align::Center — that drifted pills below the tree icon).
    let fg = contrast_fg(fill);
    let font = FontId::proportional(PILL_FONT_SIZE);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, fg);
    let pad = Vec2::new(PILL_MARGIN_X as f32 * 2.0, PILL_MARGIN_Y as f32 * 2.0);
    let size = galley.size() + pad;
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        PILL_RADIUS,
        fill,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    paint_galley_centered(ui, rect, galley, fg);
    ui.interact(rect, ui.id().with(("action_pill", text)), Sense::hover())
}

/// Place galley so its ink (mesh bounds) is centered in `rect`.
fn paint_galley_centered(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    galley: std::sync::Arc<egui::Galley>,
    fallback: Color32,
) {
    let pos = if galley.mesh_bounds.is_positive() {
        // Optical center: baseline metrics make the layout box look top-heavy.
        rect.center() - galley.mesh_bounds.center().to_vec2()
    } else {
        egui::Align2::CENTER_CENTER
            .anchor_size(rect.center(), galley.size())
            .min
    };
    ui.painter().galley(pos, galley, fallback);
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

fn paint_color_swatch(ui: &mut egui::Ui, hex: &str) -> Option<egui::Response> {
    let c = parse_hex_color(hex)?;
    let fill = rgba(c);
    let size = Vec2::splat(16.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        3.0,
        fill,
        Stroke::new(1.0, Color32::from_gray(80)),
        egui::StrokeKind::Outside,
    );
    Some(resp)
}

fn paint_target_thumb(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    target: &str,
) -> egui::Response {
    let resp = if let Some(tex) = icons.for_target(ui.ctx(), catalog, target) {
        let [tw, th] = tex.size();
        let size = thumb_display_size(tw as f32, th as f32);
        ui.add(
            egui::Image::new((tex.id(), size))
                .fit_to_exact_size(size)
                .maintain_aspect_ratio(true)
                .corner_radius(3.0)
                .bg_fill(Color32::from_black_alpha(20)),
        )
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
        resp
    };
    attach_item_icon_tooltip(&resp, catalog, target);
    resp
}

fn paint_image_search_extras(
    ui: &mut egui::Ui,
    action: &Action,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    is_dark: bool,
    drag_handle: &mut egui::Rect,
) -> bool {
    let ActionKind::ImageSearch { targets, .. } = &action.kind else {
        return false;
    };
    let mut tip_hovered = false;
    let pastel = rgba(action_pastel_color(action.type_key(), is_dark));
    let count_pill = paint_pill(ui, &format!("🔍 {}", targets.len()), pastel);
    extend_drag_handle(drag_handle, count_pill.rect);
    if count_pill.hovered() {
        tip_hovered = true;
    }
    for target in targets.iter().take(MAX_TARGET_THUMBS) {
        let thumb = paint_target_thumb(ui, catalog, icons, target);
        extend_drag_handle(drag_handle, thumb.rect);
        if thumb.hovered() {
            tip_hovered = true;
        }
    }
    if targets.len() > MAX_TARGET_THUMBS {
        let overflow = paint_pill(
            ui,
            &format!("+{}", image_search_overflow_count(targets.len())),
            pastel,
        );
        extend_drag_handle(drag_handle, overflow.rect);
        if overflow.hovered() {
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
    ui.label(egui::RichText::new("Items").size(PILL_FONT_SIZE).strong());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(4.0);
        for target in targets {
            paint_target_thumb(ui, catalog, icons, target);
        }
    });
}

/// Right-edge space covered by a floating vertical scrollbar (egui default allocates 0).
fn floating_scrollbar_overlay_width(ui: &egui::Ui) -> f32 {
    let scroll = &ui.spacing().scroll;
    if scroll.floating {
        (scroll.bar_width - scroll.floating_allocated_width).max(0.0)
    } else {
        0.0
    }
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
    // Match TreeView node height; only image-search rows grow for thumbs.
    let row_h = action_row_height(action, ui.spacing().interact_size.y);

    // egui_ltreeview remembers a content-based min width, so `available_width` can stay
    // wider than the visible panel after the user shrinks the window. Cap the row to the
    // clip rect and anchor logs/delete on the visible right edge — inset further so a
    // floating scrollbar cannot cover the buttons.
    let spacing = ui.spacing().item_spacing.x;
    let visible_end = ui.clip_rect().right() - floating_scrollbar_overlay_width(ui);
    let row_start = ui.cursor().min.x;
    let max_visible_w = (visible_end - row_start).max(0.0);
    let row_w = ui.available_width().min(max_visible_w);

    let mut chrome_rect = egui::Rect::NOTHING;
    let mut drag_handle_rect = egui::Rect::NOTHING;
    let row = ui.allocate_ui_with_layout(
        Vec2::new(row_w, row_h),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = spacing;

            let del = icon_btn(ui, "🗑", "Delete");
            if del.contains_pointer() {
                chrome_hovered = true;
            }
            if del.clicked() {
                action_click = RowAction::Delete;
            }

            let logs = icon_btn(ui, "📋", "Logs");
            // contains_pointer: the full-row sense below steals `.hovered()` over these buttons.
            if logs.contains_pointer() {
                chrome_hovered = true;
            }
            if logs.clicked() {
                action_click = RowAction::Logs;
            }
            chrome_rect = logs.rect.union(del.rect);

            let content_w = ui.available_width().max(0.0);
            let (content_rect, _) =
                ui.allocate_exact_size(Vec2::new(content_w, row_h), Sense::hover());
            // new_child (not scope_builder): clipped overflow must not expand the row width.
            {
                let mut content = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(content_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                content.set_clip_rect(content_rect.intersect(ui.clip_rect()));
                content.spacing_mut().item_spacing.x = spacing;
                extend_drag_handle(
                    &mut drag_handle_rect,
                    paint_action_icon(&mut content, action, is_dark).rect,
                );

                for pill in action.tree_summary_pills() {
                    let resp = paint_summary_pill(&mut content, action, &pill, known_vars, is_dark);
                    extend_drag_handle(&mut drag_handle_rect, resp.rect);
                    if resp.hovered() {
                        tip_hovered = true;
                    }
                }

                if let ActionKind::FindPixel { target_color, .. } = &action.kind {
                    if let Some(swatch) = paint_color_swatch(&mut content, target_color) {
                        extend_drag_handle(&mut drag_handle_rect, swatch.rect);
                    }
                }
                if matches!(action.kind, ActionKind::ImageSearch { .. })
                    && paint_image_search_extras(
                        &mut content,
                        action,
                        catalog,
                        icons,
                        is_dark,
                        &mut drag_handle_rect,
                    )
                {
                    tip_hovered = true;
                }
            }
        },
    );

    paint_row_highlight(ui, row.response.rect, highlight);

    // Keep row click/hover sense off the logs/delete chrome so they win interaction.
    let mut sense_rect = row.response.rect;
    if chrome_rect.width() > 0.0 {
        sense_rect.max.x = sense_rect.max.x.min(chrome_rect.min.x);
    }

    // Hover-only: a Sense::click overlay would steal TreeView selection clicks.
    // Primary/secondary/double are read geometrically (with layer_id_at) so they
    // still fire when a view tip covers the row — clickthrough for overlays.
    let sense = ui.interact(
        sense_rect,
        ui.id().with(("action_row_sense", action.id)),
        Sense::hover(),
    );

    // Geometric hit through non-interactable tooltip layers (view tip is
    // interactable(false), so layer_id_at skips it) — keeps hover/clicks alive
    // when edge-constrain slides the tip over the row. Still blocked when
    // another Window/Area covers the tree (Settings, Data Editor, etc.).
    let our_layer = ui.layer_id();
    let pointer_in_row = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| {
        sense_rect.contains(p)
            && !chrome_rect.contains(p)
            && ui.ctx().layer_id_at(p) == Some(our_layer)
    });

    let over_row = pointer_in_row && action_click == RowAction::None;
    let primary_clicked = over_row && ui.input(|i| i.pointer.primary_clicked());
    let secondary_clicked =
        over_row && ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary));
    let double_clicked = over_row
        && ui.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        });

    let hovered = (row.response.hovered() || tip_hovered || sense.hovered() || pointer_in_row)
        && !chrome_hovered;
    RowInteraction {
        action: action_click,
        hovered: hovered && action_click == RowAction::None,
        pointer_in_row: over_row,
        secondary_clicked,
        double_clicked,
        primary_clicked,
        row_rect: row.response.rect,
        drag_handle_rect,
    }
}

/// Highlight fill colors for simple vs filled execution overlays.
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
    use sqyre_domain::{ActionId, ActionKind, CoordinateRef, DetectionBranch, ScalarValue};

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
    fn overflow_count() {
        assert_eq!(image_search_overflow_count(3), 0);
        assert_eq!(image_search_overflow_count(MAX_TARGET_THUMBS), 0);
        assert_eq!(image_search_overflow_count(MAX_TARGET_THUMBS + 3), 3);
    }

    #[test]
    fn row_height_only_grows_for_image_search_targets() {
        let interact = 18.0;
        assert_eq!(default_row_height(interact), ICON_SIZE);

        let wait = Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(100),
            },
        };
        assert_eq!(action_row_height(&wait, interact), ICON_SIZE);

        let empty_search = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: String::new(),
                targets: vec![],
                search_area: CoordinateRef(String::new()),
                tolerance: 0.9,
                blur: 0,
                detection: DetectionBranch::default(),
            },
        };
        assert_eq!(action_row_height(&empty_search, interact), ICON_SIZE);

        let with_target = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: String::new(),
                targets: vec!["a~b".into()],
                search_area: CoordinateRef(String::new()),
                tolerance: 0.9,
                blur: 0,
                detection: DetectionBranch::default(),
            },
        };
        assert_eq!(
            action_row_height(&with_target, interact),
            TARGET_THUMB_MAX_H + 4.0
        );
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
    fn paint_action_row_drag_handle_is_icon_and_pills_only() {
        with_ui(|ui| {
            let action = Action {
                id: ActionId::new(),
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(100),
                },
            };
            let catalog = ProgramCatalog::default();
            let mut icons = IconCache::new();
            // Wide row so empty space exists past the pills.
            let wide = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 40.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(wide), |ui| {
                ui.set_clip_rect(wide);
                let result = paint_action_row(
                    ui,
                    &action,
                    &catalog,
                    &mut icons,
                    &HashSet::new(),
                    false,
                    RowHighlight::None,
                );
                assert!(
                    result.drag_handle_rect.width() > 0.0 && result.drag_handle_rect.height() > 0.0,
                    "expected icon/pill drag handle, got {:?}",
                    result.drag_handle_rect
                );
                assert!(
                    result.drag_handle_rect.width() < result.row_rect.width() - 40.0,
                    "drag handle should leave empty row space for scroll (handle {} vs row {})",
                    result.drag_handle_rect.width(),
                    result.row_rect.width()
                );
            });
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
                    detection: DetectionBranch::default(),
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
                    tolerance: 0.9,
                    blur: 0,
                    detection: DetectionBranch::default(),
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
    fn image_search_row_keeps_chrome_within_narrow_width() {
        with_ui(|ui| {
            let narrow = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(180.0, 40.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(narrow), |ui| {
                ui.set_clip_rect(narrow);
                let result = paint_image_search_row(ui);
                assert!(
                    result.row_rect.width() <= narrow.width() + 1.0,
                    "row expanded past available width: {} > {}",
                    result.row_rect.width(),
                    narrow.width()
                );
                assert!(
                    result.row_rect.right() <= narrow.right() + 1.0,
                    "row chrome pushed past clip: {} > {}",
                    result.row_rect.right(),
                    narrow.right()
                );
            });
        });
    }

    #[test]
    fn image_search_row_chrome_visible_when_clip_narrower_than_available() {
        with_ui(|ui| {
            // Simulates egui_ltreeview keeping a wide row while the panel clip shrinks.
            let wide = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(520.0, 40.0));
            let clip = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(180.0, 40.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(wide), |ui| {
                ui.set_clip_rect(clip);
                let result = paint_image_search_row(ui);
                assert!(
                    result.row_rect.right() <= clip.right() + 1.0,
                    "logs/delete anchored off-screen: row right {} > clip right {}",
                    result.row_rect.right(),
                    clip.right()
                );
            });
        });
    }

    #[test]
    fn row_chrome_clears_floating_scrollbar_overlay() {
        with_ui(|ui| {
            // Default egui scroll style is floating with zero allocated width.
            assert!(ui.spacing().scroll.floating);
            assert_eq!(ui.spacing().scroll.floating_allocated_width, 0.0);
            let bar = ui.spacing().scroll.bar_width;
            let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(220.0, 40.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                ui.set_clip_rect(area);
                let result = paint_image_search_row(ui);
                assert!(
                    result.row_rect.right() <= area.right() - bar + 1.0,
                    "logs/delete under scrollbar: row right {} > clear edge {}",
                    result.row_rect.right(),
                    area.right() - bar
                );
            });
        });
    }

    fn paint_image_search_row(ui: &mut egui::Ui) -> RowInteraction {
        let mut targets = Vec::new();
        for i in 0..MAX_TARGET_THUMBS {
            targets.push(format!("Prog~WideItem{i}"));
        }
        let search = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets,
                search_area: CoordinateRef("P~Box".into()),
                tolerance: 0.9,
                blur: 0,
                detection: DetectionBranch::default(),
            },
        };
        paint_action_row(
            ui,
            &search,
            &ProgramCatalog::default(),
            &mut IconCache::new(),
            &HashSet::new(),
            true,
            RowHighlight::None,
        )
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
