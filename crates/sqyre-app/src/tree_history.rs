//! Macro tree undo/redo (full-macro snapshots).

use serde_yaml::Mapping;
use sqyre_domain::{Action, ActionId, Macro, VariableDecl};
use sqyre_serialize::{action_from_map, action_to_map_with_uid};
use sqyre_validate::validate_action_tree_persist;

/// Per-macro undo/redo depth cap. Keeps memory bounded for long editing
/// sessions since each entry is a full YAML snapshot of the macro.
const MAX_TREE_HISTORY_ENTRIES: usize = 100;

#[derive(Debug, Clone)]
struct MacroMetaSnap {
    name: String,
    global_delay: i32,
    keyboard_delay: i32,
    mouse_delay: i32,
    hotkey: Vec<String>,
    hotkey_trigger: String,
    tags: Vec<String>,
    variable_decls: Vec<VariableDecl>,
}

impl MacroMetaSnap {
    fn from_macro(m: &Macro) -> Self {
        Self {
            name: m.name.clone(),
            global_delay: m.global_delay,
            keyboard_delay: m.keyboard_delay,
            mouse_delay: m.mouse_delay,
            hotkey: m.hotkey.clone(),
            hotkey_trigger: m.hotkey_trigger.clone(),
            tags: m.tags.clone(),
            variable_decls: m.variable_decls.clone(),
        }
    }

    fn apply_to(&self, m: &mut Macro) {
        m.name = self.name.clone();
        m.global_delay = self.global_delay;
        m.keyboard_delay = self.keyboard_delay;
        m.mouse_delay = self.mouse_delay;
        m.hotkey = self.hotkey.clone();
        m.hotkey_trigger = self.hotkey_trigger.clone();
        m.tags = self.tags.clone();
        m.variable_decls = self.variable_decls.clone();
        m.init_runtime_variables();
    }
}

#[derive(Debug, Clone)]
pub struct TreeSnapshot {
    root_map: Mapping,
    meta: MacroMetaSnap,
    selected: Vec<ActionId>,
}

/// Per-macro undo/redo stacks of UID-preserving full-macro snapshots.
#[derive(Debug, Default)]
pub struct TreeHistory {
    undo: Vec<TreeSnapshot>,
    redo: Vec<TreeSnapshot>,
    applying: bool,
}

impl TreeHistory {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Build a snapshot without holding `&mut TreeHistory` (for borrow splitting).
    pub fn take_snapshot(macro_: &Macro, selected: Vec<ActionId>) -> Result<TreeSnapshot, String> {
        snapshot_macro(macro_, selected)
    }

    /// Snapshot using a specific root (pre-mutation) and meta from `macro_`.
    ///
    /// Used when the live macro is already mutably borrowed for the root edit.
    pub fn take_snapshot_parts(
        root: &Action,
        meta: &Macro,
        selected: Vec<ActionId>,
    ) -> Result<TreeSnapshot, String> {
        let root_map = action_to_map_with_uid(root).map_err(|e| e.to_string())?;
        Ok(TreeSnapshot {
            root_map,
            meta: MacroMetaSnap::from_macro(meta),
            selected,
        })
    }

    /// Push a pre-built snapshot (from [`Self::take_snapshot`]).
    pub fn push_snapshot(&mut self, snap: TreeSnapshot) {
        if self.applying {
            return;
        }
        self.push_undo_clearing_redo(snap);
    }

    /// Drop the most recent undo entry (used when discarding a provisional insert).
    pub fn pop_last_undo(&mut self) {
        let _ = self.undo.pop();
    }

    /// Undo the last mutation. On failure the bad entry is dropped.
    pub fn undo(&mut self, macro_: &mut Macro, selected: &mut Vec<ActionId>) -> Result<(), String> {
        if !self.can_undo() {
            return Err("nothing to undo".into());
        }
        let current = snapshot_macro(macro_, selected.clone())
            .map_err(|e| format!("snapshot current state: {e}"))?;
        let Some(prev) = self.undo.pop() else {
            return Err("nothing to undo".into());
        };
        apply_snapshot(macro_, selected, prev, &mut self.applying)
            .map_err(|e| format!("restore previous state: {e}"))?;
        self.push_redo(current);
        Ok(())
    }

    /// Redo the last undone mutation.
    pub fn redo(&mut self, macro_: &mut Macro, selected: &mut Vec<ActionId>) -> Result<(), String> {
        if !self.can_redo() {
            return Err("nothing to redo".into());
        }
        let current = snapshot_macro(macro_, selected.clone())
            .map_err(|e| format!("snapshot current state: {e}"))?;
        let Some(next) = self.redo.pop() else {
            return Err("nothing to redo".into());
        };
        apply_snapshot(macro_, selected, next, &mut self.applying)
            .map_err(|e| format!("restore next state: {e}"))?;
        self.push_undo_only(current);
        Ok(())
    }

    fn push_undo_clearing_redo(&mut self, snap: TreeSnapshot) {
        self.undo.push(snap);
        trim(&mut self.undo);
        self.redo.clear();
    }

    fn push_undo_only(&mut self, snap: TreeSnapshot) {
        self.undo.push(snap);
        trim(&mut self.undo);
    }

    fn push_redo(&mut self, snap: TreeSnapshot) {
        self.redo.push(snap);
        trim(&mut self.redo);
    }
}

fn trim(stack: &mut Vec<TreeSnapshot>) {
    if stack.len() > MAX_TREE_HISTORY_ENTRIES {
        let drop = stack.len() - MAX_TREE_HISTORY_ENTRIES;
        stack.drain(0..drop);
    }
}

fn snapshot_macro(macro_: &Macro, selected: Vec<ActionId>) -> Result<TreeSnapshot, String> {
    let root_map = action_to_map_with_uid(&macro_.root).map_err(|e| e.to_string())?;
    Ok(TreeSnapshot {
        root_map,
        meta: MacroMetaSnap::from_macro(macro_),
        selected,
    })
}

fn selection_still_valid(root: &Action, id: ActionId) -> bool {
    root.find_by_id(id).is_some()
        || root.id == id
        || matches!(
            root.resolve_tree_id(id),
            Some(sqyre_domain::TreeNodeRef::ElseFolder { .. })
        )
}

fn apply_snapshot(
    macro_: &mut Macro,
    selected: &mut Vec<ActionId>,
    snap: TreeSnapshot,
    applying: &mut bool,
) -> Result<(), String> {
    let restored_root = action_from_map(&snap.root_map).map_err(|e| e.to_string())?;
    validate_action_tree_persist(&restored_root, None).map_err(|e| e.to_string())?;

    *applying = true;
    snap.meta.apply_to(macro_);
    macro_.root = restored_root;
    *selected = snap
        .selected
        .into_iter()
        .filter(|&id| selection_still_valid(&macro_.root, id))
        .collect();
    *applying = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, ActionKind, ScalarValue};

    fn wait(ms: i64) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(ms),
            },
        }
    }

    fn child_ids(root: &Action) -> Vec<ActionId> {
        root.children().iter().map(|c| c.id).collect()
    }

    fn record(history: &mut TreeHistory, macro_: &Macro, selected: Vec<ActionId>) {
        let snap = TreeHistory::take_snapshot(macro_, selected).unwrap();
        history.push_snapshot(snap);
    }

    #[test]
    fn undo_redo_insert_and_remove() {
        let a = wait(1);
        let b = wait(2);
        let mut macro_ = Macro::new("m", 0, vec![]);
        macro_.root = root_loop(vec![a, b]);
        let mut history = TreeHistory::default();
        let mut selected = Vec::new();

        record(&mut history, &macro_, selected.clone());
        let c = wait(3);
        let c_id = c.id;
        macro_.root.children_mut().unwrap().push(c);
        assert_eq!(child_ids(&macro_.root).len(), 3);

        assert!(history.undo(&mut macro_, &mut selected).is_ok());
        assert_eq!(child_ids(&macro_.root).len(), 2);

        assert!(history.redo(&mut macro_, &mut selected).is_ok());
        let ids = child_ids(&macro_.root);
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&c_id));
    }

    #[test]
    fn undo_restores_macro_meta() {
        let mut macro_ = Macro::new("m", 10, vec!["f1".into()]);
        macro_.root = root_loop(vec![wait(1)]);
        let mut history = TreeHistory::default();
        let mut selected = Vec::new();
        record(&mut history, &macro_, selected.clone());
        macro_.global_delay = 99;
        macro_.name = "renamed".into();
        assert!(history.undo(&mut macro_, &mut selected).is_ok());
        assert_eq!(macro_.name, "m");
        assert_eq!(macro_.global_delay, 10);
    }
}
