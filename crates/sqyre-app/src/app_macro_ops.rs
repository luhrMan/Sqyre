//! Macro CRUD, tree clipboard, and undo/redo for SqyreApp.

use crate::tree_clipboard;
use crate::tree_history::TreeHistory;
use crate::widgets::tags::{normalize_tag_path, tag_is_under_or_eq};
use crate::SqyreApp;
use eframe::egui;
use sqyre_domain::{Action, ActionId, InsertSlot, Macro};
use sqyre_hotkeys::{HotkeyTrigger, MacroHotkeyBinding};

/// Whether `m` should receive hotkeys under `filters`.
/// Empty = none (no Hotkeys checkboxes on); `""` entry = untagged; otherwise a macro
/// matches when any of its tags equals a filter or is nested under one (`filter/...`).
pub(crate) fn macro_matches_hotkey_tag(m: &Macro, filters: &[String]) -> bool {
    if filters.is_empty() {
        return false;
    }
    filters.iter().any(|filter| {
        if filter.is_empty() {
            m.tags.is_empty()
        } else {
            m.tags
                .iter()
                .any(|t| tag_is_under_or_eq(&normalize_tag_path(t), filter))
        }
    })
}

impl SqyreApp {
    /// Provide egui context so background hotkey fires can wake an idle UI frame.
    pub(crate) fn bind_hotkey_repaint(&self, ctx: egui::Context) {
        *self.hotkey_repaint.lock() = Some(ctx);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn play_ui_add_sound(&self) {
        let s = self.settings_ui.settings();
        crate::sound::play_add_sound_if(s.play_ui_sounds, s.sound_volume);
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn play_ui_add_sound(&self) {}

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn play_ui_delete_sound(&self) {
        let s = self.settings_ui.settings();
        crate::sound::play_delete_sound_if(s.play_ui_sounds, s.sound_volume);
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn play_ui_delete_sound(&self) {}

    pub(crate) fn selected_action_id(&self) -> Option<ActionId> {
        self.tree.selected_actions.last().copied()
    }

    pub(crate) fn set_selected_actions(&mut self, ids: Vec<ActionId>) {
        self.tree.selected_actions = ids;
    }

    pub(crate) fn clear_selected_actions(&mut self) {
        self.tree.selected_actions.clear();
    }

    pub(crate) fn select_one_action(&mut self, id: ActionId) {
        self.tree.selected_actions = vec![id];
    }

    pub(crate) fn remove_from_selection(&mut self, id: ActionId) {
        self.tree.selected_actions.retain(|&a| a != id);
    }

    /// Drop stale filter entries when no macro still carries that tag (or a nested path under it).
    /// Returns `true` when the filter set changed.
    pub(crate) fn sanitize_hotkey_tag_filter(&mut self) -> bool {
        let before = self.workspace.hotkey_tag_filters.len();
        self.workspace.hotkey_tag_filters.retain(|tag| {
            if tag.is_empty() {
                self.workspace.macros.iter().any(|m| m.tags.is_empty())
            } else {
                self.workspace.macros.iter().any(|m| {
                    m.tags
                        .iter()
                        .any(|t| tag_is_under_or_eq(&normalize_tag_path(t), tag))
                })
            }
        });
        self.workspace.hotkey_tag_filters.len() != before
    }

    /// Write [`Workspace::hotkey_tag_filters`] into settings when it drifted.
    pub(crate) fn persist_hotkey_tag_filter(&mut self) {
        let filter = self.workspace.hotkey_tag_filters.clone();
        if self.settings_ui.settings().hotkey_tag_filters == filter {
            return;
        }
        self.settings_ui.settings_mut().hotkey_tag_filters = filter;
        if let Err(e) = self.settings_ui.save_settings() {
            crate::log::warn(format!("failed to save hotkey tag filter: {e}"));
        }
    }

    /// Set the hotkey tag selection (sorted, deduped). Persists and refreshes bindings.
    pub(crate) fn set_hotkey_tag_filters(&mut self, tags: Vec<String>) {
        let mut tags: Vec<String> = tags
            .into_iter()
            .map(|t| {
                if t.is_empty() {
                    t
                } else {
                    normalize_tag_path(&t)
                }
            })
            .collect();
        tags.sort();
        tags.dedup();
        if self.workspace.hotkey_tag_filters == tags {
            return;
        }
        self.workspace.hotkey_tag_filters = tags;
        self.persist_hotkey_tag_filter();
        self.refresh_macro_hotkey_bindings();
    }

    /// Toggle membership of a tag in the multiselect filter.
    /// Selecting a parent covers descendants for matching; redundant child entries are dropped.
    /// If an ancestor is already selected, the click is a no-op (deselect the parent to pick leaves).
    pub(crate) fn toggle_hotkey_tag_filter(&mut self, tag: String) {
        let tag = if tag.is_empty() {
            tag
        } else {
            normalize_tag_path(&tag)
        };
        if let Some(i) = self
            .workspace
            .hotkey_tag_filters
            .iter()
            .position(|t| t == &tag)
        {
            self.workspace.hotkey_tag_filters.remove(i);
        } else if !tag.is_empty()
            && self
                .workspace
                .hotkey_tag_filters
                .iter()
                .any(|f| !f.is_empty() && tag_is_under_or_eq(&tag, f) && f != &tag)
        {
            // Covered by an ancestor selection — pick leaves only after clearing the parent.
            return;
        } else {
            // Drop filters nested under the newly selected path (now redundant).
            if !tag.is_empty() {
                self.workspace
                    .hotkey_tag_filters
                    .retain(|t| t.is_empty() || !tag_is_under_or_eq(t, &tag));
            }
            self.workspace.hotkey_tag_filters.push(tag);
            self.workspace.hotkey_tag_filters.sort();
            self.workspace.hotkey_tag_filters.dedup();
        }
        self.persist_hotkey_tag_filter();
        self.refresh_macro_hotkey_bindings();
    }

    pub(crate) fn refresh_macro_hotkey_bindings(&mut self) {
        if self.sanitize_hotkey_tag_filter() {
            self.persist_hotkey_tag_filter();
        }
        let filters = self.workspace.hotkey_tag_filters.as_slice();
        let bindings = self
            .workspace
            .macros
            .iter()
            .filter(|m| !m.hotkey.is_empty())
            .filter(|m| macro_matches_hotkey_tag(m, filters))
            .filter(|m| sqyre_validate::validate_macro(m).is_ok())
            .map(|m| {
                MacroHotkeyBinding::new(
                    m.name.clone(),
                    m.hotkey.clone(),
                    HotkeyTrigger::parse(&m.hotkey_trigger),
                )
            })
            .collect();
        self.run_session.macro_hotkeys.set_bindings(bindings);
    }

    /// Surface a failed `persist_database` to the toolbar status line.
    ///
    /// [`Self::persist_database`] already sets [`Workspace::save_error`] (macro-list banner);
    /// this also pushes the same failure into the always-visible run status.
    pub(crate) fn report_persist_failure(&mut self, action: &str, err: &str) {
        crate::log::warn(format_args!("{action}: {err}"));
        *self.run_session.state.status.lock() = format!("{action} failed: {err}");
    }

    pub(crate) fn persist_macro_at(&mut self, idx: usize) -> bool {
        if idx >= self.workspace.macros.len() {
            return false;
        }
        match self.persist_database() {
            Ok(()) => {
                self.refresh_macro_hotkey_bindings();
                true
            }
            Err(e) => {
                self.report_persist_failure("Save macro", &e);
                false
            }
        }
    }

    pub(crate) fn unique_macro_name(&self, base: &str) -> String {
        if !self.workspace.macros.iter().any(|m| m.name == base) {
            return base.to_string();
        }
        for i in 2.. {
            let candidate = format!("{base} {i}");
            if !self.workspace.macros.iter().any(|m| m.name == candidate) {
                return candidate;
            }
        }
        unreachable!()
    }

    pub(crate) fn select_macro_by_name(&mut self, name: &str) {
        if let Some(i) = self.workspace.macros.iter().position(|m| m.name == name) {
            self.workspace.selected_macro = i;
            self.clear_selected_actions();
            self.tree.tooltip.cancel();
            self.workspace
                .macro_meta
                .sync_selection(i, &self.workspace.macros[i]);
        }
    }

    pub(crate) fn create_macro(&mut self) {
        let name = self.unique_macro_name("new macro");
        let m = Macro::new(name.clone(), 0, vec![]);
        self.workspace.macros.push(m);
        self.workspace
            .macros
            .sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
        if let Err(e) = self.persist_database() {
            self.workspace.macros.retain(|m| m.name != name);
            self.report_persist_failure("Create macro", &e);
            return;
        }
        self.refresh_macro_hotkey_bindings();
        self.select_macro_by_name(&name);
        self.play_ui_add_sound();
    }

    /// Replace the selected macro from the YAML Macro Builder (transactional).
    pub(crate) fn apply_macro_from_yaml_builder(&mut self, mut incoming: Macro) {
        if self.workspace.macros.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let old_name = self.workspace.macros[idx].name.clone();
        let new_name = incoming.name.trim().to_string();
        if new_name.is_empty() {
            *self.run_session.state.status.lock() = "Apply failed: macro name is empty.".into();
            return;
        }
        // Reject collisions with other macros.
        if self
            .workspace
            .macros
            .iter()
            .enumerate()
            .any(|(i, m)| i != idx && m.name == new_name)
        {
            *self.run_session.state.status.lock() =
                format!("Apply failed: a macro named \"{new_name}\" already exists.");
            self.macro_yaml_builder.set_status_message(
                format!("Name \"{new_name}\" is already used — rename in YAML first."),
                true,
            );
            return;
        }

        // Snapshot for undo before mutate.
        self.record_tree_mutation();

        let old = self.workspace.macros[idx].clone();
        crate::macro_yaml_builder::reconcile_action_uids(&old.root, &mut incoming.root);
        // Keep root id stable.
        incoming.root.id = sqyre_domain::ActionId::root();
        incoming.init_runtime_variables();

        let backup = old;
        self.workspace.macros[idx] = incoming;

        if new_name != old_name {
            if let Some(h) = self.tree.histories.remove(&old_name) {
                self.tree.histories.insert(new_name.clone(), h);
            }
            self.macro_yaml_builder
                .on_macro_renamed(&old_name, &new_name);
            self.workspace
                .macros
                .sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
        }

        if let Err(e) = self.persist_database() {
            // Roll back.
            if let Some(i) = self
                .workspace
                .macros
                .iter()
                .position(|m| m.name == new_name || m.name == old_name)
            {
                self.workspace.macros[i] = backup;
            }
            if new_name != old_name {
                if let Some(h) = self.tree.histories.remove(&new_name) {
                    self.tree.histories.insert(old_name.clone(), h);
                }
            }
            // Drop the undo entry we just pushed for the failed apply.
            if let Some(h) = self.tree.histories.get_mut(&old_name) {
                h.pop_last_undo();
            }
            self.report_persist_failure("Apply YAML macro", &e);
            return;
        }

        self.refresh_macro_hotkey_bindings();
        self.select_macro_by_name(&new_name);
        self.tree.tooltip.cancel();
        self.tree.invalidate_paint_cache();
        // Filter selection to surviving ids.
        let root = &self.workspace.macros[self.workspace.selected_macro].root;
        self.tree
            .selected_actions
            .retain(|id| root.find_by_id(*id).is_some() || root.id == *id);
        if let Some(m) = self.workspace.macros.get(self.workspace.selected_macro) {
            self.macro_yaml_builder.on_applied(m);
        }
        *self.run_session.state.status.lock() = format!("Applied YAML to \"{new_name}\".");
    }

    /// Insert a decoded macro from the YAML Macro Builder (name already uniquified).
    pub(crate) fn import_macro_from_yaml_builder(&mut self, macro_: Macro) {
        let name = macro_.name.clone();
        let name = self.unique_macro_name(&name);
        let mut macro_ = macro_;
        macro_.name = name.clone();
        self.workspace.macros.push(macro_);
        self.workspace
            .macros
            .sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
        if let Err(e) = self.persist_database() {
            self.workspace.macros.retain(|m| m.name != name);
            self.report_persist_failure("Import YAML macro", &e);
            return;
        }
        self.refresh_macro_hotkey_bindings();
        self.select_macro_by_name(&name);
        self.play_ui_add_sound();
        *self.run_session.state.status.lock() = format!("Imported macro \"{name}\".");
        self.macro_yaml_builder.on_imported(&name);
    }

    pub(crate) fn duplicate_selected_macro(&mut self) {
        if self.workspace.macros.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let src_name = self.workspace.macros[idx].name.clone();
        let mut dup = self.workspace.macros[idx].clone();
        dup.name = self.unique_macro_name(&format!("{src_name} copy"));
        // Clear hotkey so duplicate doesn't steal the source chord.
        dup.hotkey.clear();
        let name = dup.name.clone();
        self.workspace.macros.push(dup);
        self.workspace
            .macros
            .sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
        if let Err(e) = self.persist_database() {
            self.workspace.macros.retain(|m| m.name != name);
            self.report_persist_failure("Duplicate macro", &e);
            return;
        }
        self.refresh_macro_hotkey_bindings();
        self.select_macro_by_name(&name);
        self.play_ui_add_sound();
    }

    pub(crate) fn delete_macro_named(&mut self, name: &str) {
        let Some(pos) = self.workspace.macros.iter().position(|m| m.name == name) else {
            return;
        };
        // Persist first; only drop history / YAML drafts after disk agrees.
        let removed = self.workspace.macros.remove(pos);
        if let Err(e) = self.persist_database() {
            self.workspace.macros.insert(pos, removed);
            self.report_persist_failure("Delete macro", &e);
            return;
        }
        self.tree.histories.remove(name);
        self.macro_yaml_builder.on_macro_deleted(name);
        self.refresh_macro_hotkey_bindings();
        self.play_ui_delete_sound();
        if self.workspace.macros.is_empty() {
            self.workspace.selected_macro = 0;
            self.clear_selected_actions();
            self.tree.tooltip.cancel();
            return;
        }
        self.workspace.selected_macro = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        self.clear_selected_actions();
        self.tree.tooltip.cancel();
        self.workspace.macro_meta.sync_selection(
            self.workspace.selected_macro,
            &self.workspace.macros[self.workspace.selected_macro],
        );
    }

    /// Rename the selected macro and rewrite Run Macro / overlay button refs.
    pub(crate) fn rename_selected_macro(&mut self, new_name: String) {
        if self.workspace.macros.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let old_name = self.workspace.macros[idx].name.clone();
        if old_name == new_name {
            return;
        }

        self.workspace.macros[idx].name = new_name.clone();
        for m in &mut self.workspace.macros {
            m.rename_macro_reference(&old_name, &new_name);
        }
        let overlay_changed = self
            .settings_ui
            .settings_mut()
            .rename_overlay_macro(&old_name, &new_name);
        if overlay_changed {
            self.data_editor
                .rename_overlay_form_macro(&old_name, &new_name);
            if let Err(e) = self.settings_ui.save_settings() {
                crate::log::warn(format_args!("rename macro overlay refs: {e}"));
                *self.run_session.state.status.lock() =
                    format!("Rename macro: overlay settings save failed: {e}");
            }
        }
        if let Some(hist) = self.tree.histories.remove(&old_name) {
            self.tree.histories.insert(new_name.clone(), hist);
        }
        self.macro_yaml_builder
            .on_macro_renamed(&old_name, &new_name);
        if let Err(e) = self.persist_database() {
            // Roll memory (+ overlay / history / YAML drafts) back to the old name.
            if let Some(i) = self
                .workspace
                .macros
                .iter()
                .position(|m| m.name == new_name)
            {
                self.workspace.macros[i].name = old_name.clone();
            }
            for m in &mut self.workspace.macros {
                m.rename_macro_reference(&new_name, &old_name);
            }
            if overlay_changed {
                self.settings_ui
                    .settings_mut()
                    .rename_overlay_macro(&new_name, &old_name);
                self.data_editor
                    .rename_overlay_form_macro(&new_name, &old_name);
                if let Err(se) = self.settings_ui.save_settings() {
                    crate::log::warn(format_args!("rename macro rollback overlay refs: {se}"));
                }
            }
            if let Some(hist) = self.tree.histories.remove(&new_name) {
                self.tree.histories.insert(old_name.clone(), hist);
            }
            self.macro_yaml_builder
                .on_macro_renamed(&new_name, &old_name);
            self.report_persist_failure("Rename macro", &e);
            return;
        }
        self.refresh_macro_hotkey_bindings();

        self.workspace
            .macros
            .sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
        if let Some(i) = self
            .workspace
            .macros
            .iter()
            .position(|m| m.name == new_name)
        {
            self.workspace.selected_macro = i;
        }
        self.workspace.macro_meta.sync_selection(
            self.workspace.selected_macro,
            &self.workspace.macros[self.workspace.selected_macro],
        );
    }

    pub(crate) fn apply_hotkey_to_selected(
        &mut self,
        chord: Vec<String>,
        trigger: Option<HotkeyTrigger>,
    ) {
        if self.workspace.macros.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let trigger = trigger
            .unwrap_or_else(|| HotkeyTrigger::parse(&self.workspace.macros[idx].hotkey_trigger));
        let binding =
            MacroHotkeyBinding::new(self.workspace.macros[idx].name.clone(), chord, trigger);
        self.workspace.macros[idx].hotkey = binding.chord;
        self.workspace.macros[idx].hotkey_trigger = trigger.as_str().to_string();
        self.persist_macro_at(idx);
    }

    pub(crate) fn record_tree_mutation(&mut self) {
        if self.workspace.macros.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let selected = self.tree.selected_actions.clone();
        let name = self.workspace.macros[idx].name.clone();
        let Ok(snap) = TreeHistory::take_snapshot(&self.workspace.macros[idx], selected) else {
            return;
        };
        self.tree
            .histories
            .entry(name)
            .or_default()
            .push_snapshot(snap);
        self.tree.invalidate_paint_cache();
    }

    pub(crate) fn undo_tree(&mut self) {
        if self.workspace.macros.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let name = self.workspace.macros[idx].name.clone();
        let mut selected = self.tree.selected_actions.clone();
        let mut history = self.tree.histories.remove(&name).unwrap_or_default();
        let result = history.undo(&mut self.workspace.macros[idx], &mut selected);
        self.tree.histories.insert(name.clone(), history);
        match result {
            Ok(()) => {
                // History may have restored a rename — remount history key.
                let new_name = self.workspace.macros[idx].name.clone();
                if new_name != name {
                    if let Some(h) = self.tree.histories.remove(&name) {
                        self.tree.histories.insert(new_name.clone(), h);
                    }
                    self.workspace
                        .macros
                        .sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
                    self.select_macro_by_name(&new_name);
                }
                self.set_selected_actions(selected);
                self.tree.tooltip.cancel();
                self.tree.invalidate_paint_cache();
                if let Err(e) = self.persist_database() {
                    // Persist failed after undo — reverse so memory matches disk.
                    self.reverse_last_undo_redo(/*was_undo=*/ true);
                    self.report_persist_failure("Undo", &e);
                    return;
                }
                self.refresh_macro_hotkey_bindings();
            }
            Err(e) => {
                crate::log::warn(format_args!("undo: {e}"));
                *self.run_session.state.status.lock() = format!("Undo failed: {e}");
            }
        }
    }

    pub(crate) fn redo_tree(&mut self) {
        if self.workspace.macros.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let name = self.workspace.macros[idx].name.clone();
        let mut selected = self.tree.selected_actions.clone();
        let mut history = self.tree.histories.remove(&name).unwrap_or_default();
        let result = history.redo(&mut self.workspace.macros[idx], &mut selected);
        self.tree.histories.insert(name.clone(), history);
        match result {
            Ok(()) => {
                let new_name = self.workspace.macros[idx].name.clone();
                if new_name != name {
                    if let Some(h) = self.tree.histories.remove(&name) {
                        self.tree.histories.insert(new_name.clone(), h);
                    }
                    self.workspace
                        .macros
                        .sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
                    self.select_macro_by_name(&new_name);
                }
                self.set_selected_actions(selected);
                self.tree.tooltip.cancel();
                self.tree.invalidate_paint_cache();
                if let Err(e) = self.persist_database() {
                    // Persist failed after redo — reverse so memory matches disk.
                    self.reverse_last_undo_redo(/*was_undo=*/ false);
                    self.report_persist_failure("Redo", &e);
                    return;
                }
                self.refresh_macro_hotkey_bindings();
            }
            Err(e) => {
                crate::log::warn(format_args!("redo: {e}"));
                *self.run_session.state.status.lock() = format!("Redo failed: {e}");
            }
        }
    }

    /// Undo the in-memory effect of a successful undo/redo whose persist failed.
    ///
    /// Does not call `persist_database` — disk already holds the pre-op state.
    fn reverse_last_undo_redo(&mut self, was_undo: bool) {
        if self.workspace.macros.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let name = self.workspace.macros[idx].name.clone();
        let mut selected = self.tree.selected_actions.clone();
        let mut history = self.tree.histories.remove(&name).unwrap_or_default();
        let result = if was_undo {
            history.redo(&mut self.workspace.macros[idx], &mut selected)
        } else {
            history.undo(&mut self.workspace.macros[idx], &mut selected)
        };
        self.tree.histories.insert(name.clone(), history);
        if result.is_err() {
            return;
        }
        let restored = self.workspace.macros[idx].name.clone();
        if restored != name {
            if let Some(h) = self.tree.histories.remove(&name) {
                self.tree.histories.insert(restored.clone(), h);
            }
            self.workspace
                .macros
                .sort_by(|a, b| crate::macro_meta::cmp_display_name(&a.name, &b.name));
            self.select_macro_by_name(&restored);
        }
        self.set_selected_actions(selected);
        self.tree.tooltip.cancel();
        self.tree.invalidate_paint_cache();
    }

    pub(crate) fn can_undo(&self) -> bool {
        if self.workspace.macros.is_empty() {
            return false;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        self.tree
            .histories
            .get(&self.workspace.macros[idx].name)
            .is_some_and(|h| h.can_undo())
    }

    pub(crate) fn can_redo(&self) -> bool {
        if self.workspace.macros.is_empty() {
            return false;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        self.tree
            .histories
            .get(&self.workspace.macros[idx].name)
            .is_some_and(|h| h.can_redo())
    }

    /// Shift selected actions one slot among siblings (`up` = Alt+Up).
    pub(crate) fn nudge_selection(&mut self, up: bool) -> bool {
        if self.workspace.macros.is_empty() {
            return false;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let selected = self.tree.selected_actions.clone();
        if self.workspace.macros[idx]
            .root
            .sibling_nudge_plan(&selected, up)
            .is_none()
        {
            return false;
        }
        self.record_tree_mutation();
        if !self.workspace.macros[idx]
            .root
            .nudge_siblings(&selected, up)
        {
            return false;
        }
        self.persist_macro_at(idx);
        true
    }

    pub(crate) fn can_copy_selection(&self) -> bool {
        if self.workspace.macros.is_empty() {
            return false;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let Some(aid) = self.selected_action_id().filter(|a| !a.is_root()) else {
            return false;
        };
        self.workspace.macros[idx].root.find_by_id(aid).is_some()
    }

    pub(crate) fn can_paste_clipboard(&self) -> bool {
        self.tree.clipboard.as_ref().is_some_and(|c| !c.is_empty())
            && !self.workspace.macros.is_empty()
    }

    pub(crate) fn copy_selection(&mut self, ctx: &egui::Context) -> bool {
        if self.workspace.macros.is_empty() {
            return false;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let Some(aid) = self.selected_action_id().filter(|a| !a.is_root()) else {
            return false;
        };
        let Some(action) = self.workspace.macros[idx].root.find_by_id(aid) else {
            return false;
        };
        let map = match sqyre_serialize::action_to_map(action) {
            Ok(m) => m,
            Err(e) => {
                *self.run_session.state.status.lock() = format!("Copy failed: {e}");
                return false;
            }
        };
        // Round-trip through the same nest-depth/type-key decode gates paste
        // uses, so a corrupt map can never land in the clipboard.
        if let Err(e) = sqyre_serialize::action_from_map(&map) {
            *self.run_session.state.status.lock() = format!("Copy failed: {e}");
            return false;
        }
        self.tree.clipboard = Some(vec![map]);
        // egui-winit only emits Event::Paste when the OS clipboard is non-empty.
        // Action data stays process-local; this sentinel just unblocks Ctrl+V.
        ctx.copy_text(String::from("sqyre-action"));
        true
    }

    /// Set the process-local clipboard to a list of action maps (macro record Copy).
    pub(crate) fn set_action_clipboard(
        &mut self,
        ctx: &egui::Context,
        maps: Vec<serde_yaml::Mapping>,
        yaml_preview: &str,
    ) -> bool {
        if maps.is_empty() {
            return false;
        }
        for map in &maps {
            if let Err(e) = sqyre_serialize::action_from_map(map) {
                *self.run_session.state.status.lock() = format!("Copy failed: {e}");
                return false;
            }
        }
        self.tree.clipboard = Some(maps);
        // Prefer human-readable YAML so the OS clipboard is useful; paste still
        // uses the process-local maps.
        if yaml_preview.is_empty() {
            ctx.copy_text(String::from("sqyre-action"));
        } else {
            ctx.copy_text(yaml_preview.to_string());
        }
        true
    }

    pub(crate) fn paste_clipboard(&mut self) -> bool {
        if self.workspace.macros.is_empty() {
            return false;
        }
        let Some(clip) = self.tree.clipboard.clone() else {
            return false;
        };
        if clip.is_empty() {
            return false;
        }
        let mut new_actions = Vec::with_capacity(clip.len());
        for map in &clip {
            let new_action = match sqyre_serialize::action_from_map(map) {
                Ok(a) => a,
                Err(e) => {
                    *self.run_session.state.status.lock() = format!("Paste failed: {e}");
                    return false;
                }
            };
            let idx = self
                .workspace
                .selected_macro
                .min(self.workspace.macros.len() - 1);
            if let Err(e) = sqyre_validate::validate_action_tree_persist(
                &new_action,
                Some(&self.workspace.macros[idx]),
            ) {
                *self.run_session.state.status.lock() = format!("Paste failed: {e}");
                return false;
            }
            new_actions.push(new_action);
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let selected = self.selected_action_id();
        let Some((parent, slot)) = tree_clipboard::insert_location_below_selection(
            &self.workspace.macros[idx].root,
            selected,
        ) else {
            *self.run_session.state.status.lock() = "Paste failed: no valid insert location".into();
            return false;
        };
        self.record_tree_mutation();
        let mut anchor_slot = slot;
        let mut last_id = None;
        for new_action in new_actions {
            let id = new_action.id;
            if self.workspace.macros[idx]
                .root
                .insert_at(parent, anchor_slot, new_action)
                .is_err()
            {
                *self.run_session.state.status.lock() =
                    "Paste failed: could not insert action".into();
                return false;
            }
            last_id = Some(id);
            anchor_slot = InsertSlot::After(id);
        }
        if let Some(id) = last_id {
            self.select_one_action(id);
        }
        self.tree.tooltip.cancel();
        self.persist_macro_at(idx);
        self.play_ui_add_sound();
        true
    }

    /// Insert a blank action below the current selection.
    /// Opens a provisional edit tip — Cancel removes the action without keeping it.
    ///
    /// Key/click actions inserted as [`PressState::Down`] also get a matching `Up`
    /// sibling inserted immediately below (discarded together if Cancel).
    pub(crate) fn insert_blank_action(&mut self, action: Action, edit_anchor: egui::Pos2) -> bool {
        if self.workspace.macros.is_empty() {
            return false;
        }
        let new_id = action.id;
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let selected = self.selected_action_id();
        let Some((parent, slot)) = tree_clipboard::insert_location_below_selection(
            &self.workspace.macros[idx].root,
            selected,
        ) else {
            return false;
        };
        let release = action.matching_release();
        self.record_tree_mutation();
        if self.workspace.macros[idx]
            .root
            .insert_at(parent, slot, action.clone())
            .is_err()
        {
            return false;
        }
        let mut companions = Vec::new();
        if let Some(release) = release {
            let release_id = release.id;
            if self.workspace.macros[idx]
                .root
                .insert_at(parent, sqyre_domain::InsertSlot::After(new_id), release)
                .is_ok()
            {
                companions.push(release_id);
            }
        }
        self.select_one_action(new_id);
        // Not persisted until Save; Cancel removes the provisional node(s).
        self.tree
            .tooltip
            .open_edit_new(&action, edit_anchor, companions);
        self.play_ui_add_sound();
        true
    }

    pub(crate) fn discard_provisional_actions(&mut self, action_ids: &[ActionId]) {
        if self.workspace.macros.is_empty() || action_ids.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        for &action_id in action_ids {
            let _ = self.workspace.macros[idx].root.remove_by_id(action_id);
            self.remove_from_selection(action_id);
            if self.run_session.logs_window == Some(action_id) {
                self.run_session.logs_window = None;
                self.run_session.logs_image_cache.clear();
            }
        }
        // Drop the undo entry recorded for the provisional insert so Undo is a no-op.
        let name = self.workspace.macros[idx].name.clone();
        if let Some(hist) = self.tree.histories.get_mut(&name) {
            hist.pop_last_undo();
        }
    }

    pub(crate) fn cut_selection(&mut self, ctx: &egui::Context) -> bool {
        if !self.copy_selection(ctx) {
            return false;
        }
        if self.workspace.macros.is_empty() {
            return false;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let Some(aid) = self.selected_action_id().filter(|a| !a.is_root()) else {
            return false;
        };
        self.record_tree_mutation();
        let _ = self.workspace.macros[idx].root.remove_by_id(aid);
        self.clear_selected_actions();
        if self.run_session.logs_window == Some(aid) {
            self.run_session.logs_window = None;
            self.run_session.logs_image_cache.clear();
        }
        self.tree.tooltip.cancel();
        self.persist_macro_at(idx);
        self.play_ui_delete_sound();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::macro_matches_hotkey_tag;
    use crate::SqyreApp;
    use sqyre_domain::Macro;
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn m(tags: &[&str]) -> Macro {
        let mut macro_ = Macro::new("m", 0, Vec::new());
        macro_.tags = tags.iter().map(|s| (*s).to_string()).collect();
        macro_
    }

    /// Make `path` unwritable so subsequent `db.yaml` atomic writes fail.
    #[cfg(unix)]
    fn make_unwritable(path: &std::path::Path) {
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o555);
        fs::set_permissions(path, perms).unwrap();
    }

    #[cfg(unix)]
    fn make_writable(path: &std::path::Path) {
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[cfg(unix)]
    fn with_unwritable_db_dir(f: impl FnOnce(&mut SqyreApp)) {
        let dir = tempfile::tempdir().unwrap();
        sqyre_persist::with_sqyre_dir_override(dir.path().to_path_buf(), || {
            sqyre_persist::initialize_directories().unwrap();
            let mut app = SqyreApp::for_docs();
            app.persist_database().expect("initial save");
            make_unwritable(dir.path());
            f(&mut app);
            make_writable(dir.path());
        });
    }

    #[cfg(unix)]
    fn status_text(app: &SqyreApp) -> String {
        app.run_session.state.status.lock().clone()
    }

    #[test]
    fn hotkey_tag_filter_matches() {
        let tagged = m(&["combat", "farm"]);
        let bare = m(&[]);
        assert!(!macro_matches_hotkey_tag(&tagged, &[]));
        assert!(!macro_matches_hotkey_tag(&bare, &[]));
        assert!(macro_matches_hotkey_tag(&tagged, &["combat".into()]));
        assert!(!macro_matches_hotkey_tag(&tagged, &["other".into()]));
        assert!(!macro_matches_hotkey_tag(&bare, &["combat".into()]));
        assert!(macro_matches_hotkey_tag(&bare, &["".into()]));
        assert!(!macro_matches_hotkey_tag(&tagged, &["".into()]));
        assert!(macro_matches_hotkey_tag(
            &tagged,
            &["other".into(), "farm".into()]
        ));
        assert!(macro_matches_hotkey_tag(
            &bare,
            &["combat".into(), "".into()]
        ));
    }

    #[test]
    fn hotkey_tag_filter_includes_nested_descendants() {
        let nested = m(&["combat/pve"]);
        assert!(macro_matches_hotkey_tag(&nested, &["combat".into()]));
        assert!(macro_matches_hotkey_tag(&nested, &["combat/pve".into()]));
        assert!(!macro_matches_hotkey_tag(&nested, &["combat/pvp".into()]));
        assert!(!macro_matches_hotkey_tag(
            &m(&["combatant"]),
            &["combat".into()]
        ));
        assert!(!macro_matches_hotkey_tag(
            &m(&["combat"]),
            &["combat/pve".into()]
        ));
    }

    #[test]
    #[cfg(unix)]
    fn create_macro_rolls_back_and_surfaces_persist_failure() {
        with_unwritable_db_dir(|app| {
            let before = app.workspace.macros.len();
            let names_before: Vec<_> = app
                .workspace
                .macros
                .iter()
                .map(|m| m.name.clone())
                .collect();
            app.create_macro();
            assert_eq!(app.workspace.macros.len(), before);
            assert_eq!(
                app.workspace
                    .macros
                    .iter()
                    .map(|m| m.name.clone())
                    .collect::<Vec<_>>(),
                names_before
            );
            assert!(app.workspace.save_error.is_some());
            assert!(
                status_text(app).starts_with("Create macro failed:"),
                "status={}",
                status_text(app)
            );
        });
    }

    #[test]
    #[cfg(unix)]
    fn duplicate_macro_rolls_back_and_surfaces_persist_failure() {
        with_unwritable_db_dir(|app| {
            let before = app.workspace.macros.len();
            assert!(before > 0);
            app.workspace.selected_macro = 0;
            let names_before: Vec<_> = app
                .workspace
                .macros
                .iter()
                .map(|m| m.name.clone())
                .collect();
            app.duplicate_selected_macro();
            assert_eq!(app.workspace.macros.len(), before);
            assert_eq!(
                app.workspace
                    .macros
                    .iter()
                    .map(|m| m.name.clone())
                    .collect::<Vec<_>>(),
                names_before
            );
            assert!(app.workspace.save_error.is_some());
            assert!(status_text(app).starts_with("Duplicate macro failed:"));
        });
    }

    #[test]
    #[cfg(unix)]
    fn delete_macro_rolls_back_and_surfaces_persist_failure() {
        with_unwritable_db_dir(|app| {
            let before = app.workspace.macros.len();
            assert!(before > 0);
            let name = app.workspace.macros[0].name.clone();
            app.delete_macro_named(&name);
            assert_eq!(app.workspace.macros.len(), before);
            assert!(app.workspace.macros.iter().any(|m| m.name == name));
            assert!(app.workspace.save_error.is_some());
            assert!(status_text(app).starts_with("Delete macro failed:"));
        });
    }

    #[test]
    #[cfg(unix)]
    fn rename_macro_rolls_back_and_surfaces_persist_failure() {
        with_unwritable_db_dir(|app| {
            assert!(!app.workspace.macros.is_empty());
            app.workspace.selected_macro = 0;
            let old = app.workspace.macros[0].name.clone();
            let new_name = format!("{old} renamed-ws3");
            app.rename_selected_macro(new_name.clone());
            assert!(app.workspace.macros.iter().any(|m| m.name == old));
            assert!(!app.workspace.macros.iter().any(|m| m.name == new_name));
            assert!(app.workspace.save_error.is_some());
            assert!(status_text(app).starts_with("Rename macro failed:"));
        });
    }
}
