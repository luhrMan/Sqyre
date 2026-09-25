//! Flow-control actions: While, ForEachRow, ForEachCell, Pause.

use crate::backends::PortError;
use crate::error::{ExecError, FlowSignal, Result};
use crate::path_confine::resolve_under_dir;
use crate::run::{eval_clauses, resolve_int, resolve_text, run_children, Executor};
use sqyre_domain::{
    grid_cell_rect, Action, ActionId, ConditionClause, CoordinateRef, ListColumn, Macro, MatchMode,
    ScalarValue, FOREACH_CELL_BUILTIN_BOTTOM, FOREACH_CELL_BUILTIN_COL, FOREACH_CELL_BUILTIN_COUNT,
    FOREACH_CELL_BUILTIN_LEFT, FOREACH_CELL_BUILTIN_RIGHT, FOREACH_CELL_BUILTIN_ROW,
    FOREACH_CELL_BUILTIN_TOP, FOREACH_CELL_BUILTIN_X, FOREACH_CELL_BUILTIN_Y,
    FOREACH_ROW_BUILTIN_ROW, FOREACH_ROW_BUILTIN_ROW_COUNT,
};
use sqyre_ports::{highlight_clear, highlight_fill};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct FlowLoopCtx<'a> {
    pub action_id: ActionId,
    pub name: &'a str,
    pub subactions: &'a [Action],
}

pub(crate) fn execute_while(
    exec: &mut Executor<'_>,
    ctx: &FlowLoopCtx<'_>,
    match_mode: MatchMode,
    clauses: &[ConditionClause],
    max_iterations: i32,
    macro_: &mut Macro,
) -> Result<()> {
    let action_id = ctx.action_id;
    let name = ctx.name;
    let subactions = ctx.subactions;
    let cap = if max_iterations <= 0 {
        exec.deps.while_max_iterations.max(1)
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
    if i >= cap {
        exec.log(
            action_id,
            format!(
                "While {name:?}: hit max iterations ({cap}{})",
                if max_iterations <= 0 {
                    "; max_iterations≤0 uses settings budget"
                } else {
                    ""
                }
            ),
        );
    }
    Ok(())
}

pub(crate) fn execute_for_each_row(
    exec: &mut Executor<'_>,
    ctx: &FlowLoopCtx<'_>,
    sources: &[ListColumn],
    start_row: &ScalarValue,
    end_row: &ScalarValue,
    macro_: &mut Macro,
) -> Result<()> {
    let action_id = ctx.action_id;
    let name = ctx.name;
    let subactions = ctx.subactions;
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

pub(crate) fn execute_for_each_cell(
    exec: &mut Executor<'_>,
    ctx: &FlowLoopCtx<'_>,
    cells: &CoordinateRef,
    macro_: &mut Macro,
) -> Result<()> {
    let action_id = ctx.action_id;
    let name = ctx.name;
    let subactions = ctx.subactions;

    let Some((sel_r1, sel_c1, sel_r2, sel_c2)) = cells.cell_range() else {
        return Err(ExecError::Message(format!(
            "for each cell {name:?}: select a Collection cell range"
        )));
    };

    let resolver = exec.deps.resolver.ok_or_else(|| {
        ExecError::Message(format!(
            "for each cell {name:?}: coordinate resolver is not available"
        ))
    })?;
    let area = resolver
        .collection_area(cells, macro_)
        .map_err(|e| ExecError::Message(format!("for each cell {name:?}: {e}")))?;

    let (sel_r1, sel_r2) = if sel_r1 <= sel_r2 {
        (sel_r1, sel_r2)
    } else {
        (sel_r2, sel_r1)
    };
    let (sel_c1, sel_c2) = if sel_c1 <= sel_c2 {
        (sel_c1, sel_c2)
    } else {
        (sel_c2, sel_c1)
    };

    let cell_count = ((sel_r2 - sel_r1 + 1) * (sel_c2 - sel_c1 + 1)) as i64;
    if cell_count <= 0 {
        exec.log(action_id, format!("ForEachCell: {name} no cells in range"));
        return Ok(());
    }

    let mut index = 0i64;
    for row in sel_r1..=sel_r2 {
        for col in sel_c1..=sel_c2 {
            exec.check_stopped()?;
            index += 1;
            highlight_fill(
                exec.deps.highlighter,
                &macro_.name,
                action_id,
                (index - 1) as f64 / cell_count as f64,
            );

            let (left, top, right, bottom) =
                grid_cell_rect(area.bounds(), area.rows, area.cols, row, col, row, col)
                    .ok_or_else(|| {
                        ExecError::Message(format!(
                            "for each cell {name:?}: cell {row},{col} out of bounds for {}x{} grid",
                            area.rows, area.cols
                        ))
                    })?;
            let cx = (left + right) / 2;
            let cy = (top + bottom) / 2;

            macro_
                .variables
                .set(FOREACH_CELL_BUILTIN_X, ScalarValue::Int(cx as i64));
            macro_
                .variables
                .set(FOREACH_CELL_BUILTIN_Y, ScalarValue::Int(cy as i64));
            macro_
                .variables
                .set(FOREACH_CELL_BUILTIN_ROW, ScalarValue::Int(row as i64));
            macro_
                .variables
                .set(FOREACH_CELL_BUILTIN_COL, ScalarValue::Int(col as i64));
            macro_
                .variables
                .set(FOREACH_CELL_BUILTIN_COUNT, ScalarValue::Int(cell_count));
            macro_
                .variables
                .set(FOREACH_CELL_BUILTIN_LEFT, ScalarValue::Int(left as i64));
            macro_
                .variables
                .set(FOREACH_CELL_BUILTIN_TOP, ScalarValue::Int(top as i64));
            macro_
                .variables
                .set(FOREACH_CELL_BUILTIN_RIGHT, ScalarValue::Int(right as i64));
            macro_
                .variables
                .set(FOREACH_CELL_BUILTIN_BOTTOM, ScalarValue::Int(bottom as i64));

            exec.log(
                action_id,
                format!("For each cell: {name} {row},{col} ({index}/{cell_count})"),
            );
            match run_children(exec, subactions, macro_) {
                Err(ExecError::Flow(FlowSignal::Break)) => {
                    highlight_clear(exec.deps.highlighter, &macro_.name, action_id);
                    return Ok(());
                }
                Err(ExecError::Flow(FlowSignal::Continue)) => continue,
                Err(e) => {
                    highlight_clear(exec.deps.highlighter, &macro_.name, action_id);
                    return Err(e);
                }
                Ok(()) => {}
            }
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
    let keys = sqyre_hotkeys::validate_continue_key(continue_key)
        .map_err(|e| ExecError::Message(e.to_string()))?;

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
        Err(PortError::Stopped) => Err(FlowSignal::Stopped.into()),
        Err(e) => Err(e.into()),
    }
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
        let base = variables_dir.ok_or_else(|| {
            ExecError::Message(format!(
                "for each row: file {:?} needs variables directory",
                col.source
            ))
        })?;
        let path = resolve_under_dir(base, &col.source)?;
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
