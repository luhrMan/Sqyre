//! YAML Macro Builder: schema-driven editor synced with the selected macro.

use crate::data_editor::helpers::is_editor_listed_program;
use eframe::egui::{
    self, text::CCursor, text_edit::TextEditState, Key, Modifiers, PopupCloseBehavior, RectAlign,
};
use egui::text_selection::CCursorRange;
use sqyre_domain::{blank_action, enum_values_for_field, Action, Macro, WIRE_TYPE_KEYS};
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
    Type,
    Field,
    Enum,
    Entity,
    MacroName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Suggestion {
    kind: SuggestionKind,
    label: String,
    insert: String,
    hint: String,
    /// When accepting an action type, expand a blank stub.
    expand_stub: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct YamlContext {
    /// Character range to replace on accept.
    start_char: usize,
    end_char: usize,
    kind: AcTrigger,
    query: String,
    indent: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AcTrigger {
    TypeValue,
    FieldKey,
    EnumValue(&'static str),
    EntityValue,
    MacroNameValue,
}

#[derive(Clone, Default)]
struct AcNav {
    selected: usize,
    query: String,
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
    status: Option<String>,
    status_error: bool,
    last_report: YamlValidateReport,
    last_validated_yaml: String,
    validate_since: Option<f64>,
    dirty_since: Option<f64>,
    drafts: MacroYamlBuilderDrafts,
    draft_choice: DraftChoice,
    stale_draft: Option<MacroYamlDraftEntry>,
    /// Autocomplete stays off until the user edits YAML (avoids popup-on-open).
    ac_armed: bool,
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
            status: None,
            status_error: false,
            last_report: YamlValidateReport::ok(),
            last_validated_yaml: String::new(),
            validate_since: None,
            dirty_since: None,
            drafts,
            draft_choice: DraftChoice::None,
            stale_draft: None,
            ac_armed: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open bound to the selected macro (or a blank skeleton).
    pub fn open_builder(&mut self, selected: Option<&Macro>) {
        self.open = true;
        self.status = None;
        self.status_error = false;
        self.draft_choice = DraftChoice::None;
        self.stale_draft = None;
        self.ac_armed = false;

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
                // Opaque enough to read as a grey veil over Sqyre panels.
                ui.painter().rect_filled(
                    resp.rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
                );
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
                .min_size([480.0, 420.0]),
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
            self.ac_armed = false;
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
        if self.draft_choice == DraftChoice::Stale {
            ui.label(
                egui::RichText::new(
                    "A saved draft no longer matches the current macro. Choose which to load.",
                )
                .strong(),
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
            ui.separator();
        }

        let bound = self
            .bound_name
            .clone()
            .unwrap_or_else(|| "(new macro)".into());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("Editing: {bound}")).strong());
            if self.is_dirty() {
                ui.weak("(unapplied changes)");
            }
        });
        ui.small(
            "Schema autocomplete: type keys, fields, enums, entities, macro names. Tab expands blank action stubs.",
        );

        // Reserve footer (buttons + optional status) so the editor fills the rest.
        let spacing = ui.spacing().item_spacing.y;
        let button_h = ui.spacing().interact_size.y;
        let status_h = if self.status.is_some() {
            ui.text_style_height(&egui::TextStyle::Body) + spacing
        } else {
            0.0
        };
        let footer_h = button_h + status_h + spacing * 2.0;
        let editor_h = (ui.available_height() - footer_h).max(120.0);
        let editor_w = ui.available_width();

        let suggestions = collect_runtime_suggestions(macros, catalog);
        let editor_id = egui::Id::new(WINDOW_ID).with("yaml");
        ui.allocate_ui_with_layout(
            egui::vec2(editor_w, editor_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_max_size(egui::vec2(editor_w, editor_h));
                yaml_text_edit(
                    ui,
                    editor_id,
                    &mut self.yaml,
                    &suggestions,
                    editor_h,
                    &mut self.ac_armed,
                );
            },
        );

        if self.yaml != self.last_validated_yaml && self.validate_since.is_none() {
            self.validate_since = Some(ui.ctx().input(|i| i.time));
        }

        ui.horizontal(|ui| {
            if ui.button("Validate").clicked() {
                self.last_report = report_macro_yaml(&self.yaml);
                self.last_validated_yaml = self.yaml.clone();
                if self.last_report.is_ok() {
                    self.set_status("Valid", false);
                } else {
                    self.set_status(self.last_report.summary(), true);
                }
            }
            let can_apply = !running
                && self.bound_name.is_some()
                && !self.yaml.trim().is_empty()
                && self.draft_choice == DraftChoice::None;
            if ui
                .add_enabled(can_apply, egui::Button::new("Apply"))
                .on_hover_text(if running {
                    "Cannot apply while a macro is running"
                } else {
                    "Replace the selected macro with this YAML"
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
            let existing: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();
            if ui
                .add_enabled(
                    !self.yaml.trim().is_empty() && self.draft_choice == DraftChoice::None,
                    egui::Button::new("Import as New"),
                )
                .clicked()
            {
                match prepare_import_macro_yaml(&self.yaml, &existing) {
                    Ok(m) => {
                        *outcome = YamlBuilderOutcome::ImportMacro(Box::new(m));
                    }
                    Err(e) => self.set_status(e.to_string(), true),
                }
            }
            if ui.button("Close").clicked() {
                *request_close = true;
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

        // Esc closes the modal when autocomplete is not consuming it.
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            *request_close = true;
        }
    }

    fn set_status(&mut self, msg: impl Into<String>, error: bool) {
        self.status = Some(msg.into());
        self.status_error = error;
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
            hint: "action type".into(),
            expand_stub: true,
        });
    }
    for m in macros {
        out.push(Suggestion {
            kind: SuggestionKind::MacroName,
            label: m.name.clone(),
            insert: yaml_quote_if_needed(&m.name),
            hint: "macro".into(),
            expand_stub: false,
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
        let delim = sqyre_domain::PROGRAM_DELIMITER;
        for (k, it) in &prog.items {
            let name = if it.name.trim().is_empty() {
                k.clone()
            } else {
                it.name.clone()
            };
            let full = format!("{program}{delim}{name}");
            out.push(Suggestion {
                kind: SuggestionKind::Entity,
                label: full.clone(),
                insert: yaml_quote_if_needed(&full),
                hint: format!("item · {program}"),
                expand_stub: false,
            });
        }
        if let Some(points) = prog.points.get(res).or_else(|| prog.points.values().next()) {
            for (k, pt) in points {
                let name = if pt.name.trim().is_empty() {
                    k.clone()
                } else {
                    pt.name.clone()
                };
                let full = format!("{program}{delim}{name}");
                out.push(Suggestion {
                    kind: SuggestionKind::Entity,
                    label: full.clone(),
                    insert: yaml_quote_if_needed(&full),
                    hint: format!("point · {program}"),
                    expand_stub: false,
                });
            }
        }
        if let Some(areas) = prog
            .search_areas
            .get(res)
            .or_else(|| prog.search_areas.values().next())
        {
            for (k, sa) in areas {
                let name = if sa.name.trim().is_empty() {
                    k.clone()
                } else {
                    sa.name.clone()
                };
                let full = format!("{program}{delim}{name}");
                out.push(Suggestion {
                    kind: SuggestionKind::Entity,
                    label: full.clone(),
                    insert: yaml_quote_if_needed(&full),
                    hint: format!("search area · {program}"),
                    expand_stub: false,
                });
            }
        }
        for (k, c) in &prog.collections {
            let name = if c.name.trim().is_empty() {
                k.clone()
            } else {
                c.name.clone()
            };
            let full = format!("{program}{delim}{name}");
            out.push(Suggestion {
                kind: SuggestionKind::Entity,
                label: full.clone(),
                insert: yaml_quote_if_needed(&full),
                hint: format!("collection · {program}"),
                expand_stub: false,
            });
        }
    }
    out
}

fn yaml_quote_if_needed(s: &str) -> String {
    if s.is_empty()
        || s.contains([':', '#', '{', '}', '[', ']', ',', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`'])
        || s.starts_with([' ', '\t'])
        || s.ends_with([' ', '\t'])
    {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

fn find_yaml_context(text: &str, cursor: usize) -> Option<YamlContext> {
    let cursor = cursor.min(text.len());
    let line_start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line = &text[line_start..cursor];
    let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let trimmed = line.trim_start();

    if let Some(rest) = trimmed.strip_prefix("type:") {
        let val_start = line_start + (line.len() - rest.len());
        let query = rest.trim_start().to_string();
        let q_start = val_start + (rest.len() - rest.trim_start().len());
        return Some(YamlContext {
            start_char: q_start,
            end_char: cursor,
            kind: AcTrigger::TypeValue,
            query,
            indent,
        });
    }

    // Bare key completion: incomplete key before ':'
    if !trimmed.contains(':') && !trimmed.starts_with('-') {
        return Some(YamlContext {
            start_char: line_start + indent.len(),
            end_char: cursor,
            kind: AcTrigger::FieldKey,
            query: trimmed.to_string(),
            indent,
        });
    }

    for field in [
        "point",
        "searcharea",
        "cells",
        "processpath",
        "windowtitle",
        "targetcolor",
        "target",
        "program",
        "atlas",
    ] {
        let prefix = format!("{field}:");
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let val_start = line_start + (line.len() - rest.len());
            let query = rest.trim_start().to_string();
            let q_start = val_start + (rest.len() - rest.trim_start().len());
            return Some(YamlContext {
                start_char: q_start,
                end_char: cursor,
                kind: AcTrigger::EntityValue,
                query,
                indent,
            });
        }
    }

    if let Some(rest) = trimmed.strip_prefix("macroname:") {
        let val_start = line_start + (line.len() - rest.len());
        let query = rest.trim_start().to_string();
        let q_start = val_start + (rest.len() - rest.trim_start().len());
        return Some(YamlContext {
            start_char: q_start,
            end_char: cursor,
            kind: AcTrigger::MacroNameValue,
            query,
            indent,
        });
    }

    for (field, _) in [
        ("repeatmode", "repeatmode"),
        ("button", "button"),
        ("state", "state"),
        ("match", "match"),
        ("operator", "operator"),
        ("mode", "mode"),
        ("matchmethod", "matchmethod"),
        ("sortby", "sortby"),
        ("sortthen", "sortthen"),
        ("grouping", "grouping"),
        ("selectdevice", "selectdevice"),
        ("selectbutton", "selectbutton"),
        ("selectpressmode", "selectpressmode"),
        ("hotkey_trigger", "hotkey_trigger"),
    ] {
        let prefix = format!("{field}:");
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let val_start = line_start + (line.len() - rest.len());
            let query = rest.trim_start().to_string();
            let q_start = val_start + (rest.len() - rest.trim_start().len());
            return Some(YamlContext {
                start_char: q_start,
                end_char: cursor,
                kind: AcTrigger::EnumValue(field),
                query,
                indent,
            });
        }
    }

    // List item under targets:
    if trimmed == "-" || trimmed.starts_with("- ") {
        let before = &text[..line_start];
        if before.lines().rev().any(|l| {
            let t = l.trim();
            t == "targets:" || t.starts_with("targets:")
        }) {
            let after_dash = trimmed.strip_prefix('-').unwrap_or(trimmed).trim_start();
            let val_start = cursor - after_dash.len();
            return Some(YamlContext {
                start_char: val_start,
                end_char: cursor,
                kind: AcTrigger::EntityValue,
                query: after_dash.to_string(),
                indent,
            });
        }
    }

    None
}

fn suggestions_for_context(ctx: &YamlContext, runtime: &[Suggestion]) -> Vec<Suggestion> {
    let q = ctx.query.to_ascii_lowercase();
    let mut out: Vec<Suggestion> = match ctx.kind {
        AcTrigger::TypeValue => runtime
            .iter()
            .filter(|s| s.kind == SuggestionKind::Type)
            .cloned()
            .collect(),
        AcTrigger::MacroNameValue => runtime
            .iter()
            .filter(|s| s.kind == SuggestionKind::MacroName)
            .cloned()
            .collect(),
        AcTrigger::EntityValue => runtime
            .iter()
            .filter(|s| s.kind == SuggestionKind::Entity)
            .cloned()
            .collect(),
        AcTrigger::EnumValue(field) => enum_values_for_field(field)
            .unwrap_or(&[])
            .iter()
            .map(|v| Suggestion {
                kind: SuggestionKind::Enum,
                label: (*v).into(),
                insert: (*v).into(),
                hint: field.into(),
                expand_stub: false,
            })
            .collect(),
        AcTrigger::FieldKey => {
            // Suggest common keys from all action descs + macro fields.
            let mut keys = std::collections::BTreeSet::new();
            for desc in sqyre_domain::ACTION_WIRE_DESCS {
                for f in desc.fields {
                    keys.insert(f.key);
                }
            }
            keys.into_iter()
                .map(|k| Suggestion {
                    kind: SuggestionKind::Field,
                    label: k.into(),
                    insert: format!("{k}: "),
                    hint: "field".into(),
                    expand_stub: false,
                })
                .collect()
        }
    };
    if !q.is_empty() {
        out.retain(|s| s.label.to_ascii_lowercase().contains(&q));
    }
    out.truncate(AC_LIMIT);
    out
}

fn blank_action_stub_yaml(type_key: &str, indent: &str) -> Option<String> {
    let action = blank_action(type_key)?;
    let map = action_to_map(&action).ok()?;
    let raw = serde_yaml::to_string(&serde_yaml::Value::Mapping(map)).ok()?;
    // Drop document start and re-indent.
    let body = raw
        .lines()
        .filter(|l| *l != "---")
        .map(|l| format!("{indent}{l}"))
        .collect::<Vec<_>>()
        .join("\n");
    Some(body)
}

fn apply_completion(text: &mut String, ctx: &YamlContext, suggestion: &Suggestion) {
    let start = ctx.start_char.min(text.len());
    let end = ctx.end_char.min(text.len());
    if suggestion.expand_stub {
        if let Some(stub) = blank_action_stub_yaml(&suggestion.insert, &ctx.indent) {
            // Replace from start of the type line through the incomplete value.
            let line_start = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            // If we're on `type: …`, replace that line with the full stub.
            let line_end = text[end..]
                .find('\n')
                .map(|i| end + i)
                .unwrap_or(text.len());
            text.replace_range(line_start..line_end, &stub);
            return;
        }
    }
    text.replace_range(start..end, &suggestion.insert);
}

fn yaml_text_edit(
    ui: &mut egui::Ui,
    id: egui::Id,
    value: &mut String,
    runtime: &[Suggestion],
    fill_height: f32,
    ac_armed: &mut bool,
) {
    let ac_id = id.with("yaml_ac");
    let was_open = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(ac_id.with("open")))
        .unwrap_or(false);
    let (down, up, accept, dismiss) = take_ac_keys(ui, was_open);

    let mono_row = ui.text_style_height(&egui::TextStyle::Monospace).max(1.0);
    let rows = (((fill_height - 8.0) / mono_row).floor() as usize).max(8);

    let output = egui::TextEdit::multiline(value)
        .id(id)
        .desired_width(f32::INFINITY)
        .desired_rows(rows)
        .hint_text("name: …\nroot:\n  type: loop\n  …")
        .font(egui::TextStyle::Monospace)
        .show(ui);

    if output.response.changed() {
        *ac_armed = true;
    }
    if !*ac_armed {
        ui.ctx()
            .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
        return;
    }

    let cursor = output
        .cursor_range
        .map(|r| r.primary.index.0)
        .unwrap_or(value.len());

    let ctx = find_yaml_context(value, cursor);
    let filtered = ctx
        .as_ref()
        .map(|c| suggestions_for_context(c, runtime))
        .unwrap_or_default();
    let open = !filtered.is_empty() && ctx.is_some();

    ui.ctx()
        .data_mut(|d| d.insert_temp(ac_id.with("open"), open));

    if !open {
        return;
    }
    let ctx = ctx.unwrap();

    let mut nav = ui
        .ctx()
        .data(|d| d.get_temp::<AcNav>(ac_id.with("nav")))
        .unwrap_or_default();
    let prev_selected = nav.selected;
    if nav.query != ctx.query {
        nav.selected = 0;
        nav.query = ctx.query.clone();
    }
    if down {
        nav.selected = (nav.selected + 1).min(filtered.len().saturating_sub(1));
    }
    if up {
        nav.selected = nav.selected.saturating_sub(1);
    }
    let selection_moved = nav.selected != prev_selected || down || up;
    if dismiss {
        ui.ctx()
            .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
        return;
    }

    // Anchor under the text caret (not the bottom of the whole TextEdit).
    let mut anchor_rect = output
        .cursor_range
        .map(|range| {
            let local = output.galley.pos_from_cursor(range.primary);
            egui::Rect::from_min_max(
                output.galley_pos + local.min.to_vec2(),
                output.galley_pos + local.max.to_vec2(),
            )
        })
        .unwrap_or(output.response.rect);
    if let Some(to_global) = ui
        .ctx()
        .layer_transform_to_global(output.response.layer_id)
    {
        anchor_rect = to_global * anchor_rect;
    }

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
    .gap(2.0)
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
                let selected = i == nav.selected;
                let label = format!("{}  —  {}", s.label, s.hint);
                let resp =
                    ui.selectable_label(selected, egui::RichText::new(label).monospace());
                if selected && selection_moved && needs_scroll {
                    resp.scroll_to_me(None);
                }
                if resp.clicked() {
                    apply_completion(value, &ctx, s);
                    if let Some(mut state) = TextEditState::load(ui.ctx(), id) {
                        let idx = ctx.start_char + s.insert.len();
                        state
                            .cursor
                            .set_char_range(Some(CCursorRange::one(CCursor::new(idx))));
                        state.store(ui.ctx(), id);
                    }
                    ui.memory_mut(|m| m.request_focus(id));
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
                }
            }
        };
        if needs_scroll {
            egui::ScrollArea::vertical()
                .max_height(AC_POPUP_MAX_H)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(popup_w);
                    paint_rows(ui);
                });
        } else {
            paint_rows(ui);
        }
    });

    if accept && !filtered.is_empty() {
        let s = &filtered[nav.selected.min(filtered.len() - 1)];
        apply_completion(value, &ctx, s);
        if let Some(mut state) = TextEditState::load(ui.ctx(), id) {
            let idx = ctx.start_char + s.insert.len();
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(idx))));
            state.store(ui.ctx(), id);
        }
        ui.memory_mut(|m| m.request_focus(id));
        ui.ctx()
            .data_mut(|d| d.insert_temp(ac_id.with("open"), false));
    }

    ui.ctx()
        .data_mut(|d| d.insert_temp(ac_id.with("nav"), nav));
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
        if i.consume_key(Modifiers::NONE, Key::Tab)
            || i.consume_key(Modifiers::NONE, Key::Enter)
        {
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
    fn type_context_detected() {
        let text = "  type: wa";
        let ctx = find_yaml_context(text, text.len()).unwrap();
        assert_eq!(ctx.kind, AcTrigger::TypeValue);
        assert_eq!(ctx.query, "wa");
    }

    #[test]
    fn enum_context_detected() {
        let text = "  repeatmode: onc";
        let ctx = find_yaml_context(text, text.len()).unwrap();
        assert!(matches!(ctx.kind, AcTrigger::EnumValue("repeatmode")));
    }

    #[test]
    fn stub_expand_produces_type_line() {
        let stub = blank_action_stub_yaml("wait", "  ").unwrap();
        assert!(stub.contains("type: wait"));
        assert!(stub.contains("time:"));
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
