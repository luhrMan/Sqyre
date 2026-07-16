//! Flow-control actions: While, ForEachRow, Pause.

use crate::error::{ExecError, FlowSignal, Result};
use crate::highlight::{highlight_clear, highlight_fill};
use crate::run::{eval_clauses, resolve_int, resolve_text, run_children, Executor};
use sqyre_domain::{
    Action, ActionId, ConditionClause, ListColumn, Macro, MatchMode, ScalarValue,
    FOREACH_ROW_BUILTIN_ROW, FOREACH_ROW_BUILTIN_ROW_COUNT,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_while(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    name: &str,
    match_mode: MatchMode,
    clauses: &[ConditionClause],
    max_iterations: i32,
    subactions: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    let cap = if max_iterations <= 0 {
        i32::MAX
    } else {
        max_iterations
    };
    let mut i = 0;
    while i < cap {
        exec.check_stopped()?;
        if !eval_clauses(match_mode, clauses, macro_)? {
            exec.log(
                action_id,
                format!("While {name:?}: condition false after {i} iteration(s)"),
            );
            break;
        }
        i += 1;
        exec.log(action_id, format!("While: {name} iteration {i}"));
        match run_children(exec, subactions, macro_) {
            Err(ExecError::Flow(FlowSignal::Break)) => break,
            Err(ExecError::Flow(FlowSignal::Continue)) => continue,
            Err(e) => return Err(e),
            Ok(()) => {}
        }
    }
    if i >= cap && max_iterations > 0 {
        exec.log(
            action_id,
            format!("While {name:?}: hit max iterations ({max_iterations})"),
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_for_each_row(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    name: &str,
    sources: &[ListColumn],
    start_row: &ScalarValue,
    end_row: &ScalarValue,
    subactions: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    if sources.is_empty() {
        return Err(ExecError::Message(format!(
            "for each row {name:?}: at least one source is required"
        )));
    }

    let loaded: Vec<Vec<String>> = sources
        .iter()
        .map(|col| load_lines(col, exec.deps.variables_dir))
        .collect::<Result<Vec<_>>>()?;

    let row_count = loaded[0].len();
    for (j, lines) in loaded.iter().enumerate().skip(1) {
        if lines.len() < row_count {
            return Err(ExecError::Message(format!(
                "for each row {name:?}: source {} ({}) has {} lines, need at least {row_count}",
                j + 1,
                sources[j].output_var,
                lines.len()
            )));
        }
    }

    let start = resolve_row_bound(start_row, 1, macro_)?;
    let end = resolve_row_bound(end_row, row_count as i32, macro_)?;
    let start = start.max(1) as usize;
    let end = (end as usize).min(row_count);
    if start > end || row_count == 0 {
        exec.log(
            action_id,
            format!("ForEachRow: {name} no rows in range {start}..={end} (count {row_count})"),
        );
        return Ok(());
    }

    for i in (start - 1)..end {
        exec.check_stopped()?;
        if row_count > 0 {
            highlight_fill(
                exec.deps.highlighter,
                &macro_.name,
                action_id,
                i as f64 / row_count as f64,
            );
        }
        for (j, col) in sources.iter().enumerate() {
            let line = loaded[j].get(i).cloned().unwrap_or_default();
            if !col.output_var.is_empty() {
                macro_
                    .variables
                    .set(&col.output_var, ScalarValue::String(line));
            }
        }
        macro_
            .variables
            .set(FOREACH_ROW_BUILTIN_ROW, ScalarValue::Int((i + 1) as i64));
        macro_.variables.set(
            FOREACH_ROW_BUILTIN_ROW_COUNT,
            ScalarValue::Int(row_count as i64),
        );
        exec.log(
            action_id,
            format!("For each row: {name} row {}/{row_count}", i + 1),
        );
        match run_children(exec, subactions, macro_) {
            Err(ExecError::Flow(FlowSignal::Break)) => break,
            Err(ExecError::Flow(FlowSignal::Continue)) => continue,
            Err(e) => {
                highlight_clear(exec.deps.highlighter, &macro_.name, action_id);
                return Err(e);
            }
            Ok(()) => {}
        }
    }
    highlight_clear(exec.deps.highlighter, &macro_.name, action_id);
    Ok(())
}

pub(crate) fn execute_pause(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    message: &str,
    continue_key: &[String],
    pass_through: bool,
    macro_: &Macro,
) -> Result<()> {
    let keys = normalize_continue_key(continue_key);
    validate_continue_key(&keys)?;

    let msg = match resolve_text(message, macro_) {
        Ok(s) => s,
        Err(_) => message.to_string(),
    };
    let key_label = format_continue_key(&keys);
    if msg.is_empty() {
        exec.log(action_id, format!("Pause: waiting for {key_label}"));
    } else {
        exec.log(
            action_id,
            format!("Pause: waiting for {key_label} — {msg:?}"),
        );
    }

    let waiter = exec.deps.continue_waiter.ok_or_else(|| {
        ExecError::Message("pause: continue key wait is not available in this build".into())
    })?;

    let dummy = AtomicBool::new(false);
    let stop = exec.deps.stop_flag.unwrap_or(&dummy);
    match waiter.wait_for_continue(&keys, pass_through, stop) {
        Ok(()) => {
            if stop.load(Ordering::SeqCst) {
                return Err(FlowSignal::Stopped.into());
            }
            exec.log(action_id, format!("Pause: continued ({key_label})"));
            Ok(())
        }
        Err(e) if e.contains("stopped") => Err(FlowSignal::Stopped.into()),
        Err(e) => Err(ExecError::Message(e)),
    }
}

fn normalize_continue_key(keys: &[String]) -> Vec<String> {
    keys.iter()
        .map(|k| k.trim().to_ascii_lowercase())
        .filter(|k| !k.is_empty())
        .collect()
}

fn validate_continue_key(keys: &[String]) -> Result<()> {
    if keys.is_empty() {
        return Err(ExecError::Message("pause: continue key not set".into()));
    }
    let mut sorted = keys.to_vec();
    sorted.sort();
    let mut failsafe: Vec<String> = vec!["ctrl".into(), "esc".into(), "shift".into()];
    failsafe.sort();
    if sorted == failsafe {
        return Err(ExecError::Message(
            "pause: continue key cannot match the failsafe hotkey (esc + ctrl + shift)".into(),
        ));
    }
    Ok(())
}

fn format_continue_key(keys: &[String]) -> String {
    keys.join(" + ")
}

fn row_bound_is_set(v: &ScalarValue) -> bool {
    match v {
        ScalarValue::Null => false,
        ScalarValue::String(s) => !s.trim().is_empty(),
        _ => true,
    }
}

fn resolve_row_bound(v: &ScalarValue, default: i32, macro_: &Macro) -> Result<i32> {
    if !row_bound_is_set(v) {
        return Ok(default);
    }
    resolve_int(v, macro_)
}

fn load_lines(col: &ListColumn, variables_dir: Option<&Path>) -> Result<Vec<String>> {
    let raw = if col.is_file {
        let path = if Path::new(&col.source).is_absolute() {
            PathBuf::from(&col.source)
        } else {
            let base = variables_dir.ok_or_else(|| {
                ExecError::Message(format!(
                    "for each row: relative file {:?} needs variables directory",
                    col.source
                ))
            })?;
            base.join(&col.source)
        };
        fs::read_to_string(&path).map_err(|e| {
            ExecError::Message(format!("failed to read file {}: {e}", path.display()))
        })?
    } else {
        col.source.clone()
    };

    let mut lines: Vec<String> = raw.split('\n').map(|s| s.to_string()).collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    if col.skip_blank_lines {
        lines.retain(|l| !l.trim().is_empty());
    }
    Ok(lines)
}
