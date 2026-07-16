//! Nested macro orchestration: RunMacro.

use crate::error::{ExecError, FlowSignal, Result};
use crate::highlight::{highlight_clear, highlight_cursor, highlight_fill};
use crate::run::{execute_action, Executor};
use sqyre_domain::{Action, ActionId, ActionKind, Macro};

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
        .macros
        .ok_or_else(|| ExecError::Message("run macro: macro catalog not configured".into()))?;
    let mut target = (*lookup.get(macro_name).ok_or_else(|| {
        ExecError::Message(format!("run macro: macro {macro_name:?} not found"))
    })?)
    .clone();
    if !matches!(target.root.kind, ActionKind::Loop { .. }) {
        return Err(ExecError::Message(format!(
            "run macro: macro {macro_name:?} has no root"
        )));
    }

    target.init_runtime_variables();
    let monitor_sizes = match exec.capturer.as_mut() {
        Some(c) => c.monitor_sizes().unwrap_or_else(|_| vec![(0, 0)]),
        None => vec![(0, 0)],
    };
    crate::run::apply_monitor_sizes(&mut target, &monitor_sizes);
    exec.log(action_id, format!("Run Macro: {macro_name}"));

    let caller_name = caller.name.clone();
    highlight_fill(exec.highlighter, &caller_name, action_id, 0.0);

    let children: Vec<Action> = target.root.children().to_vec();
    let total = children.len();
    let result = (|| {
        for (i, child) in children.iter().enumerate() {
            exec.check_stopped()?;
            match execute_action(exec, child, &mut target) {
                Err(ExecError::Flow(FlowSignal::Break)) => break,
                Err(ExecError::Flow(FlowSignal::Continue)) => continue,
                Err(e) => return Err(e),
                Ok(()) => {}
            }
            if total > 0 {
                highlight_fill(
                    exec.highlighter,
                    &caller_name,
                    action_id,
                    (i + 1) as f64 / total as f64,
                );
            }
        }
        Ok(())
    })();

    highlight_clear(exec.highlighter, &caller_name, action_id);
    highlight_cursor(exec.highlighter, &target.name, None);
    result
}
