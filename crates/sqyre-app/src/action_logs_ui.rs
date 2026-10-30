//! Per-action Logs window: filterable text/images, zoomable image viewer,
//! keyboard navigation, follow-while-running, and hover/click "where found"
//! find markers that highlight detections in logged images and on the desktop.

use crate::image_view::{self, ImageViewTransform};
use crate::preview_tooltip::PreviewTooltipCache;
use crate::theme::selection_stroke;
use eframe::egui::{self, ColorImage, Key, Modifiers, TextureHandle, TextureOptions};
use sqyre_domain::ActionId;
use sqyre_ports::{
    lines_for, ActionLogEntry, FindContext, ImageAnnotation, LogFind, LogImage, SharedActionLog,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Texture key for a log image (entry index + optional step within an item pipeline).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum TexKey {
    Entry(usize),
    Step {
        entry: usize,
        step: usize,
    },
    Thumb(usize),
    /// Find-context image shown in a hover tooltip (not otherwise displayed).
    FindCtx {
        entry: usize,
        find: usize,
    },
}

/// Which entry kinds the filter passes.
#[derive(Clone, Copy)]
struct KindFilter {
    text: bool,
    images: bool,
    items: bool,
}

impl Default for KindFilter {
    fn default() -> Self {
        Self {
            text: true,
            images: true,
            items: true,
        }
    }
}

/// Image open in the zoom view (owned so it survives log rotation).
struct ZoomedImage {
    key: TexKey,
    image: LogImage,
}

/// Window-local result: stay open or close. The window follows the macro
/// tree's selection — switching actions happens by clicking the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogsNav {
    Stay,
    Close,
}

/// Retained textures + interaction state for the open logs window.
#[derive(Default)]
pub struct LogsWindowState {
    action: Option<ActionId>,
    textures: HashMap<TexKey, (usize, TextureHandle)>,
    /// Entry index of the selected [`ActionLogEntry::ItemPipeline`], if any.
    pub selected_item: Option<usize>,
    zoom: Option<ZoomedImage>,
    zoom_view: ImageViewTransform,
    search: String,
    kinds: KindFilter,
    /// Entry index of the keyboard-focused item card.
    focused_item: Option<usize>,
    scroll_to_focused: bool,
    /// `(entry index, find index)` pinned by click — desktop outline + mark.
    pinned_find: Option<(usize, usize)>,
    /// `(entry index, find index)` hovered last frame — images paint before
    /// find chips, so marks read the previous frame's hover.
    hovered_prev: Option<(usize, usize)>,
    /// Click-inspected mark in the zoom view — its tooltip stays up until
    /// another click or Esc.
    inspect: Option<Inspect>,
    /// Per-section open overrides keyed by the section entry's index; absent
    /// means the default — only the latest attempt section is expanded.
    sections: HashMap<usize, bool>,
    /// Entry count seen last frame — a shrink means the log was cleared and a
    /// new run may reuse "Attempt N" labels, so stale toggles are dropped.
    last_len: usize,
    /// Stick the list scroll to the newest entries.
    follow: bool,
}

impl LogsWindowState {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    fn ensure_action(&mut self, action_id: ActionId, is_running: bool) {
        if self.action != Some(action_id) {
            *self = Self {
                action: Some(action_id),
                follow: is_running,
                ..Default::default()
            };
        }
    }

    fn texture(
        &mut self,
        ctx: &egui::Context,
        action_id: ActionId,
        key: TexKey,
        image: &LogImage,
    ) -> Option<TextureHandle> {
        let ptr = Arc::as_ptr(&image.pixels) as usize;
        let size = [image.width as usize, image.height as usize];
        if image.pixels.len() != size[0] * size[1] * 4 {
            return None;
        }
        if let Some((old_ptr, tex)) = self.textures.get_mut(&key) {
            if *old_ptr != ptr {
                let color = ColorImage::from_rgba_unmultiplied(size, &image.pixels);
                tex.set(color, TextureOptions::NEAREST);
                *old_ptr = ptr;
            }
            return Some(tex.clone());
        }
        let color = ColorImage::from_rgba_unmultiplied(size, &image.pixels);
        let name = format!(
            "action-log-{}-{:?}-{}",
            action_id.as_str(),
            key,
            image.label
        );
        let tex = ctx.load_texture(name, color, TextureOptions::NEAREST);
        self.textures.insert(key, (ptr, tex.clone()));
        Some(tex)
    }
}

/// Draw the floating Logs window for `action_id`.
#[expect(
    clippy::too_many_arguments,
    reason = "window needs entries, caches, previews, and run state"
)]
pub fn show_logs_window(
    ctx: &egui::Context,
    action_id: ActionId,
    title: &str,
    action_log: &SharedActionLog,
    state: &mut LogsWindowState,
    previews: &mut PreviewTooltipCache,
    is_running: bool,
    pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
) -> LogsNav {
    state.ensure_action(action_id, is_running);
    let entries = action_log.entries_for(action_id);
    if entries.len() < state.last_len {
        state.sections.clear();
    }
    state.last_len = entries.len();
    let q = state.search.trim().to_string();
    let visible: Vec<usize> = (0..entries.len())
        .filter(|&i| entry_matches(&entries[i], &q, &state.kinds))
        .collect();
    let items = item_indices(&entries, &q, &state.kinds);

    // Drop snapshot indices whose entries rotated out of the (capped) log.
    if let Some(sel) = state.selected_item {
        if !matches!(entries.get(sel), Some(ActionLogEntry::ItemPipeline { .. })) {
            state.selected_item = None;
        }
    }
    if let Some(f) = state.focused_item {
        if !items.contains(&f) {
            state.focused_item = None;
        }
    }
    if let Some(loc) = state.pinned_find {
        if find_at(&entries, loc).is_none() {
            state.pinned_find = None;
        }
    }
    if let Some(loc) = state.hovered_prev {
        if find_at(&entries, loc).is_none() {
            state.hovered_prev = None;
        }
    }

    let counts = counts_text(&entries, &visible);
    let outlines_ok = previews.desktop_outlines_supported();
    let mut nav = LogsNav::Stay;
    let mut open = true;
    let mut hovered_now: Option<(usize, usize)> = None;
    let id = egui::Id::new(("sqyre_logs", action_id.as_str()));
    crate::widgets::fit_dialog_window(
        egui::Window::new(title)
            .open(&mut open)
            .default_size([640.0, 560.0])
            .min_width(420.0)
            .min_height(280.0),
        ctx,
        id,
        pending_scale,
    )
    .show(ctx, |ui| {
        // ── Toolbar ─────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            if ui
                .button("Copy text")
                .on_hover_text("Copy the filtered log lines")
                .clicked()
            {
                let filtered: Vec<ActionLogEntry> =
                    visible.iter().map(|&i| entries[i].clone()).collect();
                ui.ctx().copy_text(lines_for(&filtered).join("\n"));
            }
            if crate::widgets::cancel_icon_button(ui, "Clear this action's log entries").clicked() {
                action_log.clear_for(action_id);
            }
            crate::widgets::icon_toggle(
                ui,
                &mut state.follow,
                "Keep scrolled to the newest entries while the macro runs",
                egui_phosphor::regular::ARROW_DOWN,
                egui_phosphor::fill::ARROW_DOWN,
            );
            ui.weak(&counts);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::widgets::cancel_icon_button(ui, "Close").clicked() {
                    nav = LogsNav::Close;
                }
            });
        });

        // ── Search + kind toggles ───────────────────────────────────────
        let search_out = crate::widgets::search_field(
            ui,
            &mut state.search,
            crate::widgets::SearchFieldOpts {
                hint: "Filter logs…  ( / )",
                focus_id: None,
                clearable: true,
                stable_id: Some(egui::Id::new("action_logs_search")),
            },
        );
        let search_resp = search_out.response;
        ui.horizontal(|ui| {
            if ui.selectable_label(state.kinds.text, "Text").clicked() {
                state.kinds.text = !state.kinds.text;
            }
            if ui.selectable_label(state.kinds.images, "Images").clicked() {
                state.kinds.images = !state.kinds.images;
            }
            if ui.selectable_label(state.kinds.items, "Items").clicked() {
                state.kinds.items = !state.kinds.items;
            }
        });
        crate::widgets::section_separator(ui);

        // ── Keys (top-layer only) ───────────────────────────────────────
        let top = crate::focus_nav::is_top_layer(ui.ctx(), ui.layer_id());
        if top {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Slash))
                || ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::F))
            {
                search_resp.request_focus();
            }
            if !search_resp.has_focus() && state.zoom.is_none() && state.selected_item.is_none() {
                let down = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown));
                let up = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp));
                if down || up {
                    state.focused_item =
                        step_item(&items, state.focused_item, if down { 1 } else { -1 });
                    state.scroll_to_focused = true;
                }
                if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                    if let Some(f) = state.focused_item {
                        state.selected_item = Some(f);
                    }
                }
            }
        }

        // ── Esc cascade: inspect tip → zoom → detail → search → unpin → close
        if crate::widgets::consume_escape(ui) {
            if state.inspect.is_some() {
                state.inspect = None;
            } else if state.zoom.is_some() {
                state.zoom = None;
            } else if state.selected_item.is_some() {
                state.selected_item = None;
            } else if search_resp.has_focus() || !state.search.is_empty() {
                state.search.clear();
                search_resp.surrender_focus();
            } else if state.pinned_find.is_some() {
                state.pinned_find = None;
            } else {
                nav = LogsNav::Close;
            }
        }

        // ── Body ────────────────────────────────────────────────────────
        let list_h = crate::pickers::popup_scroll_max_height(ui, 0.0);
        if state.zoom.is_some() {
            show_zoom_view(ui, action_id, state, &entries, list_h, top);
        } else {
            let list_w = crate::widgets::visible_width(ui);
            crate::pickers::dialog_scroll(list_w, list_h)
                .stick_to_bottom(state.follow && is_running)
                .show(ui, |ui| {
                    ui.set_max_width(list_w);
                    if entries.is_empty() {
                        ui.label("No logs yet — run the macro.");
                    } else if let Some(sel) = state.selected_item {
                        if let Some(ActionLogEntry::ItemPipeline {
                            title,
                            summary,
                            steps,
                            details,
                            finds,
                            ..
                        }) = entries.get(sel)
                        {
                            show_item_detail(
                                ui,
                                action_id,
                                state,
                                &entries,
                                &items,
                                outlines_ok,
                                sel,
                                title,
                                summary,
                                details,
                                finds,
                                steps,
                                &mut hovered_now,
                            );
                        }
                    } else {
                        show_list(
                            ui,
                            action_id,
                            state,
                            &entries,
                            &visible,
                            outlines_ok,
                            &mut hovered_now,
                        );
                    }
                });
        }

        // ── Desktop outline feed (hover wins over pinned) ───────────────
        if outlines_ok {
            if let Some(loc) = hovered_now {
                if let Some(f) = find_at(&entries, loc) {
                    previews.request_outline_abs_hover(f.screen);
                }
            } else if let Some(loc) = state.pinned_find {
                if let Some(f) = find_at(&entries, loc) {
                    previews.request_outline_abs(f.screen);
                }
            }
        }
        state.hovered_prev = hovered_now;
    });

    if !open && nav == LogsNav::Stay {
        nav = LogsNav::Close;
    }
    nav
}

fn counts_text(entries: &[ActionLogEntry], visible: &[usize]) -> String {
    let mut text = 0;
    let mut images = 0;
    let mut items = 0;
    for &i in visible {
        match &entries[i] {
            ActionLogEntry::Text(_) | ActionLogEntry::Finds { .. } | ActionLogEntry::Section(_) => {
                text += 1
            }
            ActionLogEntry::Image(_) => images += 1,
            ActionLogEntry::ItemPipeline { .. } => items += 1,
        }
    }
    if visible.len() == entries.len() {
        format!("{text} line(s) · {images} image(s) · {items} item(s)")
    } else {
        format!(
            "{}/{} shown · {text} lines · {images} images · {items} items",
            visible.len(),
            entries.len()
        )
    }
}

/// Filter predicate: kind toggle + fuzzy query over text / labels / finds.
fn entry_matches(e: &ActionLogEntry, q: &str, kinds: &KindFilter) -> bool {
    match e {
        ActionLogEntry::Text(line) => kinds.text && crate::pickers::fuzzy_match_fold(q, line),
        // Section headers are structure, not content — keep them whenever text
        // lines are shown so collapsed groups still label themselves.
        ActionLogEntry::Section(_) => kinds.text,
        ActionLogEntry::Finds { text, finds } => {
            kinds.text
                && (crate::pickers::fuzzy_match_fold(q, text)
                    || finds.iter().any(|f| find_matches(f, q)))
        }
        ActionLogEntry::Image(img) => {
            kinds.images && crate::pickers::fuzzy_match_fold(q, &img.label)
        }
        ActionLogEntry::ItemPipeline {
            title,
            summary,
            details,
            finds,
            ..
        } => {
            kinds.items
                && (crate::pickers::fuzzy_match_fold(q, title)
                    || crate::pickers::fuzzy_match_fold(q, summary)
                    || details
                        .iter()
                        .any(|d| crate::pickers::fuzzy_match_fold(q, d))
                    || finds.iter().any(|f| find_matches(f, q)))
        }
    }
}

/// Fuzzy-match a find's label and detail (e.g. OCR text found inside the box).
fn find_matches(f: &LogFind, q: &str) -> bool {
    crate::pickers::fuzzy_match_fold(q, &f.label)
        || f.detail
            .as_deref()
            .is_some_and(|d| crate::pickers::fuzzy_match_fold(q, d))
}

/// One run of filtered entries under a section header — `label: None` is the
/// leading unsectioned run (actions that never emit markers render flat).
struct SectionChunk<'a> {
    label: Option<&'a str>,
    /// Index of the `Section` entry — stable toggle key within this snapshot.
    key: usize,
    /// Indices into `entries`, already filtered.
    indices: Vec<usize>,
}

/// Group `entries` on `Section` boundaries, keeping only `visible` indices.
fn section_chunks<'a>(entries: &'a [ActionLogEntry], visible: &[usize]) -> Vec<SectionChunk<'a>> {
    let mut chunks = vec![SectionChunk {
        label: None,
        key: 0,
        indices: Vec::new(),
    }];
    let mut vi = 0usize;
    for (i, e) in entries.iter().enumerate() {
        let in_visible = vi < visible.len() && visible[vi] == i;
        if in_visible {
            vi += 1;
        }
        if let ActionLogEntry::Section(label) = e {
            chunks.push(SectionChunk {
                label: Some(label.as_str()),
                key: i,
                indices: Vec::new(),
            });
        } else if in_visible {
            chunks.last_mut().expect("leading chunk").indices.push(i);
        }
    }
    chunks
}

/// Entry indices of visible item pipelines (respects the active filter).
fn item_indices(entries: &[ActionLogEntry], q: &str, kinds: &KindFilter) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.is_item_pipeline() && entry_matches(e, q, kinds))
        .map(|(i, _)| i)
        .collect()
}

/// Step `cur` through `items` by `dir` (`±1`); `None` starts at an end.
fn step_item(items: &[usize], cur: Option<usize>, dir: i32) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    let idx = match cur.and_then(|c| items.iter().position(|&i| i == c)) {
        Some(p) => (p as i32 + dir).clamp(0, items.len() as i32 - 1) as usize,
        None if dir < 0 => items.len() - 1,
        None => 0,
    };
    Some(items[idx])
}

fn find_at(entries: &[ActionLogEntry], loc: (usize, usize)) -> Option<&LogFind> {
    entries.get(loc.0)?.finds().get(loc.1)
}

/// A find's context image plus the rect (image pixel coords) to stroke on it.
type ContextMark<'a> = (&'a LogImage, (i32, i32, i32, i32));

/// Resolve a find's context to `(image, rect-in-image-pixels)` — `Step`
/// contexts index the parent pipeline's steps; `Image` contexts carry their own.
fn find_context_image<'a>(
    entries: &'a [ActionLogEntry],
    entry_idx: usize,
    find: &'a LogFind,
) -> Option<ContextMark<'a>> {
    match &find.context {
        Some(FindContext::Image { image, rect }) => Some((image.as_ref(), *rect)),
        Some(FindContext::Step { step, rect }) => match entries.get(entry_idx) {
            Some(ActionLogEntry::ItemPipeline { steps, .. }) => {
                steps.get(*step).map(|img| (img, *rect))
            }
            _ => None,
        },
        None => None,
    }
}

/// Whether the find's context image is painted somewhere this frame (then the
/// in-place stroke covers it; otherwise the hover tooltip shows it).
/// `visible` is the list of entries currently painted in the scroll.
fn context_displayed(
    entries: &[ActionLogEntry],
    visible: &[usize],
    state: &LogsWindowState,
    entry_idx: usize,
    find: &LogFind,
) -> bool {
    match &find.context {
        Some(FindContext::Image { image, .. }) => visible.iter().any(
            |&i| matches!(&entries[i], ActionLogEntry::Image(img) if Arc::ptr_eq(&img.pixels, &image.pixels)),
        ),
        Some(FindContext::Step { .. }) => state.selected_item == Some(entry_idx),
        None => true,
    }
}

/// Context rects of the active (hovered-last-frame / pinned) finds that land
/// inside `img` — matched by shared pixel allocation.
fn find_marks_for_image(
    entries: &[ActionLogEntry],
    state: &LogsWindowState,
    img: &LogImage,
) -> Vec<(i32, i32, i32, i32)> {
    [state.hovered_prev, state.pinned_find]
        .into_iter()
        .flatten()
        .filter_map(|loc| find_at(entries, loc).map(|f| (loc.0, f)))
        .filter_map(|(ei, f)| find_context_image(entries, ei, f))
        .filter(|(ctx_img, _)| Arc::ptr_eq(&ctx_img.pixels, &img.pixels))
        .map(|(_, r)| r)
        .collect()
}

/// What the zoom-view click-inspect tooltip shows.
#[derive(Clone)]
enum Inspect {
    /// `(entry idx, find idx)` — resolved against `entries` each frame.
    Find(usize, usize),
    /// An in-image annotation (OCR word box) — payload copied out of the image.
    Annotation(ImageAnnotation),
}

/// A mark (find or annotation) resolved against a displayed image.
struct ImageMark {
    /// Inspect payload for the click tooltip.
    target: Inspect,
    /// Marker rect in `LogImage` pixel coords.
    rect: (i32, i32, i32, i32),
}

/// Every find whose context image is `img` — the zoom view strokes and
/// click-inspects all of them.
fn finds_for_image(entries: &[ActionLogEntry], img: &LogImage) -> Vec<ImageMark> {
    entries
        .iter()
        .enumerate()
        .flat_map(|(ei, e)| {
            e.finds().iter().enumerate().filter_map(move |(fi, f)| {
                find_context_image(entries, ei, f)
                    .filter(|(ci, _)| Arc::ptr_eq(&ci.pixels, &img.pixels))
                    .map(|(_, rect)| ImageMark {
                        target: Inspect::Find(ei, fi),
                        rect,
                    })
            })
        })
        .collect()
}

/// Inspect target of the smallest mark under `pos` (pointer → image coords).
fn mark_at(
    marks: &[ImageMark],
    display: egui::Rect,
    image_size: egui::Vec2,
    pos: egui::Pos2,
) -> Option<Inspect> {
    if !display.is_positive() || !display.contains(pos) {
        return None;
    }
    let ix = ((pos.x - display.left()) * image_size.x / display.width()) as i32;
    let iy = ((pos.y - display.top()) * image_size.y / display.height()) as i32;
    marks
        .iter()
        .filter(|m| {
            let (l, t, r, b) = m.rect;
            ix >= l && ix < r && iy >= t && iy < b
        })
        .min_by_key(|m| {
            let (l, t, r, b) = m.rect;
            (r - l) as i64 * (b - t) as i64
        })
        .map(|m| m.target.clone())
}

/// `rect` (LogImage pixel coords, half-open) → screen rect inside `display`.
fn find_rect_on_display(
    display: egui::Rect,
    img_w: u32,
    img_h: u32,
    rect: (i32, i32, i32, i32),
) -> egui::Rect {
    if img_w == 0 || img_h == 0 {
        return egui::Rect::NOTHING;
    }
    let (l, t, r, b) = rect;
    let sx = display.width() / img_w as f32;
    let sy = display.height() / img_h as f32;
    egui::Rect::from_min_max(
        egui::pos2(
            display.left() + l as f32 * sx,
            display.top() + t as f32 * sy,
        ),
        egui::pos2(
            display.left() + r as f32 * sx,
            display.top() + b as f32 * sy,
        ),
    )
    .intersect(display)
}

/// Stroke `rect` (LogImage pixel coords, half-open) over `display` (screen rect).
fn stroke_find_rect(
    painter: &egui::Painter,
    display: egui::Rect,
    img_w: u32,
    img_h: u32,
    rect: (i32, i32, i32, i32),
    stroke: egui::Stroke,
) {
    let mark = find_rect_on_display(display, img_w, img_h, rect);
    if mark.is_positive() {
        painter.rect_stroke(mark, 0.0, stroke, egui::StrokeKind::Inside);
    }
}

/// Stroke color for in-image annotations (OCR word boxes) — muted so find
/// marks (gold) stay visually primary.
fn annotation_stroke() -> egui::Stroke {
    egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 150, 200))
}

#[expect(
    clippy::too_many_arguments,
    reason = "find chip needs entry context, window state, and hover out-param"
)]
fn show_find_chip(
    ui: &mut egui::Ui,
    action_id: ActionId,
    state: &mut LogsWindowState,
    entries: &[ActionLogEntry],
    visible: &[usize],
    outlines_ok: bool,
    entry_idx: usize,
    find_idx: usize,
    find: &LogFind,
    hovered_now: &mut Option<(usize, usize)>,
) {
    let pinned = state.pinned_find == Some((entry_idx, find_idx));
    let resp = ui.add(egui::Button::new(&find.label).small().selected(pinned));
    if resp.hovered() {
        *hovered_now = Some((entry_idx, find_idx));
    }
    if resp.clicked() {
        state.pinned_find = if pinned {
            None
        } else {
            Some((entry_idx, find_idx))
        };
    }
    // On platforms without live desktop outlines the hover tooltip is the only
    // place to see the location — always show the context image there.
    let displayed = outlines_ok && context_displayed(entries, visible, state, entry_idx, find);
    resp.on_hover_ui(|ui| {
        if !displayed {
            if let Some((img, rect)) = find_context_image(entries, entry_idx, find) {
                if let Some(tex) = state.texture(
                    ui.ctx(),
                    action_id,
                    TexKey::FindCtx {
                        entry: entry_idx,
                        find: find_idx,
                    },
                    img,
                ) {
                    let [tw, th] = tex.size();
                    let size = image_view::fit_in_box(tw as f32, th as f32, 360.0, 270.0);
                    let (disp, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    ui.painter().image(
                        tex.id(),
                        disp,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    stroke_find_rect(
                        ui.painter(),
                        disp,
                        img.width,
                        img.height,
                        rect,
                        selection_stroke(),
                    );
                }
            }
        }
        let (l, t, r, b) = find.screen;
        ui.small(format!(
            "screen ({l},{t})–({r},{b}) · {}×{}",
            (r - l).max(0),
            (b - t).max(0)
        ));
        if pinned {
            ui.weak("click to unpin · Esc unpins");
        } else if outlines_ok {
            ui.weak("click to pin on desktop");
        } else {
            ui.weak("click to pin");
        }
    });
}

fn show_list(
    ui: &mut egui::Ui,
    action_id: ActionId,
    state: &mut LogsWindowState,
    entries: &[ActionLogEntry],
    visible: &[usize],
    outlines_ok: bool,
    hovered_now: &mut Option<(usize, usize)>,
) {
    if visible.is_empty() {
        crate::widgets::list_vacancy(ui, &state.search, 0, "entries");
        return;
    }
    let avail_w = ui.available_width().max(120.0);
    let chunks = section_chunks(entries, visible);
    // Only the latest attempt section is expanded by default; explicit toggles
    // persist per section until the action's logs are cleared or replaced.
    let last_sectioned = chunks.iter().rposition(|c| c.label.is_some());
    let mut pending_items: Vec<(usize, &ActionLogEntry)> = Vec::new();
    for (ci, chunk) in chunks.iter().enumerate() {
        let open = match chunk.label {
            None => true,
            Some(label) => {
                let open = state
                    .sections
                    .get(&chunk.key)
                    .copied()
                    .unwrap_or(last_sectioned == Some(ci));
                let caret = if open {
                    egui_phosphor::regular::CARET_DOWN
                } else {
                    egui_phosphor::regular::CARET_RIGHT
                };
                let n = chunk.indices.len();
                let header = format!(
                    "{caret}  {label} — {n} {}",
                    if n == 1 { "entry" } else { "entries" }
                );
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new(header).strong().small())
                            .frame(false),
                    )
                    .clicked()
                {
                    state.sections.insert(chunk.key, !open);
                }
                open
            }
        };
        if !open {
            continue;
        }
        ui.indent(egui::Id::new(("logs_section", ci)), |ui| {
            for &i in &chunk.indices {
                match &entries[i] {
                    ActionLogEntry::Text(line) => {
                        ui.monospace(line);
                    }
                    ActionLogEntry::Section(_) => {}
                    ActionLogEntry::Image(img) => {
                        show_labeled_image(
                            ui,
                            action_id,
                            state,
                            entries,
                            TexKey::Entry(i),
                            img,
                            avail_w,
                        );
                    }
                    ActionLogEntry::ItemPipeline { .. } => {
                        pending_items.push((i, &entries[i]));
                    }
                    ActionLogEntry::Finds { text, finds } => {
                        ui.monospace(text);
                        ui.horizontal_wrapped(|ui| {
                            for (fi, f) in finds.iter().enumerate() {
                                show_find_chip(
                                    ui,
                                    action_id,
                                    state,
                                    entries,
                                    visible,
                                    outlines_ok,
                                    i,
                                    fi,
                                    f,
                                    hovered_now,
                                );
                            }
                        });
                    }
                }
            }
        });
    }
    flush_item_gallery(ui, action_id, state, &mut pending_items);
}

const CARD_MIN: f32 = 112.0;
const CARD_MAX: f32 = 180.0;
const CARD_GAP: f32 = crate::theme::SPACE_8;
/// Frame padding so the thumb stays inside the card width.
const CARD_THUMB_INSET: f32 = 28.0;

/// Even-row gallery: more columns as the window widens; cards stretch between
/// [`CARD_MIN`] and [`CARD_MAX`]. Uses [`crate::widgets::visible_width`] so the
/// row count cannot ratchet the Logs window toward `max_size`.
fn item_gallery_metrics(count: usize, avail_w: f32) -> (f32, usize) {
    if count == 0 {
        return (CARD_MAX, 1);
    }
    let avail = avail_w.max(CARD_MIN);
    let fit = |cell: f32| {
        crate::pickers::grid_column_count_for_width(avail, cell, CARD_GAP)
            .min(count)
            .max(1)
    };
    let cols = fit(CARD_MIN);
    let stretched = (avail - CARD_GAP * (cols as f32 - 1.0)) / cols as f32;
    if stretched > CARD_MAX {
        (CARD_MAX, fit(CARD_MAX))
    } else {
        (stretched.max(CARD_MIN), cols)
    }
}

fn flush_item_gallery(
    ui: &mut egui::Ui,
    action_id: ActionId,
    state: &mut LogsWindowState,
    pending: &mut Vec<(usize, &ActionLogEntry)>,
) {
    if pending.is_empty() {
        return;
    }
    ui.add_space(crate::theme::section_gap(ui));
    crate::widgets::title_with_count(
        ui,
        egui::RichText::new("Items — click an image to inspect processing & finds")
            .strong()
            .small(),
        pending.len(),
    );
    ui.add_space(crate::theme::element_gap(ui));

    let avail = crate::widgets::visible_content_width(ui);
    let (card_w, cols) = item_gallery_metrics(pending.len(), avail);
    ui.scope(|ui| {
        ui.set_max_width(avail);
        let old_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = egui::vec2(CARD_GAP, CARD_GAP);

        let mut i = 0;
        while i < pending.len() {
            ui.horizontal(|ui| {
                ui.set_max_width(avail);
                ui.spacing_mut().item_spacing.x = CARD_GAP;
                let end = (i + cols).min(pending.len());
                for &(idx, entry) in &pending[i..end] {
                    let ActionLogEntry::ItemPipeline {
                        title,
                        summary,
                        thumbnail,
                        ..
                    } = entry
                    else {
                        continue;
                    };
                    let rect = show_item_card(
                        ui, action_id, state, idx, title, summary, thumbnail, card_w,
                    );
                    if state.scroll_to_focused && state.focused_item == Some(idx) {
                        ui.scroll_to_rect(rect, Some(egui::Align::Center));
                        state.scroll_to_focused = false;
                    }
                }
            });
            i += cols;
        }

        ui.spacing_mut().item_spacing = old_spacing;
    });
    pending.clear();
    ui.add_space(crate::theme::element_gap(ui));
}

#[expect(
    clippy::too_many_arguments,
    reason = "log card: cache, entry identity, captions, and image payloads"
)]
fn show_item_card(
    ui: &mut egui::Ui,
    action_id: ActionId,
    state: &mut LogsWindowState,
    entry_index: usize,
    title: &str,
    summary: &str,
    thumbnail: &LogImage,
    card_w: f32,
) -> egui::Rect {
    let selected = state.selected_item == Some(entry_index);
    let focused = state.focused_item == Some(entry_index);
    let frame = egui::Frame::group(ui.style()).stroke(if selected || focused {
        selection_stroke()
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    });
    let thumb_edge = (card_w - CARD_THUMB_INSET).max(64.0);
    ui.vertical(|ui| {
        ui.set_width(card_w);
        frame
            .show(ui, |ui| {
                ui.set_max_width(card_w);
                ui.add(egui::Label::new(egui::RichText::new(title).strong().small()).truncate())
                    .on_hover_text(title);
                ui.add(egui::Label::new(egui::RichText::new(summary).weak().small()).truncate())
                    .on_hover_text(summary);
                if let Some(tex) =
                    state.texture(ui.ctx(), action_id, TexKey::Thumb(entry_index), thumbnail)
                {
                    let [tw, th] = tex.size();
                    let size = fit_thumb(tw as f32, th as f32, thumb_edge);
                    let resp = ui.add(
                        egui::Image::new((tex.id(), size))
                            .sense(egui::Sense::click())
                            .corner_radius(4.0),
                    );
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if resp.clicked() {
                        state.selected_item = Some(entry_index);
                    }
                    resp.on_hover_text("Click to view processing steps and where it was found");
                }
                if ui
                    .small_button(if selected { "Open ▸" } else { "Inspect ▸" })
                    .clicked()
                {
                    state.selected_item = Some(entry_index);
                }
            })
            .response
    })
    .response
    .rect
}

#[expect(
    clippy::too_many_arguments,
    reason = "item detail: cache, entry context, find rows, and hover out-param"
)]
fn show_item_detail(
    ui: &mut egui::Ui,
    action_id: ActionId,
    state: &mut LogsWindowState,
    entries: &[ActionLogEntry],
    items: &[usize],
    outlines_ok: bool,
    entry_index: usize,
    title: &str,
    summary: &str,
    details: &[String],
    finds: &[LogFind],
    steps: &[LogImage],
    hovered_now: &mut Option<(usize, usize)>,
) {
    let pos = items.iter().position(|&i| i == entry_index);
    ui.horizontal(|ui| {
        if ui.small_button("◂ Back").clicked() {
            state.selected_item = None;
        }
        if let Some(p) = pos {
            if ui
                .small_button("◂ Prev")
                .on_hover_text("Previous item (←)")
                .clicked()
            {
                state.selected_item = step_item(items, Some(entry_index), -1);
            }
            ui.weak(format!("{}/{}", p + 1, items.len()));
            if ui
                .small_button("Next ▸")
                .on_hover_text("Next item (→)")
                .clicked()
            {
                state.selected_item = step_item(items, Some(entry_index), 1);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(summary).weak());
            ui.label(egui::RichText::new(title).strong());
        });
    });
    crate::widgets::section_separator(ui);

    if !finds.is_empty() {
        ui.label(
            egui::RichText::new("Found locations — hover to mark, click to pin")
                .strong()
                .small(),
        );
        for (fi, f) in finds.iter().enumerate() {
            ui.horizontal(|ui| {
                // No list entries are painted in detail view — `&[]` so Image
                // contexts (if any ever appear here) fall back to tooltips.
                show_find_chip(
                    ui,
                    action_id,
                    state,
                    entries,
                    &[],
                    outlines_ok,
                    entry_index,
                    fi,
                    f,
                    hovered_now,
                );
                let (l, t, r, b) = f.screen;
                ui.weak(format!(
                    "({l},{t})–({r},{b}) · {}×{}",
                    (r - l).max(0),
                    (b - t).max(0)
                ));
            });
        }
        ui.add_space(crate::theme::element_gap(ui));
    }

    if !details.is_empty() {
        ui.label(egui::RichText::new("Details").strong().small());
        for line in details {
            ui.monospace(line);
        }
        ui.add_space(crate::theme::element_gap(ui));
    }

    ui.label(
        egui::RichText::new("Processing & find steps (chronological)")
            .strong()
            .small(),
    );
    ui.add_space(crate::theme::element_gap(ui));
    let avail_w = ui.available_width().max(120.0);
    for (si, step) in steps.iter().enumerate() {
        show_labeled_image(
            ui,
            action_id,
            state,
            entries,
            TexKey::Step {
                entry: entry_index,
                step: si,
            },
            step,
            avail_w,
        );
    }
}

fn show_labeled_image(
    ui: &mut egui::Ui,
    action_id: ActionId,
    state: &mut LogsWindowState,
    entries: &[ActionLogEntry],
    key: TexKey,
    image: &LogImage,
    avail_w: f32,
) {
    ui.add_space(crate::theme::element_gap(ui));
    ui.group(|ui| {
        ui.label(egui::RichText::new(&image.label).strong().small());
        ui.label(
            egui::RichText::new(format!("{}×{}", image.width, image.height))
                .weak()
                .small(),
        );
        if let Some(tex) = state.texture(ui.ctx(), action_id, key, image) {
            let [tw, th] = tex.size();
            let size = fit_width(tw as f32, th as f32, avail_w - 16.0);
            let resp = ui.add(egui::Image::new((tex.id(), size)).sense(egui::Sense::click()));
            for mark in find_marks_for_image(entries, state, image) {
                stroke_find_rect(
                    ui.painter(),
                    resp.rect,
                    image.width,
                    image.height,
                    mark,
                    selection_stroke(),
                );
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            resp.clone()
                .on_hover_text("Click to zoom — wheel zooms, drag pans");
            if resp.clicked() {
                state.zoom = Some(ZoomedImage {
                    key,
                    image: image.clone(),
                });
                state.zoom_view.reset();
                state.inspect = None;
            }
        } else {
            crate::widgets::error_icon(ui, "Image unavailable");
        }
    });
}

fn show_zoom_view(
    ui: &mut egui::Ui,
    action_id: ActionId,
    state: &mut LogsWindowState,
    entries: &[ActionLogEntry],
    body_h: f32,
    top: bool,
) {
    let start_y = ui.cursor().min.y;
    ui.horizontal(|ui| {
        if ui.small_button("◂ Back").clicked() {
            state.zoom = None;
            state.inspect = None;
            return;
        }
        if let Some(z) = &state.zoom {
            ui.label(egui::RichText::new(&z.image.label).strong());
            ui.weak(format!("{}×{}", z.image.width, z.image.height));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    state.zoom_view.needs_reset_button(),
                    egui::Button::new("Reset view").small(),
                )
                .on_hover_text("Fit image in viewport (0)")
                .clicked()
            {
                state.zoom_view.reset();
            }
            if state.zoom_view.shows_zoom_label() {
                ui.weak(format!("{:.0}%", state.zoom_view.zoom * 100.0));
            }
        });
    });
    // Copy key + image up front so `state` stays mutable-borrowable below.
    let (key, image) = match &state.zoom {
        Some(z) => (z.key, z.image.clone()),
        None => return,
    };
    let image_size = egui::vec2(image.width as f32, image.height as f32);
    let mut marks = finds_for_image(entries, &image);
    marks.extend(image.annotations.iter().map(|a| ImageMark {
        target: Inspect::Annotation(a.clone()),
        rect: a.rect,
    }));

    let consumed = ui.cursor().min.y - start_y;
    let desired = egui::vec2(
        ui.available_width().max(160.0),
        (body_h - consumed).max(160.0),
    );
    let (viewport, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

    image_view::handle_scroll_zoom(
        ui,
        viewport,
        image_size,
        &mut state.zoom_view,
        resp.hovered(),
    );
    if top {
        let plus = ui.input_mut(|i| {
            i.consume_key(Modifiers::NONE, Key::Plus) || i.consume_key(Modifiers::NONE, Key::Equals)
        });
        let minus = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Minus));
        if plus || minus {
            let f = if plus { 1.15 } else { 1.0 / 1.15 };
            let (z2, p) = image_view::zoom_image_at_cursor(
                viewport,
                image_size,
                state.zoom_view.zoom,
                state.zoom_view.pan,
                viewport.center(),
                f,
            );
            state.zoom_view.zoom = z2;
            state.zoom_view.pan = p;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Num0)) {
            state.zoom_view.reset();
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace)) {
            state.zoom = None;
            state.inspect = None;
            return;
        }
    }

    let content = image_view::image_content_rect(
        viewport,
        image_size,
        state.zoom_view.zoom,
        state.zoom_view.pan,
    );
    // Click a detected object inside the image to inspect what was found;
    // clicking empty image space clears the inspect tooltip.
    if resp.clicked() {
        state.inspect = resp
            .interact_pointer_pos()
            .and_then(|pos| mark_at(&marks, content, image_size, pos));
    }
    if let Some(pos) = resp.hover_pos() {
        if mark_at(&marks, content, image_size, pos).is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    let painter = ui.painter_at(viewport);
    if let Some(tex) = state.texture(ui.ctx(), action_id, key, &image) {
        painter.image(
            tex.id(),
            content,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        for m in &marks {
            let stroke = match m.target {
                Inspect::Find(..) => selection_stroke(),
                Inspect::Annotation(_) => annotation_stroke(),
            };
            stroke_find_rect(&painter, content, image.width, image.height, m.rect, stroke);
        }
    } else {
        painter.rect_filled(viewport, 0.0, crate::theme::preview_unavailable_fill());
        painter.text(
            viewport.center(),
            egui::Align2::CENTER_CENTER,
            "(image unavailable)",
            egui::TextStyle::Body.resolve(ui.style()),
            crate::theme::preview_unavailable_fg(),
        );
    }
    let _ = image_view::handle_pan_drag(&resp, viewport, image_size, &mut state.zoom_view);

    // Inspect tooltip pinned above the clicked mark — recomputed each frame so
    // it tracks pan/zoom.
    if state.inspect.is_some() {
        let tip = match &state.inspect {
            Some(Inspect::Find(ei, fi)) => find_at(entries, (*ei, *fi)).and_then(|f| {
                find_context_image(entries, *ei, f)
                    .filter(|(ci, _)| Arc::ptr_eq(&ci.pixels, &image.pixels))
                    .map(|(_, r)| {
                        (
                            f.label.clone(),
                            f.detail.clone(),
                            f.screen,
                            find_rect_on_display(content, image.width, image.height, r),
                        )
                    })
            }),
            Some(Inspect::Annotation(a)) => Some((
                format!("{:?}", a.label),
                None,
                a.screen,
                find_rect_on_display(content, image.width, image.height, a.rect),
            )),
            None => None,
        };
        match tip {
            Some((label, detail, screen, mark)) if mark.is_positive() => {
                egui::Area::new(egui::Id::new(("logs_find_tip", action_id.as_str())))
                    .order(egui::Order::Foreground)
                    .pivot(egui::Align2::CENTER_BOTTOM)
                    .fixed_pos(mark.center_top() + egui::vec2(0.0, -6.0))
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label(egui::RichText::new(label).strong());
                            if let Some(d) = &detail {
                                ui.label(d);
                            }
                            let (l, t, r, b) = screen;
                            ui.weak(format!(
                                "screen ({l},{t})–({r},{b}) · {}×{}",
                                (r - l).max(0),
                                (b - t).max(0)
                            ));
                        });
                    });
            }
            // Mark rotated out or belongs to another image — drop the tip.
            _ => state.inspect = None,
        }
    }
}

fn fit_width(w: f32, h: f32, max_w: f32) -> egui::Vec2 {
    if w <= 0.0 || h <= 0.0 {
        return egui::vec2(64.0, 64.0);
    }
    let scale = (max_w / w).clamp(0.05, 1.0);
    let scale = if w < 96.0 { (96.0 / w).min(4.0) } else { scale };
    egui::vec2(w * scale, h * scale)
}

fn fit_thumb(w: f32, h: f32, max_edge: f32) -> egui::Vec2 {
    if w <= 0.0 || h <= 0.0 {
        return egui::vec2(64.0, 64.0);
    }
    let scale = image_view::fit_scale(w, h, max_edge, max_edge).clamp(0.05, 4.0);
    egui::vec2(w * scale, h * scale)
}

#[cfg(test)]
mod tests {
    use super::{
        entry_matches, item_gallery_metrics, item_indices, section_chunks, step_item, KindFilter,
        CARD_GAP, CARD_MAX, CARD_MIN,
    };
    use sqyre_ports::{ActionLogEntry, LogFind, LogImage};

    fn solid(label: &str) -> LogImage {
        LogImage::from_rgba(label.to_string(), 2, 2, vec![0u8; 16]).unwrap()
    }

    fn pipeline(title: &str) -> ActionLogEntry {
        ActionLogEntry::ItemPipeline {
            title: title.into(),
            summary: "sum".into(),
            thumbnail: solid("t"),
            steps: vec![],
            details: vec!["detail-line".into()],
            finds: vec![LogFind {
                label: "Find #1".into(),
                detail: None,
                screen: (0, 0, 4, 4),
                context: None,
            }],
        }
    }

    fn kinds() -> KindFilter {
        KindFilter::default()
    }

    #[test]
    fn entry_matches_kind_toggles() {
        let text = ActionLogEntry::Text("hello".into());
        let image = ActionLogEntry::Image(solid("cap"));
        let item = pipeline("Item");
        let k = kinds();
        assert!(entry_matches(&text, "", &k));
        assert!(entry_matches(&image, "", &k));
        assert!(entry_matches(&item, "", &k));
        let mut only_text = k;
        only_text.images = false;
        only_text.items = false;
        assert!(entry_matches(&text, "", &only_text));
        assert!(!entry_matches(&image, "", &only_text));
        assert!(!entry_matches(&item, "", &only_text));
    }

    #[test]
    fn entry_matches_query_across_fields() {
        let finds = ActionLogEntry::Finds {
            text: "OCR matched".into(),
            finds: vec![LogFind {
                label: "Find #1 (\"Submit\")".into(),
                detail: Some("found \"Submit\"".into()),
                screen: (0, 0, 4, 4),
                context: None,
            }],
        };
        let k = kinds();
        assert!(entry_matches(&finds, "submit", &k), "find label searched");
        assert!(entry_matches(&finds, "ocr", &k), "text searched");
        assert!(!entry_matches(&finds, "zzzz", &k));
        let item = pipeline("Sword");
        assert!(entry_matches(&item, "detail-line", &k), "details searched");
        assert!(entry_matches(&item, "find", &k), "find labels searched");
        let img = ActionLogEntry::Image(solid("Where found"));
        assert!(entry_matches(&img, "where", &k));
    }

    #[test]
    fn item_indices_respect_filter() {
        let entries = vec![
            ActionLogEntry::Text("a".into()),
            pipeline("Sword"),
            ActionLogEntry::Text("b".into()),
            pipeline("Potion"),
        ];
        let k = kinds();
        assert_eq!(item_indices(&entries, "", &k), vec![1, 3]);
        assert_eq!(item_indices(&entries, "sword", &k), vec![1]);
    }

    #[test]
    fn step_item_walks_indices() {
        let items = vec![1, 3, 7];
        assert_eq!(step_item(&items, None, 1), Some(1));
        assert_eq!(step_item(&items, None, -1), Some(7));
        assert_eq!(step_item(&items, Some(3), 1), Some(7));
        assert_eq!(step_item(&items, Some(3), -1), Some(1));
        assert_eq!(step_item(&items, Some(7), 1), Some(7), "clamped at end");
        assert_eq!(step_item(&items, Some(99), 1), Some(1), "stale → start");
        assert_eq!(step_item(&[], None, 1), None);
    }

    #[test]
    fn section_chunks_group_on_markers() {
        let entries = vec![
            ActionLogEntry::Text("before".into()),
            ActionLogEntry::Section("Attempt 1".into()),
            ActionLogEntry::Text("a1".into()),
            ActionLogEntry::Text("a1b".into()),
            ActionLogEntry::Section("Attempt 2".into()),
            ActionLogEntry::Text("a2".into()),
        ];
        let visible: Vec<usize> = (0..entries.len()).collect();
        let chunks = section_chunks(&entries, &visible);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].label, None);
        assert_eq!(chunks[0].indices, vec![0]);
        assert_eq!(chunks[1].label, Some("Attempt 1"));
        assert_eq!(chunks[1].indices, vec![2, 3]);
        assert_eq!(chunks[2].label, Some("Attempt 2"));
        assert_eq!(chunks[2].indices, vec![5]);
    }

    #[test]
    fn section_chunks_respect_filter() {
        let entries = vec![
            ActionLogEntry::Section("Attempt 1".into()),
            ActionLogEntry::Text("keep".into()),
            ActionLogEntry::Image(solid("cap")),
            ActionLogEntry::Text("keep2".into()),
        ];
        // Only the two text lines are visible (images toggled off).
        let visible = vec![0, 1, 3];
        let chunks = section_chunks(&entries, &visible);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].indices, Vec::<usize>::new());
        assert_eq!(chunks[1].label, Some("Attempt 1"));
        assert_eq!(chunks[1].indices, vec![1, 3]);
    }

    #[test]
    fn few_items_stay_at_max_card_width() {
        let (w, cols) = item_gallery_metrics(2, 640.0);
        assert!((w - CARD_MAX).abs() < 0.01);
        assert_eq!(cols, 2);
    }

    #[test]
    fn many_items_fill_row_and_wrap() {
        let avail = 640.0;
        let (w, cols) = item_gallery_metrics(20, avail);
        let max_cols = crate::pickers::grid_column_count_for_width(avail, CARD_MIN, CARD_GAP);
        assert_eq!(cols, max_cols);
        let expected = (avail - CARD_GAP * (cols as f32 - 1.0)) / cols as f32;
        assert!((w - expected.clamp(CARD_MIN, CARD_MAX)).abs() < 0.01);
    }

    #[test]
    fn wider_window_adds_columns() {
        let (_, narrow) = item_gallery_metrics(20, 420.0);
        let (_, wide) = item_gallery_metrics(20, 900.0);
        assert!(wide > narrow);
    }
}
