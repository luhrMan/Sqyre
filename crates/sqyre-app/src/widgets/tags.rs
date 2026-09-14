//! Removable tag chips with draft entry and completion suggestions.

use eframe::egui::{self, Key, Modifiers};

/// Collapse `/`-separated tag paths: trim, drop empty segments, rejoin.
/// `" combat/pve/ "` → `"combat/pve"`; `"///"` → `""`.
pub fn normalize_tag_path(tag: &str) -> String {
    tag.split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// True when `tag` equals `prefix` or is a nested path under it (`prefix/...`).
pub fn tag_is_under_or_eq(tag: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return tag.is_empty();
    }
    tag == prefix
        || tag
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// True when `filters` contains `path` or an ancestor path that covers it.
pub fn filters_cover_path(filters: &[String], path: &str) -> bool {
    filters.iter().any(|f| tag_is_under_or_eq(path, f))
}

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
/// Paths are normalized (`a//b/` → `a/b`) so nested tags stay consistent.
pub fn try_add_tag(tags: &mut Vec<String>, raw: &str) -> bool {
    let t = normalize_tag_path(raw);
    if t.is_empty() || tags.iter().any(|x| normalize_tag_path(x) == t) {
        return false;
    }
    tags.push(t);
    true
}

/// Remove the first matching tag. Returns true when the list changed.
pub fn remove_tag(tags: &mut Vec<String>, tag: &str) -> bool {
    let before = tags.len();
    tags.retain(|t| t != tag);
    tags.len() != before
}

/// Result of one [`tag_chip_editor`] frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct TagChipEdit {
    /// `tags` was mutated (add or remove).
    pub changed: bool,
    /// A tag was committed (Enter, Add, or suggestion). Caller should persist/update.
    pub submitted: bool,
}

/// `None` = draft field; `Some(i)` = suggestion index.
/// Down/Right move onto and along suggestions; Up/Left return toward the field.
fn step_tag_suggest_selection(selected: Option<usize>, len: usize, next: bool) -> Option<usize> {
    if len == 0 {
        return None;
    }
    if next {
        Some(selected.map(|i| (i + 1).min(len - 1)).unwrap_or(0))
    } else {
        match selected {
            None | Some(0) => None,
            Some(i) => Some(i - 1),
        }
    }
}

#[derive(Clone, Default)]
struct TagSuggestNav {
    /// `None` keeps keyboard focus on the draft field.
    selected: Option<usize>,
    query: String,
}

#[derive(Clone, Copy, Default)]
struct TagSuggestKeys {
    next: bool,
    prev: bool,
    accept: bool,
}

/// Capture nav keys when suggestions were showing last frame.
fn take_tag_suggest_keys(
    ui: &mut egui::Ui,
    was_open: bool,
    selected: Option<usize>,
    draft_has_text: bool,
) -> TagSuggestKeys {
    if !was_open {
        return TagSuggestKeys::default();
    }
    let on_suggest = selected.is_some();
    let keys = TagSuggestKeys {
        next: ui.input(|i| {
            i.key_pressed(Key::ArrowDown) || (on_suggest && i.key_pressed(Key::ArrowRight))
        }),
        prev: ui.input(|i| {
            i.key_pressed(Key::ArrowUp) || (on_suggest && i.key_pressed(Key::ArrowLeft))
        }),
        accept: (on_suggest || draft_has_text) && ui.input(|i| i.key_pressed(Key::Enter)),
    };
    ui.input_mut(|i| {
        i.consume_key(Modifiers::NONE, Key::ArrowDown);
        i.consume_key(Modifiers::NONE, Key::ArrowUp);
        if on_suggest {
            i.consume_key(Modifiers::NONE, Key::ArrowLeft);
            i.consume_key(Modifiers::NONE, Key::ArrowRight);
        }
        if keys.accept {
            i.consume_key(Modifiers::NONE, Key::Enter);
        }
    });
    keys
}

/// Paint removable chips + draft field (+ optional Add button) + suggestions.
///
/// Returns whether `tags` changed and whether a tag was committed.
pub fn tag_chip_editor(
    ui: &mut egui::Ui,
    tags: &mut Vec<String>,
    draft: &mut String,
    all_suggestions: &[String],
    opts: TagChipOptions<'_>,
) -> TagChipEdit {
    let mut edit = TagChipEdit::default();
    // Stable across chip-count changes so focus survives add/remove + Update/load_form.
    let draft_id = ui.id().with("tag_draft_edit");
    let nav_id = ui.id().with("tag_suggest_nav");
    let open_id = nav_id.with("open");
    let refocus_id = nav_id.with("refocus");
    let mut nav = ui
        .ctx()
        .data(|d| d.get_temp::<TagSuggestNav>(nav_id))
        .unwrap_or_default();
    let was_open = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(open_id))
        .unwrap_or(false);
    let refocus = ui
        .ctx()
        .data_mut(|d| d.remove_temp::<bool>(refocus_id))
        .unwrap_or(false);
    let keys = take_tag_suggest_keys(ui, was_open, nav.selected, !draft.trim().is_empty());

    let tag_resp = if opts.draft_first {
        ui.horizontal_wrapped(|ui| {
            let resp = paint_tag_draft(ui, draft_id, draft, &opts, &mut edit, tags);
            crate::action_tooltip::help::label(ui, "Tags:", opts.draft_hover.unwrap_or(""));
            paint_tag_chips(ui, tags, opts.enabled, opts.reorderable, &mut edit.changed);
            resp
        })
        .inner
    } else {
        ui.horizontal_wrapped(|ui| {
            paint_tag_chips(ui, tags, opts.enabled, opts.reorderable, &mut edit.changed);
        });
        ui.horizontal(|ui| paint_tag_draft(ui, draft_id, draft, &opts, &mut edit, tags))
            .inner
    };
    if refocus {
        tag_resp.request_focus();
    }

    let suggestions = if opts.enabled && !draft.trim().is_empty() {
        tag_completion_options(draft, tags, all_suggestions, opts.suggestion_limit)
    } else {
        Vec::new()
    };
    if nav.query != *draft {
        nav.query = draft.clone();
        nav.selected = None;
    }
    if let Some(i) = nav.selected {
        if i >= suggestions.len() {
            nav.selected = suggestions.len().checked_sub(1);
        }
    }
    if !suggestions.is_empty() && (was_open || tag_resp.has_focus()) {
        if keys.next {
            nav.selected = step_tag_suggest_selection(nav.selected, suggestions.len(), true);
        }
        if keys.prev {
            nav.selected = step_tag_suggest_selection(nav.selected, suggestions.len(), false);
        }
        // First frame suggestions appear: Down was not consumed above.
        if !was_open && nav.selected.is_none() && ui.input(|i| i.key_pressed(Key::ArrowDown)) {
            nav.selected = step_tag_suggest_selection(None, suggestions.len(), true);
            ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::ArrowDown);
            });
        }
    } else if suggestions.is_empty() {
        nav.selected = None;
    }

    let mut pending: Option<String> = None;
    if keys.accept {
        pending = nav
            .selected
            .and_then(|i| suggestions.get(i).cloned())
            .or_else(|| {
                let t = draft.trim();
                (!t.is_empty()).then(|| t.to_string())
            });
    }

    if opts.enabled && !suggestions.is_empty() {
        if opts.suggestions_with_separator {
            ui.separator();
        }
        ui.horizontal_wrapped(|ui| {
            for (i, sug) in suggestions.iter().enumerate() {
                let selected = nav.selected == Some(i);
                if ui
                    .add(egui::Button::new(sug).small().selected(selected))
                    .clicked()
                {
                    pending = Some(sug.clone());
                }
            }
        });
    }

    if pending.is_none()
        && opts.enabled
        && tag_resp.lost_focus()
        && ui.input(|i| i.key_pressed(Key::Enter))
    {
        let t = draft.trim();
        if !t.is_empty() {
            pending = Some(t.to_string());
        }
    }

    let had_pending = pending.is_some();
    if let Some(raw) = pending {
        if try_add_tag(tags, &raw) {
            edit.changed = true;
            edit.submitted = true;
        }
        draft.clear();
        nav = TagSuggestNav::default();
    }

    // Keep the entrybox focused so multiple tags can be added without re-clicking.
    // Next-frame flag covers Enter/button focus steal and Data Editor Update/load_form.
    if edit.submitted || had_pending {
        tag_resp.request_focus();
        ui.ctx().data_mut(|d| d.insert_temp(refocus_id, true));
    }

    let open = opts.enabled
        && !draft.trim().is_empty()
        && !suggestions.is_empty()
        && (tag_resp.has_focus() || nav.selected.is_some());
    ui.ctx().data_mut(|d| {
        d.insert_temp(nav_id, nav);
        d.insert_temp(open_id, open);
    });

    edit
}

fn paint_tag_chips(
    ui: &mut egui::Ui,
    tags: &mut Vec<String>,
    enabled: bool,
    reorderable: bool,
    changed: &mut bool,
) {
    let mut remove: Option<String> = None;
    let mut pending_reorder: Option<(usize, usize)> = None;
    let small_h = ui.text_style_height(&egui::TextStyle::Small);
    let fill = if enabled {
        crate::theme::PRIMARY
    } else {
        crate::theme::PRIMARY.gamma_multiply(0.5)
    };
    let fg = crate::theme::contrast_fg(crate::theme::PRIMARY);
    let chip = egui::Frame::NONE
        .fill(fill)
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::same(2));

    for (i, tag) in tags.iter().enumerate() {
        let mut paint_chip = |ui: &mut egui::Ui| {
            // Pill wraps label + × so `horizontal_wrapped` treats each chip as one unit.
            chip.show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.spacing_mut().button_padding = egui::vec2(0.0, 0.0);
                ui.spacing_mut().interact_size.y = small_h;
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(tag.as_str()).small().color(fg));
                    if ui
                        .add_enabled(
                            enabled,
                            egui::Button::new(
                                egui::RichText::new("×")
                                    .small()
                                    .color(crate::theme::MACRO_STOP),
                            )
                            .frame(false)
                            .min_size(egui::vec2(small_h, small_h)),
                        )
                        .on_hover_text("Remove tag")
                        .clicked()
                    {
                        remove = Some(tag.clone());
                    }
                });
            });
        };

        if reorderable && enabled {
            let id = ui.id().with(("tag_dnd", i, tag.as_str()));
            let drag = ui.dnd_drag_source(id, i, paint_chip);
            if let Some(payload) = drag.response.dnd_release_payload::<usize>() {
                let from = *payload;
                if from != i {
                    pending_reorder = Some((from, i));
                }
            } else if drag.response.dnd_hover_payload::<usize>().is_some() {
                ui.painter().rect_stroke(
                    drag.response.rect,
                    6.0,
                    egui::Stroke::new(1.5, crate::theme::MACRO_START),
                    egui::StrokeKind::Outside,
                );
            }
        } else {
            paint_chip(ui);
        }
    }
    if let Some((from, to)) = pending_reorder {
        if from < tags.len() && to < tags.len() && from != to {
            if from < to {
                tags[from..=to].rotate_left(1);
            } else {
                tags[to..=from].rotate_right(1);
            }
            *changed = true;
        }
    }
    if let Some(tag) = remove {
        if remove_tag(tags, &tag) {
            *changed = true;
        }
    }
}

fn paint_tag_draft(
    ui: &mut egui::Ui,
    draft_id: egui::Id,
    draft: &mut String,
    opts: &TagChipOptions<'_>,
    edit: &mut TagChipEdit,
    tags: &mut Vec<String>,
) -> egui::Response {
    let tag_te = egui::TextEdit::singleline(draft)
        .id(draft_id)
        .desired_width(140.0)
        .hint_text("Add tag…");
    let mut tag_resp = ui.add_enabled(opts.enabled, tag_te);
    if let Some(tip) = opts.draft_hover {
        tag_resp = tag_resp.on_hover_text(tip);
    }
    let add_clicked = opts.show_add_button
        && ui
            .add_enabled(
                opts.enabled,
                egui::Button::new(egui::RichText::new("Add tag").color(crate::theme::MACRO_START)),
            )
            .clicked();
    if opts.enabled && add_clicked {
        if try_add_tag(tags, draft) {
            edit.changed = true;
        }
        // Mark submitted so the entrybox is refocused even when the add was a no-op.
        edit.submitted = true;
        draft.clear();
    }
    tag_resp
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
    /// When true, chips can be drag-reordered (tag priority lists).
    pub reorderable: bool,
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
            reorderable: false,
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

    #[test]
    fn suggest_selection_moves_from_field_and_back() {
        assert_eq!(step_tag_suggest_selection(None, 3, true), Some(0));
        assert_eq!(step_tag_suggest_selection(Some(0), 3, true), Some(1));
        assert_eq!(step_tag_suggest_selection(Some(2), 3, true), Some(2));
        assert_eq!(step_tag_suggest_selection(Some(0), 3, false), None);
        assert_eq!(step_tag_suggest_selection(Some(2), 3, false), Some(1));
        assert_eq!(step_tag_suggest_selection(None, 3, false), None);
        assert_eq!(step_tag_suggest_selection(None, 0, true), None);
    }

    #[test]
    fn normalize_and_under_paths() {
        assert_eq!(normalize_tag_path(" combat/pve/ "), "combat/pve");
        assert_eq!(normalize_tag_path("a//b"), "a/b");
        assert_eq!(normalize_tag_path("///"), "");
        assert!(tag_is_under_or_eq("combat", "combat"));
        assert!(tag_is_under_or_eq("combat/pve", "combat"));
        assert!(!tag_is_under_or_eq("combatant", "combat"));
        assert!(!tag_is_under_or_eq("combat", "combat/pve"));
        assert!(filters_cover_path(&["combat".into()], "combat/pve"));
        assert!(!filters_cover_path(&["combat/pve".into()], "combat"));
    }
}
