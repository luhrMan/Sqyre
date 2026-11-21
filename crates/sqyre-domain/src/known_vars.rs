//! Known-variable-name collection for a macro (leaf over `Macro` + `variables`).

use crate::{
    monitor_builtin_var_names, Action, ActionKind, KnownVariableNames, Macro,
    FOREACH_CELL_BUILTIN_VARS, FOREACH_ROW_BUILTIN_ROW, FOREACH_ROW_BUILTIN_ROW_COUNT,
    IMAGE_SEARCH_BUILTIN_VARS,
};

/// Collect defined variable names from decls, action bindings, and relevant builtins.
///
/// Includes `monitor1Width` / `monitor1Height` (one display). Prefer
/// [`collect_known_variable_names_with_monitors`] when the live display count is known.
pub fn collect_known_variable_names(macro_: &Macro) -> KnownVariableNames {
    collect_known_variable_names_with_monitors(macro_, 1)
}

/// Like [`collect_known_variable_names`], plus monitor builtins for `num_monitors`.
///
/// Names keep their declared/canonical casing for display (autocomplete, chips).
/// Lookup remains case-insensitive and O(1) via [`is_known_variable`].
pub fn collect_known_variable_names_with_monitors(
    macro_: &Macro,
    num_monitors: usize,
) -> KnownVariableNames {
    let mut known = KnownVariableNames::default();
    let mut has_image_search = false;
    let mut has_for_each_row = false;
    let mut has_for_each_cell = false;

    for d in &macro_.variable_decls {
        known.insert(&d.name);
    }

    macro_.root.walk(&mut |action: &Action| {
        match &action.kind {
            ActionKind::ImageSearch(_) => has_image_search = true,
            ActionKind::ForEachRow { .. } => has_for_each_row = true,
            ActionKind::ForEachCell { .. } => has_for_each_cell = true,
            _ => {}
        }
        for b in action.variable_bindings() {
            known.insert(&b.name);
        }
    });

    if has_image_search {
        for n in IMAGE_SEARCH_BUILTIN_VARS {
            known.insert(n);
        }
    }
    if has_for_each_row {
        known.insert(FOREACH_ROW_BUILTIN_ROW);
        known.insert(FOREACH_ROW_BUILTIN_ROW_COUNT);
    }
    if has_for_each_cell {
        for n in FOREACH_CELL_BUILTIN_VARS {
            known.insert(n);
        }
    }

    for name in monitor_builtin_var_names(num_monitors) {
        known.insert(&name);
    }

    known
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{root_loop, ScalarValue, VariableDecl};

    #[test]
    fn collect_includes_decls_bindings_and_builtins() {
        let mut m = Macro::new("m", 0, vec![]);
        m.variable_decls.push(VariableDecl {
            name: "Seed".into(),
            ..Default::default()
        });
        m.root = root_loop(vec![
            Action::new(ActionKind::SetVariable {
                assignments: vec![crate::VariableAssignment::new("Count", ScalarValue::Null)],
            }),
            Action::new(ActionKind::ForEachRow {
                name: "rows".into(),
                sources: vec![],
                start_row: ScalarValue::Null,
                end_row: ScalarValue::Null,
                subactions: vec![],
            }),
        ]);
        let known = collect_known_variable_names(&m);
        assert!(is_known_variable(&known, "seed"));
        assert!(is_known_variable(&known, "COUNT"));
        assert!(is_known_variable(&known, "Row"));
        assert!(is_known_variable(&known, "RowCount"));
        assert!(is_known_variable(&known, "monitor1Width"));
        assert!(is_known_variable(&known, "monitor1Height"));
        assert!(known.iter().any(|n| n == "Seed"));
        assert!(known.iter().any(|n| n == "Count"));
    }

    #[test]
    fn monitors_extend_known_names() {
        let known = collect_known_variable_names_with_monitors(&Macro::new("m", 0, vec![]), 2);
        assert!(is_known_variable(&known, "monitor2Width"));
    }
}
