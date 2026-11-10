use eframe::egui::{self, Key, Modifiers};

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
    if crate::widgets::consume_escape(ui) {
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
    /// Red eraser that clears the query.
    pub clearable: bool,
    /// Stable edit id so a sibling eraser does not drop keyboard focus.
    pub stable_id: Option<egui::Id>,
    /// Viewport height. `None` uses the footer/pane rule.
    pub max_height: Option<f32>,
    /// Use [`popup_scroll_max_height`] even when [`Self::footer_reserve`] is 0
    /// (dialogs that cap to the visible window, not the leftover ui height).
    pub use_visible_cap: bool,
}

/// Search row plus the scroll under it.
pub struct PickerScrollOutput {
    pub search_changed: bool,
    /// The search field, when the row drew one. Callers that focus on open
    /// request focus here in addition to [`crate::widgets::focus_search_once`].
    #[expect(
        dead_code,
        reason = "returned for callers that request focus after paint; unused while focus_id covers it"
    )]
    pub search_response: Option<egui::Response>,
    #[expect(
        dead_code,
        reason = "content size for callers measuring list overflow; reserved with the scroll shell"
    )]
    pub content_size: egui::Vec2,
    /// Height of the search row and the below-search widgets.
    #[expect(
        dead_code,
        reason = "chrome height for pane layout; reserved with the scroll shell"
    )]
    pub chrome_height: f32,
}

impl PickerScrollOpts<'_> {
    /// Popup list that fills remaining height; Save floats over the scroll
    /// via [`crate::widgets::float_bottom_right`] (no reserved footer strip).
    /// Override [`Self::footer_reserve`] when real content is laid out below.
    pub fn list(_ui: &egui::Ui) -> Self {
        Self {
            footer_reserve: 0.0,
            trailing: None,
            below_search: None,
            id_salt: None,
            hint_text: Some(HINT_LIST),
            clearable: false,
            stable_id: None,
            max_height: None,
            use_visible_cap: false,
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
            clearable: false,
            stable_id: None,
            max_height: None,
            use_visible_cap: false,
        }
    }

    /// Override search placeholder (or `None` to hide).
    pub fn with_hint(mut self, hint: Option<&'static str>) -> Self {
        self.hint_text = hint;
        self
    }
}

/// Search row → optional below-search → capped vertical scroll.
///
/// No separator under the search bar — pickers stay flush (point / items / macros
/// share this chrome). `body` receives the lowercase trimmed query.
///
/// Scroll height is measured after the search row so the pane fits remaining space.
/// Returns whether search text changed this frame (callers can re-arm scroll-to-selection).
pub fn picker_searchable_scroll(
    ui: &mut egui::Ui,
    search: &mut String,
    opts: PickerScrollOpts<'_>,
    body: impl FnMut(&mut egui::Ui, &str),
) -> PickerScrollOutput {
    picker_searchable_scroll_ex(ui, search, opts, None, body)
}

/// Like [`picker_searchable_scroll`], with optional one-shot search focus (`focus_id`).
pub fn picker_searchable_scroll_ex(
    ui: &mut egui::Ui,
    search: &mut String,
    mut opts: PickerScrollOpts<'_>,
    focus_id: Option<egui::Id>,
    mut body: impl FnMut(&mut egui::Ui, &str),
) -> PickerScrollOutput {
    let mut search_changed = false;
    let mut search_response = None;
    let chrome_start = ui.min_rect().height();
    // Run trailing / below-search in a scope so their borrows end before `body`
    // (body often needs the same locals, e.g. data-editor tag priority).
    {
        let mut trailing = opts.trailing.take();
        let mut below_search = opts.below_search.take();
        let clearable = opts.clearable;
        let stable_id = opts.stable_id;
        // `hint_text: None` hides the search field (query still filters); the
        // row then exists only for the trailing buttons.
        let has_search = opts.hint_text.is_some();
        if has_search || trailing.is_some() {
            crate::widgets::show_overflow(ui, "picker_search_row", 1.0, |ui| {
                crate::widgets::fill_row(
                    ui,
                    |ui| {
                        if let Some(trailing) = trailing.as_mut() {
                            trailing(ui);
                        }
                    },
                    |ui| {
                        if !has_search {
                            return;
                        }
                        // Fill leftover width without a fixed TextEdit min (default ~200)
                        // that would floor the data-editor left pane above the split clamp.
                        let out = crate::widgets::search_field(
                            ui,
                            search,
                            crate::widgets::SearchFieldOpts {
                                hint: opts.hint_text.unwrap_or_default(),
                                focus_id,
                                clearable,
                                stable_id,
                                trailing: None,
                            },
                        );
                        if out.changed {
                            search_changed = true;
                        }
                        search_response = Some(out.response);
                    },
                );
            });
        }
        if let Some(below) = below_search.as_mut() {
            crate::widgets::show_overflow(ui, "picker_below_search", 0.45, |ui| {
                below(ui);
            });
        }
    }
    let chrome_height = ui.min_rect().height() - chrome_start;
    let q = search.trim().to_ascii_lowercase();
    // Fixed panes use remaining height. Dialogs that pass `use_visible_cap`
    // (or a real footer) cap to the visible window instead.
    let max_h = if let Some(h) = opts.max_height {
        h
    } else if opts.use_visible_cap || opts.footer_reserve > 0.0 {
        popup_scroll_max_height(ui, opts.footer_reserve)
    } else {
        ui.available_height().max(40.0)
    };
    // Bidirectional: vertical-only + auto_shrink(false) expands to content width
    // and can push dialog edges off-screen when the pane is narrow.
    let max_w = ui.available_width().max(1.0);
    let scroll = crate::widgets::show_dialog_scroll_mod(
        ui,
        max_w,
        max_h,
        |s| match opts.id_salt {
            Some(salt) => s.id_salt(salt),
            None => s,
        },
        |ui| {
            // Cap soft wrap to the viewport; hard min-widths still H-scroll via both axes.
            ui.set_max_width(max_w);
            body(ui, &q);
        },
    );
    PickerScrollOutput {
        search_changed,
        search_response,
        content_size: scroll.content_size,
        chrome_height,
    }
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
