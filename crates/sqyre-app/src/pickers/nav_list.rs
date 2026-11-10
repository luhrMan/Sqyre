//! Shared keyboard-nav selectable list body for modal pickers.

use super::scroll::maybe_scroll_to;
use crate::widgets::list_vacancy;
use eframe::egui;

/// Vacancy / scroll chrome for [`paint_nav_rows`].
pub struct NavListChrome<'a> {
    pub scroll_to_selection: &'a mut bool,
    pub vacancy_noun: &'a str,
    pub q: &'a str,
    pub show_vacancy: bool,
}

/// Paint filtered selectable rows with scroll-into-view for the current selection.
///
/// `paint_row` draws one row and returns its response. `on_click` runs when that
/// response is clicked. Returns whether any selected row was scrolled into view.
pub fn paint_nav_rows<T>(
    ui: &mut egui::Ui,
    rows: impl IntoIterator<Item = T>,
    mut is_selected: impl FnMut(&T) -> bool,
    mut paint_row: impl FnMut(&mut egui::Ui, &T, bool) -> egui::Response,
    mut on_click: impl FnMut(&T),
    chrome: NavListChrome<'_>,
) -> bool {
    let mut did_scroll = false;
    let mut visible = 0usize;
    for row in rows {
        visible += 1;
        let selected = is_selected(&row);
        let resp = paint_row(ui, &row, selected);
        if selected && *chrome.scroll_to_selection && !did_scroll {
            maybe_scroll_to(ui, &resp, chrome.scroll_to_selection);
            did_scroll = true;
        }
        if resp.clicked() {
            on_click(&row);
        }
    }
    if chrome.show_vacancy {
        list_vacancy(ui, chrome.q, visible, chrome.vacancy_noun);
    }
    did_scroll
}

/// Convenience for plain text selectable rows (macro picker).
#[expect(
    clippy::too_many_arguments,
    reason = "row iterators plus vacancy chrome; prefer NavListChrome at call sites that need more"
)]
pub fn paint_nav_selectable_rows<T>(
    ui: &mut egui::Ui,
    rows: impl IntoIterator<Item = T>,
    is_selected: impl FnMut(&T) -> bool,
    mut label: impl FnMut(&T) -> egui::RichText,
    on_click: impl FnMut(&T),
    scroll_to_selection: &mut bool,
    vacancy_noun: &str,
    q: &str,
) -> bool {
    paint_nav_rows(
        ui,
        rows,
        is_selected,
        |ui, row, selected| ui.selectable_label(selected, label(row)),
        on_click,
        NavListChrome {
            scroll_to_selection,
            vacancy_noun,
            q,
            show_vacancy: true,
        },
    )
}
