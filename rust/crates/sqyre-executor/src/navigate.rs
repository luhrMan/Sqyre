//! NavigateSelect / NavigateKey execution.

use crate::backends::MoveOptions;
use crate::error::{ExecError, FlowSignal, Result};
use crate::run::{resolve_int, resolve_text, run_children, Executor};
use sqyre_domain::{Action, ActionId, ActionKind, CoordinateRef, Macro, ScalarValue};
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
    let ActionKind::NavigateSelect {
        program,
        graph_name,
        chord_up,
        chord_down,
        chord_left,
        chord_right,
        chord_select,
        chord_back,
        wrap_edges,
        move_cursor_with_nav,
        smooth,
        pass_through,
        hold_repeat,
        select_device,
        select_button,
        select_key,
        select_press_mode,
        in_graph,
        in_row,
        in_col,
        in_collection,
        output_ref,
        output_graph,
        output_row,
        output_col,
        output_collection,
        subactions,
    } = &action.kind
    else {
        return Err(ExecError::Message(
            "navigate select: internal kind mismatch".into(),
        ));
    };

    let graph = resolve_graph_name(macro_, graph_name, in_graph, in_collection)?;
    if graph.is_empty() {
        return Err(ExecError::Message(
            "navigate select: graph/collection not set".into(),
        ));
    }

    let resolver = exec.resolver.ok_or_else(|| {
        ExecError::Message("navigate select: coordinate resolver not configured".into())
    })?;
    let (rows, cols) = resolver.collection_grid(program, &graph).map_err(|e| {
        ExecError::Message(format!("navigate select: collection {program}/{graph}: {e}"))
    })?;
    if rows < 1 || cols < 1 {
        return Err(ExecError::Message(format!(
            "navigate select: invalid grid {rows}x{cols}"
        )));
    }

    let mut row = resolve_cell_start(macro_, in_row, 1)?.clamp(1, rows);
    let mut col = resolve_cell_start(macro_, in_col, 1)?.clamp(1, cols);

    write_outputs(
        macro_,
        program,
        &graph,
        row,
        col,
        output_ref,
        output_graph,
        output_row,
        output_col,
        output_collection,
    );

    if *move_cursor_with_nav {
        move_to_cell(exec, macro_, program, &graph, row, col, *smooth)?;
    }

    exec.log(
        action.id,
        format!("Navigate Select: {program} · {graph} @ {row},{col} ({rows}x{cols})"),
    );

    let mut chords: Vec<Vec<String>> = Vec::new();
    let mut hold_mask: Vec<bool> = Vec::new();
    let mut builtins: Vec<Option<BuiltinChord>> = Vec::new();
    let mut key_branch_idxs: Vec<Option<usize>> = Vec::new();

    let mut push_builtin =
        |keys: &[String], kind: BuiltinChord, hold: bool| {
            if keys.iter().any(|k| !k.trim().is_empty()) {
                chords.push(keys.to_vec());
                hold_mask.push(hold);
                builtins.push(Some(kind));
                key_branch_idxs.push(None);
            }
        };

    push_builtin(chord_up, BuiltinChord::Up, *hold_repeat);
    push_builtin(chord_down, BuiltinChord::Down, *hold_repeat);
    push_builtin(chord_left, BuiltinChord::Left, *hold_repeat);
    push_builtin(chord_right, BuiltinChord::Right, *hold_repeat);
    push_builtin(chord_select, BuiltinChord::Select, false);
    push_builtin(chord_back, BuiltinChord::Back, false);

    for (i, child) in subactions.iter().enumerate() {
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
            "navigate select: no chords configured (nav, select, back, or Nav Key children)"
                .into(),
        ));
    }

    let dummy = AtomicBool::new(false);
    let stop = exec.stop_flag.unwrap_or(&dummy);

    loop {
        exec.check_stopped()?;
        let idx = {
            let waiter = exec.continue_waiter.ok_or_else(|| {
                ExecError::Message(
                    "navigate select: key wait is not available in this build".into(),
                )
            })?;
            match waiter.wait_for_any_chord(&chords, &hold_mask, *pass_through, stop) {
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
                    row = step(row, -1, rows, *wrap_edges);
                    on_nav(
                        exec,
                        action.id,
                        macro_,
                        program,
                        &graph,
                        &mut row,
                        &mut col,
                        *move_cursor_with_nav,
                        *smooth,
                        output_ref,
                        output_graph,
                        output_row,
                        output_col,
                        output_collection,
                    )?;
                }
                BuiltinChord::Down => {
                    row = step(row, 1, rows, *wrap_edges);
                    on_nav(
                        exec,
                        action.id,
                        macro_,
                        program,
                        &graph,
                        &mut row,
                        &mut col,
                        *move_cursor_with_nav,
                        *smooth,
                        output_ref,
                        output_graph,
                        output_row,
                        output_col,
                        output_collection,
                    )?;
                }
                BuiltinChord::Left => {
                    col = step(col, -1, cols, *wrap_edges);
                    on_nav(
                        exec,
                        action.id,
                        macro_,
                        program,
                        &graph,
                        &mut row,
                        &mut col,
                        *move_cursor_with_nav,
                        *smooth,
                        output_ref,
                        output_graph,
                        output_row,
                        output_col,
                        output_collection,
                    )?;
                }
                BuiltinChord::Right => {
                    col = step(col, 1, cols, *wrap_edges);
                    on_nav(
                        exec,
                        action.id,
                        macro_,
                        program,
                        &graph,
                        &mut row,
                        &mut col,
                        *move_cursor_with_nav,
                        *smooth,
                        output_ref,
                        output_graph,
                        output_row,
                        output_col,
                        output_collection,
                    )?;
                }
                BuiltinChord::Select => {
                    write_outputs(
                        macro_,
                        program,
                        &graph,
                        row,
                        col,
                        output_ref,
                        output_graph,
                        output_row,
                        output_col,
                        output_collection,
                    );
                    perform_select(
                        exec,
                        select_device,
                        select_button,
                        select_key,
                        select_press_mode,
                    )?;
                    exec.log(
                        action.id,
                        format!("Navigate Select: select @ {row},{col}"),
                    );
                    return Ok(());
                }
                BuiltinChord::Back => {
                    write_outputs(
                        macro_,
                        program,
                        &graph,
                        row,
                        col,
                        output_ref,
                        output_graph,
                        output_row,
                        output_col,
                        output_collection,
                    );
                    exec.log(action.id, format!("Navigate Select: back @ {row},{col}"));
                    return Ok(());
                }
            }
            continue;
        }

        if let Some(Some(branch_i)) = key_branch_idxs.get(idx) {
            let Some(branch) = subactions.get(*branch_i) else {
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
            write_outputs(
                macro_,
                program,
                &graph,
                row,
                col,
                output_ref,
                output_graph,
                output_row,
                output_col,
                output_collection,
            );
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

fn on_nav(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    macro_: &mut Macro,
    program: &str,
    graph: &str,
    row: &mut i32,
    col: &mut i32,
    move_cursor: bool,
    smooth: bool,
    output_ref: &str,
    output_graph: &str,
    output_row: &str,
    output_col: &str,
    output_collection: &str,
) -> Result<()> {
    write_outputs(
        macro_,
        program,
        graph,
        *row,
        *col,
        output_ref,
        output_graph,
        output_row,
        output_col,
        output_collection,
    );
    if move_cursor {
        move_to_cell(exec, macro_, program, graph, *row, *col, smooth)?;
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

fn resolve_graph_name(
    macro_: &Macro,
    graph_name: &str,
    in_graph: &str,
    in_collection: &str,
) -> Result<String> {
    for src in [in_graph, in_collection] {
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
        if !resolved.trim().is_empty() && resolved != src {
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
    output_ref: &str,
    output_graph: &str,
    output_row: &str,
    output_col: &str,
    output_collection: &str,
) {
    let cell = CoordinateRef::collection(program, graph, row, col, row, col);
    if !output_ref.trim().is_empty() {
        macro_
            .variables
            .set(output_ref.trim(), ScalarValue::String(cell.0.clone()));
    }
    if !output_graph.trim().is_empty() {
        macro_
            .variables
            .set(output_graph.trim(), ScalarValue::String(graph.to_string()));
    }
    if !output_collection.trim().is_empty() {
        macro_.variables.set(
            output_collection.trim(),
            ScalarValue::String(graph.to_string()),
        );
    }
    if !output_row.trim().is_empty() {
        macro_
            .variables
            .set(output_row.trim(), ScalarValue::Int(row as i64));
    }
    if !output_col.trim().is_empty() {
        macro_
            .variables
            .set(output_col.trim(), ScalarValue::Int(col as i64));
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
    let resolver = exec.resolver.ok_or_else(|| {
        ExecError::Message("navigate select: coordinate resolver not configured".into())
    })?;
    let cell = CoordinateRef::collection(program, graph, row, col, row, col);
    let (x, y) = resolver.resolve_point(&cell, macro_).map_err(|e| {
        ExecError::Message(format!("navigate select: resolve cell {row},{col}: {e}"))
    })?;
    exec.automation.move_to(
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

fn perform_select(
    exec: &mut Executor<'_>,
    device: &str,
    button: &str,
    key: &str,
    mode: &str,
) -> Result<()> {
    let mode = mode.trim().to_ascii_lowercase();
    let device = device.trim().to_ascii_lowercase();
    match device.as_str() {
        "" | "mouse" => {
            let btn = if button.trim().is_empty() {
                "left"
            } else {
                button.trim()
            };
            match mode.as_str() {
                "down" | "hold" => exec.automation.click(btn, true).map_err(ExecError::Message)?,
                "up" => exec.automation.click(btn, false).map_err(ExecError::Message)?,
                _ => {
                    exec.automation.click(btn, true).map_err(ExecError::Message)?;
                    exec.automation.click(btn, false).map_err(ExecError::Message)?;
                }
            }
        }
        "keyboard" => {
            let k = key.trim();
            if k.is_empty() {
                return Err(ExecError::Message(
                    "navigate select: select key not set".into(),
                ));
            }
            match mode.as_str() {
                "down" | "hold" => exec.automation.key_down(k).map_err(ExecError::Message)?,
                "up" => exec.automation.key_up(k).map_err(ExecError::Message)?,
                _ => {
                    exec.automation.key_down(k).map_err(ExecError::Message)?;
                    exec.automation.key_up(k).map_err(ExecError::Message)?;
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
    use sqyre_domain::{root_loop, ActionId, CoordinateRef};
    use std::sync::Mutex;

    struct GridResolver {
        rows: i32,
        cols: i32,
    }

    impl crate::backends::CoordinateResolver for GridResolver {
        fn resolve_point(
            &self,
            r: &CoordinateRef,
            _macro_: &Macro,
        ) -> std::result::Result<(i32, i32), String> {
            let (r1, c1, _, _) = r.cell_range().ok_or("expected cell")?;
            Ok((c1 * 10, r1 * 10))
        }
        fn resolve_search_area(
            &self,
            _r: &CoordinateRef,
            _macro_: &Macro,
        ) -> std::result::Result<(i32, i32, i32, i32), String> {
            Ok((0, 0, 100, 100))
        }
        fn collection_grid(
            &self,
            _program: &str,
            _collection: &str,
        ) -> std::result::Result<(i32, i32), String> {
            Ok((self.rows, self.cols))
        }
    }

    #[test]
    fn select_exits_and_writes_outputs() {
        let mut backend = RecordingBackend::default();
        let waiter = ImmediateContinueWaiter {
            any_queue: Mutex::new(vec![0]), // select is only chord
            ..Default::default()
        };
        let resolver = GridResolver { rows: 3, cols: 4 };
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateSelect {
                program: "P".into(),
                graph_name: "bag".into(),
                chord_up: vec![],
                chord_down: vec![],
                chord_left: vec![],
                chord_right: vec![],
                chord_select: vec!["enter".into()],
                chord_back: vec![],
                wrap_edges: false,
                move_cursor_with_nav: true,
                smooth: false,
                pass_through: false,
                hold_repeat: false,
                select_device: "mouse".into(),
                select_button: "left".into(),
                select_key: String::new(),
                select_press_mode: "click".into(),
                in_graph: String::new(),
                in_row: "2".into(),
                in_col: "3".into(),
                in_collection: String::new(),
                output_ref: "ref".into(),
                output_graph: "g".into(),
                output_row: "r".into(),
                output_col: "c".into(),
                output_collection: "col".into(),
                subactions: vec![],
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: Some(&waiter),
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
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
        let resolver = GridResolver { rows: 2, cols: 2 };
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateSelect {
                program: "P".into(),
                graph_name: "bag".into(),
                chord_up: vec![],
                chord_down: vec![],
                chord_left: vec![],
                chord_right: vec![],
                chord_select: vec![],
                chord_back: vec![],
                wrap_edges: false,
                move_cursor_with_nav: false,
                smooth: false,
                pass_through: false,
                hold_repeat: false,
                select_device: String::new(),
                select_button: String::new(),
                select_key: String::new(),
                select_press_mode: String::new(),
                in_graph: String::new(),
                in_row: String::new(),
                in_col: String::new(),
                in_collection: String::new(),
                output_ref: String::new(),
                output_graph: String::new(),
                output_row: "r".into(),
                output_col: "c".into(),
                output_collection: String::new(),
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::NavigateKey {
                        name: "Inspect".into(),
                        chord: vec!["i".into()],
                        exit: true,
                        subactions: vec![Action {
                            id: ActionId::new(),
                            kind: ActionKind::Click {
                                button: "right".into(),
                                state: true,
                            },
                        }],
                    },
                }],
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: Some(&waiter),
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert!(backend
            .log
            .iter()
            .any(|e| e == "click:right:down"));
        assert_eq!(
            macro_.variables.get("r").map(|v| v.as_display()),
            Some("1".into())
        );
    }
}
