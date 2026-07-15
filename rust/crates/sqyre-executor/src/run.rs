use crate::backends::{
    AutomationBackend, CoordinateResolver, IconStore, MoveOptions, ScreenCapturer, TemplateMatcher,
};
use crate::error::{ExecError, FlowSignal, Result};
use crate::search::{execute_find_pixel, execute_image_search};
use sqyre_domain::{Action, ActionKind, Macro, ScalarValue};
use std::sync::atomic::{AtomicBool, Ordering};

/// Executor holding injected backends.
pub struct Executor<'a> {
    pub automation: &'a mut dyn AutomationBackend,
    pub capturer: Option<&'a mut dyn ScreenCapturer>,
    pub matcher: Option<&'a dyn TemplateMatcher>,
    pub resolver: Option<&'a dyn CoordinateResolver>,
    pub icons: Option<&'a dyn IconStore>,
    pub stop_requested: bool,
    /// Shared stop flag (Esc / UI Stop).
    pub stop_flag: Option<&'a AtomicBool>,
}

impl<'a> Executor<'a> {
    pub fn new(automation: &'a mut dyn AutomationBackend) -> Self {
        Self {
            automation,
            capturer: None,
            matcher: None,
            resolver: None,
            icons: None,
            stop_requested: false,
            stop_flag: None,
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
}

/// Dependencies for a full macro run.
pub struct ExecDeps<'a> {
    pub automation: &'a mut dyn AutomationBackend,
    pub capturer: Option<&'a mut dyn ScreenCapturer>,
    pub matcher: Option<&'a dyn TemplateMatcher>,
    pub resolver: Option<&'a dyn CoordinateResolver>,
    pub icons: Option<&'a dyn IconStore>,
    pub stop_flag: Option<&'a AtomicBool>,
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
            stop_flag: None,
        },
    )
}

pub fn execute_macro_with(macro_: &mut Macro, deps: ExecDeps<'_>) -> Result<()> {
    macro_.init_runtime_variables();
    let mut exec = Executor {
        automation: deps.automation,
        capturer: deps.capturer,
        matcher: deps.matcher,
        resolver: deps.resolver,
        icons: deps.icons,
        stop_requested: false,
        stop_flag: deps.stop_flag,
    };
    let root = macro_.root.clone();
    match execute_action(&mut exec, &root, macro_) {
        Err(ExecError::Flow(FlowSignal::Stopped)) => Ok(()),
        other => other,
    }
}

pub fn execute_action(exec: &mut Executor<'_>, action: &Action, macro_: &mut Macro) -> Result<()> {
    exec.check_stopped()?;
    let result = dispatch(exec, action, macro_);
    apply_delay(exec, action, macro_);
    result
}

fn apply_delay(exec: &mut Executor<'_>, action: &Action, macro_: &Macro) {
    if macro_.global_delay > 0 {
        exec.automation.milli_sleep(macro_.global_delay);
    }
    match action.type_key() {
        "key" | "type" if macro_.keyboard_delay > 0 => {
            exec.automation.milli_sleep(macro_.keyboard_delay);
        }
        "move" | "click" if macro_.mouse_delay > 0 => {
            exec.automation.milli_sleep(macro_.mouse_delay);
        }
        _ => {}
    }
}

fn dispatch(exec: &mut Executor<'_>, action: &Action, macro_: &mut Macro) -> Result<()> {
    match &action.kind {
        ActionKind::Wait { time } => {
            let ms = resolve_int(time, macro_)?;
            if ms > 0 {
                exec.automation.milli_sleep(ms);
            }
            Ok(())
        }
        ActionKind::Break => Err(FlowSignal::Break.into()),
        ActionKind::Continue => Err(FlowSignal::Continue.into()),
        ActionKind::Loop {
            count, subactions, ..
        } => run_loop(exec, count, subactions, macro_),
        ActionKind::Conditional {
            match_mode,
            clauses,
            subactions,
            ..
        } => {
            let ok = eval_clauses(match_mode, clauses, macro_);
            if ok {
                run_children(exec, subactions, macro_)
            } else {
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
                exec.automation.type_char(&ch.to_string());
                if *delay_ms > 0 {
                    exec.automation.milli_sleep(*delay_ms);
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
                        eprintln!("Move: failed to resolve point {}: {e}, using (0,0)", point.as_str());
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
        } => {
            let v = match value {
                serde_yaml::Value::String(s) => ScalarValue::String(resolve_text(s, macro_)?),
                other => ScalarValue::from_yaml(other),
            };
            macro_.variables.set(variable_name, v);
            Ok(())
        }
        ActionKind::ImageSearch { .. } => execute_image_search(exec, action, macro_),
        ActionKind::FindPixel { .. } => execute_find_pixel(exec, action, macro_),
        ActionKind::FocusWindow { .. } => {
            // Platform window focus lands later; no-op so macros can smoke-test.
            Ok(())
        }
        ActionKind::Pause { .. }
        | ActionKind::RunMacro { .. }
        | ActionKind::Calculate { .. }
        | ActionKind::SaveVariable { .. }
        | ActionKind::Ocr { .. }
        | ActionKind::ForEachRow { .. }
        | ActionKind::While { .. }
        | ActionKind::NavigateSelect { .. } => Err(ExecError::Message(format!(
            "executor: action '{}' not implemented yet",
            action.type_key()
        ))),
    }
}

fn run_loop(
    exec: &mut Executor<'_>,
    count: &ScalarValue,
    subactions: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    let n = resolve_int(count, macro_)?.max(0);
    for _ in 0..n {
        exec.check_stopped()?;
        match run_children(exec, subactions, macro_) {
            Err(ExecError::Flow(FlowSignal::Break)) => break,
            Err(ExecError::Flow(FlowSignal::Continue)) => continue,
            Err(e) => return Err(e),
            Ok(()) => {}
        }
    }
    Ok(())
}

fn run_children(exec: &mut Executor<'_>, children: &[Action], macro_: &mut Macro) -> Result<()> {
    for child in children {
        execute_action(exec, child, macro_)?;
    }
    Ok(())
}

fn eval_clauses(
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

fn resolve_int(v: &ScalarValue, macro_: &Macro) -> Result<i32> {
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

fn resolve_text(text: &str, macro_: &Macro) -> Result<String> {
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
    use crate::backends::{CoordinateResolver, RecordingBackend};
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
                stop_flag: None,
            },
        )
        .unwrap();
        assert!(backend
            .log
            .iter()
            .any(|e| e == "move:42,99,smooth=false"));
        assert!(backend.log.iter().any(|e| e == "click:left:down"));
    }
}
