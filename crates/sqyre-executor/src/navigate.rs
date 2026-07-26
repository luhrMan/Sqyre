//! NavigateSelect / NavigateKey execution.

use crate::backends::{MoveOptions, PortError};
use crate::error::{ExecError, FlowSignal, Result};
use crate::run::{resolve_int, resolve_text, run_children, Executor};
use sqyre_domain::{
    Action, ActionId, ActionKind, AtlasLayout, AtlasNode, AtlasPos, CoordinateRef, Macro, NavDir,
    NavInputs, NavOptions, NavOutputs, NavSelectAction, NavigateSelectData, ScalarValue,
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

impl BuiltinChord {
    fn as_dir(self) -> Option<NavDir> {
        match self {
            Self::Up => Some(NavDir::Up),
            Self::Down => Some(NavDir::Down),
            Self::Left => Some(NavDir::Left),
            Self::Right => Some(NavDir::Right),
            Self::Select | Self::Back => None,
        }
    }
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

    let atlas_name = resolve_atlas_name(macro_, &data.atlas, &data.inputs)?;
    if atlas_name.is_empty() {
        return Err(ExecError::Message("navigate select: atlas not set".into()));
    }

    let resolver = exec.deps.resolver.ok_or_else(|| {
        ExecError::Message("navigate select: coordinate resolver not configured".into())
    })?;
    let members = resolver
        .atlas_members(&data.program, &atlas_name)
        .map_err(|e| {
            ExecError::Message(format!(
                "navigate select: atlas {}/{}: {e}",
                data.program, atlas_name
            ))
        })?;
    if members.is_empty() {
        return Err(ExecError::Message(format!(
            "navigate select: atlas {atlas_name:?} has no collections"
        )));
    }

    let mut nodes = Vec::with_capacity(members.len());
    for name in &members {
        let (rows, cols) = resolver.collection_grid(&data.program, name).map_err(|e| {
            ExecError::Message(format!(
                "navigate select: collection {}/{}: {e}",
                data.program, name
            ))
        })?;
        if rows < 1 || cols < 1 {
            return Err(ExecError::Message(format!(
                "navigate select: invalid grid {rows}x{cols} for {name}"
            )));
        }
        let cell = CoordinateRef::collection(&data.program, name, 1, 1, rows, cols);
        let bounds = resolver.resolve_search_area(&cell, macro_).map_err(|e| {
            ExecError::Message(format!("navigate select: resolve bounds for {name}: {e}"))
        })?;
        nodes.push(AtlasNode {
            collection: name.clone(),
            bounds,
            rows,
            cols,
        });
    }
    let layout = AtlasLayout::new(nodes);

    let start_collection = resolve_start_collection(macro_, &data.inputs)?;
    let start_node = if start_collection.is_empty() {
        0
    } else {
        layout.find_collection(&start_collection).ok_or_else(|| {
            ExecError::Message(format!(
                "navigate select: start collection {start_collection:?} is not in atlas {atlas_name:?}"
            ))
        })?
    };
    let start_rows = layout.nodes()[start_node].rows;
    let start_cols = layout.nodes()[start_node].cols;
    let mut pos = AtlasPos {
        node: start_node,
        row: resolve_cell_start(macro_, &data.inputs.row, 1)?.clamp(1, start_rows),
        col: resolve_cell_start(macro_, &data.inputs.col, 1)?.clamp(1, start_cols),
    };

    write_outputs(
        macro_,
        &data.program,
        &atlas_name,
        &layout,
        pos,
        &data.outputs,
    );

    if data.options.move_cursor_with_nav {
        move_to_cell(
            exec,
            macro_,
            &data.program,
            &layout,
            pos,
            data.options.smooth,
        )?;
    }

    let cur = &layout.nodes()[pos.node];
    exec.log(
        action.id,
        format!(
            "Navigate Select: {} · {atlas_name} / {} @ {},{} ({}x{})",
            data.program, cur.collection, pos.row, pos.col, cur.rows, cur.cols
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
                Err(PortError::Stopped) => return Err(FlowSignal::Stopped.into()),
                Err(e) => return Err(e.into()),
            }
        };
        if stop.load(Ordering::SeqCst) {
            return Err(FlowSignal::Stopped.into());
        }

        if let Some(b) = builtins.get(idx).copied().flatten() {
            match b {
                BuiltinChord::Up
                | BuiltinChord::Down
                | BuiltinChord::Left
                | BuiltinChord::Right => {
                    let dir = b.as_dir().expect("nav chord has direction");
                    pos = layout.step(pos, dir, data.options.wrap_edges);
                    on_nav(
                        exec,
                        action.id,
                        macro_,
                        &data.program,
                        &atlas_name,
                        &layout,
                        &mut pos,
                        &data.options,
                        &data.outputs,
                    )?;
                }
                BuiltinChord::Select => {
                    write_outputs(
                        macro_,
                        &data.program,
                        &atlas_name,
                        &layout,
                        pos,
                        &data.outputs,
                    );
                    perform_select(exec, &data.select)?;
                    let cur = &layout.nodes()[pos.node];
                    exec.log(
                        action.id,
                        format!(
                            "Navigate Select: select {} @ {},{}",
                            cur.collection, pos.row, pos.col
                        ),
                    );
                    return Ok(());
                }
                BuiltinChord::Back => {
                    write_outputs(
                        macro_,
                        &data.program,
                        &atlas_name,
                        &layout,
                        pos,
                        &data.outputs,
                    );
                    let cur = &layout.nodes()[pos.node];
                    exec.log(
                        action.id,
                        format!(
                            "Navigate Select: back {} @ {},{}",
                            cur.collection, pos.row, pos.col
                        ),
                    );
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
            write_outputs(
                macro_,
                &data.program,
                &atlas_name,
                &layout,
                pos,
                &data.outputs,
            );
            let label = if name.trim().is_empty() {
                "Nav Key".to_string()
            } else {
                name.clone()
            };
            let kids = kids.clone();
            let exit = *exit;
            let cur = &layout.nodes()[pos.node];
            exec.log(
                action.id,
                format!(
                    "Navigate Select: branch {label:?} {} @ {},{}",
                    cur.collection, pos.row, pos.col
                ),
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
    atlas: &str,
    layout: &AtlasLayout,
    pos: &mut AtlasPos,
    options: &NavOptions,
    outputs: &NavOutputs,
) -> Result<()> {
    write_outputs(macro_, program, atlas, layout, *pos, outputs);
    if options.move_cursor_with_nav {
        move_to_cell(exec, macro_, program, layout, *pos, options.smooth)?;
    }
    let cur = &layout.nodes()[pos.node];
    exec.log(
        action_id,
        format!(
            "Navigate Select: cell {} @ {},{}",
            cur.collection, pos.row, pos.col
        ),
    );
    Ok(())
}

fn resolve_atlas_name(macro_: &Macro, atlas: &str, inputs: &NavInputs) -> Result<String> {
    if !inputs.atlas.trim().is_empty() {
        if let Some(v) = macro_.variables.get(inputs.atlas.trim()) {
            let s = v.as_display();
            if !s.trim().is_empty() {
                return Ok(s);
            }
        }
        let resolved = resolve_text(&inputs.atlas, macro_).unwrap_or_else(|_| inputs.atlas.clone());
        if !resolved.trim().is_empty() && resolved != inputs.atlas {
            return Ok(resolved);
        }
        if !inputs.atlas.trim().is_empty() {
            return Ok(inputs.atlas.trim().to_string());
        }
    }
    Ok(atlas.trim().to_string())
}

fn resolve_start_collection(macro_: &Macro, inputs: &NavInputs) -> Result<String> {
    let t = inputs.collection.trim();
    if t.is_empty() {
        return Ok(String::new());
    }
    if let Some(v) = macro_.variables.get(t) {
        let s = v.as_display();
        if !s.trim().is_empty() {
            return Ok(s);
        }
    }
    let resolved = resolve_text(t, macro_).unwrap_or_else(|_| t.to_string());
    Ok(resolved.trim().to_string())
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
    atlas: &str,
    layout: &AtlasLayout,
    pos: AtlasPos,
    outputs: &NavOutputs,
) {
    let Some(node) = layout.nodes().get(pos.node) else {
        return;
    };
    let cell = CoordinateRef::collection(
        program,
        &node.collection,
        pos.row,
        pos.col,
        pos.row,
        pos.col,
    );
    if !outputs.output_ref.trim().is_empty() {
        macro_.variables.set(
            outputs.output_ref.trim(),
            ScalarValue::String(cell.0.clone()),
        );
    }
    if !outputs.output_atlas.trim().is_empty() {
        macro_.variables.set(
            outputs.output_atlas.trim(),
            ScalarValue::String(atlas.to_string()),
        );
    }
    if !outputs.output_collection.trim().is_empty() {
        macro_.variables.set(
            outputs.output_collection.trim(),
            ScalarValue::String(node.collection.clone()),
        );
    }
    if !outputs.output_row.trim().is_empty() {
        macro_
            .variables
            .set(outputs.output_row.trim(), ScalarValue::Int(pos.row as i64));
    }
    if !outputs.output_col.trim().is_empty() {
        macro_
            .variables
            .set(outputs.output_col.trim(), ScalarValue::Int(pos.col as i64));
    }
}

fn move_to_cell(
    exec: &mut Executor<'_>,
    macro_: &Macro,
    program: &str,
    layout: &AtlasLayout,
    pos: AtlasPos,
    smooth: bool,
) -> Result<()> {
    let node = layout
        .nodes()
        .get(pos.node)
        .ok_or_else(|| ExecError::Message("navigate select: invalid atlas position".into()))?;
    let resolver = exec.deps.resolver.ok_or_else(|| {
        ExecError::Message("navigate select: coordinate resolver not configured".into())
    })?;
    let cell = CoordinateRef::collection(
        program,
        &node.collection,
        pos.row,
        pos.col,
        pos.row,
        pos.col,
    );
    let (x, y) = resolver.resolve_point(&cell, macro_).map_err(|e| {
        ExecError::Message(format!(
            "navigate select: resolve cell {} @ {},{}: {e}",
            node.collection, pos.row, pos.col
        ))
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
                "down" | "hold" => exec.input_click_down(btn)?,
                "up" => exec.input_click_up(btn)?,
                _ => {
                    exec.input_click_down(btn)?;
                    exec.input_click_up(btn)?;
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
                "down" | "hold" => exec.input_key_down(k)?,
                "up" => exec.input_key_up(k)?,
                _ => {
                    exec.input_key_down(k)?;
                    exec.input_key_up(k)?;
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
    use crate::test_support::{AtlasMemberSpec, FixedCollection, FixedResolver};
    use sqyre_domain::{
        root_loop, ActionId, NavChords, NavInputs, NavOptions, NavOutputs, NavSelectAction,
        NavigateSelectData, PressState,
    };
    use std::sync::Mutex;

    #[test]
    fn select_exits_and_writes_outputs() {
        let mut backend = RecordingBackend::default();
        let waiter = ImmediateContinueWaiter {
            any_queue: Mutex::new(vec![0]), // select is only chord
            ..Default::default()
        };
        let resolver = FixedResolver::with_atlas(
            vec![AtlasMemberSpec {
                name: "bag".into(),
                collection: FixedCollection {
                    rows: 3,
                    cols: 4,
                    bounds: (0, 0, 120, 90),
                },
            }],
            vec!["bag".into()],
        );
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateSelect(Box::new(NavigateSelectData {
                program: "P".into(),
                atlas: "inventory".into(),
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
                    output_atlas: "a".into(),
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
                release_held_inputs: true,
                while_max_iterations: crate::run::DEFAULT_WHILE_MAX_ITERATIONS,
                run_macro_max_depth: crate::run::DEFAULT_RUN_MACRO_MAX_DEPTH,
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
            macro_.variables.get("a").map(|v| v.as_display()),
            Some("inventory".into())
        );
        assert_eq!(
            macro_.variables.get("col").map(|v| v.as_display()),
            Some("bag".into())
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
        let resolver = FixedResolver::with_atlas(
            vec![AtlasMemberSpec {
                name: "bag".into(),
                collection: FixedCollection {
                    rows: 2,
                    cols: 2,
                    bounds: (0, 0, 100, 100),
                },
            }],
            vec!["bag".into()],
        );
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateSelect(Box::new(NavigateSelectData {
                program: "P".into(),
                atlas: "inventory".into(),
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
                                state: PressState::Down,
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
                release_held_inputs: true,
                while_max_iterations: crate::run::DEFAULT_WHILE_MAX_ITERATIONS,
                run_macro_max_depth: crate::run::DEFAULT_RUN_MACRO_MAX_DEPTH,
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

    #[test]
    fn walks_right_into_neighbor_collection() {
        let mut backend = RecordingBackend::default();
        // chords: up, down, left, right, select — index 3 = right, then select
        let waiter = ImmediateContinueWaiter {
            any_queue: Mutex::new(vec![3, 4]),
            ..Default::default()
        };
        let resolver = FixedResolver::with_atlas(
            vec![
                AtlasMemberSpec {
                    name: "A".into(),
                    collection: FixedCollection {
                        rows: 2,
                        cols: 2,
                        bounds: (0, 0, 100, 100),
                    },
                },
                AtlasMemberSpec {
                    name: "B".into(),
                    collection: FixedCollection {
                        rows: 2,
                        cols: 2,
                        bounds: (120, 0, 220, 100),
                    },
                },
            ],
            vec!["A".into(), "B".into()],
        );
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateSelect(Box::new(NavigateSelectData {
                program: "P".into(),
                atlas: "inv".into(),
                chords: NavChords {
                    up: vec!["up".into()],
                    down: vec!["down".into()],
                    left: vec!["left".into()],
                    right: vec!["right".into()],
                    select: vec!["enter".into()],
                    ..Default::default()
                },
                options: NavOptions {
                    wrap_edges: false,
                    move_cursor_with_nav: false,
                    ..Default::default()
                },
                inputs: NavInputs {
                    collection: "A".into(),
                    row: "1".into(),
                    col: "2".into(),
                    ..Default::default()
                },
                outputs: NavOutputs {
                    output_collection: "col".into(),
                    output_row: "r".into(),
                    output_col: "c".into(),
                    ..Default::default()
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
                release_held_inputs: true,
                while_max_iterations: crate::run::DEFAULT_WHILE_MAX_ITERATIONS,
                run_macro_max_depth: crate::run::DEFAULT_RUN_MACRO_MAX_DEPTH,
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
            macro_.variables.get("col").map(|v| v.as_display()),
            Some("B".into())
        );
        assert_eq!(
            macro_.variables.get("c").map(|v| v.as_display()),
            Some("1".into())
        );
    }
}
