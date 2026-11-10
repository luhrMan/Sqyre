//! Removable tag chips with draft entry and completion suggestions.

use crate::help_text as ht;
use eframe::egui::{self, Key, Modifiers, PopupCloseBehavior, RectAlign, Sense};
use std::collections::HashMap;

/// Max height of the tag suggestion dropdown popup.
const TAG_SUGGEST_POPUP_HEIGHT: f32 = 180.0;

/// Draft field width inside the tag pair (icon and Add sit outside this).
pub(crate) const TAG_DRAFT_WIDTH: f32 = 140.0;

/// Gap between a program icon and the tag text inside a pill or menu row.
const TAG_ICON_GAP: f32 = 4.0;

/// Collapse `/`-separated tag paths: trim, drop empty segments, rejoin.
/// `" combat/pve/ "` → `"combat/pve"`; `"///"` → `""`.
pub fn normalize_tag_path(tag: &str) -> String {
    tag.split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// Process icon for a tag when the tag, or its first path segment, is a program name.
///
/// Signed filters (`+Game`, `−Game/raid`) use the bare name. Missing keys and an
/// absent map paint the tag with no icon.
pub(crate) fn program_icon_for_tag<'a, T>(
    icons: Option<&'a HashMap<String, T>>,
    tag: &str,
) -> Option<&'a T> {
    let icons = icons?;
    let bare = sqyre_domain::tag_filter_name(tag).unwrap_or_default();
    let bare = normalize_tag_path(&bare);
    if bare.is_empty() {
        return None;
    }
    if let Some(icon) = icons.get(&bare) {
        return Some(icon);
    }
    let root = bare.split('/').next().unwrap_or("");
    if root.is_empty() || root == bare {
        None
    } else {
        icons.get(root)
    }
}

/// Icon side inside a Small-text pill: tall enough to read, short enough that
/// the pill stays the text line when the font is already that tall.
fn tag_icon_side(text_h: f32) -> f32 {
    text_h.clamp(12.0, crate::icon_cache::PROCESS_ICON_SIDE)
}

/// Extra pill width for a leading icon, and the icon's side length.
fn tag_icon_extra(text_h: f32, has_icon: bool) -> (f32, f32) {
    if has_icon {
        let side = tag_icon_side(text_h);
        (side + TAG_ICON_GAP, side)
    } else {
        (0.0, 0.0)
    }
}

fn pill_outer_size(text: egui::Vec2, has_icon: bool) -> egui::Vec2 {
    let (extra_w, side) = tag_icon_extra(text.y, has_icon);
    let pad = crate::theme::SPACE_2 * 2.0;
    egui::vec2(text.x + pad + extra_w, text.y.max(side) + pad)
}

/// Width of a tag pill, including a leading program icon when `has_icon`.
pub(crate) fn tag_pill_width(ui: &egui::Ui, text: &str, has_icon: bool) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::TextStyle::Small.resolve(ui.style()),
        egui::Color32::WHITE,
    );
    pill_outer_size(galley.size(), has_icon).x
}

/// Paint `tex` fitted inside a `side` square centered at `center`. Returns the
/// fitted rect. Uses the texture id directly so the icon is not a loading slot.
fn paint_tag_icon(
    ui: &egui::Ui,
    tex: &egui::TextureHandle,
    center: egui::Pos2,
    side: f32,
) -> egui::Rect {
    let [tw, th] = tex.size();
    let size = crate::image_view::fit_icon_thumb(tw as f32, th as f32, side, side);
    let rect = egui::Rect::from_center_size(center, size);
    ui.painter().image(
        tex.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    rect
}

fn paint_tag_suggestion(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    icon: Option<&egui::TextureHandle>,
) -> egui::Response {
    let Some(tex) = icon else {
        return ui.selectable_label(selected, label);
    };
    let side = crate::icon_cache::PROCESS_ICON_SIDE;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
        Sense::click(),
    );
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, selected);
        if selected || response.hovered() {
            ui.painter()
                .rect_filled(rect, visuals.corner_radius, visuals.bg_fill);
        }
        let pad = ui.spacing().button_padding.x;
        let icon_rect = paint_tag_icon(
            ui,
            tex,
            egui::pos2(rect.left() + pad + side * 0.5, rect.center().y),
            side,
        );
        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::TextStyle::Button.resolve(ui.style()),
            visuals.text_color(),
        );
        let text_rect = egui::Rect::from_min_size(
            egui::pos2(
                icon_rect.right() + TAG_ICON_GAP,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley.size(),
        );
        crate::theme::paint_galley_centered(ui, text_rect, galley, visuals.text_color());
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::SelectableLabel, true, label.to_owned())
    });
    response
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
    /// User-typed filter. Arrow-key highlight fills the draft from a suggestion
    /// but leaves this alone so the dropdown does not collapse while browsing.
    query: String,
}

/// Put the highlighted suggestion into `draft`, or restore the typed query when
/// selection returns to the field.
fn apply_suggest_draft(
    draft: &mut String,
    nav: &TagSuggestNav,
    suggestions: &[String],
    signed_filters: bool,
) {
    match nav.selected {
        Some(i) => {
            let Some(name) = suggestions.get(i) else {
                return;
            };
            let fill = if signed_filters {
                signed_suggestion_raw(&nav.query, name)
            } else {
                name.clone()
            };
            if *draft != fill {
                *draft = fill;
            }
        }
        None if *draft != nav.query => {
            *draft = nav.query.clone();
        }
        None => {}
    }
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
    tag_chip_editor_impl(ui, tags, draft, all_suggestions, opts, true).0
}

/// Tag draft field (icon, entry, Add) without chips.
///
/// Same ids, suggestions, and commit behavior as [`tag_chip_editor`]. The rect
/// is the pair (or the draft row when there is no label icon). Chips are
/// painted separately with [`tag_chip_list`].
pub fn tag_draft_editor(
    ui: &mut egui::Ui,
    tags: &mut Vec<String>,
    draft: &mut String,
    all_suggestions: &[String],
    opts: TagChipOptions<'_>,
) -> (TagChipEdit, egui::Rect) {
    tag_chip_editor_impl(ui, tags, draft, all_suggestions, opts, false)
}

/// Height of one empty/created-tag chip row (Small text + pill padding).
///
/// The macro header reserves this even when a macro has no tags so Data Editor /
/// Settings / list-toggle stay on the same plane as a tagged header.
fn tag_chip_row_height(ui: &egui::Ui) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        "Ag".to_owned(),
        egui::TextStyle::Small.resolve(ui.style()),
        egui::Color32::WHITE,
    );
    pill_outer_size(galley.size(), false).y
}

/// Created-tag chips, wrapping within the row. Returns whether `tags` changed
/// and the row's rectangle. An empty list still reserves one chip-row of height
/// so callers keep a stable band under the draft field.
pub fn tag_chip_list(
    ui: &mut egui::Ui,
    tags: &mut Vec<String>,
    enabled: bool,
    reorderable: bool,
    signed_filters: bool,
    program_icons: Option<&HashMap<String, egui::TextureHandle>>,
) -> (bool, egui::Rect) {
    if tags.is_empty() {
        // Height only — do not claim available_width (self-sizing parents ratchet).
        let (_, rect) = ui.allocate_space(egui::vec2(0.0, tag_chip_row_height(ui)));
        return (false, rect);
    }
    let wrap_cap = (ui.clip_rect().right() - ui.cursor().left()).max(20.0);
    let mut changed = false;
    let rect = ui
        .horizontal_wrapped(|ui| {
            ui.set_max_width(wrap_cap);
            paint_tag_chips(
                ui,
                tags,
                &TagChipOptions {
                    enabled,
                    reorderable,
                    signed_filters,
                    program_icons,
                    ..TagChipOptions::default()
                },
                &mut changed,
            );
        })
        .response
        .rect;
    (changed, rect)
}

fn tag_chip_editor_impl(
    ui: &mut egui::Ui,
    tags: &mut Vec<String>,
    draft: &mut String,
    all_suggestions: &[String],
    opts: TagChipOptions<'_>,
    include_chips: bool,
) -> (TagChipEdit, egui::Rect) {
    let mut edit = TagChipEdit::default();
    // Stable across chip-count changes so focus survives add/remove + Update/load_form.
    let draft_id = opts
        .draft_id
        .unwrap_or_else(|| ui.id().with("tag_draft_edit"));
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

    // The wrapping rows wrap at `max_rect`, but the pane's clip can be tighter
    // (left pane cut by the window edge): chips would wrap *past* the clip and
    // clip their labels. Cap the row to the clip so wraps land inside the pane.
    let wrap_cap = (ui.clip_rect().right() - ui.cursor().left()).max(20.0);
    // icon | entrybox | add | tags — chips wrap after the add button, except
    // `single_row` where overflow collapses into a `+N` pill.
    let mut widget_rect = egui::Rect::NOTHING;
    let mut paint_row = |ui: &mut egui::Ui, cap_width: bool| {
        if cap_width {
            ui.set_max_width(wrap_cap);
        }
        let resp = if let Some(name) = opts.label_name {
            let framed =
                crate::widgets::pair_labeled(ui, egui_phosphor::regular::TAG, name, |ui| {
                    paint_tag_draft(ui, draft_id, draft, &opts, &mut edit, tags)
                });
            widget_rect = framed.response.rect;
            framed.inner
        } else {
            let row =
                ui.horizontal(|ui| paint_tag_draft(ui, draft_id, draft, &opts, &mut edit, tags));
            widget_rect = row.response.rect;
            row.inner
        };
        if include_chips {
            paint_locked_tags(ui, &opts, opts.single_row);
            paint_tag_chips(ui, tags, &opts, &mut edit.changed);
        }
        resp
    };
    // A bare `ui.horizontal` is only `interact_size` tall. In a row that already
    // holds a taller pair (the macro name beside this draft) that short box is
    // centered, so the tag field starts lower. The draft is one pair already;
    // a single-row editor is one `wrap_unit` so it asks for its real height.
    //
    // The draft-only editor stays a direct child of `ui`. A scope would wrap
    // inside itself, so a wrapping header row could not move this pair onto
    // the next line.
    let tag_resp = if !include_chips {
        paint_row(ui, false)
    } else if opts.single_row {
        super::wrap_unit(ui, |ui| paint_row(ui, true)).inner
    } else {
        ui.horizontal_wrapped(|ui| paint_row(ui, true)).inner
    };
    if refocus {
        tag_resp.request_focus();
    }

    // Add-button commit clears the draft without going through `pending`.
    if edit.submitted {
        nav = TagSuggestNav::default();
    } else if tag_resp.changed() {
        // Typing (not arrow-fill) owns the filter query and clears highlight.
        nav.query = draft.clone();
        nav.selected = None;
    } else if nav.selected.is_none() && nav.query != *draft {
        // External draft mutation while the field owns focus (e.g. load_form).
        nav.query = draft.clone();
    }

    // Focused empty field → full unused-tag list; typing filters it.
    // Filter from the typed query so arrow-fill does not shrink the list.
    let already = tags_with_locked(tags, opts.locked);
    let suggestions = if opts.enabled
        && (draft_focused || was_open || !nav.query.trim().is_empty() || !draft.trim().is_empty())
    {
        if opts.signed_filters {
            let q = nav.query.trim().trim_start_matches(['+', '-']).trim();
            tag_completion_options_signed(q, &already, all_suggestions)
        } else {
            tag_completion_options(&nav.query, &already, all_suggestions)
        }
    } else {
        Vec::new()
    };
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
    // Arrow highlight fills the entrybox; Up / empty list restores the typed query.
    apply_suggest_draft(draft, &nav, &suggestions, opts.signed_filters);

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
        let icon_pad = if suggestions
            .iter()
            .any(|sug| program_icon_for_tag(opts.program_icons, sug).is_some())
        {
            crate::icon_cache::PROCESS_ICON_SIDE + TAG_ICON_GAP
        } else {
            0.0
        };
        let popup_width = tag_resp.rect.width().max(140.0) + icon_pad;
        let shown = egui::Popup::from_response(&tag_resp)
            .id(nav_id.with("popup"))
            .open(true)
            .align(RectAlign::BOTTOM_START)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .width(popup_width)
            .show(|ui| {
                ui.set_min_width(popup_width);
                ui.set_max_height(TAG_SUGGEST_POPUP_HEIGHT);
                crate::widgets::show_dialog_scroll(
                    ui,
                    popup_width,
                    TAG_SUGGEST_POPUP_HEIGHT,
                    |ui| {
                        ui.set_max_width(popup_width);
                        for (i, sug) in suggestions.iter().enumerate() {
                            let selected = nav.selected == Some(i);
                            let resp = paint_tag_suggestion(
                                ui,
                                sug.as_str(),
                                selected,
                                program_icon_for_tag(opts.program_icons, sug.as_str()),
                            );
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
        if let Some(shown) = shown {
            // Inside `parent_popup` hosts (e.g. the data editor sort menu) a
            // suggestion click lands on this popup's own layer; registering the
            // rect keeps the parent open.
            crate::widgets::register_nested_popup(ui.ctx(), shown.response.rect);
        }
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
        let added = commit_tag(tags, &raw, &opts);
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

    (edit, widget_rect)
}

/// Allocate + paint a Small-text pill: `fill` rounded rect, `SPACE_2` padding,
/// ink optically centered with [`crate::theme::paint_galley_centered`] (galley
/// line boxes sit high — centering the mesh like `paint_pill`/`paint_keycap`
/// keeps the text visually centered). No `Frame`: frame children inherit the
/// parent layout, which mis-centers inside `ui.horizontal`.
///
/// `icon` sits inside the pill, ahead of the text. The text stays optically
/// centered in its own box so the icon does not pull it off the baseline.
fn paint_tag_pill_fill(
    ui: &mut egui::Ui,
    text: &str,
    fill: egui::Color32,
    icon: Option<&egui::TextureHandle>,
) -> egui::Response {
    let fg = crate::theme::contrast_fg(fill);
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::TextStyle::Small.resolve(ui.style()),
        fg,
    );
    let pad = crate::theme::SPACE_2 * 2.0;
    let (_, side) = tag_icon_extra(galley.size().y, icon.is_some());
    let (rect, response) = ui.allocate_exact_size(
        pill_outer_size(galley.size(), icon.is_some()),
        Sense::hover(),
    );
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 6.0, fill);
        let text_rect = if let Some(tex) = icon {
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.left() + pad * 0.5 + side * 0.5, rect.center().y),
                egui::Vec2::splat(side),
            );
            paint_tag_icon(ui, tex, icon_rect.center(), side);
            let left = icon_rect.right() + TAG_ICON_GAP;
            egui::Rect::from_min_size(
                egui::pos2(left, rect.center().y - galley.size().y * 0.5),
                galley.size(),
            )
        } else {
            rect
        };
        crate::theme::paint_galley_centered(ui, text_rect, galley, fg);
    }
    response
}

/// One read-only tag pill — same chrome as the editor chips ([`paint_tag_chips`]):
/// PRIMARY fill, 6px radius, Small contrast text inside a SPACE_2 margin.
/// `icon` is painted inside the pill ahead of the label when present.
/// Interaction (hover text, context menu) is the caller's; the returned
/// response covers the painted pill.
pub fn tag_pill(
    ui: &mut egui::Ui,
    label: &str,
    icon: Option<&egui::TextureHandle>,
) -> egui::Response {
    paint_tag_pill_fill(ui, label, crate::theme::accent(), icon)
}

fn paint_tag_chips(
    ui: &mut egui::Ui,
    tags: &mut Vec<String>,
    opts: &TagChipOptions<'_>,
    changed: &mut bool,
) {
    let enabled = opts.enabled;
    let reorderable = opts.reorderable;
    let signed_filters = opts.signed_filters;
    let single_row = opts.single_row;
    let program_icons = opts.program_icons;
    let mut remove: Option<String> = None;
    let mut pending_reorder: Option<(usize, usize)> = None;
    let mut toggle: Option<usize> = None;
    let mut overflow: usize = 0;
    let fill = if enabled {
        crate::theme::accent()
    } else {
        crate::theme::accent().gamma_multiply(0.5)
    };

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
        // Scope-based children (Frame/dnd_drag_source) place via `new_child`,
        // which never triggers the parent's `main_wrap` — and `available_width`
        // reports the whole row width, not the remainder. A chip that doesn't
        // fit the remaining cursor span would sit past the row end and clip.
        let shown_text = format!("{polarity}{label}");
        let label_galley = ui.painter().layout_no_wrap(
            shown_text,
            egui::TextStyle::Small.resolve(ui.style()),
            egui::Color32::WHITE,
        );
        let icon = program_icon_for_tag(program_icons, &label).cloned();
        let (icon_extra, _) = tag_icon_extra(label_galley.size().y, icon.is_some());
        let chip_w = label_galley.size().x + crate::theme::SPACE_2 * 2.0 + icon_extra;
        // `Frame`/`dnd` children expand `region.max_rect` as they allocate, so
        // `available_rect_before_wrap` over-reports space near the pane edge.
        // Wrap when the chip would cross the painted clip instead.
        let row_right = ui.max_rect().right().min(ui.clip_rect().right());
        let at_row_start = ui.cursor().left() <= ui.max_rect().left() + 0.5;
        if ui.cursor().left() + chip_w > row_right && (single_row || !at_row_start) {
            if single_row {
                // `end_row` can't help inside `ui.horizontal`: a chip painted
                // anyway would have its label clipped at the row edge. Count
                // the ones that don't fit and show a `+N` pill instead — even
                // at row start, where an elided sliver would paint past the clip.
                overflow += 1;
                continue;
            }
            ui.end_row();
        }
        let paint_chip = |ui: &mut egui::Ui| -> egui::Rect {
            // Elide to the row's width: a chip wider than the pane cannot wrap
            // below itself, so its label would clip the pane edge. Bound by the
            // clip — `available_width` over-reports past it (see wrap check).
            let chip_font = egui::TextStyle::Small.resolve(ui.style());
            let row_right = ui.max_rect().right().min(ui.clip_rect().right());
            let budget = (row_right
                - ui.cursor().left()
                - crate::theme::SPACE_2 * 2.0
                - icon_extra
                - if signed_filters { 10.0 } else { 0.0 })
            .max(20.0);
            let shown =
                crate::widgets::elide::elide_to_width(ui, label.as_str(), budget, chip_font);
            let resp = paint_tag_pill_fill(
                ui,
                &format!("{polarity}{}", shown.text),
                fill,
                icon.as_ref(),
            );
            if enabled && !signed_filters {
                let tip = if shown.elided {
                    format!("{label} — right-click for options.")
                } else {
                    "Right-click for options.".to_owned()
                };
                resp.on_hover_text(tip).rect
            } else {
                resp.rect
            }
        };

        let (chip_rect, chip_anchor) = if reorderable && enabled {
            let id = ui.id().with(("tag_dnd", i, tag.as_str()));
            let drag = ui.dnd_drag_source(id, i, paint_chip);
            drag.response.clone().on_hover_text(ht::TAG_CHIP_ORDER);
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
            (drag.inner, Some(drag.response))
        } else {
            let rect = paint_chip(ui);
            let anchor = if enabled && signed_filters {
                // Not a drag source — the chip itself can take a click to flip
                // the include/exclude sign. Kept off draggable chips: a
                // click-sense overlay would swallow the press and break reorder.
                let resp = ui.interact(
                    rect,
                    ui.id().with(("tag_chip", i, tag.as_str())),
                    Sense::click(),
                );
                crate::widgets::controls::paint_keyboard_focus_ring(ui, rect, &resp);
                let info_label = format!("Tag filter {tag} — click to toggle include/exclude");
                resp.clone().widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, info_label.clone())
                });
                if resp.clicked() {
                    toggle = Some(i);
                }
                let resp = resp.on_hover_text(ht::TAG_CHIP_TOGGLE);
                Some(resp)
            } else {
                None
            };
            (rect, anchor)
        };
        if enabled {
            let menu_id = ui.id().with(("tag_chip_menu", i, tag.as_str()));
            crate::widgets::rect_context_menu(ui, menu_id, chip_rect, chip_anchor.as_ref(), |ui| {
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
    if overflow > 0 {
        let label = format!("+{overflow}");
        let hidden = tags[tags.len() - overflow..].join(", ");
        let chip_w = ui
            .painter()
            .layout_no_wrap(
                label.clone(),
                egui::TextStyle::Small.resolve(ui.style()),
                egui::Color32::WHITE,
            )
            .size()
            .x
            + crate::theme::SPACE_2 * 2.0;
        let row_right = ui.max_rect().right().min(ui.clip_rect().right());
        if ui.cursor().left() + chip_w <= row_right {
            paint_tag_pill_fill(ui, &label, fill, None)
                .on_hover_text(format!("{overflow} more: {hidden}"));
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
    let hint = if let Some(hint) = opts.draft_hint {
        hint
    } else if opts.signed_filters {
        "Add +tag or -tag…"
    } else {
        "Add tag…"
    };
    let tag_te = egui::TextEdit::singleline(draft)
        .id(draft_id)
        .desired_width(opts.draft_width.unwrap_or(TAG_DRAFT_WIDTH))
        .hint_text(hint);
    let mut tag_resp = ui.add_enabled(opts.enabled, tag_te);
    // Hint alone is not always the AccessKit name; pin it so tests/AT can find the field.
    tag_resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, hint));
    if let Some(tip) = opts.draft_hover {
        tag_resp = tag_resp.on_hover_text(tip);
    }
    let add_clicked =
        opts.show_add_button && crate::widgets::add_button(ui, "Add tag", opts.enabled).clicked();
    if opts.enabled && add_clicked {
        let added = commit_tag(tags, draft, opts);
        if added {
            edit.changed = true;
        }
        // Mark submitted so the entrybox is refocused even when the add was a no-op.
        edit.submitted = true;
        draft.clear();
    }
    tag_resp
}

#[derive(Clone, Copy)]
pub struct TagChipOptions<'a> {
    pub enabled: bool,
    pub show_add_button: bool,
    /// Field name for the tag icon ahead of the draft entry; `None` paints no icon.
    pub label_name: Option<&'a str>,
    /// Icon tooltip / draft hover text.
    pub draft_hover: Option<&'a str>,
    /// Override the draft field hint (and AccessKit name when empty).
    pub draft_hint: Option<&'a str>,
    /// When true, chips can be drag-reordered (tag priority lists).
    pub reorderable: bool,
    /// When true, chips are Image Search `+`/`−` filters with a polarity toggle.
    pub signed_filters: bool,
    /// When true, keep one horizontal row; chips past the pane edge collapse
    /// into a `+N` pill instead of wrapping.
    pub single_row: bool,
    /// Chips painted ahead of `tags` that cannot be added again or removed.
    pub locked: &'a [String],
    /// Hover text for [`Self::locked`] chips.
    pub locked_hover: Option<&'a str>,
    /// Program name → process icon. A chip or suggestion paints the icon when
    /// its tag, or the tag's first path segment, is a key.
    pub program_icons: Option<&'a HashMap<String, egui::TextureHandle>>,
    /// Draft entry width. `None` keeps [`TAG_DRAFT_WIDTH`].
    pub draft_width: Option<f32>,
    /// Stable draft [`egui::Id`]. `None` derives one from the current [`egui::Ui`],
    /// which changes if the editor moves between a fixed row and a wrapping row.
    pub draft_id: Option<egui::Id>,
}

impl Default for TagChipOptions<'_> {
    fn default() -> Self {
        Self {
            enabled: true,
            show_add_button: true,
            label_name: None,
            draft_hover: None,
            draft_hint: None,
            reorderable: false,
            signed_filters: false,
            single_row: false,
            locked: &[],
            locked_hover: None,
            program_icons: None,
            draft_width: None,
            draft_id: None,
        }
    }
}

fn tags_with_locked(tags: &[String], locked: &[String]) -> Vec<String> {
    let mut already = Vec::with_capacity(tags.len() + locked.len());
    already.extend(tags.iter().cloned());
    already.extend(locked.iter().cloned());
    already
}

fn blocked_by_locked(raw: &str, locked: &[String], signed_filters: bool) -> bool {
    let name = if signed_filters {
        sqyre_domain::tag_filter_name(raw)
            .unwrap_or_else(|| normalize_tag_path(raw.trim().trim_start_matches(['+', '-'])))
    } else {
        normalize_tag_path(raw)
    };
    !name.is_empty() && locked.iter().any(|tag| normalize_tag_path(tag) == name)
}

fn commit_tag(tags: &mut Vec<String>, raw: &str, opts: &TagChipOptions<'_>) -> bool {
    if blocked_by_locked(raw, opts.locked, opts.signed_filters) {
        return false;
    }
    if opts.signed_filters {
        try_add_signed_tag_filter(tags, raw)
    } else {
        try_add_tag(tags, raw)
    }
}

/// Name-synced chips. They use the same pill as editable tags and do not offer Remove.
fn paint_locked_tags(ui: &mut egui::Ui, opts: &TagChipOptions<'_>, single_row: bool) {
    if opts.locked.is_empty() {
        return;
    }
    let fill = if opts.enabled {
        crate::theme::accent()
    } else {
        crate::theme::accent().gamma_multiply(0.5)
    };
    for tag in opts.locked {
        let label = tag.trim();
        if label.is_empty() {
            continue;
        }
        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::TextStyle::Small.resolve(ui.style()),
            egui::Color32::WHITE,
        );
        let icon = program_icon_for_tag(opts.program_icons, label).cloned();
        let chip_w = pill_outer_size(galley.size(), icon.is_some()).x;
        let row_right = ui.max_rect().right().min(ui.clip_rect().right());
        let at_row_start = ui.cursor().left() <= ui.max_rect().left() + 0.5;
        if ui.cursor().left() + chip_w > row_right && (single_row || !at_row_start) {
            if single_row {
                continue;
            }
            ui.end_row();
        }
        let resp = paint_tag_pill_fill(ui, label, fill, icon.as_ref());
        let resp = if let Some(tip) = opts.locked_hover {
            resp.on_hover_text(tip)
        } else {
            resp
        };
        let access = format!("Tag {label}");
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, true, access.clone())
        });
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

    #[test]
    fn apply_suggest_draft_fills_and_restores() {
        let suggestions = ["alpha".into(), "beta".into()];
        let mut draft = String::from("al");
        let mut nav = TagSuggestNav {
            selected: Some(0),
            query: "al".into(),
        };
        apply_suggest_draft(&mut draft, &nav, &suggestions, false);
        assert_eq!(draft, "alpha");
        nav.selected = Some(1);
        apply_suggest_draft(&mut draft, &nav, &suggestions, false);
        assert_eq!(draft, "beta");
        nav.selected = None;
        apply_suggest_draft(&mut draft, &nav, &suggestions, false);
        assert_eq!(draft, "al");
        draft.clear();
        nav.query = "+al".into();
        nav.selected = Some(0);
        apply_suggest_draft(&mut draft, &nav, &suggestions, true);
        assert_eq!(draft, "+alpha");
    }

    #[test]
    fn arrow_highlight_fills_draft_without_collapsing_list() {
        use egui_kittest::kittest::Queryable;
        let all: Vec<String> = vec!["alpha".into(), "alpine".into(), "beta".into()];
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
        harness
            .get_by_role(egui::accesskit::Role::TextInput)
            .type_text("al");
        harness.run_steps(2);
        assert_eq!(harness.state().1, "al");
        harness.key_press(Key::ArrowDown);
        harness.run_steps(2);
        assert_eq!(harness.state().1, "alpha");
        // Typed filter is preserved: second Down still reaches alpine.
        harness.key_press(Key::ArrowDown);
        harness.run_steps(2);
        assert_eq!(harness.state().1, "alpine");
        assert!(
            harness.query_by_label("alpha").is_some() && harness.query_by_label("alpine").is_some(),
            "arrow-fill must not collapse the filtered suggestion list"
        );
        harness.key_press(Key::ArrowUp);
        harness.run_steps(2);
        assert_eq!(harness.state().1, "alpha");
        harness.key_press(Key::ArrowUp);
        harness.run_steps(2);
        assert_eq!(harness.state().1, "al");
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
    fn single_row_chip_at_row_start_overflows_instead_of_sliver() {
        // single_row: a chip that cannot fit must count toward the `+N` pill —
        // never paint an elided `b…` sliver. The wrapped-mode escape hatch for
        // chips at row start must not apply: there is no row below to wrap to.
        let mut harness = egui_kittest::Harness::builder()
            .with_size([300.0, 60.0])
            .build_ui_state(
                move |ui, tags: &mut Vec<String>| {
                    let pane =
                        egui::Rect::from_min_size(egui::pos2(100.0, 8.0), egui::vec2(30.0, 24.0));
                    let mut row = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                    row.set_clip_rect(egui::Rect::from_min_max(
                        pane.min,
                        egui::pos2(120.0, pane.max.y),
                    ));
                    row.horizontal(|ui| {
                        let mut changed = false;
                        paint_tag_chips(
                            ui,
                            tags,
                            &TagChipOptions {
                                single_row: true,
                                ..TagChipOptions::default()
                            },
                            &mut changed,
                        );
                    });
                },
                vec!["backup".to_string()],
            );
        harness.run_steps(3);
        let texts: Vec<String> = harness
            .output()
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t == "+1"),
            "expected the +1 overflow pill, got {texts:?}"
        );
        assert!(
            !texts
                .iter()
                .any(|t| t.contains("backup") || t.contains('…')),
            "elided chip sliver painted: {texts:?}"
        );
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

    #[test]
    fn locked_name_tag_is_not_added() {
        let locked = vec!["Game".to_string()];
        let opts = TagChipOptions {
            locked: &locked,
            ..TagChipOptions::default()
        };
        let mut tags = Vec::new();
        assert!(!commit_tag(&mut tags, "Game", &opts));
        assert!(!commit_tag(&mut tags, " Game ", &opts));
        assert!(tags.is_empty());
        assert!(commit_tag(&mut tags, "loot", &opts));
        assert_eq!(tags, vec!["loot".to_string()]);
    }

    #[test]
    fn locked_name_tag_tooltip_width_does_not_ratchet() {
        let ctx = egui::Context::default();
        let locked = vec!["Game".to_string()];
        let mut tags = Vec::new();
        let mut draft = String::new();
        let mut hover: Option<egui::Pos2> = None;
        let mut widths = Vec::new();
        for frame in 0..10 {
            let events = hover
                .map(|pos| vec![egui::Event::PointerMoved(pos)])
                .unwrap_or_default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1024.0, 768.0),
                )),
                time: Some(frame as f64 * 0.25),
                predicted_dt: 0.25,
                events,
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| {
                ui.ctx()
                    .all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);
                tag_chip_editor(
                    ui,
                    &mut tags,
                    &mut draft,
                    &[],
                    TagChipOptions {
                        locked: &locked,
                        locked_hover: Some(crate::action_tooltip::help::DE_PROGRAM_NAME_TAG),
                        ..TagChipOptions::default()
                    },
                );
            });
            if hover.is_none() {
                hover = out.shapes.iter().find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if text.galley.text() == "Game" => {
                        Some(text.pos + text.galley.size() * 0.5)
                    }
                    _ => None,
                });
            }
            if let Some(width) = out.shapes.iter().find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text().contains("program's name") => {
                    Some(text.galley.size().x)
                }
                _ => None,
            }) {
                widths.push(width);
            }
            out.drop_without_applying_deltas();
        }
        assert!(
            widths.len() >= 2,
            "name-tag tooltip never opened (hover={hover:?})"
        );
        let (first, last) = (widths[0], *widths.last().unwrap());
        assert!(
            (last - first).abs() <= 1.0,
            "name-tag tooltip width ratcheted: {first:.1} -> {last:.1} ({widths:?})"
        );
    }

    #[test]
    fn program_icon_follows_name_nested_path_and_sign() {
        let icons = HashMap::from([("Game".to_string(), 7u8)]);
        assert_eq!(program_icon_for_tag(Some(&icons), "Game"), Some(&7));
        assert_eq!(program_icon_for_tag(Some(&icons), " Game/raid "), Some(&7));
        assert_eq!(program_icon_for_tag(Some(&icons), "+Game"), Some(&7));
        assert_eq!(program_icon_for_tag(Some(&icons), "-Game/night"), Some(&7));
        assert_eq!(program_icon_for_tag(Some(&icons), "Other"), None);
        assert_eq!(program_icon_for_tag::<u8>(None, "Game"), None);
    }

    #[test]
    fn program_icon_is_prepended_on_pill_and_suggestion() {
        let ctx = egui::Context::default();
        let tex = ctx.load_texture(
            "game-icon",
            egui::ColorImage::filled([4, 4], egui::Color32::RED),
            egui::TextureOptions::LINEAR,
        );
        let tex_id = tex.id();
        let mut program_icons = HashMap::new();
        program_icons.insert("Game".to_string(), tex.clone());
        let suggestions = vec!["Game".to_string()];
        let mut tags = Vec::new();
        let mut draft = String::new();
        let mut click: Option<egui::Pos2> = None;
        let mut pill_ok = false;
        let mut menu_ok = false;
        for frame in 0..8 {
            let mut events = Vec::new();
            if let Some(pos) = click {
                if frame == 1 {
                    events.push(egui::Event::PointerMoved(pos));
                    events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    });
                } else if frame == 2 {
                    events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
            }
            let out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(480.0, 320.0),
                    )),
                    time: Some(frame as f64 * 0.1),
                    predicted_dt: 0.1,
                    events,
                    ..Default::default()
                },
                |ui| {
                    tag_pill(ui, "Game", Some(&tex));
                    tag_chip_editor(
                        ui,
                        &mut tags,
                        &mut draft,
                        &suggestions,
                        TagChipOptions {
                            program_icons: Some(&program_icons),
                            ..TagChipOptions::default()
                        },
                    );
                },
            );
            if click.is_none() {
                click = out.shapes.iter().find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if text.galley.text().contains("Add tag") => {
                        Some(text.pos + text.galley.size() * 0.5)
                    }
                    _ => None,
                });
            }
            let games: Vec<egui::Rect> = out
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if text.galley.text() == "Game" => {
                        let visual = text.galley.mesh_bounds.translate(text.pos.to_vec2());
                        Some(if visual.is_positive() {
                            visual
                        } else {
                            egui::Rect::from_min_size(text.pos, text.galley.size())
                        })
                    }
                    _ => None,
                })
                .collect();
            let icons: Vec<egui::Rect> = out
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Mesh(mesh) if mesh.texture_id == tex_id => Some(mesh_bounds(mesh)),
                    _ => None,
                })
                .collect();
            let pills: Vec<egui::Rect> = out
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect) => Some(rect.rect),
                    _ => None,
                })
                .collect();
            let icon_left_of = |text: &egui::Rect| {
                icons.iter().find(|icon| {
                    icon.right() <= text.left() + 2.0
                        && (icon.center().y - text.center().y).abs() < 14.0
                })
            };
            if let Some(text) = games.first() {
                if let Some(icon) = icon_left_of(text) {
                    if pills
                        .iter()
                        .any(|pill| pill.contains_rect(*icon) && pill.contains_rect(*text))
                    {
                        pill_ok = true;
                    }
                }
            }
            if games
                .iter()
                .filter(|text| icon_left_of(text).is_some())
                .count()
                >= 2
            {
                menu_ok = true;
            }
            out.drop_without_applying_deltas();
        }
        assert!(pill_ok, "program icon was not inside the Game pill");
        assert!(
            menu_ok,
            "program icon was not prepended in the suggestion row"
        );
    }

    fn mesh_bounds(mesh: &egui::Mesh) -> egui::Rect {
        let mut min = egui::pos2(f32::MAX, f32::MAX);
        let mut max = egui::pos2(f32::MIN, f32::MIN);
        for v in &mesh.vertices {
            min = min.min(v.pos);
            max = max.max(v.pos);
        }
        egui::Rect::from_min_max(min, max)
    }
}
