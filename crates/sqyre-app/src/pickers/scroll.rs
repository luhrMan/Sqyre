use eframe::egui;
use egui::containers::scroll_area::{DragScroll, ScrollSource};

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
            hint_text: None,
        }
    }

    /// Fixed pane that ends at the list (e.g. data editor left column).
    pub fn pane() -> Self {
        Self {
            footer_reserve: 0.0,
            trailing: None,
            below_search: None,
            id_salt: None,
            hint_text: None,
        }
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
    mut opts: PickerScrollOpts<'_>,
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
            if ui.add(edit).changed() {
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
