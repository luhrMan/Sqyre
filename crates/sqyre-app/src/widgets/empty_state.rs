//! Shared empty / filter-miss chrome for lists and panels.
//!
//! Layout: strong title, optional weak sentence, optional primary then secondary CTA.

use crate::theme::{SPACE_12, SPACE_4, SPACE_8};
use eframe::egui;

/// Which optional action button was clicked, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmptyStateAction {
    #[default]
    None,
    Primary,
    Secondary,
}

/// Title + optional body for a list vacancy (zero rows or filter miss).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListVacancyCopy {
    pub title: String,
    pub body: &'static str,
}

/// Copy for an empty or filter-miss list body (`entity_plural` e.g. `"macros"`).
pub fn list_vacancy_copy(query: &str, entity_plural: &str) -> ListVacancyCopy {
    if query.trim().is_empty() {
        ListVacancyCopy {
            title: format!("No {entity_plural} yet"),
            body: "Nothing here yet — add one when you are ready.",
        }
    } else {
        ListVacancyCopy {
            title: format!("No matching {entity_plural}"),
            body: "Try a different search, or clear the filter.",
        }
    }
}

/// Paint a titled empty state: strong title, optional weak body, optional CTAs.
///
/// Primary is painted before secondary. Does not claim full available height
/// (avoids side-panel / dialog size ratchet).
pub fn empty_state(
    ui: &mut egui::Ui,
    title: &str,
    body: Option<&str>,
    primary: Option<&str>,
    secondary: Option<&str>,
) -> EmptyStateAction {
    ui.add_space(SPACE_8);
    // A single word wider than the pane (extreme user-dragged splits) wraps
    // mid-word ("collection/s yet"); truncating reads cleaner there.
    let font = egui::TextStyle::Body.resolve(ui.style());
    let avail = ui.available_width();
    let splits_word = title.split_whitespace().any(|w| {
        ui.painter()
            .layout_no_wrap(w.to_owned(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
            > avail
    });
    let title_label = egui::Label::new(egui::RichText::new(title).strong());
    if splits_word {
        ui.add(title_label.truncate());
    } else {
        ui.add(title_label);
    }
    if let Some(body) = body {
        ui.add_space(SPACE_4);
        ui.label(egui::RichText::new(body).weak());
    }
    let mut clicked = EmptyStateAction::None;
    if primary.is_some() || secondary.is_some() {
        ui.add_space(SPACE_12);
        ui.horizontal(|ui| {
            if let Some(label) = primary {
                if ui.button(label).clicked() {
                    clicked = EmptyStateAction::Primary;
                }
            }
            if let Some(label) = secondary {
                if ui.button(label).clicked() {
                    clicked = EmptyStateAction::Secondary;
                }
            }
        });
    }
    clicked
}

/// Titled empty / no-match copy inside a filtered list body (no CTAs).
pub fn list_vacancy(ui: &mut egui::Ui, query: &str, visible: usize, entity_plural: &str) {
    if visible > 0 {
        return;
    }
    let copy = list_vacancy_copy(query, entity_plural);
    let _ = empty_state(ui, &copy.title, Some(copy.body), None, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_vacancy_copy_empty_vs_filter() {
        let empty = list_vacancy_copy("", "macros");
        assert_eq!(empty.title, "No macros yet");
        assert!(!empty.body.is_empty());

        let filtered = list_vacancy_copy("xyz", "programs");
        assert_eq!(filtered.title, "No matching programs");
        assert!(filtered.body.contains("clear the filter"));
    }

    #[test]
    fn list_vacancy_copy_trims_query() {
        let copy = list_vacancy_copy("   ", "items");
        assert_eq!(copy.title, "No items yet");
    }

    #[test]
    fn empty_state_action_default_is_none() {
        assert_eq!(EmptyStateAction::default(), EmptyStateAction::None);
    }
}
