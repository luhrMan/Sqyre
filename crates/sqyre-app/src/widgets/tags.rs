//! Removable tag chips with draft entry and completion suggestions.

use eframe::egui::{self, Key, Modifiers, PopupCloseBehavior, RectAlign};

/// Max height of the tag suggestion dropdown popup.
const TAG_SUGGEST_POPUP_HEIGHT: f32 = 180.0;

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
///
/// Empty `search` returns every unused tag (full dropdown on focus).
pub fn tag_completion_options(
    search: &str,
    already: &[String],
    all_tags: &[String],
) -> Vec<String> {
    let search_l = search.trim().to_lowercase();
    all_tags
        .iter()
        .filter(|t| !already.iter().any(|c| c == *t))
        .filter(|t| search_l.is_empty() || t.to_lowercase().contains(&search_l))
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

/// Try to append a signed Image Search tag filter (`+tag` / `-tag`).
///
/// Bare input defaults to include (`+`). If the same tag name already exists with
/// either polarity, the polarity is updated (or left unchanged) and returns whether
/// the list changed.
pub fn try_add_signed_tag_filter(tags: &mut Vec<String>, raw: &str) -> bool {
    let (include, name) = match sqyre_domain::parse_tag_filter(raw) {
        Some((inc, name)) => (inc, normalize_tag_path(&name)),
        None => {
            let name = normalize_tag_path(raw.trim().trim_start_matches(['+', '-']));
            (true, name)
        }
    };
    if name.is_empty() {
        return false;
    }
    let formatted = sqyre_domain::format_tag_filter(include, &name);
    if let Some(existing) = tags
        .iter_mut()
        .find(|t| sqyre_domain::tag_filter_name(t).as_deref() == Some(name.as_str()))
    {
        if *existing == formatted {
            return false;
        }
        *existing = formatted;
        return true;
    }
    tags.push(formatted);
    true
}

/// Toggle include/exclude polarity of a stored signed filter chip.
pub fn toggle_signed_tag_filter(tags: &mut [String], index: usize) -> bool {
    let Some(entry) = tags.get_mut(index) else {
        return false;
    };
    let Some((include, name)) = sqyre_domain::parse_tag_filter(entry) else {
        return false;
    };
    *entry = sqyre_domain::format_tag_filter(!include, &name);
    true
}

/// Filter `all_tags` by substring match, excluding tag names already present
/// (ignoring `+`/`-` polarity on `already`).
///
/// Empty `search` returns every unused tag (full dropdown on focus).
pub fn tag_completion_options_signed(
    search: &str,
    already: &[String],
    all_tags: &[String],
) -> Vec<String> {
    let search_l = search.trim().to_lowercase();
    let already_names: Vec<String> = already
        .iter()
        .filter_map(|t| tag_filter_bare_name(t))
        .collect();
    all_tags
        .iter()
        .filter(|t| !already_names.iter().any(|c| c == *t))
        .filter(|t| search_l.is_empty() || t.to_lowercase().contains(&search_l))
        .cloned()
        .collect()
}

fn tag_filter_bare_name(raw: &str) -> Option<String> {
    sqyre_domain::tag_filter_name(raw)
        .map(|n| normalize_tag_path(&n))
        .filter(|n| !n.is_empty())
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

/// Capture nav / commit keys for the draft field.
///
/// Enter commits when the draft has text, or when a suggestion is highlighted —
/// so free-typed tags and dropdown picks both work, and parent Enter handlers
/// (e.g. edit-tip Save) do not steal the key.
fn take_tag_suggest_keys(
    ui: &mut egui::Ui,
    was_open: bool,
    selected: Option<usize>,
    draft_has_text: bool,
    draft_focused: bool,
) -> TagSuggestKeys {
    let on_suggest = was_open && selected.is_some();
    let can_accept = draft_has_text || on_suggest;
    let accept =
        can_accept && (draft_focused || was_open) && ui.input(|i| i.key_pressed(Key::Enter));
    if !was_open {
        if accept {
            ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Enter);
            });
            return TagSuggestKeys {
                accept: true,
                ..TagSuggestKeys::default()
            };
        }
        return TagSuggestKeys::default();
    }
    let keys = TagSuggestKeys {
        next: ui.input(|i| {
            i.key_pressed(Key::ArrowDown) || (on_suggest && i.key_pressed(Key::ArrowRight))
        }),
        prev: ui.input(|i| {
            i.key_pressed(Key::ArrowUp) || (on_suggest && i.key_pressed(Key::ArrowLeft))
        }),
        accept,
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

/// Apply draft polarity (`+`/`-`) to a bare suggestion name for signed filters.
fn signed_suggestion_raw(draft: &str, name: &str) -> String {
    let t = draft.trim();
    if t.starts_with('-') {
        format!("-{name}")
    } else if t.starts_with('+') {
        format!("+{name}")
    } else {
        name.to_string()
    }
}

/// Paint removable chips + draft field (+ optional Add button) + suggestion dropdown.
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
    let draft_focused = ui.ctx().memory(|m| m.has_focus(draft_id));
    let keys = take_tag_suggest_keys(
        ui,
        was_open,
        nav.selected,
        !draft.trim().is_empty(),
        draft_focused,
    );

    let tag_resp = if opts.draft_first {
        ui.horizontal_wrapped(|ui| {
            let resp = paint_tag_draft(ui, draft_id, draft, &opts, &mut edit, tags);
            crate::action_tooltip::help::label(ui, "Tags:", opts.draft_hover.unwrap_or(""));
            paint_tag_chips(
                ui,
                tags,
                opts.enabled,
                opts.reorderable,
                opts.signed_filters,
                &mut edit.changed,
            );
            resp
        })
        .inner
    } else {
        ui.horizontal_wrapped(|ui| {
            paint_tag_chips(
                ui,
                tags,
                opts.enabled,
                opts.reorderable,
                opts.signed_filters,
                &mut edit.changed,
            );
        });
        ui.horizontal(|ui| paint_tag_draft(ui, draft_id, draft, &opts, &mut edit, tags))
            .inner
    };
    if refocus {
        tag_resp.request_focus();
    }

    // Focused empty field → full unused-tag list; typing filters it.
    let suggestions = if opts.enabled && (draft_focused || was_open || !draft.trim().is_empty()) {
        if opts.signed_filters {
            let q = draft.trim().trim_start_matches(['+', '-']).trim();
            tag_completion_options_signed(q, tags, all_suggestions)
        } else {
            tag_completion_options(draft, tags, all_suggestions)
        }
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
            .map(|name| {
                if opts.signed_filters {
                    signed_suggestion_raw(draft, &name)
                } else {
                    name
                }
            })
            .or_else(|| {
                let t = draft.trim();
                (!t.is_empty()).then(|| t.to_string())
            });
    }

    // `was_open`: egui drops the draft's focus on the click frame before the popup paints,
    // so the popup must survive that frame for the suggestion click to register.
    let show_popup = opts.enabled
        && pending.is_none()
        && !suggestions.is_empty()
        && (was_open || tag_resp.has_focus() || nav.selected.is_some());
    if show_popup {
        let popup_width = tag_resp.rect.width().max(140.0);
        egui::Popup::from_response(&tag_resp)
            .id(nav_id.with("popup"))
            .open(true)
            .align(RectAlign::BOTTOM_START)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .width(popup_width)
            .show(|ui| {
                ui.set_min_width(popup_width);
                ui.set_max_height(TAG_SUGGEST_POPUP_HEIGHT);
                crate::pickers::dialog_scroll(popup_width, TAG_SUGGEST_POPUP_HEIGHT).show(
                    ui,
                    |ui| {
                        ui.set_max_width(popup_width);
                        for (i, sug) in suggestions.iter().enumerate() {
                            let selected = nav.selected == Some(i);
                            let resp = ui.selectable_label(selected, sug.as_str());
                            if resp.clicked() {
                                pending = Some(if opts.signed_filters {
                                    signed_suggestion_raw(draft, sug)
                                } else {
                                    sug.clone()
                                });
                            }
                            if selected {
                                resp.scroll_to_me(None);
                            }
                        }
                    },
                );
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
        let added = if opts.signed_filters {
            try_add_signed_tag_filter(tags, &raw)
        } else {
            try_add_tag(tags, &raw)
        };
        if added {
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

    // After a commit, reopen next frame once refocused (empty draft → full list).
    let open = opts.enabled
        && !had_pending
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
    signed_filters: bool,
    changed: &mut bool,
) {
    let mut remove: Option<String> = None;
    let mut pending_reorder: Option<(usize, usize)> = None;
    let mut toggle: Option<usize> = None;
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
        .inner_margin(egui::Margin::same(crate::theme::SPACE_2 as i8));

    for (i, tag) in tags.iter().enumerate() {
        let (polarity, label) = if signed_filters {
            match sqyre_domain::parse_tag_filter(tag) {
                Some((true, name)) => ("+", name),
                Some((false, name)) => ("−", name),
                None => ("+", tag.clone()),
            }
        } else {
            ("", tag.clone())
        };
        let paint_chip = |ui: &mut egui::Ui| -> egui::Rect {
            // Zero item_spacing keeps polarity + label as one compact pill.
            let resp = chip
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.spacing_mut().interact_size.y = small_h;
                    ui.horizontal(|ui| {
                        if signed_filters {
                            ui.label(egui::RichText::new(polarity).small().color(fg).strong());
                        }
                        ui.label(egui::RichText::new(label.as_str()).small().color(fg));
                    });
                })
                .response;
            if enabled {
                resp.on_hover_text("Right-click to remove.").rect
            } else {
                resp.rect
            }
        };

        let chip_rect = if reorderable && enabled {
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
            drag.inner
        } else {
            paint_chip(ui)
        };
        if enabled {
            let menu_id = ui.id().with(("tag_chip_menu", i, tag.as_str()));
            crate::widgets::rect_context_menu(ui, menu_id, chip_rect, |ui| {
                if signed_filters {
                    let flip = if polarity == "+" {
                        "Exclude (−)"
                    } else {
                        "Include (+)"
                    };
                    if crate::widgets::menu_item(ui, flip, true) {
                        toggle = Some(i);
                    }
                }
                if crate::widgets::menu_item_danger(ui, "Remove", true) {
                    remove = Some(tag.clone());
                }
            });
        }
    }
    if let Some(i) = toggle {
        if toggle_signed_tag_filter(tags, i) {
            *changed = true;
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
    let hint = if opts.signed_filters {
        "Add +tag or -tag…"
    } else {
        "Add tag…"
    };
    let tag_te = egui::TextEdit::singleline(draft)
        .id(draft_id)
        .desired_width(140.0)
        .hint_text(hint);
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
        let added = if opts.signed_filters {
            try_add_signed_tag_filter(tags, draft)
        } else {
            try_add_tag(tags, draft)
        };
        if added {
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
    pub draft_hover: Option<&'a str>,
    /// When true, paint the draft field before the `Tags:` label in the chip row.
    pub draft_first: bool,
    /// When true, chips can be drag-reordered (tag priority lists).
    pub reorderable: bool,
    /// When true, chips are Image Search `+`/`−` filters with a polarity toggle.
    pub signed_filters: bool,
}

impl Default for TagChipOptions<'_> {
    fn default() -> Self {
        Self {
            enabled: true,
            show_add_button: true,
            draft_hover: None,
            draft_first: false,
            reorderable: false,
            signed_filters: false,
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
        let opts = tag_completion_options("hel", &["healing".into()], &all);
        assert_eq!(opts, vec!["helm".to_string()]);
    }

    #[test]
    fn completion_empty_search_lists_unused() {
        let all = vec!["alpha".into(), "beta".into(), "gamma".into()];
        let opts = tag_completion_options("", &["beta".into()], &all);
        assert_eq!(opts, vec!["alpha".to_string(), "gamma".to_string()]);
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
    fn signed_filter_add_and_toggle() {
        let mut tags = Vec::new();
        assert!(try_add_signed_tag_filter(&mut tags, "weapon"));
        assert_eq!(tags, vec!["+weapon"]);
        assert!(try_add_signed_tag_filter(&mut tags, "-holy"));
        assert_eq!(tags, vec!["+weapon", "-holy"]);
        // Same name updates polarity.
        assert!(try_add_signed_tag_filter(&mut tags, "-weapon"));
        assert_eq!(tags, vec!["-weapon", "-holy"]);
        assert!(toggle_signed_tag_filter(&mut tags, 0));
        assert_eq!(tags, vec!["+weapon", "-holy"]);
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

    fn pointer_click(
        harness: &mut egui_kittest::Harness<'_, (Vec<String>, String)>,
        at: egui::Pos2,
    ) {
        harness.hover_at(at);
        harness.run_steps(2);
        for pressed in [true, false] {
            harness.input_mut().events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.run_steps(1);
        }
        harness.run_steps(2);
    }

    #[test]
    fn clicking_suggestion_adds_tag() {
        use egui_kittest::kittest::Queryable;
        let all: Vec<String> = vec!["alpha".into(), "beta".into()];
        let mut harness = egui_kittest::Harness::builder()
            .with_size([400.0, 300.0])
            .build_ui_state(
                move |ui, (tags, draft): &mut (Vec<String>, String)| {
                    tag_chip_editor(ui, tags, draft, &all, TagChipOptions::default());
                },
                (Vec::new(), String::new()),
            );
        harness.run_steps(2);
        let field = harness
            .get_by_role(egui::accesskit::Role::TextInput)
            .rect()
            .center();
        pointer_click(&mut harness, field);
        let beta = harness.get_by_label("beta").rect().center();
        pointer_click(&mut harness, beta);
        assert_eq!(harness.state().0, vec!["beta".to_string()]);
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
