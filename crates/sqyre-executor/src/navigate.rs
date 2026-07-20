//! NavigateSelect / NavigateKey execution.

use crate::backends::MoveOptions;
use crate::error::{ExecError, FlowSignal, Result};
use crate::run::{resolve_int, resolve_text, run_children, Executor};
use sqyre_domain::{
    Action, ActionId, ActionKind, CoordinateRef, Macro, NavInputs, NavOptions, NavOutputs,
    NavSelectAction, NavigateSelectData, ScalarValue,
};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy)]
enum BuiltinChord {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
}

pub(crate) fn execute_navigate_select(
    exec: &mut Executor<'_>,
    action: &Action,
    macro_: &mut Macro,
) -> Result<()> {
    let ActionKind::NavigateSelect(data) = &action.kind else {
        return Err(ExecError::Message(
            "navigate select: internal kind mismatch".into(),
        ));
    };
    let data: &NavigateSelectData = data;

    let graph = resolve_graph_name(macro_, &data.graph_name, &data.inputs)?;
    if graph.is_empty() {
        return Err(ExecError::Message(
            "navigate select: graph/collection not set".into(),
        ));
    }

    let resolver = exec.deps.resolver.ok_or_else(|| {
        ExecError::Message("navigate select: coordinate resolver not configured".into())
    })?;
    let (rows, cols) = resolver
        .collection_grid(&data.program, &graph)
        .map_err(|e| {
            ExecError::Message(format!(
                "navigate select: collection {}/{}: {e}",
                data.program, graph
            ))
        })?;
    if rows < 1 || cols < 1 {
        return Err(ExecError::Message(format!(
            "navigate select: invalid grid {rows}x{cols}"
        )));
    }

    let mut row = resolve_cell_start(macro_, &data.inputs.row, 1)?.clamp(1, rows);
    let mut col = resolve_cell_start(macro_, &data.inputs.col, 1)?.clamp(1, cols);

    write_outputs(macro_, &data.program, &graph, row, col, &data.outputs);

    if data.options.move_cursor_with_nav {
        move_to_cell(
            exec,
            macro_,
            &data.program,
            &graph,
            row,
            col,
            data.options.smooth,
        )?;
    }

    exec.log(
        action.id,
        format!(
            "Navigate Select: {} · {graph} @ {row},{col} ({rows}x{cols})",
            data.program
        ),
    );

    let mut chords: Vec<Vec<String>> = Vec::new();
    let mut hold_mask: Vec<bool> = Vec::new();
    let mut builtins: Vec<Option<BuiltinChord>> = Vec::new();
    let mut key_branch_idxs: Vec<Option<usize>> = Vec::new();

    let mut push_builtin = |keys: &[String], kind: BuiltinChord, hold: bool| {
        if keys.iter().any(|k| !k.trim().is_empty()) {
            chords.push(keys.to_vec());
            hold_mask.push(hold);
            builtins.push(Some(kind));
            key_branch_idxs.push(None);
        }
    };

    push_builtin(&data.chords.up, BuiltinChord::Up, data.options.hold_repeat);
    push_builtin(
        &data.chords.down,
        BuiltinChord::Down,
        data.options.hold_repeat,
    );
    push_builtin(
        &data.chords.left,
        BuiltinChord::Left,
        data.options.hold_repeat,
    );
    push_builtin(
        &data.chords.right,
        BuiltinChord::Right,
        data.options.hold_repeat,
    );
    push_builtin(&data.chords.select, BuiltinChord::Select, false);
    push_builtin(&data.chords.back, BuiltinChord::Back, false);

    for (i, child) in data.subactions.iter().enumerate() {
        if let ActionKind::NavigateKey { chord, .. } = &child.kind {
            if chord.iter().any(|k| !k.trim().is_empty()) {
                chords.push(chord.clone());
                hold_mask.push(false);
                builtins.push(None);
                key_branch_idxs.push(Some(i));
            }
        }
    }

    if chords.is_empty() {
        return Err(ExecError::Message(
            "navigate select: no chords configured (nav, select, back, or Nav Key children)".into(),
        ));
    }

    let dummy = AtomicBool::new(false);
    let stop = exec.deps.stop_flag.unwrap_or(&dummy);

    loop {
        exec.check_stopped()?;
        let idx = {
            let waiter = exec.deps.continue_waiter.ok_or_else(|| {
                ExecError::Message(
                    "navigate select: key wait is not available in this build".into(),
                )
            })?;
            match waiter.wait_for_any_chord(&chords, &hold_mask, data.options.pass_through, stop) {
                Ok(i) => i,
                Err(e) if e.contains("stopped") => return Err(FlowSignal::Stopped.into()),
                Err(e) => return Err(ExecError::Message(e)),
            }
        };
        if stop.load(Ordering::SeqCst) {
            return Err(FlowSignal::Stopped.into());
        }

        if let Some(b) = builtins.get(idx).copied().flatten() {
            match b {
                BuiltinChord::Up => {
                    row = step(row, -1, rows, data.options.wrap_edges);
                    on_nav(
                        exec,
                        action.id,
                        macro_,
                        &data.program,
                        &graph,
                        &mut row,
                        &mut col,
                        &data.options,
                        &data.outputs,
                    )?;
                }
                BuiltinChord::Down => {
                    row = step(row, 1, rows, data.options.wrap_edges);
                    on_nav(
                        exec,
                        action.id,
                        macro_,
                        &data.program,
                        &graph,
                        &mut row,
                        &mut col,
                        &data.options,
                        &data.outputs,
                    )?;
                }
                BuiltinChord::Left => {
                    col = step(col, -1, cols, data.options.wrap_edges);
                    on_nav(
                        exec,
                        action.id,
                        macro_,
                        &data.program,
                        &graph,
                        &mut row,
                        &mut col,
                        &data.options,
                        &data.outputs,
                    )?;
                }
                BuiltinChord::Right => {
                    col = step(col, 1, cols, data.options.wrap_edges);
                    on_nav(
                        exec,
                        action.id,
                        macro_,
                        &data.program,
                        &graph,
                        &mut row,
                        &mut col,
                        &data.options,
                        &data.outputs,
                    )?;
                }
                BuiltinChord::Select => {
                    write_outputs(macro_, &data.program, &graph, row, col, &data.outputs);
                    perform_select(exec, &data.select)?;
                    exec.log(action.id, format!("Navigate Select: select @ {row},{col}"));
                    return Ok(());
                }
                BuiltinChord::Back => {
                    write_outputs(macro_, &data.program, &graph, row, col, &data.outputs);
                    exec.log(action.id, format!("Navigate Select: back @ {row},{col}"));
                    return Ok(());
                }
            }
            continue;
        }

        if let Some(Some(branch_i)) = key_branch_idxs.get(idx) {
            let Some(branch) = data.subactions.get(*branch_i) else {
                continue;
            };
            let ActionKind::NavigateKey {
                name,
                exit,
                subactions: kids,
                ..
            } = &branch.kind
            else {
                continue;
            };
            write_outputs(macro_, &data.program, &graph, row, col, &data.outputs);
            let label = if name.trim().is_empty() {
                "Nav Key".to_string()
            } else {
                name.clone()
            };
            let kids = kids.clone();
            let exit = *exit;
            exec.log(
                action.id,
                format!("Navigate Select: branch {label:?} @ {row},{col}"),
            );
            match run_children(exec, &kids, macro_) {
                Err(ExecError::Flow(FlowSignal::Break)) => return Ok(()),
                Err(e) => return Err(e),
                Ok(()) => {}
            }
            if exit {
                return Ok(());
            }
        }
    }
}

pub(crate) fn execute_navigate_key(
    _exec: &mut Executor<'_>,
    _action: &Action,
    _macro_: &mut Macro,
) -> Result<()> {
    Err(ExecError::Message(
        "navigate key: only runs as a child of Navigate Select".into(),
    ))
}

#[allow(clippy::too_many_arguments)]
fn on_nav(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    macro_: &mut Macro,
    program: &str,
    graph: &str,
    row: &mut i32,
    col: &mut i32,
    options: &NavOptions,
    outputs: &NavOutputs,
) -> Result<()> {
    write_outputs(macro_, program, graph, *row, *col, outputs);
    if options.move_cursor_with_nav {
        move_to_cell(exec, macro_, program, graph, *row, *col, options.smooth)?;
    }
    exec.log(action_id, format!("Navigate Select: cell {row},{col}"));
    Ok(())
}

fn step(cur: i32, delta: i32, max: i32, wrap: bool) -> i32 {
    let next = cur + delta;
    if wrap {
        if next < 1 {
            max
        } else if next > max {
            1
        } else {
            next
        }
    } else {
        next.clamp(1, max)
    }
}

fn resolve_graph_name(macro_: &Macro, graph_name: &str, inputs: &NavInputs) -> Result<String> {
    for src in [&inputs.graph, &inputs.collection] {
        if src.trim().is_empty() {
            continue;
        }
        if let Some(v) = macro_.variables.get(src.trim()) {
            let s = v.as_display();
            if !s.trim().is_empty() {
                return Ok(s);
            }
        }
        let resolved = resolve_text(src, macro_).unwrap_or_else(|_| src.to_string());
        if !resolved.trim().is_empty() && resolved != *src {
            return Ok(resolved);
        }
    }
    Ok(graph_name.trim().to_string())
}

fn resolve_cell_start(macro_: &Macro, field: &str, default: i32) -> Result<i32> {
    let t = field.trim();
    if t.is_empty() {
        return Ok(default);
    }
    if let Ok(n) = t.parse::<i32>() {
        return Ok(n);
    }
    if let Some(v) = macro_.variables.get(t) {
        return resolve_int(v, macro_);
    }
    let resolved = resolve_text(t, macro_).unwrap_or_else(|_| t.to_string());
    if let Ok(n) = resolved.trim().parse::<i32>() {
        return Ok(n);
    }
    Ok(default)
}

fn write_outputs(
    macro_: &mut Macro,
    program: &str,
    graph: &str,
    row: i32,
    col: i32,
    outputs: &NavOutputs,
) {
    let cell = CoordinateRef::collection(program, graph, row, col, row, col);
    if !outputs.output_ref.trim().is_empty() {
        macro_.variables.set(
            outputs.output_ref.trim(),
            ScalarValue::String(cell.0.clone()),
        );
    }
    if !outputs.output_graph.trim().is_empty() {
        macro_.variables.set(
            outputs.output_graph.trim(),
            ScalarValue::String(graph.to_string()),
        );
    }
    if !outputs.output_collection.trim().is_empty() {
        macro_.variables.set(
            outputs.output_collection.trim(),
            ScalarValue::String(graph.to_string()),
        );
    }
    if !outputs.output_row.trim().is_empty() {
        macro_
            .variables
            .set(outputs.output_row.trim(), ScalarValue::Int(row as i64));
    }
    if !outputs.output_col.trim().is_empty() {
        macro_
            .variables
            .set(outputs.output_col.trim(), ScalarValue::Int(col as i64));
    }
}

fn move_to_cell(
    exec: &mut Executor<'_>,
    macro_: &Macro,
    program: &str,
    graph: &str,
    row: i32,
    col: i32,
    smooth: bool,
) -> Result<()> {
    let resolver = exec.deps.resolver.ok_or_else(|| {
        ExecError::Message("navigate select: coordinate resolver not configured".into())
    })?;
    let cell = CoordinateRef::collection(program, graph, row, col, row, col);
    let (x, y) = resolver.resolve_point(&cell, macro_).map_err(|e| {
        ExecError::Message(format!("navigate select: resolve cell {row},{col}: {e}"))
    })?;
    exec.deps.automation.move_to(
        x,
        y,
        MoveOptions {
            smooth,
            low: 0.0,
            high: 0.0,
            delay_ms: 0,
        },
    );
    Ok(())
}

fn perform_select(exec: &mut Executor<'_>, select: &NavSelectAction) -> Result<()> {
    let mode = select.press_mode.trim().to_ascii_lowercase();
    let device = select.device.trim().to_ascii_lowercase();
    match device.as_str() {
        "" | "mouse" => {
            let btn = if select.button.trim().is_empty() {
                "left"
            } else {
                select.button.trim()
            };
            match mode.as_str() {
                "down" | "hold" => exec
                    .deps
                    .automation
                    .click(btn, true)
                    .map_err(ExecError::Message)?,
                "up" => exec
                    .deps
                    .automation
                    .click(btn, false)
                    .map_err(ExecError::Message)?,
                _ => {
                    exec.deps
                        .automation
                        .click(btn, true)
                        .map_err(ExecError::Message)?;
                    exec.deps
                        .automation
                        .click(btn, false)
                        .map_err(ExecError::Message)?;
                }
            }
        }
        "keyboard" => {
            let k = select.key.trim();
            if k.is_empty() {
                return Err(ExecError::Message(
                    "navigate select: select key not set".into(),
                ));
            }
            match mode.as_str() {
                "down" | "hold" => exec
                    .deps
                    .automation
                    .key_down(k)
                    .map_err(ExecError::Message)?,
                "up" => exec.deps.automation.key_up(k).map_err(ExecError::Message)?,
                _ => {
                    exec.deps
                        .automation
                        .key_down(k)
                        .map_err(ExecError::Message)?;
                    exec.deps.automation.key_up(k).map_err(ExecError::Message)?;
                }
            }
        }
        other => {
            return Err(ExecError::Message(format!(
                "navigate select: unknown select device {other:?}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{ImmediateContinueWaiter, RecordingBackend};
    use crate::run::{execute_macro_with, ExecDeps};
    use crate::test_support::FixedResolver;
    use sqyre_domain::{
        root_loop, ActionId, NavChords, NavInputs, NavOptions, NavOutputs,
        NavSelectAction, NavigateSelectData,
    };
    use std::sync::Mutex;

    #[test]
    fn select_exits_and_writes_outputs() {
        let mut backend = RecordingBackend::default();
        let waiter = ImmediateContinueWaiter {
            any_queue: Mutex::new(vec![0]), // select is only chord
            ..Default::default()
        };
        let resolver = FixedResolver::with_grid(3, 4);
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateSelect(Box::new(NavigateSelectData {
                program: "P".into(),
                graph_name: "bag".into(),
                chords: NavChords {
                    select: vec!["enter".into()],
                    ..Default::default()
                },
                options: NavOptions {
                    move_cursor_with_nav: true,
                    ..Default::default()
                },
                select: NavSelectAction::default(),
                inputs: NavInputs {
                    row: "2".into(),
                    col: "3".into(),
                    ..Default::default()
                },
                outputs: NavOutputs {
                    output_ref: "ref".into(),
                    output_graph: "g".into(),
                    output_row: "r".into(),
                    output_col: "c".into(),
                    output_collection: "col".into(),
                },
                ..Default::default()
            })),
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                close_matches_distance: 0,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: Some(&waiter),
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert_eq!(
            macro_.variables.get("ref").map(|v| v.as_display()),
            Some("P~bag@2,3-2,3".into())
        );
        assert_eq!(
            macro_.variables.get("r").map(|v| v.as_display()),
            Some("2".into())
        );
        assert_eq!(
            macro_.variables.get("c").map(|v| v.as_display()),
            Some("3".into())
        );
        assert!(backend.log.iter().any(|e| e.starts_with("move:")));
        assert!(backend.log.iter().any(|e| e.contains("click:left:down")));
        assert!(backend.log.iter().any(|e| e.contains("click:left:up")));
    }

    #[test]
    fn navigate_key_branch_runs_children() {
        let mut backend = RecordingBackend::default();
        let waiter = ImmediateContinueWaiter {
            // index 0 = only Nav Key chord
            any_queue: Mutex::new(vec![0]),
            ..Default::default()
        };
        let resolver = FixedResolver::with_grid(2, 2);
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateSelect(Box::new(NavigateSelectData {
                program: "P".into(),
                graph_name: "bag".into(),
                chords: NavChords::default(),
                outputs: NavOutputs {
                    output_row: "r".into(),
                    output_col: "c".into(),
                    ..Default::default()
                },
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::NavigateKey {
                        name: "Inspect".into(),
                        chord: vec!["i".into()],
                        exit: true,
                        subactions: vec![Action {
                            id: ActionId::new(),
                            kind: ActionKind::Click {
                                button: sqyre_domain::MouseButton::Right,
                                state: true,
                            },
                        }],
                    },
                }],
                ..Default::default()
            })),
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                close_matches_distance: 0,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: Some(&waiter),
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert!(backend.log.iter().any(|e| e == "click:right:down"));
        assert_eq!(
            macro_.variables.get("r").map(|v| v.as_display()),
            Some("1".into())
        );
    }
}
