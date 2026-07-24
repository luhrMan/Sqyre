//! Macro CRUD, tree clipboard, and undo/redo for SqyreApp.

use crate::tree_clipboard;
use crate::tree_history::TreeHistory;
use crate::SqyreApp;
use eframe::egui;
use sqyre_domain::{Action, ActionId, Macro};
use sqyre_hotkeys::{HotkeyTrigger, MacroHotkeyBinding};

/// Whether `m` should receive hotkeys under `filter`.
/// `None` = all macros; `Some("")` = untagged only; otherwise macros that include the tag.
pub(crate) fn macro_matches_hotkey_tag(m: &Macro, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some("") => m.tags.is_empty(),
        Some(tag) => m.tags.iter().any(|t| t == tag),
    }
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
        self.selected_actions.last().copied()
    }

    pub(crate) fn set_selected_actions(&mut self, ids: Vec<ActionId>) {
        self.selected_actions = ids;
    }

    pub(crate) fn clear_selected_actions(&mut self) {
        self.selected_actions.clear();
    }

    pub(crate) fn select_one_action(&mut self, id: ActionId) {
        self.selected_actions = vec![id];
    }

    pub(crate) fn remove_from_selection(&mut self, id: ActionId) {
        self.selected_actions.retain(|&a| a != id);
    }

    /// Clear a stale tag filter when no macro still carries that tag.
    pub(crate) fn sanitize_hotkey_tag_filter(&mut self) {
        let Some(tag) = self.hotkey_tag_filter.as_deref() else {
            return;
        };
        let still_valid = if tag.is_empty() {
            self.macros.iter().any(|m| m.tags.is_empty())
        } else {
            self.macros.iter().any(|m| m.tags.iter().any(|t| t == tag))
        };
        if !still_valid {
            self.hotkey_tag_filter = None;
        }
    }

    /// Toggle which tag's macros receive hotkeys. Clicking the active tag clears the filter.
    pub(crate) fn toggle_hotkey_tag_filter(&mut self, tag: String) {
        if self.hotkey_tag_filter.as_ref() == Some(&tag) {
            self.hotkey_tag_filter = None;
        } else {
            self.hotkey_tag_filter = Some(tag);
        }
        self.refresh_macro_hotkey_bindings();
    }

    pub(crate) fn refresh_macro_hotkey_bindings(&mut self) {
        self.sanitize_hotkey_tag_filter();
        let filter = self.hotkey_tag_filter.as_deref();
        let bindings = self
            .macros
            .iter()
            .filter(|m| !m.hotkey.is_empty())
            .filter(|m| macro_matches_hotkey_tag(m, filter))
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
            self.clear_selected_actions();
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
        self.play_ui_add_sound();
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
        self.play_ui_add_sound();
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
        self.play_ui_delete_sound();
        if self.macros.is_empty() {
            self.selected_macro = 0;
            self.clear_selected_actions();
            self.tooltip.cancel();
            return;
        }
        self.selected_macro = self.selected_macro.min(self.macros.len() - 1);
        self.clear_selected_actions();
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

    pub(crate) fn apply_hotkey_to_selected(
        &mut self,
        chord: Vec<String>,
        trigger: Option<HotkeyTrigger>,
    ) {
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
        let selected = self.selected_actions.clone();
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
        let mut selected = self.selected_actions.clone();
        let mut history = self.tree_histories.remove(&name).unwrap_or_default();
        let ok = history.undo(&mut self.macros[idx].root, &mut selected);
        self.tree_histories.insert(name, history);
        if ok {
            self.set_selected_actions(selected);
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
        let mut selected = self.selected_actions.clone();
        let mut history = self.tree_histories.remove(&name).unwrap_or_default();
        let ok = history.redo(&mut self.macros[idx].root, &mut selected);
        self.tree_histories.insert(name, history);
        if ok {
            self.set_selected_actions(selected);
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
        let Some(aid) = self.selected_action_id().filter(|a| !a.is_root()) else {
            return false;
        };
        self.macros[idx].root.find_by_id(aid).is_some()
    }

    pub(crate) fn can_paste_clipboard(&self) -> bool {
        self.action_clipboard.is_some() && !self.macros.is_empty()
    }

    pub(crate) fn copy_selection(&mut self, ctx: &egui::Context) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let Some(aid) = self.selected_action_id().filter(|a| !a.is_root()) else {
            return false;
        };
        let Some(action) = self.macros[idx].root.find_by_id(aid) else {
            return false;
        };
        let Ok(map) = sqyre_serialize::action_to_map(action) else {
            return false;
        };
        self.action_clipboard = Some(map);
        // egui-winit only emits Event::Paste when the OS clipboard is non-empty.
        // Action data stays process-local; this sentinel just unblocks Ctrl+V.
        ctx.copy_text(String::from("sqyre-action"));
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
        self.select_one_action(new_id);
        self.tooltip.cancel();
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
        let release = action.matching_release();
        self.record_tree_mutation();
        if self.macros[idx]
            .root
            .insert_at(parent, slot, action.clone())
            .is_err()
        {
            return false;
        }
        let mut companions = Vec::new();
        if let Some(release) = release {
            let release_id = release.id;
            if self.macros[idx]
                .root
                .insert_at(parent, sqyre_domain::InsertSlot::After(new_id), release)
                .is_ok()
            {
                companions.push(release_id);
            }
        }
        self.select_one_action(new_id);
        // Not persisted until Save; Cancel removes the provisional node(s).
        self.tooltip.open_edit_new(&action, edit_anchor, companions);
        self.play_ui_add_sound();
        true
    }

    pub(crate) fn discard_provisional_actions(&mut self, action_ids: &[ActionId]) {
        if self.macros.is_empty() || action_ids.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        for &action_id in action_ids {
            let _ = self.macros[idx].root.remove_by_id(action_id);
            self.remove_from_selection(action_id);
            if self.logs_window == Some(action_id) {
                self.logs_window = None;
                self.logs_image_cache.clear();
            }
        }
        // Drop the undo entry recorded for the provisional insert so Undo is a no-op.
        let name = self.macros[idx].name.clone();
        if let Some(hist) = self.tree_histories.get_mut(&name) {
            hist.pop_last_undo();
        }
    }

    pub(crate) fn cut_selection(&mut self, ctx: &egui::Context) -> bool {
        if !self.copy_selection(ctx) {
            return false;
        }
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let Some(aid) = self.selected_action_id().filter(|a| !a.is_root()) else {
            return false;
        };
        self.record_tree_mutation();
        let _ = self.macros[idx].root.remove_by_id(aid);
        self.clear_selected_actions();
        if self.logs_window == Some(aid) {
            self.logs_window = None;
            self.logs_image_cache.clear();
        }
        self.tooltip.cancel();
        self.persist_macro_at(idx);
        self.play_ui_delete_sound();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::macro_matches_hotkey_tag;
    use sqyre_domain::Macro;

    fn m(tags: &[&str]) -> Macro {
        let mut macro_ = Macro::new("m", 0, Vec::new());
        macro_.tags = tags.iter().map(|s| (*s).to_string()).collect();
        macro_
    }

    #[test]
    fn hotkey_tag_filter_matches() {
        let tagged = m(&["combat", "farm"]);
        let bare = m(&[]);
        assert!(macro_matches_hotkey_tag(&tagged, None));
        assert!(macro_matches_hotkey_tag(&bare, None));
        assert!(macro_matches_hotkey_tag(&tagged, Some("combat")));
        assert!(!macro_matches_hotkey_tag(&tagged, Some("other")));
        assert!(!macro_matches_hotkey_tag(&bare, Some("combat")));
        assert!(macro_matches_hotkey_tag(&bare, Some("")));
        assert!(!macro_matches_hotkey_tag(&tagged, Some("")));
    }
}
