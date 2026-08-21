//! Action tree navigation, insertion, and drag-and-drop helpers.

use super::{Action, ActionId};

/// Tree selection / drop target: a real action, or an Else folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNodeRef {
    Action(ActionId),
    ElseFolder { parent_id: ActionId },
}

/// Insertion slot relative to a parent directory (mirrors egui_ltreeview DirPosition).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertSlot {
    First,
    Last,
    Before(ActionId),
    After(ActionId),
}

impl Action {
    /// Resolve a tree node id to either a real action or an Else folder.
    pub fn resolve_tree_id(&self, id: ActionId) -> Option<TreeNodeRef> {
        if self.id == id {
            return Some(TreeNodeRef::Action(id));
        }
        if self.find_by_id(id).is_some() {
            return Some(TreeNodeRef::Action(id));
        }
        let owner = ActionId::else_folder_owner(id);
        let owner_has_else = if owner == self.id {
            self.has_else_folder()
        } else {
            self.find_by_id(owner).is_some_and(Action::has_else_folder)
        };
        if owner_has_else && ActionId::else_folder(owner) == id {
            return Some(TreeNodeRef::ElseFolder { parent_id: owner });
        }
        None
    }

    pub fn find_by_id(&self, id: ActionId) -> Option<&Action> {
        if self.id == id {
            return Some(self);
        }
        for child in self.children() {
            if let Some(found) = child.find_by_id(id) {
                return Some(found);
            }
        }
        if let Some(else_kids) = self.else_children() {
            for child in else_kids {
                if let Some(found) = child.find_by_id(id) {
                    return Some(found);
                }
            }
        }
        None
    }

    pub fn find_by_id_mut(&mut self, id: ActionId) -> Option<&mut Action> {
        if self.id == id {
            return Some(self);
        }
        let path = self.find_child_path(id)?;
        Some(Self::follow_child_path_mut(self, &path))
    }

    /// Path of `(in_else_list, index)` steps from this node to a descendant.
    fn find_child_path(&self, id: ActionId) -> Option<Vec<(bool, usize)>> {
        for (i, child) in self.children().iter().enumerate() {
            if child.id == id {
                return Some(vec![(false, i)]);
            }
            if let Some(mut sub) = child.find_child_path(id) {
                sub.insert(0, (false, i));
                return Some(sub);
            }
        }
        if let Some(else_kids) = self.else_children() {
            for (i, child) in else_kids.iter().enumerate() {
                if child.id == id {
                    return Some(vec![(true, i)]);
                }
                if let Some(mut sub) = child.find_child_path(id) {
                    sub.insert(0, (true, i));
                    return Some(sub);
                }
            }
        }
        None
    }

    fn follow_child_path_mut<'a>(node: &'a mut Action, path: &[(bool, usize)]) -> &'a mut Action {
        let mut cur = node;
        for &(in_else, index) in path {
            let list = if in_else {
                cur.else_children_mut().expect("else path")
            } else {
                cur.children_mut().expect("then path")
            };
            cur = &mut list[index];
        }
        cur
    }

    /// Remove a descendant by id (not self). Returns the detached node.
    pub fn remove_by_id(&mut self, id: ActionId) -> Option<Action> {
        let path = self.find_child_path(id)?;
        Self::remove_at_path(self, &path)
    }

    fn remove_at_path(node: &mut Action, path: &[(bool, usize)]) -> Option<Action> {
        let [(in_else, index)] = path else {
            let (in_else, index) = path[0];
            let child = {
                let list = if in_else {
                    node.else_children_mut()?
                } else {
                    node.children_mut()?
                };
                &mut list[index]
            };
            return Self::remove_at_path(child, &path[1..]);
        };
        let list = if *in_else {
            node.else_children_mut()?
        } else {
            node.children_mut()?
        };
        Some(list.remove(*index))
    }

    /// True if `id` is this node or any descendant (then or else).
    pub fn contains_id(&self, id: ActionId) -> bool {
        self.find_by_id(id).is_some()
    }

    /// Parent id of `id` when it is a descendant of this node (not self).
    ///
    /// Else-branch children report the detection action as parent (not the Else folder sentinel).
    pub fn find_parent_id(&self, id: ActionId) -> Option<ActionId> {
        for child in self.children() {
            if child.id == id {
                return Some(self.id);
            }
            if let Some(p) = child.find_parent_id(id) {
                return Some(p);
            }
        }
        if let Some(else_kids) = self.else_children() {
            for child in else_kids {
                if child.id == id {
                    return Some(self.id);
                }
                if let Some(p) = child.find_parent_id(id) {
                    return Some(p);
                }
            }
        }
        None
    }

    fn child_list_mut_for_insert(
        &mut self,
        parent_id: ActionId,
    ) -> Result<&mut Vec<Action>, String> {
        match self.resolve_tree_id(parent_id) {
            Some(TreeNodeRef::ElseFolder { parent_id: owner }) => {
                let parent = if owner == self.id {
                    self
                } else {
                    self.find_by_id_mut(owner)
                        .ok_or_else(|| format!("parent action {owner} not found"))?
                };
                parent
                    .else_children_mut()
                    .ok_or_else(|| "else drop target has no else branch".to_string())
            }
            Some(TreeNodeRef::Action(aid)) => {
                let parent = if aid == self.id {
                    self
                } else {
                    self.find_by_id_mut(aid)
                        .ok_or_else(|| format!("parent action {aid} not found"))?
                };
                parent
                    .children_mut()
                    .ok_or_else(|| "drop target is not a branch".to_string())
            }
            None => Err(format!("parent action {parent_id} not found")),
        }
    }

    /// Insert `child` into the children of `parent_id` at `slot`.
    ///
    /// `parent_id` may be an Else folder sentinel ([`ActionId::else_folder`]).
    pub fn insert_at(
        &mut self,
        parent_id: ActionId,
        slot: InsertSlot,
        child: Action,
    ) -> Result<(), String> {
        let children = self.child_list_mut_for_insert(parent_id)?;
        match slot {
            InsertSlot::First => children.insert(0, child),
            InsertSlot::Last => children.push(child),
            InsertSlot::Before(sib) => {
                let i = children
                    .iter()
                    .position(|c| c.id == sib)
                    .ok_or_else(|| "before-sibling not found".to_string())?;
                children.insert(i, child);
            }
            InsertSlot::After(sib) => {
                let i = children
                    .iter()
                    .position(|c| c.id == sib)
                    .ok_or_else(|| "after-sibling not found".to_string())?;
                children.insert(i + 1, child);
            }
        }
        Ok(())
    }

    /// Move `source_id` under `parent_id` at `slot`. Rejects self-drops and
    /// dropping a node into its own descendant.
    pub fn move_action(
        &mut self,
        source_id: ActionId,
        parent_id: ActionId,
        slot: InsertSlot,
    ) -> Result<(), String> {
        self.move_actions(&[source_id], parent_id, slot)
    }

    /// Move several nodes under `parent_id` at `slot`, preserving `source_ids` order.
    ///
    /// Removes every source first, then inserts so sequential Before/After slots do not
    /// reverse relative order. Skips root / Else-folder sentinels and duplicate ids.
    pub fn move_actions(
        &mut self,
        source_ids: &[ActionId],
        parent_id: ActionId,
        slot: InsertSlot,
    ) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        let mut sources: Vec<ActionId> = Vec::new();
        for &id in source_ids {
            if id.is_root()
                || matches!(
                    self.resolve_tree_id(id),
                    Some(TreeNodeRef::ElseFolder { .. })
                )
                || !seen.insert(id)
            {
                continue;
            }
            sources.push(id);
        }
        if sources.is_empty() {
            return Ok(());
        }

        let parent_for_check = match self.resolve_tree_id(parent_id) {
            Some(TreeNodeRef::ElseFolder { parent_id }) => parent_id,
            _ => parent_id,
        };
        for &source_id in &sources {
            if source_id == parent_id {
                return Err("cannot drop onto self".into());
            }
            if let Some(src) = self.find_by_id(source_id) {
                if src.contains_id(parent_for_check) {
                    return Err("cannot drop into own descendant".into());
                }
            }
        }

        // Dropping Before/After a source that is itself moving is a no-op for a
        // single-node move; with multiple sources, resolve to a non-moving sibling.
        let slot = match slot {
            InsertSlot::Before(id) | InsertSlot::After(id) if sources.contains(&id) => {
                if sources.len() == 1 && sources[0] == id {
                    return Ok(());
                }
                resolve_slot_around_moving_sources(self, parent_id, slot, &sources)?
            }
            other => other,
        };

        let mut nodes = Vec::with_capacity(sources.len());
        for source_id in sources {
            let node = self
                .remove_by_id(source_id)
                .ok_or_else(|| format!("source action {source_id} not found"))?;
            nodes.push(node);
        }

        match slot {
            InsertSlot::First => {
                for node in nodes.into_iter().rev() {
                    self.insert_at(parent_id, InsertSlot::First, node)?;
                }
            }
            InsertSlot::Last => {
                for node in nodes {
                    self.insert_at(parent_id, InsertSlot::Last, node)?;
                }
            }
            InsertSlot::Before(sib) => {
                let mut anchor = InsertSlot::Before(sib);
                for node in nodes {
                    let id = node.id;
                    self.insert_at(parent_id, anchor, node)?;
                    anchor = InsertSlot::After(id);
                }
            }
            InsertSlot::After(sib) => {
                let mut anchor = InsertSlot::After(sib);
                for node in nodes {
                    let id = node.id;
                    self.insert_at(parent_id, anchor, node)?;
                    anchor = InsertSlot::After(id);
                }
            }
        }
        Ok(())
    }

    /// Parent id to pass to [`move_actions`] and the ordered sibling ids of `id`.
    ///
    /// Else-branch children use the Else folder sentinel as parent.
    fn sibling_context(&self, id: ActionId) -> Option<(ActionId, Vec<ActionId>)> {
        if id.is_root()
            || matches!(
                self.resolve_tree_id(id),
                Some(TreeNodeRef::ElseFolder { .. })
            )
        {
            return None;
        }
        let parent_id = self.find_parent_id(id)?;
        let parent = if parent_id == self.id {
            self
        } else {
            self.find_by_id(parent_id)?
        };
        if parent.children().iter().any(|c| c.id == id) {
            let ids = parent.children().iter().map(|c| c.id).collect();
            return Some((parent_id, ids));
        }
        if let Some(else_kids) = parent.else_children() {
            if else_kids.iter().any(|c| c.id == id) {
                let ids = else_kids.iter().map(|c| c.id).collect();
                return Some((ActionId::else_folder(parent_id), ids));
            }
        }
        None
    }

    /// Plan a one-slot sibling shift for `ids` (`up` = toward the start of the list).
    ///
    /// All ids must share a sibling list. Sources are returned in sibling order.
    pub fn sibling_nudge_plan(
        &self,
        ids: &[ActionId],
        up: bool,
    ) -> Option<(Vec<ActionId>, ActionId, InsertSlot)> {
        let mut seen = std::collections::HashSet::new();
        let mut sources: Vec<ActionId> = Vec::new();
        for &id in ids {
            if id.is_root()
                || matches!(
                    self.resolve_tree_id(id),
                    Some(TreeNodeRef::ElseFolder { .. })
                )
                || !seen.insert(id)
            {
                continue;
            }
            sources.push(id);
        }
        if sources.is_empty() {
            return None;
        }
        let (parent, siblings) = self.sibling_context(sources[0])?;
        if !sources.iter().all(|id| siblings.contains(id)) {
            return None;
        }
        sources.sort_by_key(|&id| siblings.iter().position(|&s| s == id).unwrap_or(usize::MAX));
        let last = *sources.last()?;
        let min = siblings.iter().position(|&s| s == sources[0])?;
        let max = siblings.iter().position(|&s| s == last)?;
        let slot = if up {
            if min == 0 {
                return None;
            }
            InsertSlot::Before(siblings[min - 1])
        } else {
            if max + 1 >= siblings.len() {
                return None;
            }
            InsertSlot::After(siblings[max + 1])
        };
        Some((sources, parent, slot))
    }

    /// Shift `ids` one slot among their shared siblings. Returns whether the tree changed.
    pub fn nudge_siblings(&mut self, ids: &[ActionId], up: bool) -> bool {
        let Some((sources, parent, slot)) = self.sibling_nudge_plan(ids, up) else {
            return false;
        };
        self.move_actions(&sources, parent, slot).is_ok()
    }

    pub fn walk<F: FnMut(&Action)>(&self, f: &mut F) {
        f(self);
        for child in self.children() {
            child.walk(f);
        }
        if let Some(else_kids) = self.else_children() {
            for child in else_kids {
                child.walk(f);
            }
        }
    }

    pub fn walk_mut<F: FnMut(&mut Action)>(&mut self, f: &mut F) {
        f(self);
        if let Some(children) = self.children_mut() {
            for child in children.iter_mut() {
                child.walk_mut(f);
            }
        }
        if let Some(else_kids) = self.else_children_mut() {
            for child in else_kids.iter_mut() {
                child.walk_mut(f);
            }
        }
    }
}

/// When the drop marker is Before/After a node that is also moving, pick a
/// stable sibling (or First/Last) that will still exist after sources detach.
fn resolve_slot_around_moving_sources(
    root: &Action,
    parent_id: ActionId,
    slot: InsertSlot,
    sources: &[ActionId],
) -> Result<InsertSlot, String> {
    let children: Vec<ActionId> = {
        // Read-only walk of the insert list (same resolution as insert_at).
        let list = match root.resolve_tree_id(parent_id) {
            Some(TreeNodeRef::ElseFolder { parent_id: owner }) => {
                let parent = if owner == root.id {
                    root
                } else {
                    root.find_by_id(owner)
                        .ok_or_else(|| format!("parent action {owner} not found"))?
                };
                parent
                    .else_children()
                    .ok_or_else(|| "else drop target has no else branch".to_string())?
            }
            Some(TreeNodeRef::Action(aid)) => {
                let parent = if aid == root.id {
                    root
                } else {
                    root.find_by_id(aid)
                        .ok_or_else(|| format!("parent action {aid} not found"))?
                };
                if !parent.is_branch() {
                    return Err("drop target is not a branch".into());
                }
                parent.children()
            }
            None => return Err(format!("parent action {parent_id} not found")),
        };
        list.iter().map(|c| c.id).collect()
    };
    let source_set: std::collections::HashSet<_> = sources.iter().copied().collect();
    let (anchor_id, after) = match slot {
        InsertSlot::Before(id) => (id, false),
        InsertSlot::After(id) => (id, true),
        other => return Ok(other),
    };
    let Some(idx) = children.iter().position(|&id| id == anchor_id) else {
        return Err("drop sibling not found".into());
    };
    if after {
        // Find first non-moving sibling after the anchor block of movers.
        let mut i = idx + 1;
        while i < children.len() && source_set.contains(&children[i]) {
            i += 1;
        }
        if i < children.len() {
            // Insert before that survivor so movers land where the marker was.
            Ok(InsertSlot::Before(children[i]))
        } else {
            // Scan backward for a non-moving sibling to place After.
            let mut j = idx;
            while j > 0 && source_set.contains(&children[j - 1]) {
                j -= 1;
            }
            if j > 0 && !source_set.contains(&children[j - 1]) {
                Ok(InsertSlot::After(children[j - 1]))
            } else if !source_set.contains(&children[0]) && children[0] != anchor_id {
                Ok(InsertSlot::Before(children[0]))
            } else {
                Ok(InsertSlot::Last)
            }
        }
    } else {
        // Before(anchor): find last non-moving sibling before the mover block.
        let mut i = idx;
        while i > 0 && source_set.contains(&children[i - 1]) {
            i -= 1;
        }
        if i > 0 {
            Ok(InsertSlot::After(children[i - 1]))
        } else {
            let mut j = idx;
            while j < children.len() && source_set.contains(&children[j]) {
                j += 1;
            }
            if j < children.len() {
                Ok(InsertSlot::Before(children[j]))
            } else {
                Ok(InsertSlot::First)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        root_loop, Action, ActionId, ActionKind, ConditionBlock, DetectionBranch, InsertSlot,
        ScalarValue, TreeNodeRef,
    };

    fn wait(id: ActionId) -> Action {
        Action {
            id,
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        }
    }

    #[test]
    fn detection_else_insert_and_walk() {
        let detection_id = ActionId::new();
        let then_id = ActionId::new();
        let else_id = ActionId::new();
        let mut root = root_loop(vec![Action {
            id: detection_id,
            kind: ActionKind::FindPixel {
                name: String::new(),
                search_area: Default::default(),
                target_color: "#fff".into(),
                color_tolerance: 0,
                detection: DetectionBranch {
                    subactions: vec![wait(then_id)],
                    ..Default::default()
                },
            },
        }]);
        root.insert_at(
            ActionId::else_folder(detection_id),
            InsertSlot::Last,
            wait(else_id),
        )
        .unwrap();
        assert!(root.find_by_id(else_id).is_some());
        assert_eq!(
            root.find_parent_id(else_id),
            Some(detection_id),
            "else children report detection as parent"
        );
        let mut seen = Vec::new();
        root.walk(&mut |a| seen.push(a.id));
        assert!(seen.contains(&else_id));
        assert!(matches!(
            root.resolve_tree_id(ActionId::else_folder(detection_id)),
            Some(TreeNodeRef::ElseFolder { parent_id: id }) if id == detection_id
        ));
    }

    #[test]
    fn conditional_else_insert_and_run_path() {
        let cond_id = ActionId::new();
        let else_id = ActionId::new();
        let mut root = root_loop(vec![Action {
            id: cond_id,
            kind: ActionKind::Conditional {
                condition: ConditionBlock::default(),
                subactions: Vec::new(),
                else_actions: Vec::new(),
            },
        }]);
        root.insert_at(
            ActionId::else_folder(cond_id),
            InsertSlot::First,
            wait(else_id),
        )
        .unwrap();
        match &root.children()[0].kind {
            ActionKind::Conditional { else_actions, .. } => {
                assert_eq!(else_actions.len(), 1);
                assert_eq!(else_actions[0].id, else_id);
            }
            other => panic!("expected Conditional, got {other:?}"),
        }
    }

    #[test]
    fn move_action_reorders_siblings() {
        let a = ActionId::new();
        let b = ActionId::new();
        let c = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b), wait(c)]);
        root.move_action(c, ActionId::root(), InsertSlot::Before(a))
            .unwrap();
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![c, a, b]);
    }

    #[test]
    fn move_actions_preserves_order_after_sibling() {
        let a = ActionId::new();
        let b = ActionId::new();
        let c = ActionId::new();
        let d = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b), wait(c), wait(d)]);
        root.move_actions(&[b, c], ActionId::root(), InsertSlot::After(d))
            .unwrap();
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![a, d, b, c]);
    }

    #[test]
    fn move_actions_preserves_order_before_sibling() {
        let a = ActionId::new();
        let b = ActionId::new();
        let c = ActionId::new();
        let d = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b), wait(c), wait(d)]);
        root.move_actions(&[c, d], ActionId::root(), InsertSlot::Before(a))
            .unwrap();
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![c, d, a, b]);
    }

    #[test]
    fn nudge_siblings_swaps_with_neighbor() {
        let a = ActionId::new();
        let b = ActionId::new();
        let c = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b), wait(c)]);
        assert!(root.nudge_siblings(&[b], true));
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![b, a, c]);
        assert!(root.nudge_siblings(&[b], false));
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![a, b, c]);
    }

    #[test]
    fn nudge_siblings_noop_at_ends() {
        let a = ActionId::new();
        let b = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b)]);
        assert!(!root.nudge_siblings(&[a], true));
        assert!(!root.nudge_siblings(&[b], false));
        assert!(!root.nudge_siblings(&[ActionId::root()], false));
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![a, b]);
    }

    #[test]
    fn nudge_siblings_moves_contiguous_block() {
        let a = ActionId::new();
        let b = ActionId::new();
        let c = ActionId::new();
        let d = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b), wait(c), wait(d)]);
        assert!(root.nudge_siblings(&[b, c], false));
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![a, d, b, c]);
    }

    #[test]
    fn nudge_siblings_else_branch() {
        let detection_id = ActionId::new();
        let then_id = ActionId::new();
        let else_a = ActionId::new();
        let else_b = ActionId::new();
        let mut root = root_loop(vec![Action {
            id: detection_id,
            kind: ActionKind::FindPixel {
                name: String::new(),
                search_area: Default::default(),
                target_color: "#fff".into(),
                color_tolerance: 0,
                detection: DetectionBranch {
                    subactions: vec![wait(then_id)],
                    ..Default::default()
                },
            },
        }]);
        root.insert_at(
            ActionId::else_folder(detection_id),
            InsertSlot::Last,
            wait(else_a),
        )
        .unwrap();
        root.insert_at(
            ActionId::else_folder(detection_id),
            InsertSlot::Last,
            wait(else_b),
        )
        .unwrap();
        assert!(root.nudge_siblings(&[else_b], true));
        let else_ids: Vec<_> = root.children()[0]
            .else_children()
            .expect("else branch")
            .iter()
            .map(|x| x.id)
            .collect();
        assert_eq!(else_ids, vec![else_b, else_a]);
        assert_eq!(root.children()[0].children()[0].id, then_id);
    }

    #[test]
    fn move_action_rejects_into_self_descendant() {
        let branch_id = ActionId::new();
        let child_id = ActionId::new();
        let mut root = root_loop(vec![Action {
            id: branch_id,
            kind: ActionKind::Loop {
                name: "inner".into(),
                count: ScalarValue::Int(1),
                subactions: vec![wait(child_id)],
            },
        }]);
        assert!(root
            .move_action(branch_id, branch_id, InsertSlot::Last)
            .is_err());
    }
}
