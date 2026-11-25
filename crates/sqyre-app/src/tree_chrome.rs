//! Macro tree row chrome: icon badge, pastel pills, swatches.

use crate::icon_cache::IconCache;
use crate::paint_ctx::VarTheme;
use crate::paint_metrics;
use crate::theme::{
    highlight_cursor_fill, highlight_invalid_fill, highlight_owner_fill, highlight_progress_fill,
    paint_galley_centered,
};
use crate::var_pills;
use crate::widgets::elide::{single_line_galley, text_hover_if};
use eframe::egui::{self, Color32, Sense, Stroke, Vec2};
use sqyre_domain::{
    action_type_label, parse_hex_color, Action, ActionId, ActionKind, KnownVariableNames,
};
use sqyre_persist::CatalogView;
use sqyre_ui_model::{action_icon_glyph, action_pastel_color, ActionDisplay, SummaryPill};
use std::collections::HashMap;

mod image_search;
use image_search::paint_image_search_extras;
pub(crate) use image_search::{
    cached_sorted_image_search_targets, image_search_tip_tag_groups,
    paint_image_search_tooltip_thumbs_pub,
};

use paint_metrics::ICON_SIZE;
/// Pill inner padding (4×2).
const PILL_MARGIN_X: i8 = crate::theme::SPACE_4 as i8;
const PILL_MARGIN_Y: i8 = crate::theme::SPACE_2 as i8;
const PILL_RADIUS: f32 = 5.0;
/// Item thumbnail max height — match action cell so rows stay uniform.
const TARGET_THUMB_MAX_H: f32 = ICON_SIZE;
/// Cap very wide icons so a row cannot grow unboundedly.
const TARGET_THUMB_MAX_W: f32 = 30.0;
/// Tight gap between image-search thumbs (default item_spacing leaves them sparse).
const TARGET_THUMB_GAP: f32 = 2.0;

/// Row height for a painted tree label (item thumbs fit in the standard band).
pub fn action_row_height(_action: &Action, interact_y: f32) -> f32 {
    paint_metrics::default_row_height(interact_y)
}

fn type_badge_side(ui: &egui::Ui) -> f32 {
    ui.text_style_height(&egui::TextStyle::Small).max(ICON_SIZE)
}

/// Entry picked from a tree row's right-click menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowAction {
    #[default]
    None,
    Edit,
    Logs,
    Delete,
}

/// Execution / related-selection highlight overlay for a tree row.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RowHighlight {
    #[default]
    None,
    Cursor,
    Fill(f32),
    /// Parent of the selected Else folder, or the opposite key/click press —
    /// softer than TreeView selection.
    Owner,
}

/// Row hover / click signals for the action tooltip state machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowInteraction {
    pub action: RowAction,
    pub hovered: bool,
    /// Pointer over the row, treating non-interactable tooltip layers as transparent
    /// (view tip clickthrough) but not other windows covering the tree.
    pub pointer_in_row: bool,
    pub double_clicked: bool,
    /// Middle button released over the row. Opens the edit tooltip.
    pub middle_clicked: bool,
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
            double_clicked: false,
            middle_clicked: false,
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

fn rgba(c: [u8; 4]) -> Color32 {
    crate::theme::rgba(c)
}

/// Foreground that contrasts with a pastel/solid fill (relative luminance).
pub(crate) use crate::theme::contrast_fg;

/// Paint the pastel type badge with a glyph.
pub fn paint_action_icon(ui: &mut egui::Ui, action: &Action, is_dark: bool) -> egui::Response {
    let type_key = action.type_key();
    let label = action_type_label(type_key);
    let pastel = rgba(action_pastel_color(type_key, is_dark));
    let font = egui::TextStyle::Small.resolve(ui.style());
    let size = Vec2::splat(type_badge_side(ui));
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        5.0,
        pastel,
        Stroke::new(1.0, pastel.gamma_multiply(0.7)),
        egui::StrokeKind::Outside,
    );
    let fg = contrast_fg(pastel);
    let side = crate::widgets::action_icon_side(font.size).min(rect.width() - 2.0);
    if let Some(icon) = crate::widgets::VectorIcon::for_action(action) {
        let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(side));
        icon.paint(ui.painter(), icon_rect, fg);
    } else {
        let glyph_font = crate::widgets::action_glyph_font(side, font.family);
        crate::theme::paint_text_centered(ui, rect, action_icon_glyph(action), glyph_font, fg);
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, label));
    resp.on_hover_text(label)
}

pub(crate) fn paint_pill_pub(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    paint_pill(ui, text, fill, true)
}

pub(crate) fn paint_pill_tip(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    let fg = contrast_fg(fill);
    let font = egui::TextStyle::Small.resolve(ui.style());
    let full = ui.painter().layout_no_wrap(text.to_owned(), font, fg);
    let pad = Vec2::new(PILL_MARGIN_X as f32 * 2.0, PILL_MARGIN_Y as f32 * 2.0);
    if full.size().x + pad.x > ui.available_width() {
        return egui::Frame::new()
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(PILL_RADIUS as u8))
            .inner_margin(egui::Margin::symmetric(PILL_MARGIN_X, PILL_MARGIN_Y))
            .show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(text).color(fg)).wrap())
            })
            .response;
    }
    paint_pill(ui, text, fill, false)
}

fn paint_pill(ui: &mut egui::Ui, text: &str, fill: Color32, show_hover: bool) -> egui::Response {
    // Allocate through the parent layout (unlike Frame::show, which top-aligns in
    // available_rect and ignores Align::Center — that drifted pills below the tree icon).
    let fg = contrast_fg(fill);
    let font = egui::TextStyle::Small.resolve(ui.style());
    let pad = Vec2::new(PILL_MARGIN_X as f32 * 2.0, PILL_MARGIN_Y as f32 * 2.0);
    let max_width = (ui.clip_rect().right() - ui.cursor().left() - pad.x).max(0.0);
    let full = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), fg);
    let galley = if full.size().x > max_width {
        single_line_galley(ui, text, font, fg, max_width)
    } else {
        full
    };
    let size = galley.size() + pad;
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        PILL_RADIUS,
        fill,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    let was_elided = galley.elided;
    paint_galley_centered(ui, rect, galley, fg);
    text_hover_if(response, text, show_hover && was_elided)
}

/// Leading program/window icon + summary pill, vertically centered in a `band_h` cell.
///
/// `ui.horizontal` alone starts at `interact_size.y`; when the pill is taller, the
/// parent row grows after the first cell and later cells center lower (wrapped
/// tooltip rows) or overflow the row clip (tree rows).
#[expect(
    clippy::too_many_arguments,
    reason = "summary pill cell carries rendering context and layout options"
)]
pub(crate) fn paint_summary_pill_cell(
    ui: &mut egui::Ui,
    action: &Action,
    pill: &SummaryPill,
    catalog: CatalogView<'_>,
    icons: &mut IconCache,
    theme: VarTheme<'_>,
    band_h: f32,
    view_tip: bool,
) -> egui::Response {
    let prev_h = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size.y = band_h;
    let resp = crate::widgets::wrap_unit(ui, |ui| {
        ui.spacing_mut().interact_size.y = prev_h;
        if let ActionKind::FocusWindow {
            process_path,
            window_title,
            previous: false,
        } = &action.kind
        {
            crate::icon_cache::paint_leading_process_icon(ui, icons, process_path, window_title);
        } else {
            crate::icon_cache::paint_leading_program_icon(ui, catalog, icons, &pill.text);
        }
        if view_tip {
            var_pills::paint_summary_pill_tip(
                ui,
                action.type_key(),
                pill,
                theme.known_vars,
                theme.is_dark,
            )
        } else {
            let budget = (ui.clip_rect().right() - ui.cursor().left()).max(0.0);
            var_pills::paint_summary_pill_limited(
                ui,
                action.type_key(),
                pill,
                theme.known_vars,
                theme.is_dark,
                budget,
                false,
            )
        }
    })
    .response;
    ui.spacing_mut().interact_size.y = prev_h;
    resp
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
        crate::theme::inner_stroke(),
        egui::StrokeKind::Outside,
    );
    Some(resp)
}

/// Full tree-row label content. Tooltip show/hide is handled by `action_tooltip`.
#[expect(
    clippy::too_many_arguments,
    reason = "row paint: catalog, theme, highlight, pills cache, and chrome flags"
)]
pub fn paint_action_row(
    ui: &mut egui::Ui,
    action: &Action,
    catalog: CatalogView<'_>,
    icons: &mut IconCache,
    known_vars: &KnownVariableNames,
    is_dark: bool,
    highlight: RowHighlight,
    pills_cache: &mut HashMap<ActionId, (u64, Vec<SummaryPill>)>,
    paint_revision: u64,
    validation_error: Option<&str>,
    show_logs: bool,
) -> RowInteraction {
    let mut tip_hovered = false;
    // Match TreeView node height.
    let interact_y = paint_metrics::row_height(ui);
    let row_h = action_row_height(action, interact_y);

    // egui_ltreeview remembers a content-based min width, so `available_width` can stay
    // wider than the visible panel after the user shrinks the window. Cap the row to the
    // clip rect so chrome stays on-screen. Floating scrollbars overlay the edge (see
    // `theme::scroll_style`) — do not reserve a gutter that reads as gray padding.
    let spacing = ui.spacing().item_spacing.x;
    let visible_end = ui.clip_rect().right();
    let row_start = ui.cursor().min.x;
    let max_visible_w = (visible_end - row_start).max(0.0);
    let row_w = ui.available_width().min(max_visible_w);

    let mut drag_handle_rect = egui::Rect::NOTHING;
    let row = ui.allocate_ui_with_layout(
        Vec2::new(row_w, row_h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
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

                // Same vertical center as the summary pills. `top_down` packs the
                // badge to the top of a row that is taller than the icon.
                content.allocate_ui_with_layout(
                    Vec2::new(type_badge_side(&content), row_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        extend_drag_handle(
                            &mut drag_handle_rect,
                            paint_action_icon(ui, action, is_dark).rect,
                        );
                    },
                );

                let pills_id = ui.id().with(("action_row_pills", action.id));
                let pills_overflow = ui
                    .ctx()
                    .data(|d| d.get_temp::<bool>(pills_id))
                    .unwrap_or(false);
                content.allocate_ui_with_layout(
                    Vec2::new(content.available_width().max(0.0), row_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        // Same pills as before when they fit. A row that ran past the
                        // clip last frame scrolls inside this slot so the tree width
                        // stays capped (egui_ltreeview ratchets on content min-width).
                        let mut paint = |ui: &mut egui::Ui| {
                            ui.spacing_mut().item_spacing.x = spacing;
                            let pills = {
                                let rev = paint_revision;
                                let id = action.id;
                                let stale = pills_cache
                                    .get(&id)
                                    .map(|(cached_rev, _)| *cached_rev != rev)
                                    .unwrap_or(true);
                                if stale {
                                    pills_cache.insert(id, (rev, action.tree_summary_pills()));
                                }
                                pills_cache
                                    .get(&id)
                                    .map(|(_, p)| p.as_slice())
                                    .unwrap_or(&[])
                            };
                            for pill in pills {
                                let resp = paint_summary_pill_cell(
                                    ui,
                                    action,
                                    pill,
                                    catalog,
                                    icons,
                                    VarTheme {
                                        known_vars,
                                        is_dark,
                                    },
                                    row_h,
                                    false,
                                );
                                extend_drag_handle(&mut drag_handle_rect, resp.rect);
                                if resp.hovered() {
                                    tip_hovered = true;
                                }
                            }

                            if let ActionKind::FindPixel(d) = &action.kind {
                                if let Some(swatch) = paint_color_swatch(ui, &d.target_color) {
                                    extend_drag_handle(&mut drag_handle_rect, swatch.rect);
                                }
                            }
                            if matches!(action.kind, ActionKind::ImageSearch(_))
                                && paint_image_search_extras(
                                    ui,
                                    action,
                                    catalog,
                                    icons,
                                    is_dark,
                                    &mut drag_handle_rect,
                                )
                            {
                                tip_hovered = true;
                            }

                            if let Some(err) = validation_error {
                                let badge = crate::widgets::error_icon(ui, err);
                                extend_drag_handle(&mut drag_handle_rect, badge.rect);
                                if badge.hovered() {
                                    tip_hovered = true;
                                }
                            }
                        };
                        let overflow_now = if pills_overflow {
                            let width = ui.available_width().max(1.0);
                            let out = egui::ScrollArea::horizontal()
                                .id_salt(pills_id)
                                .scroll_source(crate::widgets::SCROLL_SOURCE_NO_DRAG)
                                .auto_shrink([false, false])
                                .min_scrolled_width(1.0)
                                .max_width(width)
                                .max_height(row_h)
                                .show(ui, |ui| {
                                    ui.with_layout(
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            crate::widgets::enable_dense_row_extend(ui);
                                            paint(ui);
                                        },
                                    );
                                });
                            out.content_size.x > width + 8.0
                        } else {
                            paint(ui);
                            ui.min_rect().right() > ui.max_rect().right() + 8.0
                        };
                        ui.ctx().data_mut(|d| d.insert_temp(pills_id, overflow_now));
                    },
                );
            }
        },
    );

    if validation_error.is_some() {
        ui.painter()
            .rect_filled(row.response.rect, 0.0, highlight_invalid_fill());
    }
    paint_row_highlight(ui, row.response.rect, highlight);

    let sense_rect = row.response.rect;

    // Hover-only: a Sense::click overlay would steal TreeView selection clicks.
    // Primary/secondary/middle/double are read geometrically (with layer_id_at)
    // so they still fire when a view tip covers the row — clickthrough for overlays.
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
    let pointer_in_row = ui
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| sense_rect.contains(p) && ui.ctx().layer_id_at(p) == Some(our_layer));

    let menu_id = egui::Id::new(("action_row_menu", our_layer, action.id));
    let action_click =
        crate::widgets::rect_context_menu(ui, menu_id, sense_rect, Some(&sense), |ui| {
            if crate::widgets::menu_item(ui, "Edit", true) {
                return RowAction::Edit;
            }
            if show_logs && crate::widgets::menu_item(ui, "Logs", true) {
                return RowAction::Logs;
            }
            if crate::widgets::menu_item_danger(ui, "Delete", true) {
                return RowAction::Delete;
            }
            RowAction::None
        })
        .unwrap_or_default();

    let over_row = pointer_in_row && action_click == RowAction::None;
    let primary_clicked = over_row && ui.input(|i| i.pointer.primary_clicked());
    let middle_clicked =
        over_row && ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Middle));
    let double_clicked = over_row
        && ui.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        });

    let hovered = row.response.hovered() || tip_hovered || sense.hovered() || pointer_in_row;
    RowInteraction {
        action: action_click,
        hovered: hovered && action_click == RowAction::None,
        pointer_in_row: over_row,
        double_clicked,
        middle_clicked,
        primary_clicked,
        row_rect: row.response.rect,
        drag_handle_rect,
    }
}

pub(crate) fn paint_row_highlight(ui: &mut egui::Ui, rect: egui::Rect, highlight: RowHighlight) {
    match highlight {
        RowHighlight::None => {}
        RowHighlight::Cursor => {
            ui.painter().rect_filled(rect, 0.0, highlight_cursor_fill());
        }
        RowHighlight::Fill(frac) => {
            let frac = frac.clamp(0.0, 1.0);
            let mut fill_rect = rect;
            fill_rect.max.x = fill_rect.min.x + fill_rect.width() * frac;
            ui.painter()
                .rect_filled(fill_rect, 0.0, highlight_progress_fill());
        }
        RowHighlight::Owner => {
            ui.painter().rect_filled(rect, 0.0, highlight_owner_fill());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_view;
    use sqyre_domain::{ActionKind, CoordinateRef, PressState, ScalarValue};
    use sqyre_persist::{CatalogView, ProgramCatalog};

    fn with_ui(mut f: impl FnMut(&mut egui::Ui)) {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| f(ui))
            .drop_without_applying_deltas();
    }

    #[test]
    fn contrast_fg_picks_dark_on_light_fill() {
        let light = Color32::from_rgb(200, 200, 200);
        let dark = Color32::from_rgb(20, 20, 20);
        assert_eq!(contrast_fg(light), Color32::from_rgb(30, 30, 30));
        assert_eq!(contrast_fg(dark), Color32::from_rgb(240, 240, 240));
    }

    #[test]
    fn row_height_stays_uniform_with_image_search_targets() {
        let interact = 18.0;
        assert_eq!(paint_metrics::default_row_height(interact), ICON_SIZE);

        let wait = Action::new(ActionKind::Wait {
            time: ScalarValue::Int(100),
        });
        assert_eq!(action_row_height(&wait, interact), ICON_SIZE);

        let empty_search = Action::new(ActionKind::ImageSearch(Box::new(
            sqyre_domain::ImageSearchData {
                search_area: CoordinateRef(String::new()),
                tolerance: 0.9,
                ..Default::default()
            },
        )));
        assert_eq!(action_row_height(&empty_search, interact), ICON_SIZE);

        let with_target = Action::new(ActionKind::ImageSearch(Box::new(
            sqyre_domain::ImageSearchData {
                targets: vec!["a~b".into()],
                search_area: CoordinateRef(String::new()),
                tolerance: 0.9,
                ..Default::default()
            },
        )));
        assert_eq!(action_row_height(&with_target, interact), ICON_SIZE);
        const { assert!(TARGET_THUMB_MAX_H <= ICON_SIZE) };
    }

    #[test]
    fn thumb_display_size_preserves_aspect() {
        let wide = image_view::fit_icon_thumb(64.0, 32.0, TARGET_THUMB_MAX_W, TARGET_THUMB_MAX_H);
        assert!((wide.x / wide.y - 2.0).abs() < 0.01);
        assert!(wide.y <= TARGET_THUMB_MAX_H + 0.01);

        let tall = image_view::fit_icon_thumb(16.0, 32.0, TARGET_THUMB_MAX_W, TARGET_THUMB_MAX_H);
        assert!((tall.x / tall.y - 0.5).abs() < 0.01);
        assert!((tall.y - TARGET_THUMB_MAX_H).abs() < 0.01);
    }

    #[test]
    fn wrapped_summary_pill_cells_share_vertical_band() {
        let ctx = egui::Context::default();
        crate::settings::SettingsUi::install_fonts(&ctx, &sqyre_persist::UserSettings::default());
        let settings = sqyre_persist::UserSettings {
            ui: sqyre_persist::UiLayout {
                ui_scale: 1.25,
                ui_font_size: 16,
                ..Default::default()
            },
            ..Default::default()
        };
        crate::settings::SettingsUi::apply_appearance(&ctx, &settings);
        let action = Action::new(ActionKind::Click {
            button: "left".into(),
            state: PressState::Tap,
        });
        let catalog = ProgramCatalog::default();
        let mut icons = IconCache::new();
        let known = KnownVariableNames::default();
        let mut cells: Vec<egui::Rect> = Vec::new();
        // Second frame: wrap units have remembered widths.
        for _ in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(400.0);
                ui.horizontal_wrapped(|ui| {
                    let band_h = paint_metrics::summary_pill_height(ui);
                    cells = action
                        .tree_summary_pills()
                        .iter()
                        .map(|pill| {
                            let theme = VarTheme {
                                known_vars: &known,
                                is_dark: true,
                            };
                            paint_summary_pill_cell(
                                ui,
                                &action,
                                pill,
                                CatalogView::from(&catalog),
                                &mut icons,
                                theme,
                                band_h,
                                false,
                            )
                            .rect
                        })
                        .collect();
                });
            })
            .drop_without_applying_deltas();
        }
        assert!(cells.len() >= 2, "click shows button + state pills");
        for cell in &cells[1..] {
            assert!(
                (cell.top() - cells[0].top()).abs() < 0.01
                    && (cell.height() - cells[0].height()).abs() < 0.01,
                "cells diverge: {:?} vs {:?}",
                cells[0],
                cell
            );
        }
    }

    #[test]
    fn middle_click_on_action_row_sets_middle_clicked() {
        let ctx = egui::Context::default();
        let action = Action::new(ActionKind::Wait {
            time: ScalarValue::Int(100),
        });
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 200.0));
        let mut at = egui::pos2(24.0, 12.0);
        let mut saw_click = false;
        // Frame 0 paints the row (hit-testing uses last frame). Later frames
        // press and release the middle button over that rect.
        for frame in 0..4 {
            let pressed = frame == 2;
            let release = frame == 3;
            let mut events = vec![egui::Event::PointerMoved(at)];
            if pressed || release {
                events.push(egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Middle,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let input = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(frame as f64 * 0.05),
                predicted_dt: 0.05,
                events,
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| {
                let result = paint_test_row(ui, &action, false);
                if frame == 0 {
                    at = result.row_rect.center();
                }
                if release {
                    assert!(result.middle_clicked);
                    assert!(!result.primary_clicked);
                    assert!(!result.double_clicked);
                    assert_eq!(result.action, RowAction::None);
                    saw_click = true;
                }
            });
            out.drop_without_applying_deltas();
        }
        assert!(saw_click);
    }

    #[test]
    fn paint_wait_row_smoke() {
        with_ui(|ui| {
            let action = Action::new(ActionKind::Wait {
                time: ScalarValue::Int(100),
            });
            let result = paint_test_row(ui, &action, false);
            assert_eq!(result.action, RowAction::None);
        });
    }

    #[test]
    fn paint_action_row_drag_handle_is_icon_and_pills_only() {
        with_ui(|ui| {
            let action = Action::new(ActionKind::Wait {
                time: ScalarValue::Int(100),
            });
            // Wide row so empty space exists past the pills.
            let wide = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 40.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(wide), |ui| {
                ui.set_clip_rect(wide);
                let result = paint_test_row(ui, &action, false);
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
            let find = Action::new(ActionKind::FindPixel(Box::new(
                sqyre_domain::FindPixelData {
                    name: "red".into(),
                    search_area: CoordinateRef("P~A".into()),
                    target_color: "#ff0000".into(),
                    color_tolerance: 5,
                    ..Default::default()
                },
            )));
            assert_eq!(paint_test_row(ui, &find, true).action, RowAction::None);

            let mut targets = Vec::new();
            for i in 0..10 {
                targets.push(format!("Prog~Item{i}"));
            }
            let search = Action::new(ActionKind::ImageSearch(Box::new(
                sqyre_domain::ImageSearchData {
                    name: "find".into(),
                    targets,
                    search_area: CoordinateRef("P~Box".into()),
                    tolerance: 0.9,
                    ..Default::default()
                },
            )));
            assert_eq!(paint_test_row(ui, &search, false).action, RowAction::None);
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
    fn row_chrome_fills_to_clip_edge_without_scrollbar_gutter() {
        with_ui(|ui| {
            // Floating bars overlay content (floating_allocated_width == 0); rows
            // must not reserve bar_width as gray padding beside a panel separator.
            assert!(ui.spacing().scroll.floating);
            assert_eq!(ui.spacing().scroll.floating_allocated_width, 0.0);
            let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(220.0, 40.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                ui.set_clip_rect(area);
                let result = paint_image_search_row(ui);
                assert!(
                    (result.row_rect.right() - area.right()).abs() <= 1.0,
                    "row should fill to clip edge (right {}, clip {})",
                    result.row_rect.right(),
                    area.right()
                );
            });
        });
    }

    #[test]
    fn image_search_row_fills_wide_width_with_thumbs() {
        with_ui(|ui| {
            let wide = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 40.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(wide), |ui| {
                ui.set_clip_rect(wide);
                let result = paint_image_search_row_with(ui, 30);
                // Previous layout capped at 8×30px slots; a wide row should pack more.
                assert!(
                    result.drag_handle_rect.width() > 8.0 * TARGET_THUMB_MAX_W + 80.0,
                    "thumbs should continue across unused row space (handle {})",
                    result.drag_handle_rect.width()
                );
                assert!(
                    result.drag_handle_rect.right() <= result.row_rect.right() + 1.0,
                    "thumbs spilled past row chrome: {} > {}",
                    result.drag_handle_rect.right(),
                    result.row_rect.right()
                );
            });
        });
    }

    fn paint_test_row(ui: &mut egui::Ui, action: &Action, is_dark: bool) -> RowInteraction {
        let catalog = ProgramCatalog::default();
        paint_action_row(
            ui,
            action,
            CatalogView::from(&catalog),
            &mut IconCache::new(),
            &KnownVariableNames::default(),
            is_dark,
            RowHighlight::None,
            &mut HashMap::new(),
            0,
            None,
            true,
        )
    }

    fn paint_image_search_row(ui: &mut egui::Ui) -> RowInteraction {
        paint_image_search_row_with(ui, 12)
    }

    fn paint_image_search_row_with(ui: &mut egui::Ui, n: usize) -> RowInteraction {
        let mut targets = Vec::new();
        for i in 0..n {
            targets.push(format!("Prog~WideItem{i}"));
        }
        let search = Action::new(ActionKind::ImageSearch(Box::new(
            sqyre_domain::ImageSearchData {
                name: "find".into(),
                targets,
                search_area: CoordinateRef("P~Box".into()),
                tolerance: 0.9,
                ..Default::default()
            },
        )));
        let catalog = ProgramCatalog::default();
        paint_action_row(
            ui,
            &search,
            CatalogView::from(&catalog),
            &mut IconCache::new(),
            &KnownVariableNames::default(),
            true,
            RowHighlight::None,
            &mut HashMap::new(),
            0,
            None,
            true,
        )
    }

    #[test]
    fn paint_action_icon_smoke() {
        with_ui(|ui| {
            let action = Action::new(ActionKind::Click {
                button: "left".into(),
                state: PressState::Down,
            });
            paint_action_icon(ui, &action, false);
        });
    }
}
