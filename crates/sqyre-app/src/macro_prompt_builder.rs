//! Local AI Macro Builder: NL editor with `@` autocomplete, prompt copy, YAML import.

use crate::data_editor::helpers::is_editor_listed_program;
use eframe::egui::{
    self, text::CCursor, text_edit::TextEditState, Key, Modifiers, PopupCloseBehavior, RectAlign,
};
use egui::text_selection::CCursorRange;
use sqyre_domain::{action_type_table, Macro, PROGRAM_DELIMITER};
use sqyre_persist::ProgramCatalog;
use sqyre_serialize::decode_macro_from_yaml;

const WINDOW_ID: &str = "sqyre_macro_prompt_builder";
const AC_LIMIT: usize = 16;
const NL_ROWS: usize = 6;
const PROMPT_ROWS: usize = 10;
const YAML_ROWS: usize = 8;

/// Outcome from one frame of the builder window.
#[derive(Debug, Default)]
pub enum PromptBuilderOutcome {
    #[default]
    None,
    /// Decoded + validated macro ready for transactional insert.
    ImportMacro(Box<Macro>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionKind {
    Action,
    Macro,
    Entity,
    KindPrefix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub kind: SuggestionKind,
    pub label: String,
    pub insert: String,
    pub hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IncompleteAt {
    /// Character index of `@`.
    pub start_char: usize,
    /// Text after `@` up to the cursor (may include `kind:…`).
    pub query: String,
}

#[derive(Clone, Default)]
struct AtAutocompleteNav {
    selected: usize,
    query: String,
}

#[derive(Debug, Default)]
pub struct MacroPromptBuilderUi {
    pub open: bool,
    nl_draft: String,
    generated_prompt: String,
    import_yaml: String,
    status: Option<String>,
    status_error: bool,
}

impl MacroPromptBuilderUi {
    pub fn open_builder(&mut self) {
        self.open = true;
        self.status = None;
        self.status_error = false;
    }

    /// Paint the builder. Returns a macro ready to import when the user confirms.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        macros: &[Macro],
        catalog: &ProgramCatalog,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
    ) -> PromptBuilderOutcome {
        if !self.open {
            return PromptBuilderOutcome::None;
        }

        let suggestions = collect_suggestions(macros, catalog);
        let mut outcome = PromptBuilderOutcome::None;
        let mut open = self.open;

        crate::widgets::fit_dialog_popup(
            egui::Window::new("AI Macro Builder")
                .open(&mut open)
                .resizable(true)
                .default_width(640.0)
                .default_height(720.0)
                .min_size([420.0, 480.0]),
            ctx,
            egui::Id::new(WINDOW_ID),
            pending_scale,
        )
        .show(ctx, |ui| {
            crate::widgets::fill_resize_body(ui, |ui| {
                self.body(ui, macros, catalog, &suggestions, &mut outcome);
            });
        });

        self.open = open;
        outcome
    }

    fn body(
        &mut self,
        ui: &mut egui::Ui,
        macros: &[Macro],
        catalog: &ProgramCatalog,
        suggestions: &[Suggestion],
        outcome: &mut PromptBuilderOutcome,
    ) {
        let avail_h = ui.available_height();
        let section_gap = ui.spacing().item_spacing.y * 2.0;
        // Rough thirds: NL, prompt, import.
        let third = ((avail_h - section_gap * 2.0) / 3.0).max(120.0);

        ui.label(
            egui::RichText::new("Describe the macro")
                .strong()
                .size(ui.style().text_styles[&egui::TextStyle::Heading].size * 0.85),
        );
        ui.small("Type @ for actions, macros, and catalog entities (Program~Name).");
        let nl_h = (third - 36.0).max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("nl_scroll")
            .max_height(nl_h)
            .show(ui, |ui| {
                at_ref_text_edit(
                    ui,
                    egui::Id::new(WINDOW_ID).with("nl"),
                    &mut self.nl_draft,
                    suggestions,
                    NL_ROWS,
                );
            });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.nl_draft.trim().is_empty(),
                    egui::Button::new("Generate prompt"),
                )
                .on_hover_text("Build a schema-aware prompt for an external AI tool")
                .clicked()
            {
                self.generated_prompt = build_prompt(&self.nl_draft, macros, catalog);
                self.set_status("Prompt generated — copy and paste into any AI tool.", false);
            }
            if ui
                .add_enabled(
                    !self.generated_prompt.is_empty(),
                    egui::Button::new("Copy prompt"),
                )
                .clicked()
            {
                ui.ctx().copy_text(self.generated_prompt.clone());
                self.set_status("Prompt copied to clipboard.", false);
            }
        });

        ui.separator();
        ui.label(egui::RichText::new("Generated prompt").strong());
        let prompt_h = (third - 28.0).max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("prompt_scroll")
            .max_height(prompt_h)
            .show(ui, |ui| {
                let mut prompt = self.generated_prompt.clone();
                ui.add(
                    egui::TextEdit::multiline(&mut prompt)
                        .id(egui::Id::new(WINDOW_ID).with("prompt"))
                        .desired_width(f32::INFINITY)
                        .desired_rows(PROMPT_ROWS)
                        .interactive(false)
                        .font(egui::TextStyle::Monospace),
                );
            });

        ui.separator();
        ui.label(egui::RichText::new("Import AI response").strong());
        ui.small("Paste the YAML macro returned by the AI (no markdown fences).");
        let yaml_h = (third - 56.0).max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("yaml_scroll")
            .max_height(yaml_h)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.import_yaml)
                        .id(egui::Id::new(WINDOW_ID).with("yaml"))
                        .desired_width(f32::INFINITY)
                        .desired_rows(YAML_ROWS)
                        .hint_text("name: …\nroot:\n  type: loop\n  …")
                        .font(egui::TextStyle::Monospace),
                );
            });

        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.import_yaml.trim().is_empty(),
                    egui::Button::new("Validate"),
                )
                .clicked()
            {
                match prepare_import_macro(&self.import_yaml, &[]) {
                    Ok(m) => self.set_status(
                        format!("Valid macro \"{}\" — ready to import.", m.name),
                        false,
                    ),
                    Err(e) => self.set_status(e, true),
                }
            }
            let existing: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();
            if ui
                .add_enabled(
                    !self.import_yaml.trim().is_empty(),
                    egui::Button::new("Import macro"),
                )
                .on_hover_text("Decode, validate, and add as a new macro")
                .clicked()
            {
                match prepare_import_macro(&self.import_yaml, &existing) {
                    Ok(m) => {
                        self.set_status(format!("Importing \"{}\"…", m.name), false);
                        self.import_yaml.clear();
                        *outcome = PromptBuilderOutcome::ImportMacro(Box::new(m));
                    }
                    Err(e) => self.set_status(e, true),
                }
            }
        });

        if let Some(msg) = &self.status {
            let color = if self.status_error {
                crate::theme::error_fg()
            } else {
                crate::theme::ok_fg()
            };
            ui.colored_label(color, msg);
        }
    }

    fn set_status(&mut self, msg: impl Into<String>, error: bool) {
        self.status = Some(msg.into());
        self.status_error = error;
    }

    /// Status line after a successful import (called from app shell).
    pub(crate) fn set_status_after_import(&mut self, msg: impl Into<String>) {
        self.set_status(msg, false);
    }
}

// ── Autocomplete ────────────────────────────────────────────────────────────

fn at_ref_text_edit(
    ui: &mut egui::Ui,
    id: egui::Id,
    value: &mut String,
    suggestions: &[Suggestion],
    rows: usize,
) {
    let ac_id = id.with("at_ac");
    let was_open = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(ac_id.with("open")))
        .unwrap_or(false);
    let (down, up, accept, dismiss) = take_ac_keys(ui, was_open);

    let output = egui::TextEdit::multiline(value)
        .id(id)
        .desired_width(f32::INFINITY)
        .desired_rows(rows)
        .hint_text(
            "e.g. Click @entity:General~Monitor 1 Center when @action:imagesearch finds the button",
        )
        .show(ui);

    let cursor_char = output.cursor_range.map(|r| r.primary.index.0);
    show_at_autocomplete(
        ui,
        id,
        &output.response,
        value,
        cursor_char,
        suggestions,
        AcKeys {
            down,
            up,
            accept,
            dismiss,
        },
    );
}

#[derive(Clone, Copy, Default)]
struct AcKeys {
    down: bool,
    up: bool,
    accept: bool,
    dismiss: bool,
}

fn take_ac_keys(ui: &mut egui::Ui, was_open: bool) -> (bool, bool, bool, bool) {
    if !was_open {
        return (false, false, false, false);
    }
    let down = ui.input(|i| i.key_pressed(Key::ArrowDown));
    let up = ui.input(|i| i.key_pressed(Key::ArrowUp));
    let accept = ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Tab));
    let dismiss = ui.input(|i| i.key_pressed(Key::Escape));
    for key in [
        Key::ArrowDown,
        Key::ArrowUp,
        Key::Enter,
        Key::Tab,
        Key::Escape,
    ] {
        let _ = ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        let _ = ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, key));
    }
    (down, up, accept, dismiss)
}

fn show_at_autocomplete(
    ui: &mut egui::Ui,
    edit_id: egui::Id,
    response: &egui::Response,
    value: &mut String,
    cursor_char: Option<usize>,
    all: &[Suggestion],
    keys: AcKeys,
) {
    let AcKeys {
        down,
        up,
        accept,
        dismiss,
    } = keys;
    let ac_id = edit_id.with("at_ac");
    let Some(cursor_char) = cursor_char else {
        ui.ctx()
            .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
        return;
    };
    let Some(incomplete) = find_incomplete_at_token(value, cursor_char) else {
        ui.ctx()
            .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
        return;
    };
    let filtered = filter_suggestions(all, &incomplete.query, AC_LIMIT);
    if filtered.is_empty() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
        return;
    }

    let mut nav = ui
        .ctx()
        .data(|d| d.get_temp::<AtAutocompleteNav>(ac_id))
        .unwrap_or_default();
    if nav.query != incomplete.query {
        nav.query = incomplete.query.clone();
        nav.selected = 0;
    }
    if nav.selected >= filtered.len() {
        nav.selected = filtered.len().saturating_sub(1);
    }
    if down {
        nav.selected = (nav.selected + 1) % filtered.len();
    }
    if up {
        nav.selected = if nav.selected == 0 {
            filtered.len() - 1
        } else {
            nav.selected - 1
        };
    }

    let mut chosen: Option<Suggestion> = None;
    if accept {
        chosen = filtered.get(nav.selected).cloned();
    }
    if dismiss {
        ui.ctx().data_mut(|d| {
            d.insert_temp(ac_id.with("open"), false);
            d.insert_temp(ac_id, AtAutocompleteNav::default());
        });
        return;
    }

    ui.ctx()
        .data_mut(|d| d.insert_temp(ac_id.with("open"), true));

    let popup_width = response.rect.width().max(220.0);
    egui::Popup::from_response(response)
        .id(ac_id.with("popup"))
        .open(true)
        .align(RectAlign::BOTTOM_START)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .width(popup_width)
        .show(|ui| {
            ui.set_min_width(popup_width);
            ui.set_max_height(200.0);
            crate::pickers::dialog_scroll(popup_width, 200.0).show(ui, |ui| {
                ui.set_max_width(popup_width);
                for (i, sug) in filtered.iter().enumerate() {
                    let selected = i == nav.selected;
                    let row = format!("{}  —  {}", sug.insert, sug.hint);
                    let resp = ui.selectable_label(
                        selected,
                        egui::RichText::new(row).monospace().size(12.0),
                    );
                    if resp.clicked() {
                        chosen = Some(sug.clone());
                    }
                    if selected {
                        resp.scroll_to_me(None);
                    }
                }
            });
        });

    if let Some(sug) = chosen {
        let new_cursor =
            apply_at_completion(value, incomplete.start_char, cursor_char, &sug.insert);
        if let Some(mut state) = TextEditState::load(ui.ctx(), edit_id) {
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(new_cursor))));
            state.store(ui.ctx(), edit_id);
        }
        ui.memory_mut(|m| m.request_focus(edit_id));
        ui.ctx().data_mut(|d| {
            d.insert_temp(ac_id.with("open"), false);
            d.insert_temp(ac_id, AtAutocompleteNav::default());
        });
    } else {
        ui.ctx().data_mut(|d| d.insert_temp(ac_id, nav));
    }
}

/// Find an open `@…` span ending at `cursor_char`.
pub(crate) fn find_incomplete_at_token(text: &str, cursor_char: usize) -> Option<IncompleteAt> {
    let before: String = text.chars().take(cursor_char).collect();
    let start_byte = before.rfind('@')?;
    // Abort if `@` is mid-word (e.g. email).
    if start_byte > 0 {
        let prev = before[..start_byte].chars().next_back()?;
        if prev.is_alphanumeric() || prev == '_' {
            return None;
        }
    }
    let after = &before[start_byte + 1..];
    if after.contains('\n') {
        return None;
    }
    let start_char = before[..start_byte].chars().count();
    Some(IncompleteAt {
        start_char,
        query: after.to_string(),
    })
}

fn byte_index_from_char_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

pub(crate) fn apply_at_completion(
    value: &mut String,
    start_char: usize,
    cursor_char: usize,
    insert: &str,
) -> usize {
    let start_byte = byte_index_from_char_index(value, start_char);
    let end_byte = byte_index_from_char_index(value, cursor_char);
    value.replace_range(start_byte..end_byte, insert);
    start_char + insert.chars().count()
}

pub(crate) fn collect_suggestions(macros: &[Macro], catalog: &ProgramCatalog) -> Vec<Suggestion> {
    let mut out = Vec::new();
    out.push(Suggestion {
        kind: SuggestionKind::KindPrefix,
        label: "action".into(),
        insert: "@action:".into(),
        hint: "Action type".into(),
    });
    out.push(Suggestion {
        kind: SuggestionKind::KindPrefix,
        label: "macro".into(),
        insert: "@macro:".into(),
        hint: "Existing macro".into(),
    });
    out.push(Suggestion {
        kind: SuggestionKind::KindPrefix,
        label: "entity".into(),
        insert: "@entity:".into(),
        hint: "Catalog entity".into(),
    });

    for meta in action_type_table() {
        out.push(Suggestion {
            kind: SuggestionKind::Action,
            label: meta.label.to_string(),
            insert: format!("@action:{}", meta.type_key),
            hint: meta.description.to_string(),
        });
    }

    for m in macros {
        out.push(Suggestion {
            kind: SuggestionKind::Macro,
            label: m.name.clone(),
            insert: format!("@macro:{}", m.name),
            hint: "Macro".into(),
        });
    }

    push_catalog_suggestions(&mut out, catalog);
    out
}

fn push_catalog_suggestions(out: &mut Vec<Suggestion>, catalog: &ProgramCatalog) {
    let res = catalog.resolution_key();
    for program in catalog.program_names() {
        if !is_editor_listed_program(program) {
            continue;
        }
        out.push(Suggestion {
            kind: SuggestionKind::Entity,
            label: program.clone(),
            insert: format!("@entity:{program}"),
            hint: "Program".into(),
        });
        let Some(prog) = catalog.get(program) else {
            continue;
        };
        for (k, it) in &prog.items {
            let name = nonempty_or(&it.name, k);
            out.push(Suggestion {
                kind: SuggestionKind::Entity,
                label: name.clone(),
                insert: format!("@entity:{program}{PROGRAM_DELIMITER}{name}"),
                hint: format!("Item · {program}"),
            });
        }
        if let Some(points) = prog.points.get(res).or_else(|| prog.points.values().next()) {
            for (k, pt) in points {
                let name = nonempty_or(&pt.name, k);
                out.push(Suggestion {
                    kind: SuggestionKind::Entity,
                    label: name.clone(),
                    insert: format!("@entity:{program}{PROGRAM_DELIMITER}{name}"),
                    hint: format!("Point · {program}"),
                });
            }
        }
        if let Some(areas) = prog
            .search_areas
            .get(res)
            .or_else(|| prog.search_areas.values().next())
        {
            for (k, sa) in areas {
                let name = nonempty_or(&sa.name, k);
                out.push(Suggestion {
                    kind: SuggestionKind::Entity,
                    label: name.clone(),
                    insert: format!("@entity:{program}{PROGRAM_DELIMITER}{name}"),
                    hint: format!("Search Area · {program}"),
                });
            }
        }
        for (k, m) in &prog.masks {
            let name = nonempty_or(&m.name, k);
            out.push(Suggestion {
                kind: SuggestionKind::Entity,
                label: name.clone(),
                insert: format!("@entity:{program}{PROGRAM_DELIMITER}{name}"),
                hint: format!("Mask · {program}"),
            });
        }
        for (k, c) in &prog.collections {
            let name = nonempty_or(&c.name, k);
            out.push(Suggestion {
                kind: SuggestionKind::Entity,
                label: name.clone(),
                insert: format!("@entity:{program}{PROGRAM_DELIMITER}{name}"),
                hint: format!("Collection · {program}"),
            });
        }
        for (k, a) in &prog.atlases {
            let name = nonempty_or(&a.name, k);
            out.push(Suggestion {
                kind: SuggestionKind::Entity,
                label: name.clone(),
                insert: format!("@entity:{program}{PROGRAM_DELIMITER}{name}"),
                hint: format!("Atlas · {program}"),
            });
        }
    }
}

fn nonempty_or(name: &str, key: &str) -> String {
    if name.trim().is_empty() {
        key.to_string()
    } else {
        name.to_string()
    }
}

pub(crate) fn filter_suggestions(all: &[Suggestion], query: &str, limit: usize) -> Vec<Suggestion> {
    let q = query.to_ascii_lowercase();
    let (kind_filter, needle) = parse_query_filter(&q);

    let mut out: Vec<Suggestion> = all
        .iter()
        .filter(|s| match kind_filter {
            Some("action") => {
                s.kind == SuggestionKind::Action
                    || (s.kind == SuggestionKind::KindPrefix && s.label == "action")
            }
            Some("macro") => {
                s.kind == SuggestionKind::Macro
                    || (s.kind == SuggestionKind::KindPrefix && s.label == "macro")
            }
            Some("entity") => {
                s.kind == SuggestionKind::Entity
                    || (s.kind == SuggestionKind::KindPrefix && s.label == "entity")
            }
            Some(_) | None => true,
        })
        .filter(|s| {
            if needle.is_empty() {
                // With a kind selected, skip the kind-prefix row itself.
                if kind_filter.is_some() {
                    return s.kind != SuggestionKind::KindPrefix;
                }
                return true;
            }
            let insert_l = s.insert.to_ascii_lowercase();
            let label_l = s.label.to_ascii_lowercase();
            let hint_l = s.hint.to_ascii_lowercase();
            // Match against insert without the @kind: prefix for convenience.
            let bare = insert_l
                .strip_prefix("@action:")
                .or_else(|| insert_l.strip_prefix("@macro:"))
                .or_else(|| insert_l.strip_prefix("@entity:"))
                .unwrap_or(insert_l.as_str());
            bare.contains(needle)
                || label_l.contains(needle)
                || hint_l.contains(needle)
                || insert_l.contains(needle)
        })
        .cloned()
        .collect();

    // Prefer kind prefixes when the query is still a partial kind name.
    if kind_filter.is_none() && !needle.is_empty() {
        out.sort_by_key(|s| {
            let prefix_boost = if s.kind == SuggestionKind::KindPrefix {
                0u8
            } else {
                1
            };
            (prefix_boost, s.insert.clone())
        });
    }

    out.truncate(limit);
    out
}

fn parse_query_filter(q: &str) -> (Option<&str>, &str) {
    if let Some(rest) = q.strip_prefix("action:") {
        (Some("action"), rest)
    } else if let Some(rest) = q.strip_prefix("macro:") {
        (Some("macro"), rest)
    } else if let Some(rest) = q.strip_prefix("entity:") {
        (Some("entity"), rest)
    } else {
        (None, q)
    }
}

// ── Prompt generation ───────────────────────────────────────────────────────

/// Build a schema- and catalog-aware prompt for an external AI tool.
pub fn build_prompt(user_nl: &str, macros: &[Macro], catalog: &ProgramCatalog) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str(SYSTEM_PROMPT_HEADER);
    out.push_str("\n\n## Allowed action types\n");
    for meta in action_type_table() {
        out.push_str(&format!(
            "- `{}` ({}) — {}\n",
            meta.type_key, meta.label, meta.description
        ));
    }

    out.push_str("\n## Existing macros (do not redefine; reference with @macro:Name / runmacro)\n");
    if macros.is_empty() {
        out.push_str("(none)\n");
    } else {
        for m in macros {
            out.push_str(&format!("- {}\n", m.name));
        }
    }

    out.push_str("\n## Catalog entities (prefer these refs: Program~Entity)\n");
    append_catalog_inventory(&mut out, catalog);

    out.push_str("\n## User request\n");
    out.push_str(user_nl.trim());
    out.push('\n');

    out.push_str(SYSTEM_PROMPT_FOOTER);
    out
}

fn append_catalog_inventory(out: &mut String, catalog: &ProgramCatalog) {
    let res = catalog.resolution_key();
    let mut any = false;
    for program in catalog.program_names() {
        if !is_editor_listed_program(program) {
            continue;
        }
        any = true;
        out.push_str(&format!("### {program}\n"));
        let Some(prog) = catalog.get(program) else {
            continue;
        };
        if !prog.items.is_empty() {
            out.push_str("Items: ");
            out.push_str(
                &prog
                    .items
                    .iter()
                    .map(|(k, it)| {
                        format!("{program}{PROGRAM_DELIMITER}{}", nonempty_or(&it.name, k))
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push('\n');
        }
        if let Some(points) = prog.points.get(res).or_else(|| prog.points.values().next()) {
            if !points.is_empty() {
                out.push_str("Points: ");
                out.push_str(
                    &points
                        .iter()
                        .map(|(k, pt)| {
                            format!("{program}{PROGRAM_DELIMITER}{}", nonempty_or(&pt.name, k))
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                out.push('\n');
            }
        }
        if let Some(areas) = prog
            .search_areas
            .get(res)
            .or_else(|| prog.search_areas.values().next())
        {
            if !areas.is_empty() {
                out.push_str("Search areas: ");
                out.push_str(
                    &areas
                        .iter()
                        .map(|(k, sa)| {
                            format!("{program}{PROGRAM_DELIMITER}{}", nonempty_or(&sa.name, k))
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                out.push('\n');
            }
        }
        if !prog.collections.is_empty() {
            out.push_str("Collections: ");
            out.push_str(
                &prog
                    .collections
                    .iter()
                    .map(|(k, c)| {
                        format!("{program}{PROGRAM_DELIMITER}{}", nonempty_or(&c.name, k))
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push('\n');
        }
        if !prog.masks.is_empty() {
            out.push_str("Masks: ");
            out.push_str(
                &prog
                    .masks
                    .iter()
                    .map(|(k, m)| {
                        format!("{program}{PROGRAM_DELIMITER}{}", nonempty_or(&m.name, k))
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push('\n');
        }
        if !prog.atlases.is_empty() {
            out.push_str("Atlases: ");
            out.push_str(
                &prog
                    .atlases
                    .iter()
                    .map(|(k, a)| {
                        format!("{program}{PROGRAM_DELIMITER}{}", nonempty_or(&a.name, k))
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push('\n');
        }
    }
    if !any {
        out.push_str("(empty catalog — use General seed entities when available)\n");
    }
}

const SYSTEM_PROMPT_HEADER: &str = r#"You are a Sqyre macro author. Output ONLY valid Sqyre macro YAML for one macro
(the value stored under macros.<name>, including the `name` field). No markdown
fences, no commentary, no surrounding db.yaml wrapper.

HARD RULES
1. Root MUST be: { type: loop, name: root, count: 1, subactions: [...] }
2. Every action has a `type` from the allowed list below. Unknown types are invalid.
3. Prefer Program~Entity refs over raw pixels. Use General~* when no catalog entity fits.
4. Variables as ${name}. Declare user vars under `variables:` when needed.
5. Detection actions (imagesearch, ocr, findpixel) put found-path steps in
   `subactions` and miss-path steps in `elseactions`. Default outputs: foundX/foundY
   or ocrText.
6. Click/Key `state`: tap | down | up. Prefer `tap` unless holding.
7. Wait `time` is milliseconds (int or ${var}).
8. Do not invent programs/items beyond the catalog. If an image is missing, use a
   TODO~Placeholder target and keep the scaffold valid.
9. Omit empty optional fields. Omit uid.
10. Macro name must be human-readable.

REFERENCE SYNTAX
- Points / search areas / collections: Program~Name or Program~Name@r1,c1-r2,c2
- Image targets: Program~Item or Program~Item~Variant
- Tokens in the user request like @action:imagesearch, @macro:Name, @entity:Program~Name
  are hints — emit the corresponding wire fields, not the @ tokens."#;

const SYSTEM_PROMPT_FOOTER: &str = r#"
## Output
Return ONLY the macro YAML document, starting with `name:`."#;

// ── Import ──────────────────────────────────────────────────────────────────

/// Strip common markdown fences and leading/trailing noise from an AI reply.
pub fn strip_yaml_fences(raw: &str) -> String {
    let mut s = raw.trim();
    if let Some(rest) = s.strip_prefix("```yaml") {
        s = rest;
    } else if let Some(rest) = s.strip_prefix("```yml") {
        s = rest;
    } else if let Some(rest) = s.strip_prefix("```") {
        s = rest;
    }
    s = s.trim();
    if let Some(stripped) = s.strip_suffix("```") {
        s = stripped.trim();
    }
    s.to_string()
}

/// Decode + validate a single-macro YAML blob; rename on collision with `existing`.
pub fn prepare_import_macro(raw: &str, existing: &[String]) -> Result<Macro, String> {
    let yaml = strip_yaml_fences(raw);
    if yaml.is_empty() {
        return Err("Paste a macro YAML document first.".into());
    }
    // Reject full db.yaml wrappers so users get a clear message.
    if looks_like_db_yaml(&yaml) {
        return Err(
            "Paste a single macro document (with `name:` and `root:`), not a full db.yaml.".into(),
        );
    }
    let mut macro_ = decode_macro_from_yaml(&yaml).map_err(|e| e.to_string())?;
    if macro_.name.trim().is_empty() {
        return Err("Macro is missing a name.".into());
    }
    sqyre_validate::validate_macro(&macro_).map_err(|e| e.to_string())?;
    let unique = unique_macro_name(&macro_.name, existing);
    macro_.name = unique;
    Ok(macro_)
}

fn looks_like_db_yaml(yaml: &str) -> bool {
    let Ok(serde_yaml::Value::Mapping(map)) = serde_yaml::from_str(yaml) else {
        return false;
    };
    let has_macros = map.contains_key(serde_yaml::Value::String("macros".into()));
    let has_root = map.contains_key(serde_yaml::Value::String("root".into()));
    has_macros && !has_root
}

pub(crate) fn unique_macro_name(base: &str, existing: &[String]) -> String {
    if !existing.iter().any(|n| n == base) {
        return base.to_string();
    }
    for i in 2.. {
        let candidate = format!("{base} {i}");
        if !existing.iter().any(|n| n == &candidate) {
            return candidate;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, Action, ActionId, ActionKind, ScalarValue, WIRE_TYPE_KEYS};
    use sqyre_persist::{
        ensure_general_program, ProgramItem, ProgramPoint, ProgramSearchArea, GENERAL_PROGRAM,
    };

    fn demo_catalog() -> ProgramCatalog {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        let _ = ensure_general_program(&mut cat, &[(0, 0, 1920, 1080)]);
        cat.create_program("Demo").unwrap();
        cat.upsert_point(
            "Demo",
            ProgramPoint {
                name: "Spawn".into(),
                monitor: 1,
                x: ScalarValue::Int(10),
                y: ScalarValue::Int(20),
            },
        )
        .unwrap();
        cat.upsert_search_area(
            "Demo",
            ProgramSearchArea {
                name: "Board".into(),
                monitor: 1,
                left_x: ScalarValue::Int(0),
                top_y: ScalarValue::Int(0),
                right_x: ScalarValue::Int(100),
                bottom_y: ScalarValue::Int(100),
            },
        )
        .unwrap();
        cat.upsert_item(
            "Demo",
            ProgramItem {
                name: "OK".into(),
                ..Default::default()
            },
        )
        .unwrap();
        cat
    }

    fn sample_macro(name: &str) -> Macro {
        let mut m = Macro::new(name, 0, vec![]);
        m.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        }]);
        m
    }

    #[test]
    fn find_at_token_basic() {
        let t = "click @act";
        let incomplete = find_incomplete_at_token(t, t.chars().count()).unwrap();
        assert_eq!(incomplete.query, "act");
        assert_eq!(incomplete.start_char, 6);
    }

    #[test]
    fn find_at_token_rejects_email() {
        assert!(find_incomplete_at_token("user@dom", 8).is_none());
    }

    #[test]
    fn apply_completion_replaces_span() {
        let mut s = "use @ac".to_string();
        let cur = apply_at_completion(&mut s, 4, 7, "@action:click");
        assert_eq!(s, "use @action:click");
        assert_eq!(cur, s.chars().count());
    }

    #[test]
    fn filter_by_kind_and_needle() {
        let macros = [sample_macro("Buy Potion")];
        let cat = demo_catalog();
        let all = collect_suggestions(&macros, &cat);
        let actions = filter_suggestions(&all, "action:image", 20);
        assert!(actions.iter().any(|s| s.insert.contains("imagesearch")));
        assert!(actions.iter().all(|s| s.kind == SuggestionKind::Action));

        let ents = filter_suggestions(&all, "entity:Spawn", 20);
        assert!(ents.iter().any(|s| s.insert.contains("Demo~Spawn")));

        let macs = filter_suggestions(&all, "macro:Buy", 10);
        assert!(macs.iter().any(|s| s.insert == "@macro:Buy Potion"));
    }

    #[test]
    fn prompt_includes_all_wire_keys_and_catalog() {
        let macros = [sample_macro("Helper")];
        let cat = demo_catalog();
        let prompt = build_prompt("Click the OK button on Demo", &macros, &cat);
        for key in WIRE_TYPE_KEYS {
            assert!(
                prompt.contains(&format!("`{key}`")),
                "missing wire key {key} in prompt"
            );
        }
        assert!(prompt.contains("Helper"));
        assert!(prompt.contains(&format!("{GENERAL_PROGRAM}~")));
        assert!(prompt.contains("Demo~OK") || prompt.contains("Demo~Spawn"));
        assert!(prompt.contains("Click the OK button on Demo"));
        assert!(prompt.contains("Output ONLY") || prompt.contains("ONLY the macro YAML"));
    }

    #[test]
    fn strip_fences() {
        let raw = "```yaml\nname: X\nroot:\n  type: loop\n  name: root\n  count: 1\n```";
        let s = strip_yaml_fences(raw);
        assert!(s.starts_with("name:"));
        assert!(!s.contains("```"));
    }

    #[test]
    fn prepare_import_validates_and_renames() {
        let yaml = r#"
name: Demo Macro
globaldelay: 0
keyboarddelay: 25
mousedelay: 25
hotkey: []
root:
  type: loop
  name: root
  count: 1
  subactions:
    - type: wait
      time: 1
"#;
        let m = prepare_import_macro(yaml, &[]).unwrap();
        assert_eq!(m.name, "Demo Macro");

        let m2 = prepare_import_macro(yaml, &["Demo Macro".into()]).unwrap();
        assert_eq!(m2.name, "Demo Macro 2");
    }

    #[test]
    fn prepare_import_rejects_bad_root() {
        let yaml = r#"
name: Bad
root:
  type: wait
  time: 1
"#;
        let err = prepare_import_macro(yaml, &[]).unwrap_err();
        assert!(
            err.to_lowercase().contains("root") || err.to_lowercase().contains("loop"),
            "{err}"
        );
    }

    #[test]
    fn prepare_import_rejects_db_yaml() {
        let yaml = "macros: {}\nprograms: {}\n";
        let err = prepare_import_macro(yaml, &[]).unwrap_err();
        assert!(err.contains("single macro"), "{err}");
    }

    #[test]
    fn unique_name_increments() {
        assert_eq!(unique_macro_name("A", &[]), "A");
        assert_eq!(unique_macro_name("A", &["A".into()]), "A 2");
        assert_eq!(unique_macro_name("A", &["A".into(), "A 2".into()]), "A 3");
    }
}
