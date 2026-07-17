//! Macro CRUD, tree clipboard, and undo/redo for SqyreApp.

use crate::tree_clipboard;
use crate::tree_history::TreeHistory;
use crate::SqyreApp;
use eframe::egui;
use sqyre_domain::{Action, ActionId, Macro};
use sqyre_hotkeys::{HotkeyTrigger, MacroHotkeyBinding};

impl SqyreApp {
    /// Provide egui context so background hotkey fires can wake an idle UI frame.
    pub(crate) fn bind_hotkey_repaint(&self, ctx: egui::Context) {
        *self.hotkey_repaint.lock() = Some(ctx);
    }

    pub(crate) fn selected_action_id(&self) -> Option<ActionId> {
        self.selected_action
    }

    pub(crate) fn refresh_macro_hotkey_bindings(&self) {
        let bindings = self
            .macros
            .iter()
            .filter(|m| !m.hotkey.is_empty())
            .map(|m| {
                MacroHotkeyBinding::new(
                    m.name.clone(),
                    m.hotkey.clone(),
                    HotkeyTrigger::parse(&m.hotkey_trigger),
                )
            })
            .collect();
        self.macro_hotkeys.set_bindings(bindings);
    }

    pub(crate) fn persist_macro_at(&mut self, idx: usize) {
        if idx >= self.macros.len() {
            return;
        }
        let m = self.macros[idx].clone();
        self.db.macros.insert(m.name.clone(), m);
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: save macro: {e}");
            self.save_error = Some(e.to_string());
        } else {
            self.save_error = None;
        }
        self.refresh_macro_hotkey_bindings();
    }

    pub(crate) fn unique_macro_name(&self, base: &str) -> String {
        if !self.macros.iter().any(|m| m.name == base) {
            return base.to_string();
        }
        for i in 2.. {
            let candidate = format!("{base} {i}");
            if !self.macros.iter().any(|m| m.name == candidate) {
                return candidate;
            }
        }
        unreachable!()
    }

    pub(crate) fn select_macro_by_name(&mut self, name: &str) {
        if let Some(i) = self.macros.iter().position(|m| m.name == name) {
            self.selected_macro = i;
            self.selected_action = None;
            self.tooltip.cancel();
            self.macro_meta.sync_selection(i, &self.macros[i]);
        }
    }

    pub(crate) fn create_macro(&mut self) {
        let name = self.unique_macro_name("new macro");
        let m = Macro::new(name.clone(), 0, vec![]);
        self.db.macros.insert(m.name.clone(), m.clone());
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: create macro: {e}");
            self.save_error = Some(e.to_string());
            return;
        }
        self.save_error = None;
        self.macros.push(m);
        self.macros.sort_by(|a, b| a.name.cmp(&b.name));
        self.refresh_macro_hotkey_bindings();
        self.select_macro_by_name(&name);
    }

    pub(crate) fn duplicate_selected_macro(&mut self) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let src_name = self.macros[idx].name.clone();
        let mut dup = self.macros[idx].clone();
        dup.name = self.unique_macro_name(&format!("{src_name} copy"));
        // Clear hotkey so duplicate doesn't steal the source chord.
        dup.hotkey.clear();
        let name = dup.name.clone();
        self.db.macros.insert(name.clone(), dup.clone());
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: duplicate macro: {e}");
            self.save_error = Some(e.to_string());
            return;
        }
        self.save_error = None;
        self.macros.push(dup);
        self.macros.sort_by(|a, b| a.name.cmp(&b.name));
        self.refresh_macro_hotkey_bindings();
        self.select_macro_by_name(&name);
    }

    pub(crate) fn delete_macro_named(&mut self, name: &str) {
        self.db.macros.remove(name);
        self.tree_histories.remove(name);
        self.macros.retain(|m| m.name != name);
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: delete macro: {e}");
            self.save_error = Some(e.to_string());
        } else {
            self.save_error = None;
        }
        self.refresh_macro_hotkey_bindings();
        if self.macros.is_empty() {
            self.selected_macro = 0;
            self.selected_action = None;
            self.tooltip.cancel();
            return;
        }
        self.selected_macro = self.selected_macro.min(self.macros.len() - 1);
        self.selected_action = None;
        self.tooltip.cancel();
        self.macro_meta
            .sync_selection(self.selected_macro, &self.macros[self.selected_macro]);
    }

    /// Rename the selected macro, drop the old db key, and rewrite Run Macro refs.
    pub(crate) fn rename_selected_macro(&mut self, new_name: String) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let old_name = self.macros[idx].name.clone();
        if old_name == new_name {
            return;
        }

        self.macros[idx].name = new_name.clone();
        for m in &mut self.macros {
            m.rename_macro_reference(&old_name, &new_name);
        }
        if let Some(hist) = self.tree_histories.remove(&old_name) {
            self.tree_histories.insert(new_name.clone(), hist);
        }
        self.db.macros.remove(&old_name);
        self.db.replace_macros(self.macros.iter().cloned());
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: rename macro: {e}");
            self.save_error = Some(e.to_string());
        } else {
            self.save_error = None;
        }
        self.refresh_macro_hotkey_bindings();

        self.macros.sort_by(|a, b| a.name.cmp(&b.name));
        if let Some(i) = self.macros.iter().position(|m| m.name == new_name) {
            self.selected_macro = i;
        }
        self.macro_meta
            .sync_selection(self.selected_macro, &self.macros[self.selected_macro]);
    }

    pub(crate) fn apply_hotkey_to_selected(&mut self, chord: Vec<String>, trigger: Option<HotkeyTrigger>) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let trigger =
            trigger.unwrap_or_else(|| HotkeyTrigger::parse(&self.macros[idx].hotkey_trigger));
        let binding = MacroHotkeyBinding::new(self.macros[idx].name.clone(), chord, trigger);
        self.macros[idx].hotkey = binding.chord;
        self.macros[idx].hotkey_trigger = trigger.as_str().to_string();
        self.persist_macro_at(idx);
    }

    pub(crate) fn record_tree_mutation(&mut self) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let selected = self.selected_action_id();
        let name = self.macros[idx].name.clone();
        let Ok(snap) = TreeHistory::take_snapshot(&self.macros[idx].root, selected) else {
            return;
        };
        self.tree_histories
            .entry(name)
            .or_default()
            .push_snapshot(snap);
    }

    pub(crate) fn undo_tree(&mut self) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let name = self.macros[idx].name.clone();
        let mut selected = self.selected_action_id();
        let mut history = self.tree_histories.remove(&name).unwrap_or_default();
        let ok = history.undo(&mut self.macros[idx].root, &mut selected);
        self.tree_histories.insert(name, history);
        if ok {
            self.selected_action = selected;
            self.tooltip.cancel();
            self.persist_macro_at(idx);
        }
    }

    pub(crate) fn redo_tree(&mut self) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let name = self.macros[idx].name.clone();
        let mut selected = self.selected_action_id();
        let mut history = self.tree_histories.remove(&name).unwrap_or_default();
        let ok = history.redo(&mut self.macros[idx].root, &mut selected);
        self.tree_histories.insert(name, history);
        if ok {
            self.selected_action = selected;
            self.tooltip.cancel();
            self.persist_macro_at(idx);
        }
    }

    pub(crate) fn can_undo(&self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        self.tree_histories
            .get(&self.macros[idx].name)
            .is_some_and(|h| h.can_undo())
    }

    pub(crate) fn can_redo(&self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        self.tree_histories
            .get(&self.macros[idx].name)
            .is_some_and(|h| h.can_redo())
    }

    pub(crate) fn can_copy_selection(&self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let Some(aid) = self.selected_action.filter(|a| !a.is_root()) else {
            return false;
        };
        self.macros[idx].root.find_by_id(aid).is_some()
    }

    pub(crate) fn can_paste_clipboard(&self) -> bool {
        self.action_clipboard.is_some() && !self.macros.is_empty()
    }

    pub(crate) fn copy_selection(&mut self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let Some(aid) = self.selected_action.filter(|a| !a.is_root()) else {
            return false;
        };
        let Some(action) = self.macros[idx].root.find_by_id(aid) else {
            return false;
        };
        let Ok(map) = sqyre_serialize::action_to_map(action) else {
            return false;
        };
        self.action_clipboard = Some(map);
        true
    }

    pub(crate) fn paste_clipboard(&mut self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let Some(clip) = self.action_clipboard.clone() else {
            return false;
        };
        let Ok(new_action) = sqyre_serialize::action_from_map(&clip) else {
            return false;
        };
        let new_id = new_action.id;
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let selected = self.selected_action_id();
        let Some((parent, slot)) =
            tree_clipboard::insert_location_below_selection(&self.macros[idx].root, selected)
        else {
            return false;
        };
        self.record_tree_mutation();
        if self.macros[idx]
            .root
            .insert_at(parent, slot, new_action)
            .is_err()
        {
            return false;
        }
        self.selected_action = Some(new_id);
        self.tooltip.cancel();
        self.persist_macro_at(idx);
        true
    }

    /// Insert a blank action below the current selection.
    /// Opens a provisional edit tip — Cancel removes the action without keeping it.
    pub(crate) fn insert_blank_action(&mut self, action: Action, edit_anchor: egui::Pos2) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let new_id = action.id;
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let selected = self.selected_action_id();
        let Some((parent, slot)) =
            tree_clipboard::insert_location_below_selection(&self.macros[idx].root, selected)
        else {
            return false;
        };
        self.record_tree_mutation();
        if self.macros[idx]
            .root
            .insert_at(parent, slot, action.clone())
            .is_err()
        {
            return false;
        }
        self.selected_action = Some(new_id);
        // Not persisted until Save; Cancel removes the provisional node.
        self.tooltip.open_edit_new(&action, edit_anchor);
        true
    }

    pub(crate) fn discard_provisional_action(&mut self, action_id: ActionId) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let _ = self.macros[idx].root.remove_by_id(action_id);
        if self.selected_action == Some(action_id) {
            self.selected_action = None;
        }
        if self.logs_window == Some(action_id) {
            self.logs_window = None;
            self.logs_image_cache.clear();
        }
        // Drop the undo entry recorded for the provisional insert so Undo is a no-op.
        let name = self.macros[idx].name.clone();
        if let Some(hist) = self.tree_histories.get_mut(&name) {
            hist.pop_last_undo();
        }
    }

    pub(crate) fn cut_selection(&mut self) -> bool {
        if !self.copy_selection() {
            return false;
        }
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let Some(aid) = self.selected_action.filter(|a| !a.is_root()) else {
            return false;
        };
        self.record_tree_mutation();
        let _ = self.macros[idx].root.remove_by_id(aid);
        self.selected_action = None;
        if self.logs_window == Some(aid) {
            self.logs_window = None;
            self.logs_image_cache.clear();
        }
        self.tooltip.cancel();
        self.persist_macro_at(idx);
        true
    }

}
