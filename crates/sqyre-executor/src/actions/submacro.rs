//! Nested macro orchestration: RunMacro.

use crate::error::{ExecError, Result};
use crate::highlight::{highlight_clear, highlight_cursor, highlight_fill};
use crate::run::{execute_action, Executor};
use sqyre_domain::{ActionId, ActionKind, Macro};

pub(crate) fn execute_run_macro(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    macro_name: &str,
    caller: &mut Macro,
) -> Result<()> {
    if macro_name.trim().is_empty() {
        return Err(ExecError::Message("run macro: macro name not set".into()));
    }
    let lookup = exec
        .deps
        .macros
        .ok_or_else(|| ExecError::Message("run macro: macro catalog not configured".into()))?;
    let mut target = (*lookup
        .get(macro_name)
        .ok_or_else(|| ExecError::Message(format!("run macro: macro {macro_name:?} not found")))?)
    .clone();
    if !matches!(target.root.kind, ActionKind::Loop { .. }) {
        return Err(ExecError::Message(format!(
            "run macro: macro {macro_name:?} has no root"
        )));
    }

    target.init_runtime_variables();
    let monitor_sizes = match exec.deps.capturer.as_mut() {
        Some(c) => c.monitor_sizes().unwrap_or_else(|_| vec![(0, 0)]),
        None => vec![(0, 0)],
    };
    crate::run::apply_monitor_sizes(&mut target, &monitor_sizes);
    exec.log(action_id, format!("Run Macro: {macro_name}"));

    let caller_name = caller.name.clone();
    highlight_fill(exec.deps.highlighter, &caller_name, action_id, 0.0);

    // Run the target root Loop so its count and Break/Continue semantics match a direct run.
    let result = execute_action(exec, &target.root.clone(), &mut target);

    highlight_clear(exec.deps.highlighter, &caller_name, action_id);
    highlight_cursor(exec.deps.highlighter, &target.name, None);
    result
}
