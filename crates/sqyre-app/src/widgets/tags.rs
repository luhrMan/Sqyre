//! Removable tag chips with draft entry and completion suggestions.

use eframe::egui;

/// Filter `all_tags` by substring match, excluding tags already present.
pub fn tag_completion_options(
    search: &str,
    already: &[String],
    all_tags: &[String],
    limit: usize,
) -> Vec<String> {
    let search_l = search.trim().to_lowercase();
    if search_l.is_empty() {
        return Vec::new();
    }
    all_tags
        .iter()
        .filter(|t| !already.iter().any(|c| c == *t))
        .filter(|t| t.to_lowercase().contains(&search_l))
        .take(limit)
        .cloned()
        .collect()
}

/// Try to append a trimmed unique tag. Returns true when the list changed.
pub fn try_add_tag(tags: &mut Vec<String>, raw: &str) -> bool {
    let t = raw.trim();
    if t.is_empty() || tags.iter().any(|x| x == t) {
        return false;
    }
    tags.push(t.to_string());
    true
}

/// Remove the first matching tag. Returns true when the list changed.
pub fn remove_tag(tags: &mut Vec<String>, tag: &str) -> bool {
    let before = tags.len();
    tags.retain(|t| t != tag);
    tags.len() != before
}

/// Paint removable chips + draft field (+ optional Add button) + suggestions.
///
/// Returns `true` when `tags` changed.
pub fn tag_chip_editor(
    ui: &mut egui::Ui,
    tags: &mut Vec<String>,
    draft: &mut String,
    all_suggestions: &[String],
    opts: TagChipOptions<'_>,
) -> bool {
    let mut changed = false;

    if opts.draft_first {
        ui.horizontal_wrapped(|ui| {
            paint_tag_draft(ui, draft, &opts, &mut changed, tags);
            let label = ui.label("Tags:");
            if let Some(tip) = opts.draft_hover {
                label.on_hover_text(tip);
            }
            paint_tag_chips(ui, tags, opts.enabled, &mut changed);
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            paint_tag_chips(ui, tags, opts.enabled, &mut changed);
        });

        ui.horizontal(|ui| {
            paint_tag_draft(ui, draft, &opts, &mut changed, tags);
        });
    }

    if opts.enabled && !draft.trim().is_empty() {
        let suggestions =
            tag_completion_options(draft, tags, all_suggestions, opts.suggestion_limit);
        if !suggestions.is_empty() {
            if opts.suggestions_with_separator {
                ui.separator();
            }
            ui.horizontal_wrapped(|ui| {
                for sug in suggestions {
                    if ui.small_button(&sug).clicked() {
                        if try_add_tag(tags, &sug) {
                            changed = true;
                        }
                        draft.clear();
                    }
                }
            });
        }
    }

    changed
}

fn paint_tag_chips(ui: &mut egui::Ui, tags: &mut Vec<String>, enabled: bool, changed: &mut bool) {
    let mut remove: Option<String> = None;
    let fill = if enabled {
        crate::theme::PRIMARY
    } else {
        crate::theme::PRIMARY.gamma_multiply(0.5)
    };
    let fg = crate::theme::contrast_fg(crate::theme::PRIMARY);
    let chip = egui::Frame::NONE
        .fill(fill)
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(5, 1));

    for tag in tags.iter() {
        // Pill wraps label + × so `horizontal_wrapped` treats each chip as one unit.
        chip.show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tag.as_str()).size(11.0).color(fg));
                if ui
                    .add_enabled(
                        enabled,
                        egui::Button::new(
                            egui::RichText::new("×")
                                .size(11.0)
                                .color(crate::theme::MACRO_STOP),
                        )
                        .frame(false)
                        .min_size(egui::vec2(10.0, 10.0)),
                    )
                    .on_hover_text("Remove tag")
                    .clicked()
                {
                    remove = Some(tag.clone());
                }
            });
        });
    }
    if let Some(tag) = remove {
        if remove_tag(tags, &tag) {
            *changed = true;
        }
    }
}

fn paint_tag_draft(
    ui: &mut egui::Ui,
    draft: &mut String,
    opts: &TagChipOptions<'_>,
    changed: &mut bool,
    tags: &mut Vec<String>,
) {
    let tag_te = egui::TextEdit::singleline(draft)
        .desired_width(140.0)
        .hint_text("Add tag…");
    let mut tag_resp = ui.add_enabled(opts.enabled, tag_te);
    if let Some(tip) = opts.draft_hover {
        tag_resp = tag_resp.on_hover_text(tip);
    }
    // Singleline TextEdit loses focus on Enter, so check lost_focus — not has_focus.
    let add_enter = tag_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let add_clicked = opts.show_add_button
        && ui
            .add_enabled(
                opts.enabled,
                egui::Button::new(egui::RichText::new("Add tag").color(crate::theme::MACRO_START)),
            )
            .clicked();
    if opts.enabled && (add_enter || add_clicked) {
        if try_add_tag(tags, draft) {
            *changed = true;
        }
        draft.clear();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TagChipOptions<'a> {
    pub enabled: bool,
    pub show_add_button: bool,
    pub suggestion_limit: usize,
    pub suggestions_with_separator: bool,
    pub draft_hover: Option<&'a str>,
    /// When true, paint the draft field before the `Tags:` label in the chip row.
    pub draft_first: bool,
}

impl Default for TagChipOptions<'_> {
    fn default() -> Self {
        Self {
            enabled: true,
            show_add_button: true,
            suggestion_limit: 8,
            suggestions_with_separator: false,
            draft_hover: None,
            draft_first: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_filters_and_excludes() {
        let all = vec![
            "healing".into(),
            "helm".into(),
            "herb".into(),
            "other".into(),
        ];
        let opts = tag_completion_options("hel", &["healing".into()], &all, 10);
        assert_eq!(opts, vec!["helm".to_string()]);
    }

    #[test]
    fn add_remove_unique() {
        let mut tags = vec!["alpha".into()];
        assert!(!try_add_tag(&mut tags, "  "));
        assert!(!try_add_tag(&mut tags, "alpha"));
        assert!(try_add_tag(&mut tags, "beta"));
        assert_eq!(tags, vec!["alpha", "beta"]);
        assert!(remove_tag(&mut tags, "alpha"));
        assert_eq!(tags, vec!["beta"]);
    }
}
