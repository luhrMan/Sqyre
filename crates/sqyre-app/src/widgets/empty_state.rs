//! Shared empty / filter-miss chrome for lists and panels.
//!
//! Layout: strong title, optional weak sentence, optional primary then secondary CTA.

use eframe::egui;

/// Gap above the title when painting an empty state.
const PAD_TOP: f32 = 8.0;
/// Gap between title and body sentence.
const GAP_TITLE_BODY: f32 = 4.0;
/// Gap between copy and the CTA row.
const GAP_BEFORE_CTA: f32 = 12.0;

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
    ui.add_space(PAD_TOP);
    ui.label(egui::RichText::new(title).strong());
    if let Some(body) = body {
        ui.add_space(GAP_TITLE_BODY);
        ui.label(egui::RichText::new(body).weak());
    }
    let mut clicked = EmptyStateAction::None;
    if primary.is_some() || secondary.is_some() {
        ui.add_space(GAP_BEFORE_CTA);
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
