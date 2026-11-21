use crate::actions::{
    execute_focus_window, execute_for_each_cell, execute_for_each_row, execute_pause,
    execute_run_macro, execute_save_variable, execute_set_variable, execute_while, FlowLoopCtx,
};
use crate::backends::{AutomationBackend, MoveOptions};
use crate::error::{ExecError, FlowSignal, Result};
use crate::navigate::{execute_navigate_key, execute_navigate_select};
use crate::search::{execute_find_pixel, execute_image_search, execute_ocr};
use sqyre_domain::{
    action_type_label, Action, ActionId, ActionKind, LoopJumpMode, Macro, MouseButton, PressState,
    ScalarValue,
};
use sqyre_ports::{clear_highlights, highlight_cursor};
use std::collections::BTreeSet;
use std::time::Instant;

pub use crate::ctx::{ExecDeps, Executor};
pub(crate) use crate::resolve::{eval_clauses, resolve_int, resolve_text};

/// Run a macro from a clean runtime variable store.
pub fn execute_macro(macro_: &mut Macro, automation: &mut dyn AutomationBackend) -> Result<()> {
    execute_macro_with(macro_, ExecDeps::new(automation))
}

pub fn execute_macro_with(macro_: &mut Macro, deps: ExecDeps<'_>) -> Result<()> {
    let macro_started = Instant::now();
    let mut exec = Executor {
        deps,
        stop_requested: false,
        held_keys: BTreeSet::new(),
        held_buttons: BTreeSet::new(),
        run_macro_stack: vec![macro_.name.clone()],
        published_vars_revision: None,
        capture_dirty: true,
        previous_focus: None,
        last_pointer: None,
    };
    macro_.init_runtime_variables();
    let monitor_sizes = match exec.deps.capturer.as_mut() {
        Some(c) => c.monitor_sizes().unwrap_or_else(|_| vec![(0, 0)]),
        None => vec![(0, 0)],
    };
    apply_monitor_sizes(macro_, &monitor_sizes);
    exec.publish_runtime_vars(macro_);
    let root = macro_.root.clone();
    let root_id = root.id;
    let result = match execute_action(&mut exec, &root, macro_) {
        Err(ExecError::Flow(FlowSignal::Stopped)) => Ok(()),
        other => other,
    };
    exec.log_timing(root_id, "macro total", macro_started.elapsed());
    clear_highlights(exec.deps.highlighter);
    if exec.deps.release_held_inputs {
        exec.release_held_inputs();
    }
    result
}

/// Set `monitorNWidth` / `monitorNHeight` builtins.
pub(crate) fn apply_monitor_sizes(macro_: &mut Macro, sizes: &[(i32, i32)]) {
    if sizes.is_empty() {
        macro_.variables.set("monitor1Width", ScalarValue::Int(0));
        macro_.variables.set("monitor1Height", ScalarValue::Int(0));
        return;
    }
    for (i, (w, h)) in sizes.iter().enumerate() {
        let n = i + 1;
        macro_
            .variables
            .set(format!("monitor{n}Width"), ScalarValue::Int(i64::from(*w)));
        macro_
            .variables
            .set(format!("monitor{n}Height"), ScalarValue::Int(i64::from(*h)));
    }
}

fn action_headline(action: &Action) -> String {
    format!(
        "{}: {}",
        action_type_label(action.type_key()),
        action.display_name()
    )
}

pub fn execute_action(exec: &mut Executor<'_>, action: &Action, macro_: &mut Macro) -> Result<()> {
    exec.check_stopped()?;
    // Skip root loop — no cursor on macro root.
    if !action.id.is_root() {
        highlight_cursor(exec.deps.highlighter, &macro_.name, Some(action.id));
    }
    exec.log_with(action.id, || action_headline(action));
    let action_started = Instant::now();
    let dispatch_started = Instant::now();
    let result = dispatch(exec, action, macro_);
    exec.log_timing(action.id, "dispatch", dispatch_started.elapsed());
    // Delay only on success or Break/Continue (not Stopped/errors).
    let delay_result = match &result {
        Ok(())
        | Err(ExecError::Flow(FlowSignal::Break))
        | Err(ExecError::Flow(FlowSignal::Continue)) => {
            exec.publish_runtime_vars(macro_);
            let delay_started = Instant::now();
            let delay_out = apply_delay(exec, action, macro_);
            exec.log_timing(action.id, "post-delay", delay_started.elapsed());
            delay_out
        }
        Err(_) => Ok(()),
    };
    exec.log_timing(action.id, "total", action_started.elapsed());
    match delay_result {
        Ok(()) => result,
        Err(e) => Err(e),
    }
}

fn apply_delay(exec: &mut Executor<'_>, action: &Action, macro_: &Macro) -> Result<()> {
    if macro_.global_delay > 0 {
        exec.interruptible_sleep(macro_.global_delay)?;
    }
    match sqyre_domain::action_delay_class(action.type_key()) {
        sqyre_domain::DelayClass::Keyboard if macro_.keyboard_delay > 0 => {
            exec.interruptible_sleep(macro_.keyboard_delay)?;
        }
        sqyre_domain::DelayClass::Mouse if macro_.mouse_delay > 0 => {
            exec.interruptible_sleep(macro_.mouse_delay)?;
        }
        _ => {}
    }
    Ok(())
}

fn dispatch(exec: &mut Executor<'_>, action: &Action, macro_: &mut Macro) -> Result<()> {
    match &action.kind {
        ActionKind::Wait { time } => {
            let ms = resolve_int(time, macro_)?;
            if ms > 0 {
                exec.interruptible_sleep(ms)?;
            }
            Ok(())
        }
        ActionKind::LoopJump { mode } => match mode {
            LoopJumpMode::Break => {
                exec.log(action.id, "Break");
                Err(FlowSignal::Break.into())
            }
            LoopJumpMode::Continue => {
                exec.log(action.id, "Continue");
                Err(FlowSignal::Continue.into())
            }
        },
        ActionKind::Loop {
            name,
            count,
            subactions,
        } => run_loop(exec, action.id, name, count, subactions, macro_),
        ActionKind::Conditional {
            condition,
            subactions,
            else_actions,
        } => {
            let ok = eval_clauses(condition.match_mode, &condition.clauses, macro_)?;
            if ok {
                exec.log(
                    action.id,
                    format!("Conditional {:?}: true, running branch", condition.name),
                );
                run_children(exec, subactions, macro_)
            } else if else_actions.is_empty() {
                exec.log(
                    action.id,
                    format!("Conditional {:?}: false, skipping branch", condition.name),
                );
                Ok(())
            } else {
                exec.log(
                    action.id,
                    format!("Conditional {:?}: false, running else", condition.name),
                );
                run_children(exec, else_actions, macro_)
            }
        }
        ActionKind::Click { button, state } => {
            if *button == MouseButton::Scroll {
                let up = matches!(*state, PressState::Up);
                exec.mark_capture_dirty();
                exec.deps.automation.scroll(up).map_err(ExecError::from)
            } else {
                match *state {
                    PressState::Down => exec.input_click_down(button.as_str()),
                    PressState::Up => exec.input_click_up(button.as_str()),
                    PressState::Tap => exec.input_click_tap(button.as_str(), macro_.mouse_delay),
                }
            }
        }
        ActionKind::Key { key, state } => match *state {
            PressState::Down => exec.input_key_down(key),
            PressState::Up => exec.input_key_up(key),
            PressState::Tap => {
                exec.input_key_down(key)?;
                exec.input_key_up(key)
            }
        },
        ActionKind::Type { text, delay_ms } => {
            let resolved = resolve_text(text, macro_)?;
            for ch in resolved.chars() {
                exec.check_stopped()?;
                exec.mark_capture_dirty();
                exec.deps.automation.type_char(ch)?;
                if *delay_ms > 0 {
                    exec.interruptible_sleep(*delay_ms)?;
                }
            }
            Ok(())
        }
        ActionKind::Move {
            point,
            smooth,
            smooth_low,
            smooth_high,
            smooth_delay_ms,
        } => {
            let (x, y) = if let Some(resolver) = exec.deps.resolver {
                resolver.resolve_point(point, macro_).map_err(|e| {
                    ExecError::Message(format!(
                        "Move: failed to resolve point {}: {e}",
                        point.as_str()
                    ))
                })?
            } else {
                return Err(ExecError::Message(
                    "Move: coordinate resolver not configured".into(),
                ));
            };
            exec.log(action.id, format!("Move → ({x}, {y}) [{}]", point.as_str()));
            exec.mark_capture_dirty();
            exec.move_pointer_to(
                x,
                y,
                MoveOptions {
                    smooth: *smooth,
                    low: *smooth_low,
                    high: *smooth_high,
                    delay_ms: *smooth_delay_ms,
                },
            );
            Ok(())
        }
        ActionKind::SetVariable { assignments } => {
            execute_set_variable(exec, action.id, assignments, macro_)
        }
        ActionKind::ImageSearch(_) => execute_image_search(exec, action, macro_),
        ActionKind::FindPixel(_) => execute_find_pixel(exec, action, macro_),
        ActionKind::FocusWindow {
            process_path,
            window_title,
            previous,
        } => execute_focus_window(exec, action.id, process_path, window_title, *previous),
        ActionKind::SaveVariable {
            variable_name,
            destination,
            append,
            append_newline,
        } => execute_save_variable(
            exec,
            action.id,
            variable_name,
            destination,
            *append,
            *append_newline,
            macro_,
        ),
        ActionKind::While {
            condition,
            max_iterations,
            subactions,
        } => execute_while(
            exec,
            &FlowLoopCtx {
                action_id: action.id,
                name: &condition.name,
                subactions,
            },
            condition.match_mode,
            &condition.clauses,
            *max_iterations,
            macro_,
            run_children,
        ),
        ActionKind::ForEachRow {
            name,
            sources,
            start_row,
            end_row,
            subactions,
        } => execute_for_each_row(
            exec,
            &FlowLoopCtx {
                action_id: action.id,
                name,
                subactions,
            },
            sources,
            start_row,
            end_row,
            macro_,
            run_children,
        ),
        ActionKind::ForEachCell {
            name,
            cells,
            subactions,
        } => execute_for_each_cell(
            exec,
            &FlowLoopCtx {
                action_id: action.id,
                name,
                subactions,
            },
            cells,
            macro_,
            run_children,
        ),
        ActionKind::Pause {
            message,
            continue_key,
            pass_through,
        } => execute_pause(
            exec,
            action.id,
            message,
            continue_key,
            *pass_through,
            macro_,
        ),
        ActionKind::RunMacro { macro_name } => {
            execute_run_macro(exec, action.id, macro_name, macro_)
        }
        ActionKind::Ocr(_) => execute_ocr(exec, action, macro_),
        ActionKind::NavigateSelect(_) => execute_navigate_select(exec, action, macro_),
        ActionKind::NavigateKey { .. } => execute_navigate_key(exec, action, macro_),
    }
}

fn run_loop(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    name: &str,
    count: &ScalarValue,
    subactions: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    let n = resolve_int(count, macro_)?.max(0);
    for i in 0..n {
        exec.check_stopped()?;
        exec.log(action_id, format!("Loop: {name} iteration {}", i + 1));
        match run_children(exec, subactions, macro_) {
            Err(ExecError::Flow(FlowSignal::Break)) => break,
            Err(ExecError::Flow(FlowSignal::Continue)) => continue,
            Err(e) => return Err(e),
            Ok(()) => {}
        }
    }
    Ok(())
}

pub(crate) fn run_children(
    exec: &mut Executor<'_>,
    children: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    for child in children {
        execute_action(exec, child, macro_)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::DesktopRect;
    use crate::lines_for;
    use crate::test_support::{fast_macro_of, macro_of, FixedResolver};
    use crate::test_support::{RecordingBackend, RecordingCapturer};
    use sqyre_domain::{
        root_loop, Action, ActionId, ActionKind, ConditionOperator, CoordinateRef, ScalarValue,
        VariableAssignment,
    };

    const RUN_RESOLVER: FixedResolver = FixedResolver::point_area((42, 99), (0, 0, 10, 10));

    #[test]
    fn executes_wait_loop_break() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = macro_of(vec![
            Action::new(ActionKind::Wait {
                time: ScalarValue::Int(10),
            }),
            Action::new(ActionKind::Loop {
                name: "inner".into(),
                count: ScalarValue::Int(3),
                subactions: vec![
                    Action::new(ActionKind::Wait {
                        time: ScalarValue::Int(1),
                    }),
                    Action::new(ActionKind::LoopJump {
                        mode: LoopJumpMode::Break,
                    }),
                ],
            }),
        ]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        backend.assert_logged("sleep:10");
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:1")
                .count(),
            1
        );
    }

    #[test]
    fn apply_monitor_builtins_from_capturer() {
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
            },
            ..Default::default()
        };
        let mut macro_ = macro_of(vec![]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend).capturer(&mut capturer),
        )
        .unwrap();
        assert_eq!(
            macro_.variables.get("monitor1Width"),
            Some(&ScalarValue::Int(1920))
        );
        assert_eq!(
            macro_.variables.get("monitor1Height"),
            Some(&ScalarValue::Int(1080))
        );
    }

    #[test]
    fn wait_aborts_on_stop_flag() {
        use std::sync::atomic::AtomicBool;
        let mut backend = RecordingBackend::default();
        let stop = AtomicBool::new(true);
        let mut macro_ = Macro::new("t", 0, vec![]);
        let wait_id = ActionId::new();
        macro_.root = root_loop(vec![Action {
            id: wait_id,
            kind: ActionKind::Wait {
                time: ScalarValue::Int(5000),
            },
        }]);
        // Top-level execute_macro maps Stopped → Ok; assert we did not sleep 5s.
        execute_macro_with(&mut macro_, ExecDeps::new(&mut backend).stop_flag(&stop)).unwrap();
        let slept: i32 = backend
            .log
            .iter()
            .filter_map(|e| e.strip_prefix("sleep:")?.parse::<i32>().ok())
            .sum();
        assert!(
            slept < 5000,
            "interruptible wait must abort early, slept {slept}ms: {:?}",
            backend.log
        );
    }

    #[test]
    fn stopped_action_skips_global_delay() {
        use std::sync::atomic::AtomicBool;
        let mut backend = RecordingBackend::default();
        let stop = AtomicBool::new(true);
        let mut macro_ = Macro::new("t", 250, vec![]);
        macro_.root = root_loop(vec![Action::new(ActionKind::Wait {
            time: ScalarValue::Int(10),
        })]);
        execute_macro_with(&mut macro_, ExecDeps::new(&mut backend).stop_flag(&stop)).unwrap();
        backend.assert_not_logged("sleep:250");
    }

    #[test]
    fn interruptible_sleep_returns_stopped() {
        use std::sync::atomic::AtomicBool;
        let mut backend = RecordingBackend::default();
        let stop = AtomicBool::new(true);
        let mut exec = Executor {
            deps: ExecDeps {
                stop_flag: Some(&stop),
                ..ExecDeps::new(&mut backend)
            },
            stop_requested: false,
            held_keys: BTreeSet::new(),
            held_buttons: BTreeSet::new(),
            run_macro_stack: Vec::new(),
            published_vars_revision: None,
            capture_dirty: true,
            previous_focus: None,
            last_pointer: None,
        };
        let err = exec.interruptible_sleep(1000).unwrap_err();
        assert!(matches!(err, ExecError::Flow(FlowSignal::Stopped)));
    }

    #[test]
    fn releases_held_keys_and_buttons_when_macro_ends() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = fast_macro_of(vec![
            Action::new(ActionKind::Key {
                key: "shift".into(),
                state: PressState::Down,
            }),
            Action::new(ActionKind::Click {
                button: "left".into(),
                state: PressState::Down,
            }),
        ]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            backend.log,
            vec![
                "keydown:shift".to_string(),
                "click:left:down".to_string(),
                "keyup:shift".to_string(),
                "click:left:up".to_string(),
            ]
        );
    }

    #[test]
    fn does_not_double_release_balanced_presses() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = fast_macro_of(vec![
            Action::new(ActionKind::Key {
                key: "a".into(),
                state: PressState::Down,
            }),
            Action::new(ActionKind::Key {
                key: "a".into(),
                state: PressState::Up,
            }),
        ]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "keyup:a")
                .count(),
            1
        );
    }

    #[test]
    fn skips_held_input_release_when_disabled() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = fast_macro_of(vec![
            Action::new(ActionKind::Key {
                key: "shift".into(),
                state: PressState::Down,
            }),
            Action::new(ActionKind::Click {
                button: "left".into(),
                state: PressState::Down,
            }),
        ]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend).release_held_inputs(false),
        )
        .unwrap();
        assert_eq!(
            backend.log,
            vec!["keydown:shift".to_string(), "click:left:down".to_string(),]
        );
    }

    #[test]
    fn move_uses_coordinate_resolver() {
        let mut backend = RecordingBackend::default();
        let resolver = RUN_RESOLVER;
        let mut macro_ = macro_of(vec![
            Action::new(ActionKind::Move {
                point: CoordinateRef("Prog~Spot".into()),
                smooth: false,
                smooth_low: 0.05,
                smooth_high: 0.2,
                smooth_delay_ms: 1,
            }),
            Action::new(ActionKind::Click {
                button: "left".into(),
                state: PressState::Down,
            }),
            Action::new(ActionKind::Click {
                button: "left".into(),
                state: PressState::Up,
            }),
        ]);
        execute_macro_with(&mut macro_, ExecDeps::new(&mut backend).resolver(&resolver)).unwrap();
        backend.assert_logged("move:42,99,smooth=false");
        backend.assert_logged("click:left:down");
    }

    #[test]
    fn click_reports_pointer_target_to_sink_once_per_press() {
        let mut backend = RecordingBackend::default();
        let resolver = RUN_RESOLVER;
        let fx = sqyre_ports::SharedClickFx::new();
        let hits = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let hits2 = std::sync::Arc::clone(&hits);
        fx.set_on_click(Some(move |x, y| hits2.lock().unwrap().push((x, y))));
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![
            Action::new(ActionKind::Move {
                point: CoordinateRef("Prog~Spot".into()),
                smooth: false,
                smooth_low: 0.05,
                smooth_high: 0.2,
                smooth_delay_ms: 1,
            }),
            Action::new(ActionKind::Click {
                button: "left".into(),
                state: PressState::Tap,
            }),
        ]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .resolver(&resolver)
                .click_fx(&fx),
        )
        .unwrap();
        assert_eq!(*hits.lock().unwrap(), vec![(42, 99)]);
    }

    #[test]
    fn click_without_move_uses_backend_cursor_position() {
        let mut backend = RecordingBackend {
            pos: Some((5, 6)),
            ..RecordingBackend::default()
        };
        let fx = sqyre_ports::SharedClickFx::new();
        let hits = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let hits2 = std::sync::Arc::clone(&hits);
        fx.set_on_click(Some(move |x, y| hits2.lock().unwrap().push((x, y))));
        let mut macro_ = macro_of(vec![Action::new(ActionKind::Click {
            button: "left".into(),
            state: PressState::Down,
        })]);
        execute_macro_with(&mut macro_, ExecDeps::new(&mut backend).click_fx(&fx)).unwrap();
        assert_eq!(*hits.lock().unwrap(), vec![(5, 6)]);
    }

    #[test]
    fn tap_click_and_key_press_down_then_up() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = fast_macro_of(vec![
            Action::new(ActionKind::Click {
                button: "left".into(),
                state: PressState::Tap,
            }),
            Action::new(ActionKind::Key {
                key: "a".into(),
                state: PressState::Tap,
            }),
        ]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        let click_down = backend
            .log
            .iter()
            .filter(|e| e.as_str() == "click:left:down")
            .count();
        let click_up = backend
            .log
            .iter()
            .filter(|e| e.as_str() == "click:left:up")
            .count();
        assert_eq!(click_down, 1);
        assert_eq!(click_up, 1);
        backend.assert_logged("keydown:a");
        backend.assert_logged("keyup:a");
    }

    #[test]
    fn action_logger_tags_wait_and_click() {
        let mut backend = RecordingBackend::default();
        let logger = crate::SharedActionLog::new();
        let wait_id = ActionId::new();
        let click_id = ActionId::new();
        let mut macro_ = macro_of(vec![
            Action {
                id: wait_id,
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(10),
                },
            },
            Action {
                id: click_id,
                kind: ActionKind::Click {
                    button: "left".into(),
                    state: PressState::Down,
                },
            },
        ]);
        execute_macro_with(&mut macro_, ExecDeps::new(&mut backend).logger(&logger)).unwrap();
        let wait_lines = lines_for(&logger.entries_for(wait_id));
        let click_lines = lines_for(&logger.entries_for(click_id));
        assert!(
            wait_lines.iter().any(|l| l.starts_with("Wait:")),
            "wait lines: {wait_lines:?}"
        );
        assert!(
            click_lines.iter().any(|l| l.starts_with("Click:")),
            "click lines: {click_lines:?}"
        );
        assert!(
            wait_lines.iter().any(|l| l.starts_with("timing: total ")),
            "wait timing: {wait_lines:?}"
        );
        assert!(
            click_lines.iter().any(|l| l.starts_with("timing: total ")),
            "click timing: {click_lines:?}"
        );
        let root_lines = lines_for(&logger.entries_for(macro_.root.id));
        assert!(
            root_lines
                .iter()
                .any(|l| l.starts_with("timing: macro total ")),
            "macro timing: {root_lines:?}"
        );
        assert!(!wait_lines.is_empty());
        assert!(!click_lines.is_empty());
    }

    #[test]
    fn conditional_runs_branch_when_true_skips_when_false() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variable_decls.push(sqyre_domain::VariableDecl {
            name: "flag".into(),
            type_: sqyre_domain::VariableType::Text,
            initial_value: "yes".into(),
            description: String::new(),
        });
        macro_.root = root_loop(vec![
            Action::new(ActionKind::Conditional {
                condition: sqyre_domain::ConditionBlock {
                    name: "ok".into(),
                    match_mode: "all".into(),
                    clauses: vec![sqyre_domain::ConditionClause {
                        left: ScalarValue::String("${flag}".into()),
                        operator: ConditionOperator::Equals,
                        right: ScalarValue::String("yes".into()),
                    }],
                },
                subactions: vec![Action::new(ActionKind::Wait {
                    time: ScalarValue::Int(7),
                })],
                else_actions: Vec::new(),
            }),
            Action::new(ActionKind::Conditional {
                condition: sqyre_domain::ConditionBlock {
                    name: "no".into(),
                    match_mode: "all".into(),
                    clauses: vec![sqyre_domain::ConditionClause {
                        left: ScalarValue::String("${flag}".into()),
                        operator: ConditionOperator::Equals,
                        right: ScalarValue::String("no".into()),
                    }],
                },
                subactions: vec![Action::new(ActionKind::Wait {
                    time: ScalarValue::Int(99),
                })],
                else_actions: vec![Action::new(ActionKind::Wait {
                    time: ScalarValue::Int(5),
                })],
            }),
        ]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        backend.assert_logged("sleep:7");
        backend.assert_not_logged("sleep:99");
        backend.assert_logged("sleep:5");
    }

    #[test]
    fn conditional_any_mode_and_operators() {
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_
            .variables
            .set("name", ScalarValue::String("hello".into()));
        macro_
            .variables
            .set("empty", ScalarValue::String("".into()));
        assert!(eval_clauses(
            MatchMode::Any,
            &[
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("x".into()),
                    operator: ConditionOperator::Equals,
                    right: ScalarValue::String("y".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("${name}".into()),
                    operator: ConditionOperator::Contains,
                    right: ScalarValue::String("ell".into()),
                },
            ],
            &macro_
        )
        .unwrap());
        macro_
            .variables
            .set("item", ScalarValue::String("Mistfall Hunter".into()));
        assert!(eval_clauses(
            MatchMode::All,
            &[sqyre_domain::ConditionClause {
                left: ScalarValue::String("${item}".into()),
                operator: ConditionOperator::Equals,
                right: ScalarValue::String("Mistfall Hunter".into()),
            }],
            &macro_
        )
        .unwrap());
        assert!(eval_clauses(
            MatchMode::All,
            &[
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("${name}".into()),
                    operator: ConditionOperator::StartsWith,
                    right: ScalarValue::String("he".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("${name}".into()),
                    operator: ConditionOperator::EndsWith,
                    right: ScalarValue::String("lo".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("name".into()),
                    operator: ConditionOperator::IsSet,
                    right: ScalarValue::Null,
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("${empty}".into()),
                    operator: ConditionOperator::IsEmpty,
                    right: ScalarValue::Null,
                },
            ],
            &macro_
        )
        .unwrap());
        assert!(!eval_clauses(
            MatchMode::All,
            &[sqyre_domain::ConditionClause {
                left: ScalarValue::String("a".into()),
                operator: ConditionOperator::NotEquals,
                right: ScalarValue::String("a".into()),
            }],
            &macro_
        )
        .unwrap());
    }

    #[test]
    fn conditional_numeric_operators() {
        let macro_ = Macro::new("t", 0, vec![]);
        assert!(eval_clauses(
            MatchMode::All,
            &[
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("10".into()),
                    operator: ConditionOperator::GreaterThan,
                    right: ScalarValue::String("2".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("3".into()),
                    operator: ConditionOperator::LessOrEqual,
                    right: ScalarValue::String("3.0".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("1".into()),
                    operator: ConditionOperator::LessThan,
                    right: ScalarValue::String("2".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("5".into()),
                    operator: ConditionOperator::GreaterOrEqual,
                    right: ScalarValue::String("5".into()),
                },
            ],
            &macro_
        )
        .unwrap());
        assert!(!eval_clauses(
            MatchMode::All,
            &[sqyre_domain::ConditionClause {
                left: ScalarValue::String("1".into()),
                operator: ConditionOperator::GreaterThan,
                right: ScalarValue::String("2".into()),
            }],
            &macro_
        )
        .unwrap());
    }

    #[test]
    fn is_set_uses_variable_name_not_value() {
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_
            .variables
            .set("flag", ScalarValue::String("yes".into()));
        // `${flag}` must look up "flag", not the expanded value "yes".
        assert!(eval_clauses(
            MatchMode::All,
            &[sqyre_domain::ConditionClause {
                left: ScalarValue::String("${flag}".into()),
                operator: ConditionOperator::IsSet,
                right: ScalarValue::Null,
            }],
            &macro_
        )
        .unwrap());
        assert!(!eval_clauses(
            MatchMode::All,
            &[sqyre_domain::ConditionClause {
                left: ScalarValue::String("${missing}".into()),
                operator: ConditionOperator::IsSet,
                right: ScalarValue::Null,
            }],
            &macro_
        )
        .unwrap());
        // Bare name still works.
        assert!(eval_clauses(
            MatchMode::All,
            &[sqyre_domain::ConditionClause {
                left: ScalarValue::String("flag".into()),
                operator: ConditionOperator::IsSet,
                right: ScalarValue::Null,
            }],
            &macro_
        )
        .unwrap());
    }

    #[test]
    fn set_variable_resolves_refs_and_unresolved_errors() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variable_decls.push(sqyre_domain::VariableDecl {
            name: "base".into(),
            type_: sqyre_domain::VariableType::Text,
            initial_value: "world".into(),
            description: String::new(),
        });
        macro_.root = root_loop(vec![Action::new(ActionKind::SetVariable {
            assignments: vec![VariableAssignment::new(
                "msg",
                sqyre_domain::ScalarValue::String("hello ${base}".into()),
            )],
        })]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            macro_.variables.get("msg").map(|v| v.as_display()),
            Some("hello world".into())
        );

        macro_.root = root_loop(vec![Action::new(ActionKind::Type {
            text: "hi ${missing}".into(),
            delay_ms: 0,
        })]);
        let err = execute_macro(&mut macro_, &mut backend).unwrap_err();
        assert!(err.to_string().contains("unresolved"));
    }

    #[test]
    fn stop_flag_halts_mid_loop() {
        let mut backend = RecordingBackend::default();
        let stop = AtomicBool::new(false);
        let mut macro_ = macro_of(vec![Action::new(ActionKind::Loop {
            name: "many".into(),
            count: ScalarValue::Int(5),
            subactions: vec![Action::new(ActionKind::Wait {
                time: ScalarValue::Int(1),
            })],
        })]);
        // Latch stop after construction; executor polls each iteration.
        stop.store(true, Ordering::SeqCst);
        execute_macro_with(&mut macro_, ExecDeps::new(&mut backend).stop_flag(&stop)).unwrap();
        assert!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:1")
                .count()
                < 5
        );
    }

    #[test]
    fn applies_global_and_mouse_delays() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.global_delay = 3;
        macro_.mouse_delay = 5;
        macro_.keyboard_delay = 0;
        macro_.root = root_loop(vec![Action::new(ActionKind::Click {
            button: "left".into(),
            state: PressState::Down,
        })]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        backend.assert_logged("click:left:down");
        backend.assert_logged("sleep:3");
        backend.assert_logged("sleep:5");
    }

    #[test]
    fn loop_continue_skips_rest_of_iteration() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = fast_macro_of(vec![Action::new(ActionKind::Loop {
            name: "c".into(),
            count: ScalarValue::Int(2),
            subactions: vec![
                Action::new(ActionKind::LoopJump {
                    mode: LoopJumpMode::Continue,
                }),
                Action::new(ActionKind::Wait {
                    time: ScalarValue::Int(50),
                }),
            ],
        })]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        backend.assert_not_logged("sleep:50");
    }

    #[test]
    fn type_emits_chars_with_delay() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = fast_macro_of(vec![Action::new(ActionKind::Type {
            text: "ab".into(),
            delay_ms: 2,
        })]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.starts_with("type:"))
                .collect::<Vec<_>>(),
            vec!["type:a", "type:b"]
        );
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:2")
                .count(),
            2
        );
    }
}
