use crate::action_log::ActionLogger;
use crate::actions::{
    execute_focus_window, execute_for_each_row, execute_pause, execute_run_macro,
    execute_save_variable, execute_set_variable, execute_while,
};
use crate::backends::{
    AutomationBackend, ContinueKeyWaiter, CoordinateResolver, IconStore, MacroLookup, MoveOptions,
    OcrEngine, ScreenCapturer, WindowFocuser,
};
use crate::error::{ExecError, FlowSignal, Result};
use crate::highlight::{clear_highlights, highlight_cursor, ActionHighlighter};
use crate::navigate::{execute_navigate_key, execute_navigate_select};
use crate::runtime_vars::RuntimeVarSink;
use crate::search::{execute_find_pixel, execute_image_search, execute_ocr};
use sqyre_domain::{
    action_type_label, resolve_scalar_int, Action, ActionId, ActionKind, Macro, MatchMode,
    MouseButton, ScalarValue,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Executor: injected backends ([`ExecDeps`]) plus per-run mutable state.
pub struct Executor<'a> {
    pub deps: ExecDeps<'a>,
    /// Set when a `Stop` action or `Break`/`Continue`-style flow requests halt.
    pub stop_requested: bool,
}

impl<'a> Executor<'a> {
    pub fn new(automation: &'a mut dyn AutomationBackend) -> Self {
        Self {
            deps: ExecDeps::new(automation),
            stop_requested: false,
        }
    }

    pub fn check_stopped(&self) -> Result<()> {
        if self.stop_requested
            || self
                .deps
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
            self.deps.automation.milli_sleep(chunk);
            left -= chunk;
        }
        self.check_stopped()
    }

    pub fn log(&self, action_id: ActionId, message: impl Into<String>) {
        if let Some(logger) = self.deps.logger {
            logger.log(action_id, message.into());
        }
    }

    /// Build a log line only when a logger is attached.
    pub fn log_with(&self, action_id: ActionId, f: impl FnOnce() -> String) {
        if let Some(logger) = self.deps.logger {
            logger.log(action_id, f());
        }
    }

    pub fn log_images_enabled(&self) -> bool {
        self.deps
            .logger
            .map(|l| l.log_images_enabled())
            .unwrap_or(false)
    }

    pub fn log_image(
        &self,
        action_id: ActionId,
        label: impl Into<String>,
        image: &sqyre_match::ImageBuf,
    ) {
        if let Some(logger) = self.deps.logger {
            logger.log_image(action_id, label.into(), image);
        }
    }

    pub fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: impl Into<String>,
        summary: impl Into<String>,
        thumbnail: &sqyre_match::ImageBuf,
        steps: &[(&str, &sqyre_match::ImageBuf)],
        details: Vec<String>,
    ) {
        if let Some(logger) = self.deps.logger {
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

    /// Record how long a named step took (shown in the action logs UI).
    pub fn log_timing(&self, action_id: ActionId, step: &str, elapsed: Duration) {
        self.log(
            action_id,
            format!("timing: {step} {:.1}ms", elapsed.as_secs_f64() * 1000.0),
        );
    }

    /// Time a fallible step and log its duration even when it errors.
    pub fn timed_step<T>(&self, action_id: ActionId, step: &str, f: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let out = f();
        self.log_timing(action_id, step, started.elapsed());
        out
    }
}

/// Dependencies for a full macro run.
pub struct ExecDeps<'a> {
    pub automation: &'a mut dyn AutomationBackend,
    pub capturer: Option<&'a mut dyn ScreenCapturer>,
    /// Spatial dedup distance for image-search peaks; `0` uses the library default.
    pub close_matches_distance: i32,
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

impl<'a> ExecDeps<'a> {
    /// Backends with only automation wired; everything else absent.
    pub fn new(automation: &'a mut dyn AutomationBackend) -> Self {
        Self {
            automation,
            capturer: None,
            close_matches_distance: 0,
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
        }
    }

    pub fn capturer(mut self, c: &'a mut dyn ScreenCapturer) -> Self {
        self.capturer = Some(c);
        self
    }

    pub fn resolver(mut self, r: &'a dyn CoordinateResolver) -> Self {
        self.resolver = Some(r);
        self
    }

    pub fn icons(mut self, i: &'a dyn IconStore) -> Self {
        self.icons = Some(i);
        self
    }

    pub fn ocr(mut self, o: &'a dyn OcrEngine) -> Self {
        self.ocr = Some(o);
        self
    }

    pub fn logger(mut self, l: &'a dyn ActionLogger) -> Self {
        self.logger = Some(l);
        self
    }

    pub fn stop_flag(mut self, f: &'a AtomicBool) -> Self {
        self.stop_flag = Some(f);
        self
    }

    pub fn continue_waiter(mut self, w: &'a dyn ContinueKeyWaiter) -> Self {
        self.continue_waiter = Some(w);
        self
    }

    pub fn window_focuser(mut self, f: &'a dyn WindowFocuser) -> Self {
        self.window_focuser = Some(f);
        self
    }

    pub fn highlighter(mut self, h: &'a dyn ActionHighlighter) -> Self {
        self.highlighter = Some(h);
        self
    }

    pub fn macros(mut self, m: &'a dyn MacroLookup) -> Self {
        self.macros = Some(m);
        self
    }

    pub fn close_matches_distance(mut self, d: i32) -> Self {
        self.close_matches_distance = d;
        self
    }
}

/// Run a macro from a clean runtime variable store.
pub fn execute_macro(macro_: &mut Macro, automation: &mut dyn AutomationBackend) -> Result<()> {
    execute_macro_with(macro_, ExecDeps::new(automation))
}

pub fn execute_macro_with(macro_: &mut Macro, deps: ExecDeps<'_>) -> Result<()> {
    let macro_started = Instant::now();
    let mut exec = Executor {
        deps,
        stop_requested: false,
    };
    macro_.init_runtime_variables();
    let monitor_sizes = match exec.deps.capturer.as_mut() {
        Some(c) => c.monitor_sizes().unwrap_or_else(|_| vec![(0, 0)]),
        None => vec![(0, 0)],
    };
    apply_monitor_sizes(macro_, &monitor_sizes);
    publish_runtime_vars(exec.deps.runtime_vars, macro_);
    let root = macro_.root.clone();
    let root_id = root.id;
    let result = match execute_action(&mut exec, &root, macro_) {
        Err(ExecError::Flow(FlowSignal::Stopped)) => Ok(()),
        other => other,
    };
    exec.log_timing(root_id, "macro total", macro_started.elapsed());
    clear_highlights(exec.deps.highlighter);
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
            publish_runtime_vars(exec.deps.runtime_vars, macro_);
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
            condition,
            subactions,
        } => {
            let ok = eval_clauses(condition.match_mode, &condition.clauses, macro_)?;
            if ok {
                exec.log(
                    action.id,
                    format!("Conditional {:?}: true, running branch", condition.name),
                );
                run_children(exec, subactions, macro_)
            } else {
                exec.log(
                    action.id,
                    format!("Conditional {:?}: false, skipping branch", condition.name),
                );
                Ok(())
            }
        }
        ActionKind::Click { button, state } => {
            if *button == MouseButton::Scroll {
                exec.deps
                    .automation
                    .scroll(*state)
                    .map_err(ExecError::Message)
            } else {
                exec.deps
                    .automation
                    .click(button.as_str(), *state)
                    .map_err(ExecError::Message)
            }
        }
        ActionKind::Key { key, state } => {
            if *state {
                exec.deps
                    .automation
                    .key_down(key)
                    .map_err(ExecError::Message)
            } else {
                exec.deps.automation.key_up(key).map_err(ExecError::Message)
            }
        }
        ActionKind::Type { text, delay_ms } => {
            let resolved = resolve_text(text, macro_)?;
            for ch in resolved.chars() {
                exec.check_stopped()?;
                exec.deps.automation.type_char(ch);
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
            exec.deps.automation.move_to(
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
            condition,
            max_iterations,
            subactions,
        } => execute_while(
            exec,
            action.id,
            &condition.name,
            condition.match_mode,
            &condition.clauses,
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
            exec, action.id, name, sources, start_row, end_row, subactions, macro_,
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
        ActionKind::Ocr { .. } => execute_ocr(exec, action, macro_),
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

pub(crate) fn eval_clauses(
    match_mode: MatchMode,
    clauses: &[sqyre_domain::ConditionClause],
    macro_: &Macro,
) -> Result<bool> {
    if clauses.is_empty() {
        return Ok(true);
    }
    let want_any = match_mode == MatchMode::Any;
    for c in clauses {
        let ok = eval_one_clause(c, macro_)?;
        if want_any {
            if ok {
                return Ok(true);
            }
        } else if !ok {
            return Ok(false);
        }
    }
    Ok(!want_any)
}

fn eval_one_clause(c: &sqyre_domain::ConditionClause, macro_: &Macro) -> Result<bool> {
    // `is set` looks up the variable *name* without expanding its value.
    if c.operator.as_str() == "is set" {
        let raw = c.left.as_display();
        let name = variable_name_for_is_set(&raw);
        return Ok(macro_.variables.get(name).is_some());
    }

    let left = resolve_text(&c.left.as_display(), macro_)?;
    let right = resolve_text(&c.right.as_display(), macro_)?;
    Ok(match c.operator.as_str() {
        "==" => left == right,
        "!=" => left != right,
        "is empty" => left.trim().is_empty(),
        "contains" => left.contains(&right),
        "starts with" => left.starts_with(&right),
        "ends with" => left.ends_with(&right),
        "<" | "<=" | ">" | ">=" => compare_ordered(&left, &right, c.operator.as_str()),
        _ => false,
    })
}

/// Name for `is set`: strip `${…}` / `{…}`, otherwise use the trimmed literal.
fn variable_name_for_is_set(raw: &str) -> &str {
    let t = raw.trim();
    if let Some(inner) = t.strip_prefix("${").and_then(|x| x.strip_suffix('}')) {
        return inner.trim();
    }
    if let Some(inner) = t.strip_prefix('{').and_then(|x| x.strip_suffix('}')) {
        return inner.trim();
    }
    t
}

/// Numeric compare when both sides parse as `f64`; otherwise lexicographic.
fn compare_ordered(left: &str, right: &str, op: &str) -> bool {
    use std::cmp::Ordering;
    let ord = match (left.trim().parse::<f64>(), right.trim().parse::<f64>()) {
        (Ok(a), Ok(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => left.cmp(right),
    };
    match op {
        "<" => ord == Ordering::Less,
        "<=" => ord != Ordering::Greater,
        ">" => ord == Ordering::Greater,
        ">=" => ord != Ordering::Less,
        _ => false,
    }
}

pub(crate) fn resolve_int(v: &ScalarValue, macro_: &Macro) -> Result<i32> {
    resolve_scalar_int(v, macro_).map_err(ExecError::Message)
}

pub(crate) fn resolve_text(text: &str, macro_: &Macro) -> Result<String> {
    sqyre_domain::expand_variable_refs(text, macro_).map_err(ExecError::Message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{DesktopRect, RecordingBackend, RecordingCapturer};
    use crate::test_support::FixedResolver;
    use sqyre_domain::{
        root_loop, Action, ActionId, ActionKind, CoordinateRef, ScalarValue, VariableAssignment,
    };

    const RUN_RESOLVER: FixedResolver = FixedResolver::point_area((42, 99), (0, 0, 10, 10));

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
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.root = root_loop(vec![]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                close_matches_distance: 0,
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
                close_matches_distance: 0,
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
                close_matches_distance: 0,
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
            deps: ExecDeps {
                stop_flag: Some(&stop),
                ..ExecDeps::new(&mut backend)
            },
            stop_requested: false,
        };
        let err = exec.interruptible_sleep(1000).unwrap_err();
        assert!(matches!(err, ExecError::Flow(FlowSignal::Stopped)));
    }

    #[test]
    fn move_uses_coordinate_resolver() {
        let mut backend = RecordingBackend::default();
        let resolver = RUN_RESOLVER;
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
                close_matches_distance: 0,
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
        assert!(backend.log.iter().any(|e| e == "move:42,99,smooth=false"));
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
                close_matches_distance: 0,
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
        assert!(
            wait_lines.iter().any(|l| l.starts_with("timing: total ")),
            "wait timing: {wait_lines:?}"
        );
        assert!(
            click_lines.iter().any(|l| l.starts_with("timing: total ")),
            "click timing: {click_lines:?}"
        );
        let root_lines = logger.lines_for(macro_.root.id);
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
            Action {
                id: ActionId::new(),
                kind: ActionKind::Conditional {
                    condition: sqyre_domain::ConditionBlock {
                        name: "ok".into(),
                        match_mode: "all".into(),
                        clauses: vec![sqyre_domain::ConditionClause {
                            left: ScalarValue::String("${flag}".into()),
                            operator: "==".into(),
                            right: ScalarValue::String("yes".into()),
                        }],
                    },
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
                    condition: sqyre_domain::ConditionBlock {
                        name: "no".into(),
                        match_mode: "all".into(),
                        clauses: vec![sqyre_domain::ConditionClause {
                            left: ScalarValue::String("${flag}".into()),
                            operator: "==".into(),
                            right: ScalarValue::String("no".into()),
                        }],
                    },
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
        )
        .unwrap());
        assert!(eval_clauses(
            MatchMode::All,
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
        )
        .unwrap());
        assert!(!eval_clauses(
            MatchMode::All,
            &[sqyre_domain::ConditionClause {
                left: ScalarValue::String("a".into()),
                operator: "!=".into(),
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
                    operator: ">".into(),
                    right: ScalarValue::String("2".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("3".into()),
                    operator: "<=".into(),
                    right: ScalarValue::String("3.0".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("1".into()),
                    operator: "<".into(),
                    right: ScalarValue::String("2".into()),
                },
                sqyre_domain::ConditionClause {
                    left: ScalarValue::String("5".into()),
                    operator: ">=".into(),
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
                operator: ">".into(),
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
                operator: "is set".into(),
                right: ScalarValue::Null,
            }],
            &macro_
        )
        .unwrap());
        assert!(!eval_clauses(
            MatchMode::All,
            &[sqyre_domain::ConditionClause {
                left: ScalarValue::String("${missing}".into()),
                operator: "is set".into(),
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
                operator: "is set".into(),
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
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                assignments: vec![VariableAssignment::new(
                    "msg",
                    sqyre_domain::ScalarValue::String("hello ${base}".into()),
                )],
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
                close_matches_distance: 0,
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
