//! Calculate, SaveVariable, While, ForEachRow, Pause, RunMacro, FocusWindow.

use crate::error::{ExecError, FlowSignal, Result};
use crate::expr::{evaluate_expression, numeric_to_scalar};
use crate::highlight::{highlight_clear, highlight_cursor, highlight_fill};
use crate::run::{
    eval_clauses, execute_action, resolve_int, resolve_text, run_children, Executor,
};
use sqyre_domain::{
    Action, ActionId, ActionKind, ConditionClause, ListColumn, Macro, ScalarValue,
    FOREACH_ROW_BUILTIN_ROW, FOREACH_ROW_BUILTIN_ROW_COUNT,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) fn execute_calculate(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    expression: &str,
    output_var: &str,
    macro_: &mut Macro,
) -> Result<()> {
    let result = evaluate_expression(expression, macro_)?;
    let scalar = numeric_to_scalar(result);
    exec.log(
        action_id,
        format!(
            "Calculate: {expression} → {output_var} = {}",
            scalar.as_display()
        ),
    );
    macro_.variables.set(output_var, scalar);
    Ok(())
}

/// Save a variable to clipboard or a file under `variables_dir`.
pub(crate) fn execute_save_variable(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    variable_name: &str,
    destination: &str,
    append: bool,
    append_newline: bool,
    macro_: &Macro,
) -> Result<()> {
    let val = macro_.variables.get(variable_name).ok_or_else(|| {
        ExecError::Message(format!("variable {variable_name} not found"))
    })?;
    let val_str = val.as_display();

    if destination == "clipboard" {
        exec.automation
            .write_clipboard(&val_str)
            .map_err(ExecError::Message)?;
        exec.log(
            action_id,
            format!("SaveVariable: {variable_name} → clipboard"),
        );
        return Ok(());
    }

    let base = exec.variables_dir.as_deref().ok_or_else(|| {
        ExecError::Message("save variable: variables directory not configured".into())
    })?;
    let file_path = if Path::new(destination).is_absolute() {
        PathBuf::from(destination)
    } else {
        base.join(destination)
    };
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            ExecError::Message(format!(
                "failed to create directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    save_to_file(&val_str, &file_path, append, append_newline)?;
    exec.log(
        action_id,
        format!(
            "SaveVariable: {variable_name} → {} ({})",
            file_path.display(),
            if append { "append" } else { "overwrite" }
        ),
    );
    Ok(())
}

fn save_to_file(value: &str, path: &Path, append: bool, append_newline: bool) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .map_err(|e| {
            ExecError::Message(format!(
                "failed to save variable to file {}: {e}",
                path.display()
            ))
        })?;
    file.write_all(value.as_bytes()).map_err(|e| {
        ExecError::Message(format!("failed to write {}: {e}", path.display()))
    })?;
    if append_newline {
        file.write_all(b"\n").map_err(|e| {
            ExecError::Message(format!(
                "failed to write newline {}: {e}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

pub(crate) fn execute_while(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    name: &str,
    match_mode: &str,
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
        if !eval_clauses(match_mode, clauses, macro_) {
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
        .map(|col| load_lines(col, exec.variables_dir.as_deref()))
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
                exec.highlighter,
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
                highlight_clear(exec.highlighter, &macro_.name, action_id);
                return Err(e);
            }
            Ok(()) => {}
        }
    }
    highlight_clear(exec.highlighter, &macro_.name, action_id);
    Ok(())
}

pub(crate) fn execute_focus_window(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    process_path: &str,
    window_title: &str,
) -> Result<()> {
    let path = process_path.trim();
    let title = window_title.trim();
    if path.is_empty() {
        return Err(ExecError::Message(
            "focus window: no executable path set".into(),
        ));
    }
    if title.is_empty() {
        return Err(ExecError::Message(
            "focus window: no window title set".into(),
        ));
    }
    let focuser = exec.window_focuser.ok_or_else(|| {
        ExecError::Message("focus window: window focuser not configured".into())
    })?;
    focuser.focus(path, title).map_err(|e| {
        ExecError::Message(format!("focus window {title:?} ({path}): {e}"))
    })?;
    exec.log(
        action_id,
        format!("Focus Window: {title} ({path})"),
    );
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

    let waiter = exec.continue_waiter.ok_or_else(|| {
        ExecError::Message("pause: continue key wait is not available in this build".into())
    })?;

    let dummy = AtomicBool::new(false);
    let stop = exec.stop_flag.unwrap_or(&dummy);
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
    let mut target = lookup.get(macro_name).ok_or_else(|| {
        ExecError::Message(format!("run macro: macro {macro_name:?} not found"))
    })?;
    if !matches!(target.root.kind, ActionKind::Loop { .. }) {
        return Err(ExecError::Message(format!(
            "run macro: macro {macro_name:?} has no root"
        )));
    }

    target.init_runtime_variables();
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

#[cfg(test)]
mod tests {
    use crate::backends::RecordingBackend;
    use crate::run::{execute_macro, execute_macro_with, ExecDeps};
    use sqyre_domain::{
        root_loop, Action, ActionId, ActionKind, ConditionClause, ListColumn, Macro, ScalarValue,
    };
    use std::fs;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn calculate_sets_output_var() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variable_decls.push(sqyre_domain::VariableDecl {
            name: "n".into(),
            type_: sqyre_domain::VariableType::Number,
            initial_value: "10".into(),
            description: String::new(),
        });
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Calculate {
                expression: "${n}*2+1".into(),
                output_var: "out".into(),
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            macro_.variables.get("out").map(|v| v.as_display()),
            Some("21".into())
        );
    }

    #[test]
    fn save_variable_clipboard_and_file() {
        let mut backend = RecordingBackend::default();
        let dir = tempfile::tempdir().unwrap();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variable_decls.push(sqyre_domain::VariableDecl {
            name: "msg".into(),
            type_: sqyre_domain::VariableType::Text,
            initial_value: "hello".into(),
            description: String::new(),
        });
        macro_.root = root_loop(vec![
            Action {
                id: ActionId::new(),
                kind: ActionKind::SaveVariable {
                    variable_name: "msg".into(),
                    destination: "clipboard".into(),
                    append: false,
                    append_newline: false,
                },
            },
            Action {
                id: ActionId::new(),
                kind: ActionKind::SaveVariable {
                    variable_name: "msg".into(),
                    destination: "out.txt".into(),
                    append: false,
                    append_newline: true,
                },
            },
        ]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                matcher: None,
                resolver: None,
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                variables_dir: Some(dir.path()),
            },
        )
        .unwrap();
        assert!(backend.log.iter().any(|e| e == "clipboard:hello"));
        assert_eq!(
            fs::read_to_string(dir.path().join("out.txt")).unwrap(),
            "hello\n"
        );
    }

    #[test]
    fn while_runs_until_condition_false() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variable_decls.push(sqyre_domain::VariableDecl {
            name: "i".into(),
            type_: sqyre_domain::VariableType::Number,
            initial_value: "0".into(),
            description: String::new(),
        });
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::While {
                name: "inc".into(),
                match_mode: "all".into(),
                clauses: vec![ConditionClause {
                    left: ScalarValue::String("${i}".into()),
                    operator: "!=".into(),
                    right: ScalarValue::String("3".into()),
                }],
                max_iterations: 10,
                subactions: vec![
                    Action {
                        id: ActionId::new(),
                        kind: ActionKind::Wait {
                            time: ScalarValue::Int(1),
                        },
                    },
                    Action {
                        id: ActionId::new(),
                        kind: ActionKind::Calculate {
                            expression: "${i}+1".into(),
                            output_var: "i".into(),
                        },
                    },
                ],
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:1")
                .count(),
            3
        );
        assert_eq!(
            macro_.variables.get("i").map(|v| v.as_display()),
            Some("3".into())
        );
    }

    #[test]
    fn for_each_row_sets_vars_and_respects_range() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::ForEachRow {
                name: "rows".into(),
                sources: vec![
                    ListColumn {
                        source: "a\nb\nc\nd".into(),
                        output_var: "letter".into(),
                        is_file: false,
                        skip_blank_lines: false,
                    },
                    ListColumn {
                        source: "1\n2\n3\n4".into(),
                        output_var: "digit".into(),
                        is_file: false,
                        skip_blank_lines: false,
                    },
                ],
                start_row: ScalarValue::Int(2),
                end_row: ScalarValue::Int(3),
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(1),
                    },
                }],
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:1")
                .count(),
            2
        );
        assert_eq!(
            macro_.variables.get("letter").map(|v| v.as_display()),
            Some("c".into())
        );
        assert_eq!(
            macro_.variables.get("digit").map(|v| v.as_display()),
            Some("3".into())
        );
        assert_eq!(
            macro_.variables.get("Row").map(|v| v.as_display()),
            Some("3".into())
        );
        assert_eq!(
            macro_.variables.get("RowCount").map(|v| v.as_display()),
            Some("4".into())
        );
    }

    #[test]
    fn for_each_row_continue_skips_rest() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::ForEachRow {
                name: "rows".into(),
                sources: vec![ListColumn {
                    source: "a\nb".into(),
                    output_var: "letter".into(),
                    is_file: false,
                    skip_blank_lines: false,
                }],
                start_row: ScalarValue::Null,
                end_row: ScalarValue::Null,
                subactions: vec![
                    Action {
                        id: ActionId::new(),
                        kind: ActionKind::Continue,
                    },
                    Action {
                        id: ActionId::new(),
                        kind: ActionKind::Wait {
                            time: ScalarValue::Int(50),
                        },
                    },
                ],
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert!(!backend.log.iter().any(|e| e == "sleep:50"));
    }

    #[test]
    fn while_respects_stop_flag() {
        let mut backend = RecordingBackend::default();
        let stop = AtomicBool::new(true);
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variables.set("i", ScalarValue::Int(0));
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::While {
                name: "forever".into(),
                match_mode: "all".into(),
                clauses: vec![],
                max_iterations: 100,
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(1),
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
                resolver: None,
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                stop_flag: Some(&stop),
                logger: None,
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();
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
    fn run_macro_executes_target_children() {
        use crate::backends::MapMacroLookup;
        use std::collections::BTreeMap;

        let mut helper = Macro::new("helper", 0, vec![]);
        helper.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(7),
            },
        }]);
        let mut lookup = MapMacroLookup::default();
        lookup.macros = BTreeMap::from([("helper".into(), helper)]);

        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("caller", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::RunMacro {
                macro_name: "helper".into(),
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                matcher: None,
                resolver: None,
                icons: None,
                macros: Some(&lookup),
                continue_waiter: None,
                window_focuser: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();
        assert!(backend.log.iter().any(|e| e == "sleep:7"));
    }

    #[test]
    fn pause_uses_continue_waiter() {
        use crate::backends::ImmediateContinueWaiter;

        let waiter = ImmediateContinueWaiter::default();
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Pause {
                message: "hold".into(),
                continue_key: vec!["f9".into()],
                pass_through: false,
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                matcher: None,
                resolver: None,
                icons: None,
                macros: None,
                continue_waiter: Some(&waiter),
                window_focuser: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();
        let log = waiter.log.lock().unwrap();
        assert!(log.iter().any(|e| e.contains("f9")));
    }

    #[test]
    fn focus_window_uses_focuser() {
        use crate::backends::RecordingWindowFocuser;

        let focuser = RecordingWindowFocuser::default();
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::FocusWindow {
                process_path: "/usr/bin/app".into(),
                window_title: "Title".into(),
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                matcher: None,
                resolver: None,
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: Some(&focuser),
                stop_flag: None,
                logger: None,
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();
        let log = focuser.log.lock().unwrap();
        assert_eq!(log.as_slice(), ["focus:/usr/bin/app:Title"]);
    }
}
