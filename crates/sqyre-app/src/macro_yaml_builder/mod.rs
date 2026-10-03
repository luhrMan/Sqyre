//! YAML Macro Builder: schema-driven editor synced with the selected macro.

mod context;
mod highlight;

use crate::data_editor::helpers::is_editor_listed_program;
use crate::status_banner::StatusBanner;
use context::{
    enter_edit, find_yaml_context, shift_lines, Completion, EnterEdit, EntityKind, YamlContext,
    INDENT,
};
use eframe::egui::{
    self, text::CCursor, text_edit::TextEditState, Key, Modifiers, PopupCloseBehavior, RectAlign,
};
use egui::text_selection::CCursorRange;
use sqyre_domain::{
    action_type_label, blank_action, Action, Macro, PROGRAM_DELIMITER, WIRE_TYPE_KEYS,
};
use sqyre_persist::{MacroYamlBuilderDrafts, MacroYamlDraftEntry, ProgramCatalog};
use sqyre_serialize::{action_to_map, encode_macro_to_yaml};
use sqyre_validate::{
    prepare_import_macro_yaml, prepare_macro_yaml, report_macro_yaml, YamlValidateReport,
};

const WINDOW_ID: &str = "sqyre_macro_yaml_builder";
const AC_LIMIT: usize = 24;
const SAVE_DEBOUNCE_SECS: f64 = 0.75;
const VALIDATE_DEBOUNCE_SECS: f64 = 0.35;
/// Autocomplete popup size budget (points).
const AC_POPUP_MAX_H: f32 = 180.0;
const AC_POPUP_MAX_W: f32 = 320.0;

/// Outcome from one frame of the builder modal.
#[derive(Debug, Default)]
pub enum YamlBuilderOutcome {
    #[default]
    None,
    /// Replace the bound selected macro.
    ApplyMacro(Box<Macro>),
    /// Insert as a new uniquely-named macro.
    ImportMacro(Box<Macro>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuggestionKind {
    /// Action type; accepting expands a blank action stub.
    Type,
    Field,
    Enum,
    Entity(EntityKind),
    MacroName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Suggestion {
    kind: SuggestionKind,
    label: String,
    insert: String,
    hint: String,
    /// Owning program for catalog entities (empty otherwise).
    program: String,
}

/// Per-editor autocomplete state (egui temp data).
#[derive(Clone, Default)]
struct AcState {
    /// Popup may show. Set by typing; cleared by Esc, caret moves, and focus loss.
    active: bool,
    /// Popup was painted last frame, so it owns ↑↓ / Enter / Tab / Esc this frame.
    shown: bool,
    /// Caret char index from the last focused frame.
    caret: Option<usize>,
    /// Caret rect (global) from the last focused frame, so the popup holds still on focus loss.
    anchor: Option<egui::Rect>,
    selected: usize,
    query: String,
}

impl AcState {
    /// Popup stays open only while the user is typing at the same caret.
    fn still_active(
        &self,
        typed: bool,
        dismissed: bool,
        focused: bool,
        caret: Option<usize>,
    ) -> bool {
        if typed {
            return true;
        }
        if dismissed {
            return false;
        }
        if focused {
            self.active && caret == self.caret
        } else {
            // Clicking a popup row drops editor focus; keep it one frame so the click lands.
            self.active && self.shown
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DraftChoice {
    None,
    /// Saved draft base no longer matches current tree YAML.
    Stale,
}

#[derive(Debug)]
pub struct MacroYamlBuilderUi {
    pub open: bool,
    /// Frozen selection name for this modal session.
    bound_name: Option<String>,
    base_yaml: String,
    yaml: String,
    status: StatusBanner,
    last_report: YamlValidateReport,
    last_validated_yaml: String,
    validate_since: Option<f64>,
    dirty_since: Option<f64>,
    drafts: MacroYamlBuilderDrafts,
    draft_choice: DraftChoice,
    stale_draft: Option<MacroYamlDraftEntry>,
}

impl Default for MacroYamlBuilderUi {
    fn default() -> Self {
        Self::load()
    }
}

impl MacroYamlBuilderUi {
    pub fn load() -> Self {
        let drafts = MacroYamlBuilderDrafts::load_default().unwrap_or_else(|e| {
            crate::log::warn(format!("failed to load YAML Macro Builder drafts: {e}"));
            MacroYamlBuilderDrafts::default()
        });
        Self {
            open: false,
            bound_name: None,
            base_yaml: String::new(),
            yaml: String::new(),
            status: StatusBanner::default(),
            last_report: YamlValidateReport::ok(),
            last_validated_yaml: String::new(),
            validate_since: None,
            dirty_since: None,
            drafts,
            draft_choice: DraftChoice::None,
            stale_draft: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open bound to the selected macro (or a blank skeleton).
    pub fn open_builder(&mut self, selected: Option<&Macro>) {
        self.open = true;
        self.status.clear();
        self.draft_choice = DraftChoice::None;
        self.stale_draft = None;

        let (name, tree_yaml) = match selected {
            Some(m) => {
                let yaml = encode_macro_to_yaml(m).unwrap_or_else(|_| minimal_skeleton(&m.name));
                (Some(m.name.clone()), yaml)
            }
            None => (None, minimal_skeleton("new macro")),
        };
        self.bound_name = name.clone();
        self.base_yaml = tree_yaml.clone();

        if let Some(name) = &name {
            if let Some(entry) = self.drafts.get(name).cloned() {
                if entry.base_yaml == tree_yaml {
                    self.yaml = entry.yaml;
                } else if entry.yaml != tree_yaml && !entry.yaml.is_empty() {
                    self.yaml = tree_yaml;
                    self.stale_draft = Some(entry);
                    self.draft_choice = DraftChoice::Stale;
                } else {
                    self.yaml = tree_yaml;
                    self.drafts.remove(name);
                }
            } else {
                self.yaml = tree_yaml;
            }
        } else {
            self.yaml = tree_yaml;
        }
        self.last_validated_yaml.clear();
        self.validate_since = Some(0.0);
    }

    fn is_dirty(&self) -> bool {
        self.yaml != self.base_yaml
    }

    fn persist_draft_now(&mut self) {
        if let Some(name) = &self.bound_name {
            if self.is_dirty() {
                self.drafts.set(
                    name.clone(),
                    MacroYamlDraftEntry {
                        base_yaml: self.base_yaml.clone(),
                        yaml: self.yaml.clone(),
                    },
                );
            } else {
                self.drafts.remove(name);
            }
            if let Err(e) = self.drafts.save_default() {
                crate::log::warn(format!("failed to save YAML builder drafts: {e}"));
            }
        }
        self.dirty_since = None;
    }

    fn maybe_persist(&mut self, now: f64, force: bool) {
        if !self.is_dirty() {
            if self.dirty_since.is_some() {
                self.persist_draft_now();
            }
            return;
        }
        if self.dirty_since.is_none() {
            self.dirty_since = Some(now);
        }
        let since = self.dirty_since.unwrap_or(now);
        if force || now - since >= SAVE_DEBOUNCE_SECS {
            self.persist_draft_now();
        }
    }

    fn maybe_validate(&mut self, now: f64) {
        if self.yaml == self.last_validated_yaml {
            return;
        }
        if self.validate_since.is_none() {
            self.validate_since = Some(now);
        }
        let since = self.validate_since.unwrap_or(now);
        if now - since < VALIDATE_DEBOUNCE_SECS {
            return;
        }
        self.last_report = report_macro_yaml(&self.yaml);
        self.last_validated_yaml = self.yaml.clone();
        self.validate_since = None;
        if self.last_report.is_ok() {
            self.set_status("Valid", false);
        } else {
            self.set_status(self.last_report.summary(), true);
        }
    }

    /// Paint the modal. When open, the caller must block the rest of the app UI.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        macros: &[Macro],
        catalog: &ProgramCatalog,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
        running: bool,
    ) -> YamlBuilderOutcome {
        let now = ctx.input(|i| i.time);
        if !self.open {
            if self.dirty_since.is_some() {
                self.maybe_persist(now, true);
            }
            return YamlBuilderOutcome::None;
        }

        // Dim / block the painted Sqyre UI underneath (not the OS desktop).
        // Main chrome is drawn first; this Foreground veil greys it out.
        egui::Area::new(egui::Id::new(WINDOW_ID).with("modal_dim"))
            .order(egui::Order::Foreground)
            .fixed_pos(ctx.content_rect().min)
            .interactable(true)
            .show(ctx, |ui| {
                let screen = ctx.content_rect();
                let resp = ui.allocate_rect(screen, egui::Sense::click_and_drag());
                ui.painter()
                    .rect_filled(resp.rect, 0.0, crate::theme::modal_scrim());
            });

        let mut outcome = YamlBuilderOutcome::None;
        let mut open = self.open;
        let mut request_close = false;

        crate::widgets::fit_dialog_popup(
            egui::Window::new("YAML Macro Builder")
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .order(egui::Order::Foreground)
                .default_width(720.0)
                .default_height(640.0)
                .min_size(crate::widgets::FLOATER_MIN_EDITOR),
            ctx,
            egui::Id::new(WINDOW_ID),
            pending_scale,
        )
        .show(ctx, |ui| {
            crate::widgets::fill_resize_body(ui, |ui| {
                self.body(
                    ui,
                    macros,
                    catalog,
                    running,
                    &mut outcome,
                    &mut request_close,
                );
            });
        });

        if request_close {
            open = false;
        }
        self.open = open;
        if !open {
            self.maybe_persist(now, true);
        } else {
            self.maybe_persist(now, false);
            self.maybe_validate(now);
            if self.validate_since.is_some() || self.dirty_since.is_some() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        outcome
    }

    fn body(
        &mut self,
        ui: &mut egui::Ui,
        macros: &[Macro],
        catalog: &ProgramCatalog,
        running: bool,
        outcome: &mut YamlBuilderOutcome,
        request_close: &mut bool,
    ) {
        let bound = self
            .bound_name
            .clone()
            .unwrap_or_else(|| "(new macro)".into());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(bound).strong().heading());
            if self.is_dirty() {
                ui.weak("(unapplied changes)");
            }
        });
        ui.label(
            egui::RichText::new(
                "Autocomplete suggests action types, fields, choices, catalog entries, and macro names. Tab expands a blank action.",
            )
            .weak(),
        );
        crate::widgets::section_separator(ui);

        if self.draft_choice == DraftChoice::Stale {
            StatusBanner::paint_warn(
                ui,
                "A saved draft no longer matches the current macro. Choose which to load.",
            );
            ui.horizontal(|ui| {
                if ui.button("Reload from Tree").clicked() {
                    self.yaml = self.base_yaml.clone();
                    self.draft_choice = DraftChoice::None;
                    self.stale_draft = None;
                    if let Some(name) = &self.bound_name {
                        self.drafts.remove(name);
                    }
                }
                if ui.button("Restore Draft").clicked() {
                    if let Some(entry) = self.stale_draft.take() {
                        self.yaml = entry.yaml;
                        self.base_yaml = entry.base_yaml;
                    }
                    self.draft_choice = DraftChoice::None;
                }
            });
            crate::widgets::section_separator(ui);
        }

        // Reserve footer (separator + buttons + optional status) so the editor fills the rest.
        let spacing = ui.spacing().item_spacing.y;
        let button_h = ui.spacing().interact_size.y;
        let status_h = if self.status.is_set() {
            ui.text_style_height(&egui::TextStyle::Body) + spacing
        } else {
            0.0
        };
        let footer_h = button_h + status_h + spacing * 3.0 + crate::theme::SPACE_8;
        let editor_h = (ui.available_height() - footer_h).max(120.0);

        let suggestions = collect_runtime_suggestions(macros, catalog);
        let editor_id = egui::Id::new(WINDOW_ID).with("yaml");
        crate::widgets::dialog_scroll(crate::widgets::visible_width(ui), editor_h)
            .id_salt("yaml_scroll")
            .show(ui, |ui| {
                yaml_text_edit(ui, editor_id, &mut self.yaml, &suggestions, editor_h);
            });

        if self.yaml != self.last_validated_yaml && self.validate_since.is_none() {
            self.validate_since = Some(ui.ctx().input(|i| i.time));
        }

        crate::widgets::section_separator(ui);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Validate").clicked() {
                self.last_report = report_macro_yaml(&self.yaml);
                self.last_validated_yaml = self.yaml.clone();
                if self.last_report.is_ok() {
                    self.set_status("Valid", false);
                } else {
                    self.set_status(self.last_report.summary(), true);
                }
            }
            let has_yaml = !self.yaml.trim().is_empty() && self.draft_choice == DraftChoice::None;
            let existing: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();
            if ui
                .add_enabled(
                    has_yaml,
                    egui::Button::new(
                        egui::RichText::new("Import as New").color(crate::theme::MACRO_START),
                    ),
                )
                .on_hover_text("Add this YAML as a new macro")
                .clicked()
            {
                match prepare_import_macro_yaml(&self.yaml, &existing) {
                    Ok(m) => {
                        *outcome = YamlBuilderOutcome::ImportMacro(Box::new(m));
                    }
                    Err(e) => self.set_status(e.to_string(), true),
                }
            }
            let can_apply = !running && self.bound_name.is_some() && has_yaml && self.is_dirty();
            if crate::widgets::dirty_action_button(ui, "Apply", can_apply)
                .on_hover_text("Replace the selected macro with this YAML")
                .on_disabled_hover_text(if running {
                    "Cannot apply while a macro is running"
                } else if self.bound_name.is_none() {
                    "No macro selected — use Import as New"
                } else {
                    "No unapplied changes"
                })
                .clicked()
            {
                match prepare_macro_yaml(&self.yaml) {
                    Ok(m) => {
                        *outcome = YamlBuilderOutcome::ApplyMacro(Box::new(m));
                    }
                    Err(e) => self.set_status(e.to_string(), true),
                }
            }
        });
        self.status.paint(ui);

        // Esc closes the modal when autocomplete is not consuming it.
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            *request_close = true;
        }
    }

    fn set_status(&mut self, msg: impl Into<String>, error: bool) {
        if error {
            self.status.set_err(msg);
        } else {
            self.status.set_ok(msg);
        }
    }

    pub(crate) fn set_status_message(&mut self, msg: impl Into<String>, error: bool) {
        self.set_status(msg, error);
    }

    /// After a successful Apply: refresh baseline from the applied macro.
    pub fn on_applied(&mut self, applied: &Macro) {
        let yaml = encode_macro_to_yaml(applied).unwrap_or_else(|_| self.yaml.clone());
        let old = self.bound_name.clone();
        self.bound_name = Some(applied.name.clone());
        self.base_yaml = yaml.clone();
        self.yaml = yaml;
        self.last_validated_yaml = self.yaml.clone();
        self.last_report = YamlValidateReport::ok();
        self.set_status(format!("Applied \"{}\".", applied.name), false);
        if let Some(old) = old {
            if old != applied.name {
                self.drafts.rename(&old, &applied.name);
            }
        }
        if let Some(name) = &self.bound_name {
            self.drafts.remove(name);
        }
        let _ = self.drafts.save_default();
        self.dirty_since = None;
    }

    pub fn on_imported(&mut self, name: &str) {
        self.set_status(format!("Imported \"{name}\"."), false);
    }

    pub fn on_macro_deleted(&mut self, name: &str) {
        self.drafts.remove(name);
        let _ = self.drafts.save_default();
        if self.bound_name.as_deref() == Some(name) {
            self.open = false;
            self.bound_name = None;
        }
    }

    pub fn on_macro_renamed(&mut self, old: &str, new: &str) {
        self.drafts.rename(old, new);
        let _ = self.drafts.save_default();
        if self.bound_name.as_deref() == Some(old) {
            self.bound_name = Some(new.to_string());
        }
    }
}

impl Drop for MacroYamlBuilderUi {
    fn drop(&mut self) {
        if self.is_dirty() {
            self.persist_draft_now();
        }
    }
}

fn minimal_skeleton(name: &str) -> String {
    format!(
        "name: {name}\nglobaldelay: 0\nkeyboarddelay: 25\nmousedelay: 25\nhotkey: []\nvariables: []\nroot:\n  type: loop\n  name: root\n  count: 1\n  subactions: []\n"
    )
}

/// Reconcile UIDs from `old` onto `new` by structural path + type key.
pub fn reconcile_action_uids(old: &Action, new: &mut Action) {
    if old.type_key() == new.type_key() {
        new.id = old.id;
    }
    let old_kids = old.children();
    if let Some(new_kids) = new.children_mut() {
        for (i, child) in new_kids.iter_mut().enumerate() {
            if let Some(prev) = old_kids.get(i) {
                reconcile_action_uids(prev, child);
            }
        }
    }
    let old_else = old.else_children().unwrap_or(&[]);
    if let Some(new_else) = new.else_children_mut() {
        for (i, child) in new_else.iter_mut().enumerate() {
            if let Some(prev) = old_else.get(i) {
                reconcile_action_uids(prev, child);
            }
        }
    }
}

// ── Autocomplete ────────────────────────────────────────────────────────────

fn collect_runtime_suggestions(macros: &[Macro], catalog: &ProgramCatalog) -> Vec<Suggestion> {
    let mut out = Vec::new();
    for key in WIRE_TYPE_KEYS {
        out.push(Suggestion {
            kind: SuggestionKind::Type,
            label: (*key).into(),
            insert: (*key).into(),
            hint: action_type_label(key).into(),
            program: String::new(),
        });
    }
    for m in macros {
        out.push(Suggestion {
            kind: SuggestionKind::MacroName,
            label: m.name.clone(),
            insert: yaml_quote_if_needed(&m.name),
            hint: "macro".into(),
            program: String::new(),
        });
    }
    let res = catalog.resolution_key();
    for program in catalog.program_names() {
        if !is_editor_listed_program(program) {
            continue;
        }
        let Some(prog) = catalog.get(program) else {
            continue;
        };
        out.push(Suggestion {
            kind: SuggestionKind::Entity(EntityKind::Program),
            label: program.clone(),
            insert: yaml_quote_if_needed(program),
            hint: "program".into(),
            program: program.clone(),
        });
        let mut push_ref = |kind, key: &str, name: &str, what: &str| {
            let full = format!("{program}{PROGRAM_DELIMITER}{}", nonempty_or(name, key));
            out.push(Suggestion {
                kind: SuggestionKind::Entity(kind),
                insert: yaml_quote_if_needed(&full),
                label: full,
                hint: format!("{what} · {program}"),
                program: program.clone(),
            });
        };
        for (k, it) in &prog.items {
            push_ref(EntityKind::Item, k, &it.name, "item");
        }
        if let Some(points) = prog.points.get(res).or_else(|| prog.points.values().next()) {
            for (k, pt) in points {
                push_ref(EntityKind::Point, k, &pt.name, "point");
            }
        }
        if let Some(areas) = prog
            .search_areas
            .get(res)
            .or_else(|| prog.search_areas.values().next())
        {
            for (k, sa) in areas {
                push_ref(EntityKind::SearchArea, k, &sa.name, "search area");
            }
        }
        for (k, c) in &prog.collections {
            push_ref(EntityKind::Collection, k, &c.name, "collection");
        }
        // `atlas:` holds the bare atlas name; its program sits in the sibling `program:`.
        for (k, a) in &prog.atlases {
            let name = nonempty_or(&a.name, k);
            out.push(Suggestion {
                kind: SuggestionKind::Entity(EntityKind::Atlas),
                insert: yaml_quote_if_needed(&name),
                label: name,
                hint: format!("atlas · {program}"),
                program: program.clone(),
            });
        }
    }
    out
}

fn nonempty_or(name: &str, key: &str) -> String {
    if name.trim().is_empty() {
        key.to_string()
    } else {
        name.to_string()
    }
}

fn yaml_quote_if_needed(s: &str) -> String {
    if s.is_empty()
        || s.contains([
            ':', '#', '{', '}', '[', ']', ',', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`',
        ])
        || s.starts_with([' ', '\t'])
        || s.ends_with([' ', '\t'])
    {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

fn suggestions_for_context(ctx: &YamlContext, runtime: &[Suggestion]) -> Vec<Suggestion> {
    let q = ctx
        .query
        .trim_start_matches(['"', '\''])
        .to_ascii_lowercase();
    let of_kind = |kind: SuggestionKind| {
        runtime
            .iter()
            .filter(move |s| s.kind == kind)
            .cloned()
            .collect::<Vec<_>>()
    };
    let plain = |kind, value: &str, hint: &str| Suggestion {
        kind,
        label: value.into(),
        insert: value.into(),
        hint: hint.into(),
        program: String::new(),
    };
    let mut out: Vec<Suggestion> = match &ctx.completion {
        Completion::ActionType => of_kind(SuggestionKind::Type),
        Completion::MacroName => of_kind(SuggestionKind::MacroName),
        Completion::Entity { kind, program } => of_kind(SuggestionKind::Entity(*kind))
            .into_iter()
            .filter(|s| program.as_ref().is_none_or(|p| s.program == *p))
            .collect(),
        Completion::Enum { field, values } => values
            .iter()
            .map(|v| plain(SuggestionKind::Enum, v, field))
            .collect(),
        Completion::Key(keys) => keys
            .iter()
            .map(|k| Suggestion {
                insert: format!("{k}: "),
                ..plain(SuggestionKind::Field, k, "field")
            })
            .collect(),
    };
    if !q.is_empty() {
        out.retain(|s| s.label.to_ascii_lowercase().contains(&q));
    }
    out.truncate(AC_LIMIT);
    out
}

/// Blank `type_key` action as YAML; the first line starts with `line_prefix`
/// (e.g. `  - `) and the rest align under it.
fn blank_action_stub_yaml(type_key: &str, line_prefix: &str) -> Option<String> {
    let action = blank_action(type_key)?;
    let map = action_to_map(&action).ok()?;
    let raw = serde_yaml::to_string(&serde_yaml::Value::Mapping(map)).ok()?;
    let rest_prefix = " ".repeat(line_prefix.len());
    let body = raw
        .lines()
        .filter(|l| *l != "---")
        .enumerate()
        .map(|(i, l)| {
            let prefix = if i == 0 { line_prefix } else { &rest_prefix };
            format!("{prefix}{l}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(body)
}

/// Apply `suggestion` over `ctx`'s range; returns the byte offset for the new caret.
fn apply_completion(text: &mut String, ctx: &YamlContext, suggestion: &Suggestion) -> usize {
    let start = ctx.start.min(text.len());
    let end = ctx.end.min(text.len());
    if suggestion.kind == SuggestionKind::Type {
        if let Some(stub) = blank_action_stub_yaml(&suggestion.insert, &ctx.line_prefix) {
            // Replace the whole `type: …` line with the full stub.
            let line_start = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = text[end..]
                .find('\n')
                .map(|i| end + i)
                .unwrap_or(text.len());
            text.replace_range(line_start..line_end, &stub);
            return line_start + stub.len();
        }
    }
    text.replace_range(start..end, &suggestion.insert);
    start + suggestion.insert.len()
}

fn byte_index_from_char_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn yaml_text_edit(
    ui: &mut egui::Ui,
    id: egui::Id,
    value: &mut String,
    runtime: &[Suggestion],
    fill_height: f32,
) {
    let ac_id = id.with("yaml_ac");
    let mut st = ui
        .ctx()
        .data(|d| d.get_temp::<AcState>(ac_id))
        .unwrap_or_default();
    let (down, up, accept, dismiss) = take_ac_keys(ui, st.shown);
    if !st.shown && ui.memory(|m| m.has_focus(id)) {
        handle_indent_keys(ui, id, value);
    }

    let mono_row = ui.text_style_height(&egui::TextStyle::Monospace).max(1.0);
    let rows = (((fill_height - 8.0) / mono_row).floor() as usize).max(8);

    let gutter = highlight::gutter_width(ui, value);
    let mut layouter = highlight::yaml_layouter(ui);
    let output = egui::TextEdit::multiline(value)
        .id(id)
        // Keep Tab in the editor (indent) instead of moving focus.
        .lock_focus(true)
        .desired_width(f32::INFINITY)
        .desired_rows(rows)
        .hint_text("name: …\nroot:\n  type: loop\n  …")
        .font(egui::TextStyle::Monospace)
        .margin(egui::Margin {
            left: gutter.ceil() as i8,
            right: 4,
            top: 2,
            bottom: 2,
        })
        .layouter(&mut layouter)
        .show(ui);

    let focused = output.response.has_focus();
    let caret = output.cursor_range.map(|r| r.primary.index.0);
    let caret_line = caret.map(|c| value.chars().take(c).filter(|ch| *ch == '\n').count());
    highlight::paint_gutter(ui, &output, gutter, caret_line);
    st.active = st.still_active(output.response.changed(), dismiss, focused, caret);
    if focused {
        st.caret = caret;
        // Anchor under the text caret (not the bottom of the whole TextEdit).
        st.anchor = output.cursor_range.map(|range| {
            let local = output.galley.pos_from_cursor(range.primary);
            let rect = egui::Rect::from_min_max(
                output.galley_pos + local.min.to_vec2(),
                output.galley_pos + local.max.to_vec2(),
            );
            ui.ctx()
                .layer_transform_to_global(output.response.layer_id)
                .map_or(rect, |t| t * rect)
        });
    }

    let ctx = st
        .caret
        .filter(|_| st.active)
        .and_then(|c| find_yaml_context(value, byte_index_from_char_index(value, c)));
    let filtered = ctx
        .as_ref()
        .map(|c| suggestions_for_context(c, runtime))
        .unwrap_or_default();
    let (Some(ctx), Some(anchor_rect)) = (ctx.filter(|_| !filtered.is_empty()), st.anchor) else {
        st.shown = false;
        ui.ctx().data_mut(|d| d.insert_temp(ac_id, st));
        return;
    };

    let prev_selected = st.selected;
    if st.query != ctx.query {
        st.selected = 0;
        st.query = ctx.query.clone();
    }
    st.selected = st.selected.min(filtered.len() - 1);
    if down {
        st.selected = (st.selected + 1).min(filtered.len() - 1);
    }
    if up {
        st.selected = st.selected.saturating_sub(1);
    }
    let selection_moved = st.selected != prev_selected || down || up;
    let mut chosen = accept.then(|| filtered[st.selected].clone());

    let popup_w = AC_POPUP_MAX_W;
    let popup_id = ac_id.with("popup");
    // Lock alignment: flipping TOP/BOTTOM each frame when height is still settling
    // is what makes short lists flicker.
    egui::Popup::new(
        popup_id,
        ui.ctx().clone(),
        anchor_rect,
        output.response.layer_id,
    )
    .align(RectAlign::BOTTOM_START)
    .align_alternatives(&[])
    .gap(crate::theme::SPACE_4)
    .close_behavior(PopupCloseBehavior::IgnoreClicks)
    .width(popup_w)
    .open(true)
    .show(|ui| {
        ui.set_width(popup_w);
        // Avoid ScrollArea for short lists — auto_shrink + scroll_to_me thrash
        // the popup size every frame when content fits without scrolling.
        let needs_scroll = filtered.len() > 8;
        let mut paint_rows = |ui: &mut egui::Ui| {
            for (i, s) in filtered.iter().enumerate() {
                let selected = i == st.selected;
                let label = format!("{}  —  {}", s.label, s.hint);
                let resp = ui.selectable_label(selected, egui::RichText::new(label).monospace());
                if selected && selection_moved && needs_scroll {
                    resp.scroll_to_me(None);
                }
                if resp.clicked() {
                    chosen = Some(s.clone());
                }
            }
        };
        if needs_scroll {
            let popup_h = AC_POPUP_MAX_H;
            crate::widgets::dialog_scroll(popup_w, popup_h).show(ui, |ui| {
                crate::widgets::enable_dense_row_extend(ui);
                paint_rows(ui);
            });
        } else {
            paint_rows(ui);
        }
    });

    st.shown = true;
    if let Some(s) = chosen {
        let caret_byte = apply_completion(value, &ctx, &s);
        let caret = value[..caret_byte].chars().count();
        if let Some(mut state) = TextEditState::load(ui.ctx(), id) {
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(caret))));
            state.store(ui.ctx(), id);
        }
        ui.memory_mut(|m| m.request_focus(id));
        st.caret = Some(caret);
        // A completed key flows straight into its value suggestions; values close the popup.
        st.active = s.kind == SuggestionKind::Field;
        st.shown = false;
    } else if !focused {
        st.active = false;
        st.shown = false;
    }
    ui.ctx().data_mut(|d| d.insert_temp(ac_id, st));
}

/// Enter indents the new line to its YAML depth; Tab / Shift+Tab shift by [`INDENT`].
fn handle_indent_keys(ui: &mut egui::Ui, id: egui::Id, value: &mut String) {
    let Some(mut state) = TextEditState::load(ui.ctx(), id) else {
        return;
    };
    let Some(range) = state.cursor.char_range() else {
        return;
    };
    // Shift+Tab first: an unmodified pattern also matches Shift.
    let (untab, tab, enter) = ui.input_mut(|i| {
        let untab = i.consume_key(Modifiers::SHIFT, Key::Tab);
        let tab = !untab && i.consume_key(Modifiers::NONE, Key::Tab);
        let enter = i.consume_key(Modifiers::NONE, Key::Enter);
        (untab, tab, enter)
    });
    let [a, b] = [range.primary.index.0, range.secondary.index.0]
        .map(|c| byte_index_from_char_index(value, c));
    let (start, end) = (a.min(b), a.max(b));
    let (new_start, new_end) = if enter {
        value.replace_range(start..end, "");
        let caret = match enter_edit(value, start) {
            EnterEdit::Insert(s) => {
                value.insert_str(start, &s);
                start + s.len()
            }
            EnterEdit::EndList {
                start: line_start,
                indent,
            } => {
                value.replace_range(line_start..start, &" ".repeat(indent));
                line_start + indent
            }
        };
        (caret, caret)
    } else if untab || (tab && start != end) {
        shift_lines(value, start, end, untab)
    } else if tab {
        value.insert_str(start, INDENT);
        (start + INDENT.len(), start + INDENT.len())
    } else {
        return;
    };
    let to_char = |b: usize| CCursor::new(value[..b].chars().count());
    state.cursor.set_char_range(Some(CCursorRange::two(
        to_char(new_start),
        to_char(new_end),
    )));
    state.store(ui.ctx(), id);
}

fn take_ac_keys(ui: &mut egui::Ui, ac_open: bool) -> (bool, bool, bool, bool) {
    if !ac_open {
        return (false, false, false, false);
    }
    let mut down = false;
    let mut up = false;
    let mut accept = false;
    let mut dismiss = false;
    ui.input_mut(|i| {
        if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
            down = true;
        }
        if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
            up = true;
        }
        if i.consume_key(Modifiers::NONE, Key::Tab) || i.consume_key(Modifiers::NONE, Key::Enter) {
            accept = true;
        }
        if i.consume_key(Modifiers::NONE, Key::Escape) {
            dismiss = true;
        }
    });
    (down, up, accept, dismiss)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, ActionId, ActionKind, ScalarValue};

    #[test]
    fn popup_opens_only_while_typing() {
        let open = AcState {
            active: true,
            shown: true,
            caret: Some(4),
            ..Default::default()
        };
        assert!(open.still_active(false, false, true, Some(4)));
        assert!(!open.still_active(false, true, true, Some(4)), "Esc closes");
        assert!(
            !open.still_active(false, false, true, Some(9)),
            "click elsewhere closes"
        );
        assert!(
            open.still_active(true, false, true, Some(9)),
            "typing reopens"
        );
        assert!(
            open.still_active(false, false, false, None),
            "row click frame"
        );
        let idle = AcState::default();
        assert!(
            !idle.still_active(false, false, true, Some(2)),
            "click never opens"
        );
        assert!(!idle.still_active(false, false, false, None));
    }

    #[test]
    fn completion_handles_non_ascii_text() {
        let mut text = String::from("name: café\n  type: wa");
        let cursor = byte_index_from_char_index(&text, text.chars().count());
        let ctx = find_yaml_context(&text, cursor).unwrap();
        let s = Suggestion {
            kind: SuggestionKind::Enum,
            label: "wait".into(),
            insert: "wait".into(),
            hint: String::new(),
            program: String::new(),
        };
        let caret = apply_completion(&mut text, &ctx, &s);
        assert_eq!(text, "name: café\n  type: wait");
        assert_eq!(caret, text.len());
    }

    #[test]
    fn stub_expand_produces_type_line() {
        let stub = blank_action_stub_yaml("wait", "  ").unwrap();
        assert!(stub.contains("type: wait"));
        assert!(stub.contains("time:"));
    }

    #[test]
    fn stub_expand_keeps_list_dash() {
        let mut text = String::from("root:\n  subactions:\n  - type: wa");
        let ctx = find_yaml_context(&text, text.len()).unwrap();
        let s = Suggestion {
            kind: SuggestionKind::Type,
            label: "wait".into(),
            insert: "wait".into(),
            hint: String::new(),
            program: String::new(),
        };
        apply_completion(&mut text, &ctx, &s);
        let stub: Vec<&str> = text.lines().skip(2).collect();
        assert!(stub[0].starts_with("  - "), "{text}");
        assert!(stub[1..].iter().all(|l| l.starts_with("    ")), "{text}");
        assert!(text.contains("type: wait"));
    }

    #[test]
    fn targets_only_suggest_items() {
        let mut catalog = ProgramCatalog::default();
        catalog.create_program("Shop").unwrap();
        catalog
            .upsert_item(
                "Shop",
                sqyre_persist::ProgramItem {
                    name: "Potion".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        catalog
            .upsert_point(
                "Shop",
                sqyre_persist::ProgramPoint {
                    name: "Door".into(),
                    monitor: 1,
                    x: ScalarValue::Int(1),
                    y: ScalarValue::Int(1),
                },
            )
            .unwrap();
        let runtime = collect_runtime_suggestions(&[], &catalog);
        let text = "root:\n  subactions:\n  - type: imagesearch\n    targets:\n    - Sh";
        let ctx = find_yaml_context(text, text.len()).unwrap();
        let labels: Vec<String> = suggestions_for_context(&ctx, &runtime)
            .into_iter()
            .map(|s| s.label)
            .collect();
        assert_eq!(labels, ["Shop~Potion"]);
    }

    #[test]
    fn reconcile_preserves_matching_uids() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        };
        let id = a.id;
        let mut old = root_loop(vec![a]);
        let mut new = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(2),
            },
        }]);
        // Force root ids to match type.
        reconcile_action_uids(&old, &mut new);
        assert_eq!(new.children()[0].id, id);
        let _ = &mut old;
    }

    #[test]
    fn prepare_import_path_still_works() {
        let yaml = r#"
name: demo
root:
  type: loop
  name: root
  count: 1
  subactions: []
"#;
        let m = prepare_import_macro_yaml(yaml, &[]).unwrap();
        assert_eq!(m.name, "demo");
    }
}
