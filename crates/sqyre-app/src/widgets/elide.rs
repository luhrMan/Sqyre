//! Shared single-line text elision helpers.

use eframe::egui::{self, Color32, FontId, Response, TextFormat, TextStyle, Ui};
use std::sync::Arc;

/// Elided display text and whether any content was hidden.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Elided {
    pub text: String,
    pub elided: bool,
}

/// Elide `text` to a single line that fits `max_width`, appending `…` only when needed.
pub fn elide_to_width(ui: &Ui, text: &str, max_width: f32, font: FontId) -> Elided {
    if text.is_empty() {
        return Elided {
            text: String::new(),
            elided: false,
        };
    }

    let full = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE);
    if full.size().x <= max_width {
        return Elided {
            text: text.to_owned(),
            elided: false,
        };
    }

    const ELLIPSIS: char = '…';
    let ellipsis_w = ui
        .painter()
        .layout_no_wrap(ELLIPSIS.to_string(), font.clone(), Color32::WHITE)
        .size()
        .x;
    let budget = (max_width - ellipsis_w).max(0.0);
    if budget <= 0.0 {
        return Elided {
            text: ELLIPSIS.to_string(),
            elided: true,
        };
    }

    let char_count = text.chars().count();
    let mut lo = 0usize;
    let mut hi = char_count;
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let candidate: String = text.chars().take(mid).collect();
        let width = ui
            .painter()
            .layout_no_wrap(candidate, font.clone(), Color32::WHITE)
            .size()
            .x;
        if width <= budget {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    let mut display: String = text.chars().take(lo).collect();
    display.push(ELLIPSIS);
    Elided {
        text: display,
        elided: true,
    }
}

/// Lay out a single-line galley, using an ellipsis when it exceeds `max_width`.
pub fn single_line_galley(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.painter().layout_job(job)
}

/// Attach the complete text when `resp` represents an elided string.
pub fn text_hover_if(resp: Response, full: &str, elided: bool) -> Response {
    if elided {
        resp.on_hover_text(full)
    } else {
        resp
    }
}

pub(crate) fn combo_text_elided(ui: &Ui, response: &Response, text: &str) -> bool {
    let text_width = crate::widgets::text_width(ui, text, TextStyle::Button);
    let text_budget = (response.rect.width()
        - ui.spacing().button_padding.x * 2.0
        - ui.spacing().icon_spacing
        - ui.spacing().icon_width)
        .max(0.0);
    text_width > text_budget + 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elide_to_width_reports_fit_long_and_tiny_budgets() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = TextStyle::Body.resolve(ui.style());
            let fitting = elide_to_width(ui, "Short", 200.0, font.clone());
            assert_eq!(
                fitting,
                Elided {
                    text: "Short".to_owned(),
                    elided: false,
                }
            );

            let long = elide_to_width(ui, "A long label", 30.0, font.clone());
            assert!(long.elided);
            assert!(long.text.ends_with('…'));

            let tiny = elide_to_width(ui, "A long label", 0.0, font);
            assert_eq!(
                tiny,
                Elided {
                    text: "…".to_owned(),
                    elided: true,
                }
            );
        })
        .drop_without_applying_deltas();
    }

    #[test]
    fn single_line_galley_marks_overflow_as_elided() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let galley = single_line_galley(
                ui,
                "A long label",
                TextStyle::Body.resolve(ui.style()),
                Color32::WHITE,
                30.0,
            );
            assert!(galley.elided);
            assert!(galley.size().x <= 30.0);
        })
        .drop_without_applying_deltas();
    }
}
