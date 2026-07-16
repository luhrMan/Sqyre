use crate::action_log::ActionLogger;
use crate::backends::{
    AutomationBackend, ContinueKeyWaiter, CoordinateResolver, IconStore, MacroLookup, MoveOptions,
    OcrEngine, ScreenCapturer, TemplateMatcher, WindowFocuser,
};
use crate::error::{ExecError, FlowSignal, Result};
use crate::highlight::{
    clear_highlights, highlight_cursor, ActionHighlighter,
};
use crate::misc::{
    execute_focus_window, execute_for_each_row, execute_pause, execute_run_macro,
    execute_save_variable, execute_set_variable, execute_while,
};
use crate::navigate::{execute_navigate_key, execute_navigate_select};
use crate::runtime_vars::RuntimeVarSink;
use crate::search::{execute_find_pixel, execute_image_search, execute_ocr};
use sqyre_domain::{action_type_label, Action, ActionId, ActionKind, Macro, ScalarValue};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Executor holding injected backends.
pub struct Executor<'a> {
    pub automation: &'a mut dyn AutomationBackend,
    pub capturer: Option<&'a mut dyn ScreenCapturer>,
    pub matcher: Option<&'a dyn TemplateMatcher>,
    pub resolver: Option<&'a dyn CoordinateResolver>,
    pub icons: Option<&'a dyn IconStore>,
    pub macros: Option<&'a dyn MacroLookup>,
    pub continue_waiter: Option<&'a dyn ContinueKeyWaiter>,
    pub window_focuser: Option<&'a dyn WindowFocuser>,
    pub ocr: Option<&'a dyn OcrEngine>,
    pub stop_requested: bool,
    /// Shared stop flag (Esc / UI Stop).
    pub stop_flag: Option<&'a AtomicBool>,
    /// Optional per-action run log.
    pub logger: Option<&'a dyn ActionLogger>,
    /// Optional active-action highlight sink.
    pub highlighter: Option<&'a dyn ActionHighlighter>,
    /// Optional live runtime-variable publisher for the UI.
    pub runtime_vars: Option<&'a dyn RuntimeVarSink>,
    /// `~/.sqyre/variables` (or override) for SaveVariable / ForEachRow file sources.
    pub variables_dir: Option<&'a Path>,
}

impl<'a> Executor<'a> {
    pub fn new(automation: &'a mut dyn AutomationBackend) -> Self {
        Self {
            automation,
            capturer: None,
            matcher: None,
            resolver: None,
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
                ocr: None,
            stop_requested: false,
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        }
    }

    pub fn check_stopped(&self) -> Result<()> {
        if self.stop_requested
            || self
                .stop_flag
                .is_some_and(|f| f.load(Ordering::SeqCst))
        {
            Err(FlowSignal::Stopped.into())
        } else {
            Ok(())
        }
    }

    /// Sleep in ≤50ms chunks, aborting with [`FlowSignal::Stopped`].
    pub fn interruptible_sleep(&mut self, ms: i32) -> Result<()> {
        let mut left = ms.max(0);
        while left > 0 {
            self.check_stopped()?;
            let chunk = left.min(50);
            self.automation.milli_sleep(chunk);
            left -= chunk;
        }
        self.check_stopped()
    }

    pub fn log(&self, action_id: ActionId, message: impl Into<String>) {
        if let Some(logger) = self.logger {
            logger.log(action_id, message.into());
        }
    }

    pub fn log_images_enabled(&self) -> bool {
        self.logger
            .map(|l| l.log_images_enabled())
            .unwrap_or(false)
    }

    pub fn log_image(
        &self,
        action_id: ActionId,
        label: impl Into<String>,
        image: &sqyre_match::ImageBuf,
    ) {
        if let Some(logger) = self.logger {
            logger.log_image(action_id, label.into(), image);
        }
    }

    pub fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: impl Into<String>,
        summary: impl Into<String>,
        thumbnail: &sqyre_match::ImageBuf,
        steps: &[(String, sqyre_match::ImageBuf)],
        details: Vec<String>,
    ) {
        if let Some(logger) = self.logger {
            logger.log_item_pipeline(
                action_id,
                title.into(),
                summary.into(),
                thumbnail,
                steps,
                details,
            );
        }
    }
}

/// Dependencies for a full macro run.
pub struct ExecDeps<'a> {
    pub automation: &'a mut dyn AutomationBackend,
    pub capturer: Option<&'a mut dyn ScreenCapturer>,
    pub matcher: Option<&'a dyn TemplateMatcher>,
    pub resolver: Option<&'a dyn CoordinateResolver>,
    pub icons: Option<&'a dyn IconStore>,
    pub macros: Option<&'a dyn MacroLookup>,
    pub continue_waiter: Option<&'a dyn ContinueKeyWaiter>,
    pub window_focuser: Option<&'a dyn WindowFocuser>,
    pub ocr: Option<&'a dyn OcrEngine>,
    pub stop_flag: Option<&'a AtomicBool>,
    pub logger: Option<&'a dyn ActionLogger>,
    pub highlighter: Option<&'a dyn ActionHighlighter>,
    pub runtime_vars: Option<&'a dyn RuntimeVarSink>,
    pub variables_dir: Option<&'a Path>,
}

/// Run a macro from a clean runtime variable store.
pub fn execute_macro(macro_: &mut Macro, automation: &mut dyn AutomationBackend) -> Result<()> {
    execute_macro_with(
        macro_,
        ExecDeps {
            automation,
            capturer: None,
            matcher: None,
            resolver: None,
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
                ocr: None,
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
}

pub fn execute_macro_with(macro_: &mut Macro, mut deps: ExecDeps<'_>) -> Result<()> {
    macro_.init_runtime_variables();
    let monitor_sizes = match deps.capturer.as_mut() {
        Some(c) => c.monitor_sizes().unwrap_or_else(|_| vec![(0, 0)]),
        None => vec![(0, 0)],
    };
    apply_monitor_sizes(macro_, &monitor_sizes);
    publish_runtime_vars(deps.runtime_vars, macro_);
    let mut exec = Executor {
        automation: deps.automation,
        capturer: deps.capturer,
        matcher: deps.matcher,
        resolver: deps.resolver,
        icons: deps.icons,
        macros: deps.macros,
        continue_waiter: deps.continue_waiter,
        window_focuser: deps.window_focuser,
        ocr: deps.ocr,
        stop_requested: false,
        stop_flag: deps.stop_flag,
        logger: deps.logger,
        highlighter: deps.highlighter,
        runtime_vars: deps.runtime_vars,
        variables_dir: deps.variables_dir,
    };
    let root = macro_.root.clone();
    let result = match execute_action(&mut exec, &root, macro_) {
        Err(ExecError::Flow(FlowSignal::Stopped)) => Ok(()),
        other => other,
    };
    clear_highlights(exec.highlighter);
    result
}

/// Set `monitorNWidth` / `monitorNHeight` builtins.
pub(crate) fn apply_monitor_sizes(macro_: &mut Macro, sizes: &[(i32, i32)]) {
    if sizes.is_empty() {
        macro_
            .variables
            .set("monitor1Width", ScalarValue::Int(0));
        macro_
            .variables
            .set("monitor1Height", ScalarValue::Int(0));
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
        highlight_cursor(exec.highlighter, &macro_.name, Some(action.id));
    }
    exec.log(action.id, action_headline(action));
    let result = dispatch(exec, action, macro_);
    // Delay only on success or Break/Continue (not Stopped/errors).
    match &result {
        Ok(())
        | Err(ExecError::Flow(FlowSignal::Break))
        | Err(ExecError::Flow(FlowSignal::Continue)) => {
            publish_runtime_vars(exec.runtime_vars, macro_);
            apply_delay(exec, action, macro_)?;
        }
        Err(_) => {}
    }
    result
}

fn publish_runtime_vars(sink: Option<&dyn RuntimeVarSink>, macro_: &Macro) {
    let Some(sink) = sink else {
        return;
    };
    let pairs: Vec<(String, String)> = macro_
        .variables
        .iter()
        .map(|(n, v)| (n.to_string(), v.as_display()))
        .collect();
    sink.publish(&pairs);
}

fn apply_delay(exec: &mut Executor<'_>, action: &Action, macro_: &Macro) -> Result<()> {
    if macro_.global_delay > 0 {
        exec.interruptible_sleep(macro_.global_delay)?;
    }
    match action.type_key() {
        "key" | "type" if macro_.keyboard_delay > 0 => {
            exec.interruptible_sleep(macro_.keyboard_delay)?;
        }
        "move" | "click" if macro_.mouse_delay > 0 => {
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
        ActionKind::Break => {
            exec.log(action.id, "Break");
            Err(FlowSignal::Break.into())
        }
        ActionKind::Continue => {
            exec.log(action.id, "Continue");
            Err(FlowSignal::Continue.into())
        }
        ActionKind::Loop {
            name,
            count,
            subactions,
        } => run_loop(exec, action.id, name, count, subactions, macro_),
        ActionKind::Conditional {
            name,
            match_mode,
            clauses,
            subactions,
        } => {
            let ok = eval_clauses(match_mode, clauses, macro_);
            if ok {
                exec.log(
                    action.id,
                    format!("Conditional {name:?}: true, running branch"),
                );
                run_children(exec, subactions, macro_)
            } else {
                exec.log(
                    action.id,
                    format!("Conditional {name:?}: false, skipping branch"),
                );
                Ok(())
            }
        }
        ActionKind::Click { button, state } => {
            if button == "scroll" {
                exec.automation.scroll(*state).map_err(ExecError::Message)
            } else {
                exec.automation
                    .click(button, *state)
                    .map_err(ExecError::Message)
            }
        }
        ActionKind::Key { key, state } => {
            if *state {
                exec.automation.key_down(key).map_err(ExecError::Message)
            } else {
                exec.automation.key_up(key).map_err(ExecError::Message)
            }
        }
        ActionKind::Type { text, delay_ms } => {
            let resolved = resolve_text(text, macro_)?;
            for ch in resolved.chars() {
                exec.check_stopped()?;
                exec.automation.type_char(&ch.to_string());
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
            let (x, y) = if let Some(resolver) = exec.resolver {
                match resolver.resolve_point(point, macro_) {
                    Ok(xy) => xy,
                    Err(e) => {
                        let msg = format!(
                            "Move: failed to resolve point {}: {e}, using (0,0)",
                            point.as_str()
                        );
                        eprintln!("{msg}");
                        exec.log(action.id, msg);
                        (0, 0)
                    }
                }
            } else {
                (0, 0)
            };
            exec.automation.move_to(
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
        ActionKind::SetVariable {
            variable_name,
            value,
        } => execute_set_variable(exec, action.id, variable_name, value, macro_),
        ActionKind::ImageSearch { .. } => execute_image_search(exec, action, macro_),
        ActionKind::FindPixel { .. } => execute_find_pixel(exec, action, macro_),
        ActionKind::FocusWindow {
            process_path,
            window_title,
        } => execute_focus_window(exec, action.id, process_path, window_title),
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
            name,
            match_mode,
            clauses,
            max_iterations,
            subactions,
        } => execute_while(
            exec,
            action.id,
            name,
            match_mode,
            clauses,
            *max_iterations,
            subactions,
            macro_,
        ),
        ActionKind::ForEachRow {
            name,
            sources,
            start_row,
            end_row,
            subactions,
        } => execute_for_each_row(
            exec,
            action.id,
            name,
            sources,
            start_row,
            end_row,
            subactions,
            macro_,
        ),
        ActionKind::Pause {
            message,
            continue_key,
            pass_through,
        } => execute_pause(exec, action.id, message, continue_key, *pass_through, macro_),
        ActionKind::RunMacro { macro_name } => {
            execute_run_macro(exec, action.id, macro_name, macro_)
        }
        ActionKind::Ocr { .. } => execute_ocr(exec, action, macro_),
        ActionKind::NavigateSelect { .. } => execute_navigate_select(exec, action, macro_),
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

pub(crate) fn eval_clauses(
    match_mode: &str,
    clauses: &[sqyre_domain::ConditionClause],
    macro_: &Macro,
) -> bool {
    if clauses.is_empty() {
        return true;
    }
    let results: Vec<bool> = clauses
        .iter()
        .map(|c| {
            let left = c.left.as_display();
            let right = c.right.as_display();
            let left = resolve_text(&left, macro_).unwrap_or(left);
            let right = resolve_text(&right, macro_).unwrap_or(right);
            match c.operator.as_str() {
                "==" => left == right,
                "!=" => left != right,
                "is set" => macro_.variables.get(&strip_ref(&left)).is_some(),
                "is empty" => left.trim().is_empty(),
                "contains" => left.contains(&right),
                "starts with" => left.starts_with(&right),
                "ends with" => left.ends_with(&right),
                _ => false,
            }
        })
        .collect();
    if match_mode == "any" {
        results.into_iter().any(|b| b)
    } else {
        results.into_iter().all(|b| b)
    }
}

fn strip_ref(s: &str) -> String {
    let t = s.trim();
    if let Some(inner) = t.strip_prefix("${").and_then(|x| x.strip_suffix('}')) {
        inner.to_string()
    } else {
        t.to_string()
    }
}

pub(crate) fn resolve_int(v: &ScalarValue, macro_: &Macro) -> Result<i32> {
    match v {
        ScalarValue::Int(i) => Ok(*i as i32),
        ScalarValue::Float(f) => Ok(*f as i32),
        ScalarValue::String(s) => {
            let resolved = resolve_text(s, macro_)?;
            resolved
                .trim()
                .parse()
                .map_err(|_| ExecError::Message(format!("cannot parse int from {resolved:?}")))
        }
        ScalarValue::Bool(b) => Ok(if *b { 1 } else { 0 }),
        ScalarValue::Null => Ok(0),
    }
}

pub(crate) fn resolve_text(text: &str, macro_: &Macro) -> Result<String> {
    let segs = sqyre_varref::segments(text);
    if segs.is_empty() {
        return Ok(text.to_string());
    }
    let mut out = String::new();
    for seg in segs {
        if !seg.is_ref {
            out.push_str(&seg.text);
            continue;
        }
        let val = macro_
            .variables
            .get(&seg.name)
            .ok_or_else(|| ExecError::Message(format!("unresolved variable ${{{}}}", seg.name)))?;
        out.push_str(&val.as_display());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{CoordinateResolver, DesktopRect, RecordingBackend, RecordingCapturer};
    use sqyre_domain::{root_loop, Action, ActionId, ActionKind, CoordinateRef, ScalarValue};

    struct FixedResolver;

    impl CoordinateResolver for FixedResolver {
        fn resolve_point(
            &self,
            _r: &CoordinateRef,
            _macro_: &Macro,
        ) -> std::result::Result<(i32, i32), String> {
            Ok((42, 99))
        }
        fn resolve_search_area(
            &self,
            _r: &CoordinateRef,
            _macro_: &Macro,
        ) -> std::result::Result<(i32, i32, i32, i32), String> {
            Ok((0, 0, 10, 10))
        }
    }

    #[test]
    fn executes_wait_loop_break() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![
            Action {
                id: ActionId::new(),
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(10),
                },
            },
            Action {
                id: ActionId::new(),
                kind: ActionKind::Loop {
                    name: "inner".into(),
                    count: ScalarValue::Int(3),
                    subactions: vec![
                        Action {
                            id: ActionId::new(),
                            kind: ActionKind::Wait {
                                time: ScalarValue::Int(1),
                            },
                        },
                        Action {
                            id: ActionId::new(),
                            kind: ActionKind::Break,
                        },
                    ],
                },
            },
        ]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert!(backend.log.iter().any(|e| e == "sleep:10"));
        assert_eq!(
            backend.log.iter().filter(|e| e.as_str() == "sleep:1").count(),
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
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: None,
                icons: None,
                macros: None,
                continue_waiter: None,
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
                ocr: None,
                stop_flag: Some(&stop),
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();
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
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(10),
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
                ocr: None,
                stop_flag: Some(&stop),
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();
        assert!(
            !backend.log.iter().any(|e| e == "sleep:250"),
            "global delay must not run after stop: {:?}",
            backend.log
        );
    }

    #[test]
    fn interruptible_sleep_returns_stopped() {
        use std::sync::atomic::AtomicBool;
        let mut backend = RecordingBackend::default();
        let stop = AtomicBool::new(true);
        let mut exec = Executor {
            automation: &mut backend,
            capturer: None,
            matcher: None,
            resolver: None,
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_requested: false,
            stop_flag: Some(&stop),
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        };
        let err = exec.interruptible_sleep(1000).unwrap_err();
        assert!(matches!(err, ExecError::Flow(FlowSignal::Stopped)));
    }

    #[test]
    fn move_uses_coordinate_resolver() {
        let mut backend = RecordingBackend::default();
        let resolver = FixedResolver;
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![
            Action {
                id: ActionId::new(),
                kind: ActionKind::Move {
                    point: CoordinateRef("Prog~Spot".into()),
                    smooth: false,
                    smooth_low: 0.05,
                    smooth_high: 0.2,
                    smooth_delay_ms: 1,
                },
            },
            Action {
                id: ActionId::new(),
                kind: ActionKind::Click {
                    button: "left".into(),
                    state: true,
                },
            },
            Action {
                id: ActionId::new(),
                kind: ActionKind::Click {
                    button: "left".into(),
                    state: false,
                },
            },
        ]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: None,
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: None,
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
        assert!(backend
            .log
            .iter()
            .any(|e| e == "move:42,99,smooth=false"));
        assert!(backend.log.iter().any(|e| e == "click:left:down"));
    }

    #[test]
    fn action_logger_tags_wait_and_click() {
        let mut backend = RecordingBackend::default();
        let logger = crate::SharedActionLog::new();
        let wait_id = ActionId::new();
        let click_id = ActionId::new();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![
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
                    state: true,
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
                ocr: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();
        let wait_lines = logger.lines_for(wait_id);
        let click_lines = logger.lines_for(click_id);
        assert!(
            wait_lines.iter().any(|l| l.starts_with("Wait:")),
            "wait lines: {wait_lines:?}"
        );
        assert!(
            click_lines.iter().any(|l| l.starts_with("Click:")),
            "click lines: {click_lines:?}"
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
            Action {
                id: ActionId::new(),
                kind: ActionKind::Conditional {
                    name: "ok".into(),
                    match_mode: "all".into(),
                    clauses: vec![sqyre_domain::ConditionClause {
                        left: ScalarValue::String("${flag}".into()),
                        operator: "==".into(),
                        right: ScalarValue::String("yes".into()),
                    }],
                    subactions: vec![Action {
                        id: ActionId::new(),
                        kind: ActionKind::Wait {
                            time: ScalarValue::Int(7),
                        },
                    }],
                },
            },
            Action {
                id: ActionId::new(),
                kind: ActionKind::Conditional {
                    name: "no".into(),
                    match_mode: "all".into(),
                    clauses: vec![sqyre_domain::ConditionClause {
                        left: ScalarValue::String("${flag}".into()),
                        operator: "==".into(),
                        right: ScalarValue::String("no".into()),
                    }],
                    subactions: vec![Action {
                        id: ActionId::new(),
                        kind: ActionKind::Wait {
                            time: ScalarValue::Int(99),
                        },
                    }],
                },
            },
        ]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert!(backend.log.iter().any(|e| e == "sleep:7"));
        assert!(!backend.log.iter().any(|e| e == "sleep:99"));
    }

    #[test]
    fn conditional_any_mode_and_operators() {
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variables.set("name", ScalarValue::String("hello".into()));
        macro_.variables.set("empty", ScalarValue::String("".into()));
        assert!(eval_clauses(
            "any",
            &[
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("x".into()),
                    operator: "==".into(),
                    right: ScalarValue::String("y".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("${name}".into()),
                    operator: "contains".into(),
                    right: ScalarValue::String("ell".into()),
                },
            ],
            &macro_
        ));
        assert!(eval_clauses(
            "all",
            &[
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("${name}".into()),
                    operator: "starts with".into(),
                    right: ScalarValue::String("he".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("${name}".into()),
                    operator: "ends with".into(),
                    right: ScalarValue::String("lo".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("name".into()),
                    operator: "is set".into(),
                    right: ScalarValue::Null,
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("${empty}".into()),
                    operator: "is empty".into(),
                    right: ScalarValue::Null,
                },
            ],
            &macro_
        ));
        assert!(!eval_clauses(
            "all",
            &[sqyre_domain::ConditionClause {
                left: ScalarValue::String("a".into()),
                operator: "!=".into(),
                right: ScalarValue::String("a".into()),
            }],
            &macro_
        ));
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
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                variable_name: "msg".into(),
                value: serde_yaml::Value::String("hello ${base}".into()),
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert_eq!(
            macro_.variables.get("msg").map(|v| v.as_display()),
            Some("hello world".into())
        );

        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Type {
                text: "hi ${missing}".into(),
                delay_ms: 0,
            },
        }]);
        let err = execute_macro(&mut macro_, &mut backend).unwrap_err();
        assert!(err.to_string().contains("unresolved"));
    }

    #[test]
    fn stop_flag_halts_mid_loop() {
        let mut backend = RecordingBackend::default();
        let stop = AtomicBool::new(false);
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Loop {
                name: "many".into(),
                count: ScalarValue::Int(5),
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(1),
                    },
                }],
            },
        }]);
        // Latch stop after construction; executor polls each iteration.
        stop.store(true, Ordering::SeqCst);
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
                ocr: None,
                stop_flag: Some(&stop),
                logger: None,
                highlighter: None,
                runtime_vars: None,
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
    fn applies_global_and_mouse_delays() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.global_delay = 3;
        macro_.mouse_delay = 5;
        macro_.keyboard_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Click {
                button: "left".into(),
                state: true,
            },
        }]);
        execute_macro(&mut macro_, &mut backend).unwrap();
        assert!(backend.log.iter().any(|e| e == "click:left:down"));
        assert!(backend.log.iter().any(|e| e == "sleep:3"));
        assert!(backend.log.iter().any(|e| e == "sleep:5"));
    }

    #[test]
    fn loop_continue_skips_rest_of_iteration() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Loop {
                name: "c".into(),
                count: ScalarValue::Int(2),
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
    fn type_emits_chars_with_delay() {
        let mut backend = RecordingBackend::default();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::Type {
                text: "ab".into(),
                delay_ms: 2,
            },
        }]);
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
