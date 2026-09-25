use eframe::egui::{self, Key, Modifiers};
use egui::containers::scroll_area::{DragScroll, ScrollSource};

/// Default search hint for popup pickers.
pub const HINT_LIST: &str = "Search…";
/// Default search hint for fixed left panes (data editor).
pub const HINT_PANE: &str = "Search programs or names…";

/// Local list/dialog keys (consumed so they do not leak under the surface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListNavAction {
    None,
    Up,
    Down,
    Activate,
    Cancel,
}

/// ↑↓ / Enter / Esc for an open list or picker. Prefer over ad-hoc `key_pressed`.
pub fn poll_list_nav(ui: &mut egui::Ui) -> ListNavAction {
    if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        ListNavAction::Cancel
    } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
        ListNavAction::Down
    } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
        ListNavAction::Up
    } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
        ListNavAction::Activate
    } else {
        ListNavAction::None
    }
}

/// Move `selected` within `0..len` for ↑↓. Returns whether Enter should activate.
pub fn apply_list_nav(selected: &mut usize, len: usize, action: ListNavAction) -> bool {
    if len == 0 {
        *selected = 0;
        return false;
    }
    *selected = (*selected).min(len - 1);
    match action {
        ListNavAction::Down => {
            *selected = (*selected + 1).min(len - 1);
            false
        }
        ListNavAction::Up => {
            *selected = selected.saturating_sub(1);
            false
        }
        ListNavAction::Activate => true,
        ListNavAction::None | ListNavAction::Cancel => false,
    }
}

/// Weak empty / no-match copy inside a filtered list body.
pub fn paint_list_vacancy(ui: &mut egui::Ui, query: &str, visible: usize, entity_plural: &str) {
    if visible > 0 {
        return;
    }
    if query.trim().is_empty() {
        ui.weak(format!("No {entity_plural} yet."));
    } else {
        ui.weak(format!("No matching {entity_plural}."));
    }
}

/// Focus the search field once when a picker/dialog opens (`id` should be stable per window).
pub fn focus_search_once(ui: &mut egui::Ui, id: egui::Id, resp: &egui::Response) {
    let key = id.with("focus_search_once");
    let needs = ui.ctx().data_mut(|d| *d.get_temp_mut_or(key, true));
    if needs {
        resp.request_focus();
        ui.ctx().data_mut(|d| d.insert_temp(key, false));
    }
}

/// Clear one-shot focus so the next open focuses again.
pub fn reset_focus_search(ctx: &egui::Context, id: egui::Id) {
    ctx.data_mut(|d| d.insert_temp(id.with("focus_search_once"), true));
}

/// Wheel + scrollbar + click-drag. egui's default drag is touch-only (`OnTouch`).
pub(crate) const SCROLL_SOURCE: ScrollSource = ScrollSource::ALL;

/// Scroll source for areas that implement their own drag-scroll (e.g. macro tree).
pub(crate) const SCROLL_SOURCE_NO_DRAG: ScrollSource = ScrollSource {
    scroll_bar: true,
    drag: DragScroll::Never,
    mouse_wheel: true,
};

/// Vertical [`ScrollArea`] with click-drag scrolling enabled.
pub(crate) fn scroll_vertical() -> egui::ScrollArea {
    egui::ScrollArea::vertical().scroll_source(SCROLL_SOURCE)
}

/// Bidirectional [`ScrollArea`] with click-drag scrolling enabled.
pub(crate) fn scroll_both() -> egui::ScrollArea {
    egui::ScrollArea::both().scroll_source(SCROLL_SOURCE)
}

/// Bidirectional scroll that fills a capped viewport without expanding the parent.
///
/// Vertical-only `ScrollArea` + `auto_shrink([false, false])` expands to content
/// width and ratchets windows off-screen; enabling both axes keeps width at the
/// viewport (`(true, false) => inner_size` in egui).
pub(crate) fn dialog_scroll(max_w: f32, max_h: f32) -> egui::ScrollArea {
    scroll_both()
        .auto_shrink([false, false])
        .max_width(max_w.max(1.0))
        .max_height(max_h.max(1.0))
}

pub(crate) fn maybe_scroll_to(ui: &mut egui::Ui, resp: &egui::Response, scroll: &mut bool) {
    if *scroll {
        ui.scroll_to_rect(resp.rect, Some(egui::Align::Center));
        *scroll = false;
    }
}

/// Options for [`picker_searchable_scroll`].
pub struct PickerScrollOpts<'a> {
    /// Space still laid out below the scroll (Save/Cancel). Use `0` when the list fills the pane.
    pub footer_reserve: f32,
    /// Extra widgets after the search field (e.g. Refresh).
    pub trailing: Option<&'a mut dyn FnMut(&mut egui::Ui)>,
    /// Widgets between the search row and the scroll (outside drag-scroll).
    pub below_search: Option<&'a mut dyn FnMut(&mut egui::Ui)>,
    pub id_salt: Option<&'static str>,
    /// Placeholder text inside the search field.
    pub hint_text: Option<&'a str>,
}

impl PickerScrollOpts<'_> {
    /// Popup list with Save/Cancel (or similar) below the scroll.
    pub fn list(_ui: &egui::Ui) -> Self {
        Self {
            footer_reserve: 52.0,
            trailing: None,
            below_search: None,
            id_salt: None,
            hint_text: Some(HINT_LIST),
        }
    }

    /// Fixed pane that ends at the list (e.g. data editor left column).
    pub fn pane() -> Self {
        Self {
            footer_reserve: 0.0,
            trailing: None,
            below_search: None,
            id_salt: None,
            hint_text: Some(HINT_PANE),
        }
    }

    /// Override search placeholder (or `None` to hide).
    pub fn with_hint(mut self, hint: Option<&'static str>) -> Self {
        self.hint_text = hint;
        self
    }
}

/// Search row → optional below-search → separator → capped vertical scroll.
/// `body` receives lowercase trimmed query.
///
/// Scroll height is measured after the search row so the pane fits remaining space.
/// Returns whether search text changed this frame (callers can re-arm scroll-to-selection).
pub fn picker_searchable_scroll(
    ui: &mut egui::Ui,
    search: &mut String,
    opts: PickerScrollOpts<'_>,
    body: impl FnMut(&mut egui::Ui, &str),
) -> bool {
    picker_searchable_scroll_ex(ui, search, opts, None, body)
}

/// Like [`picker_searchable_scroll`], with optional one-shot search focus (`focus_id`).
pub fn picker_searchable_scroll_ex(
    ui: &mut egui::Ui,
    search: &mut String,
    mut opts: PickerScrollOpts<'_>,
    focus_id: Option<egui::Id>,
    mut body: impl FnMut(&mut egui::Ui, &str),
) -> bool {
    let mut search_changed = false;
    // Run trailing / below-search in a scope so their borrows end before `body`
    // (body often needs the same locals, e.g. data-editor tag priority).
    {
        let mut trailing = opts.trailing.take();
        let mut below_search = opts.below_search.take();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(
                egui_phosphor::regular::MAGNIFYING_GLASS,
            ))
            .on_hover_text("Search");
            // Fill leftover width without a fixed TextEdit min (default ~200)
            // that would floor the data-editor left pane above the split clamp.
            let mut edit = egui::TextEdit::singleline(search).desired_width(f32::INFINITY);
            if let Some(hint) = opts.hint_text {
                edit = edit.hint_text(hint);
            }
            let resp = ui.add(edit);
            if let Some(fid) = focus_id {
                focus_search_once(ui, fid, &resp);
            }
            if resp.changed() {
                search_changed = true;
            }
            if let Some(trailing) = trailing.as_mut() {
                trailing(ui);
            }
        });
        if let Some(below) = below_search.as_mut() {
            below(ui);
        }
    }
    ui.separator();
    let q = search.trim().to_ascii_lowercase();
    // Fixed panes (footer_reserve == 0) use remaining height only — no popup screen cap.
    let pane = opts.footer_reserve <= 0.0;
    let max_h = if pane {
        ui.available_height().max(40.0)
    } else {
        popup_scroll_max_height(ui, opts.footer_reserve)
    };
    // Bidirectional: vertical-only + auto_shrink(false) expands to content width
    // and can push dialog edges off-screen when the pane is narrow.
    let max_w = ui.available_width().max(1.0);
    let mut scroll = dialog_scroll(max_w, max_h);
    if let Some(salt) = opts.id_salt {
        scroll = scroll.id_salt(salt);
    }
    scroll.show(ui, |ui| {
        ui.set_max_width(max_w);
        body(ui, &q);
    });
    search_changed
}

/// Finite height for scroll panes inside popup / dialog windows.
///
/// Uses [`crate::widgets::visible_height`] so resizable dialogs fill with the
/// window without treating leftover room toward Window `max_size` as a minimum
/// (that ratchets the window open — see `fill_resize_body`).
///
/// `footer_reserve` is space still to be laid out below the scroll (buttons, status).
pub fn popup_scroll_max_height(ui: &egui::Ui, footer_reserve: f32) -> f32 {
    (crate::widgets::visible_height(ui) - footer_reserve).max(40.0)
}
