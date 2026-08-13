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
            id_salt: None,
            hint_text: None,
        }
    }

    /// Fixed pane that ends at the list (e.g. data editor left column).
    pub fn pane() -> Self {
        Self {
            footer_reserve: 0.0,
            trailing: None,
            id_salt: None,
            hint_text: None,
        }
    }
}

/// Search row → separator → capped vertical scroll. `body` receives lowercase trimmed query.
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
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(
            egui_phosphor::regular::MAGNIFYING_GLASS,
        ))
        .on_hover_text("Search");
        let mut edit = egui::TextEdit::singleline(search);
        if let Some(hint) = opts.hint_text {
            edit = edit.hint_text(hint);
        }
        if ui.add(edit).changed() {
            search_changed = true;
        }
        if let Some(trailing) = opts.trailing.as_mut() {
            trailing(ui);
        }
    });
    ui.separator();
    let q = search.trim().to_ascii_lowercase();
    // Fixed panes (footer_reserve == 0) use remaining height only — no popup screen cap.
    let max_h = if opts.footer_reserve <= 0.0 {
        ui.available_height().max(40.0)
    } else {
        popup_scroll_max_height(ui, opts.footer_reserve)
    };
    let mut scroll = scroll_vertical().auto_shrink([false, false]);
    if let Some(salt) = opts.id_salt {
        scroll = scroll.id_salt(salt);
    }
    scroll.max_height(max_h).show(ui, |ui| {
        body(ui, &q);
    });
    search_changed
}

/// Finite height for scroll panes inside content-sized popup windows.
/// Without this, `ScrollArea` + `auto_shrink([false, false])` grows the window forever.
///
/// `footer_reserve` is space still to be laid out below the scroll (buttons, status).
pub fn popup_scroll_max_height(ui: &egui::Ui, footer_reserve: f32) -> f32 {
    const FALLBACK: f32 = 360.0;
    let screen_cap = (ui.ctx().content_rect().height() * 0.65).max(100.0);
    let h = ui.available_height() - footer_reserve;
    let capped = if h.is_finite() { h.max(40.0) } else { FALLBACK };
    capped.min(screen_cap)
}
