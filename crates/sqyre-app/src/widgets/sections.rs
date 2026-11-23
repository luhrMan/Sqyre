//! Section structure: edge-to-edge separators and split views.

use eframe::egui::{self, CornerRadius, Rangef, Rect, Sense, UiBuilder, UiKind, UiStackInfo};

use crate::theme::{frame_fill, PANEL_SPLITTER_W, SPACE_2, SPACE_4};

/// [`egui::UiStack`] tag on a [`split_view`] pane; value is the pane's separator span.
const PANE_TAG: &str = "sqyre_split_pane";
/// Content inset from the splitter line, so text and widgets never touch it.
pub const PANE_PAD: f32 = SPACE_4;
/// How far a [`tinted_section`] fill reaches past its content on each side.
const TINT_PAD: f32 = SPACE_4;

/// Sidebar | content panes with a vertical splitter between them.
pub struct SplitView {
    pub left: egui::Ui,
    pub right: egui::Ui,
    /// Hit / paint area of the splitter (`PANEL_SPLITTER_W` wide).
    pub splitter: Rect,
}

/// Split `body` into a `left_w` sidebar, splitter, and content pane.
///
/// Panes are `new_child`s (not `scope_builder`) so their min_size never advances
/// the parent. Content is inset from the splitter by [`PANE_PAD`];
/// [`section_separator`]s inside a pane run up to the splitter line.
pub fn split_view(ui: &mut egui::Ui, body: Rect, left_w: f32) -> SplitView {
    let left_w = left_w.min((body.width() - PANEL_SPLITTER_W).max(0.0));
    let left = Rect::from_min_size(body.min, egui::vec2(left_w, body.height()));
    let splitter = Rect::from_min_size(
        egui::pos2(left.right(), body.top()),
        egui::vec2(PANEL_SPLITTER_W, body.height()),
    );
    let right = Rect::from_min_max(egui::pos2(splitter.right(), body.top()), body.max);
    let line_x = splitter.center().x;

    let mut left_content = left;
    left_content.max.x = (left.max.x - PANE_PAD).max(left.min.x);
    let mut right_content = right;
    right_content.min.x = (right.min.x + PANE_PAD).min(right.max.x);

    SplitView {
        left: pane(ui, left_content, Rangef::new(left.min.x, line_x)),
        right: pane(ui, right_content, Rangef::new(line_x, right.max.x)),
        splitter,
    }
}

fn pane(ui: &mut egui::Ui, content: Rect, span: Rangef) -> egui::Ui {
    let mut pane = ui.new_child(
        UiBuilder::new()
            .max_rect(content)
            .layout(egui::Layout::top_down(egui::Align::Min))
            .ui_stack_info(UiStackInfo::new(UiKind::GenericArea).with_tag_value(PANE_TAG, span)),
    );
    pane.set_clip_rect(content.intersect(ui.clip_rect()));
    pane.set_max_size(content.size());
    pane
}

/// Horizontal separator that spans its section edge-to-edge: the nearest framed
/// ancestor (panel, window, card, popup) through its inner margin, or a
/// [`split_view`] pane up to the splitter line.
///
/// Total vertical band (including egui `item_spacing` before/after) equals one
/// [`section_gap`] — never add `add_space` around it. Older geometry reserved
/// ~two half-gaps and left headers/footers looking padded by empty rules.
///
/// In horizontal layouts this is a compact vertical [`egui::Separator`].
pub fn section_separator(ui: &mut egui::Ui) -> egui::Response {
    if ui.layout().main_dir().is_horizontal() {
        // egui's 6px default slot + item_spacing reads as a hole in button rows.
        return ui.add(egui::Separator::default().spacing(SPACE_2));
    }
    let Some(span) = section_span(ui) else {
        return ui.separator();
    };
    // Layout inserts `item_spacing` before this widget and again before the
    // next. Cancel the leading spacing and size the slot so
    // `slot + trailing item_spacing == section_gap`.
    let gap = crate::theme::section_gap(ui);
    let auto = ui.spacing().item_spacing.y;
    ui.add_space(-auto);
    let slot = (gap - auto).max(1.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(0.0, slot), Sense::hover());
    // Test the full line, not the zero-width slot: a horizontally scrolled
    // viewport clips the slot's x even while most of the line is on screen.
    let line = Rect::from_x_y_ranges(span, rect.y_range());
    if ui.is_rect_visible(line) {
        // Own clip on x so the line reaches through margins / scroll viewports;
        // keep the y clip so scrolled-out separators stay hidden.
        let clip = Rect::from_x_y_ranges(span, ui.clip_rect().y_range());
        egui::Painter::new(ui.ctx().clone(), ui.layer_id(), clip).hline(
            span,
            rect.center().y,
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
    }
    response
}

/// Collapsed-by-default disclosure group — the one shared chrome for hiding
/// secondary or expert options so every surface looks and behaves alike.
///
/// For the standard expert bucket use [`advanced_section`]; pass a feature
/// name ("Appearance", …) when a whole optional group is being hidden.
/// `id_salt` keeps open state stable per call site.
pub fn collapsible_section<R>(
    ui: &mut egui::Ui,
    title: &str,
    id_salt: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    collapsible_header(title, id_salt).show(ui, add_contents)
}

/// The header [`collapsible_section`] uses, for callers that need
/// `.open(…)` / `.show_unindented(…)` before showing.
pub fn collapsible_header(title: &str, id_salt: &str) -> egui::CollapsingHeader {
    egui::CollapsingHeader::new(title)
        .default_open(false)
        .id_salt(id_salt)
}

/// Standard "Advanced" disclosure — collapsed by default. Every form/tooltip
/// puts its expert fields behind this instead of ad-hoc collapses.
pub fn advanced_section<R>(
    ui: &mut egui::Ui,
    id_salt: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    collapsible_section(ui, "Advanced", id_salt, add_contents)
}

/// Inset from the pane edge for [`float_bottom_right`] chips.
const FLOAT_FOOTER_INSET: f32 = SPACE_4;

/// Floating control strip pinned inside `rect`'s bottom-right corner — replaces
/// a reserved footer row without consuming layout height. `add_contents` paints
/// right-to-left inside a window-frame chip (first widget at the right edge).
/// No shadow: the chip already floats inside a shadow-casting window.
///
/// Lists under a floating Save should fill remaining height (`footer_reserve: 0`)
/// rather than reserving a dead band — the chip overlays the scroll edge.
pub fn float_bottom_right(ui: &mut egui::Ui, rect: Rect, add_contents: impl FnOnce(&mut egui::Ui)) {
    // Clear the floating-scrollbar rail so the chip never sits on the bars.
    let inset = FLOAT_FOOTER_INSET;
    let strip_h = ui.spacing().interact_size.y + SPACE_4;
    let strip = Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - strip_h - inset),
        rect.max - egui::vec2(inset, inset),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(strip)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    egui::Frame::window(ui.style())
        .inner_margin(egui::Margin::symmetric(SPACE_4 as i8, 2))
        .shadow(egui::Shadow::NONE)
        .show(&mut child, add_contents);
}

/// Faint [`frame_fill`] behind `add_contents` so grouped rows read as one block.
///
/// The tint extends [`TINT_PAD`] past the content on each side (into frame
/// margins / scroll viewports, capped at the section edge). Nested tints stack,
/// so deeper groups read slightly stronger.
pub fn tinted_section<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let content_x = ui.max_rect().x_range();
    let span = section_span(ui).unwrap_or(content_x);
    let clip = Rect::from_x_y_ranges(span, ui.clip_rect().y_range());
    let painter = egui::Painter::new(ui.ctx().clone(), ui.layer_id(), clip);
    let bg = painter.add(egui::Shape::Noop);
    let inner = ui.scope(add_contents);
    let x = Rangef::new(
        (content_x.min - TINT_PAD).max(span.min),
        (content_x.max + TINT_PAD).min(span.max),
    );
    let rect =
        Rect::from_x_y_ranges(x, inner.response.rect.y_range()).expand2(egui::vec2(0.0, SPACE_2));
    if ui.is_rect_visible(rect) {
        painter.set(
            bg,
            egui::epaint::RectShape::filled(rect, CornerRadius::same(4), frame_fill()),
        );
    }
    inner.inner
}

/// Outer rect of the nearest framed ancestor (window, panel, card, popup)
/// including its inner margin — the painted edge, not the content region.
pub fn frame_outer_rect(ui: &egui::Ui) -> Option<Rect> {
    for node in ui.stack().iter() {
        let frame = node.frame();
        let m = frame.inner_margin;
        let framed = node.kind() == Some(UiKind::Frame)
            && (m.left > 0 || m.right > 0 || !frame.stroke.is_empty());
        if !framed {
            continue;
        }
        let inner = if node.id == ui.id() {
            ui.max_rect()
        } else {
            node.max_rect
        };
        return Some(Rect::from_min_max(
            inner.min - egui::vec2(f32::from(m.left), f32::from(m.top)),
            inner.max + egui::vec2(f32::from(m.right), f32::from(m.bottom)),
        ));
    }
    None
}

/// Outer x-range of the section containing `ui`.
fn section_span(ui: &egui::Ui) -> Option<Rangef> {
    let mut pane: Option<Rangef> = None;
    for node in ui.stack().iter() {
        if pane.is_none() {
            if let Some(span) = node.tags().get_downcast::<Rangef>(PANE_TAG) {
                pane = Some(*span);
                continue;
            }
        }
        let frame = node.frame();
        let m = frame.inner_margin;
        let framed = node.kind() == Some(UiKind::Frame)
            && (m.left > 0 || m.right > 0 || !frame.stroke.is_empty());
        if !framed {
            continue;
        }
        // The innermost Ui reflects `set_max_width` narrowing; stack snapshots do not.
        let inner = if node.id == ui.id() {
            ui.max_rect().x_range()
        } else {
            node.max_rect.x_range()
        };
        let outer = Rangef::new(
            inner.min - f32::from(m.left),
            inner.max + f32::from(m.right),
        );
        return Some(match pane {
            None => outer,
            // A pane edge flush with the frame content bleeds through the margin;
            // the splitter-side edge stays on the splitter line.
            Some(p) => Rangef::new(
                if p.min <= inner.min + 0.5 {
                    outer.min
                } else {
                    p.min
                },
                if p.max >= inner.max - 0.5 {
                    outer.max
                } else {
                    p.max
                },
            ),
        });
    }
    pane
}
