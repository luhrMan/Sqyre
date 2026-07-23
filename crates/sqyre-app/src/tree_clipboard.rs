//! Macro tree copy/paste.
//!
//! Clipboard is process-local YAML maps via [`sqyre_serialize::action_to_map`]
//! (no UIDs) so paste assigns fresh identities.

use sqyre_domain::{Action, ActionId, InsertSlot};

/// Parent + slot for inserting relative to the current selection.
///
/// Insert location below the current selection:
/// - no selection / root → append under root
/// - detection Else folder → first child of that else branch
/// - branch selected → first child of that branch
/// - leaf selected → next sibling after the leaf
pub(crate) fn insert_location_below_selection(
    root: &Action,
    selected: Option<ActionId>,
) -> Option<(ActionId, InsertSlot)> {
    let Some(sel) = selected.filter(|id| !id.is_root()) else {
        return Some((root.id, InsertSlot::Last));
    };
    if matches!(
        root.resolve_tree_id(sel),
        Some(sqyre_domain::TreeNodeRef::ElseFolder { .. })
    ) {
        return Some((sel, InsertSlot::First));
    }
    let Some(node) = root.find_by_id(sel) else {
        return Some((root.id, InsertSlot::Last));
    };
    if node.is_branch() {
        return Some((sel, InsertSlot::First));
    }
    // Else-branch leaves: parent insert target is the Else folder sentinel.
    if let Some(parent_id) = root.find_parent_id(sel) {
        if let Some(parent) = root.find_by_id(parent_id) {
            if parent.has_else_folder()
                && parent
                    .else_children()
                    .is_some_and(|kids| kids.iter().any(|c| c.id == sel))
            {
                return Some((
                    ActionId::else_folder(parent_id),
                    InsertSlot::After(sel),
                ));
            }
        }
        return Some((parent_id, InsertSlot::After(sel)));
    }
    Some((root.id, InsertSlot::Last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, ActionKind, ScalarValue};
    use sqyre_serialize::{action_from_map, action_to_map};

    fn wait(time: i64) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(time),
            },
        }
    }

    fn branch(name: &str, kids: Vec<Action>) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Loop {
                name: name.into(),
                count: ScalarValue::Int(1),
                subactions: kids,
            },
        }
    }

    #[test]
    fn location_no_selection_appends_root() {
        let root = root_loop(vec![wait(1)]);
        assert_eq!(
            insert_location_below_selection(&root, None),
            Some((root.id, InsertSlot::Last))
        );
    }

    #[test]
    fn location_root_selected_appends_root() {
        let root = root_loop(vec![wait(1)]);
        assert_eq!(
            insert_location_below_selection(&root, Some(ActionId::root())),
            Some((root.id, InsertSlot::Last))
        );
    }

    #[test]
    fn location_branch_inserts_as_first_child() {
        let inner = branch("inner", vec![wait(1)]);
        let inner_id = inner.id;
        let root = root_loop(vec![inner]);
        assert_eq!(
            insert_location_below_selection(&root, Some(inner_id)),
            Some((inner_id, InsertSlot::First))
        );
    }

    #[test]
    fn location_leaf_inserts_after_sibling() {
        let a = wait(1);
        let b = wait(2);
        let a_id = a.id;
        let root = root_loop(vec![a, b]);
        assert_eq!(
            insert_location_below_selection(&root, Some(a_id)),
            Some((root.id, InsertSlot::After(a_id)))
        );
    }

    #[test]
    fn paste_below_selection_inserts_and_gets_fresh_id() {
        let wait_a = wait(10);
        let wait_a_id = wait_a.id;
        let loop_n = branch("L", vec![]);
        let loop_id = loop_n.id;
        let wait_c = wait(30);
        let mut root = root_loop(vec![wait_a, loop_n, wait_c]);

        let clip = action_to_map(&wait(77)).expect("encode");
        assert!(
            !clip.contains_key(serde_yaml::Value::String("uid".into())),
            "copy map must omit uid"
        );
        let pasted = action_from_map(&clip).expect("decode");
        let pasted_id = pasted.id;
        assert_ne!(pasted_id, wait_a_id);

        let (parent, slot) = insert_location_below_selection(&root, Some(wait_a_id)).expect("loc");
        root.insert_at(parent, slot, pasted).expect("insert");

        let ids: Vec<_> = root.children().iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), 4);
        assert_eq!(ids[0], wait_a_id);
        assert_eq!(ids[1], pasted_id);
        assert_eq!(ids[2], loop_id);
    }
}
