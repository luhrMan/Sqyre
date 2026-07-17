//! Helpers shared across the image search, OCR, and find-pixel implementations.

use crate::error::{ExecError, FlowSignal, Result};
use crate::run::{execute_action, Executor};
use sqyre_domain::{Action, Macro, ScalarValue, WaitTilFoundConfig};
use std::time::{Duration, Instant};

pub(super) fn run_children_flow(
    exec: &mut Executor<'_>,
    children: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    for child in children {
        execute_action(exec, child, macro_)?;
    }
    Ok(())
}

pub(super) fn set_coord_outputs(
    macro_: &mut Macro,
    coords: &sqyre_domain::CoordinateOutputs,
    x: i32,
    y: i32,
) {
    if !coords.output_x_variable.is_empty() {
        macro_
            .variables
            .set(&coords.output_x_variable, ScalarValue::Int(x as i64));
    }
    if !coords.output_y_variable.is_empty() {
        macro_
            .variables
            .set(&coords.output_y_variable, ScalarValue::Int(y as i64));
    }
}

pub(super) fn clear_coord_outputs(macro_: &mut Macro, coords: &sqyre_domain::CoordinateOutputs) {
    macro_.variables.delete(&coords.output_x_variable);
    macro_.variables.delete(&coords.output_y_variable);
}

/// Swallow Break/Continue from nested detection children.
pub(super) fn run_detection_children(
    exec: &mut Executor<'_>,
    children: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    match run_children_flow(exec, children, macro_) {
        Err(ExecError::Flow(FlowSignal::Break | FlowSignal::Continue)) => Ok(()),
        other => other,
    }
}

/// Run children when the attempt hit, or when `run_branch_on_no_find` on a miss.
/// Returns `hit` unchanged.
pub(super) fn run_detection_outcome(
    exec: &mut Executor<'_>,
    hit: bool,
    run_branch_on_no_find: bool,
    children: &[Action],
    macro_: &mut Macro,
) -> Result<bool> {
    if hit || run_branch_on_no_find {
        run_detection_children(exec, children, macro_)?;
    }
    Ok(hit)
}

pub(super) fn retry_while_not_found(
    exec: &mut Executor<'_>,
    wait: &WaitTilFoundConfig,
    default_interval_ms: i32,
    mut retry: impl FnMut(&mut Executor<'_>) -> Result<bool>,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(wait.wait_til_found_seconds.max(0) as u64);
    let mut interval = wait.effective_interval_ms(default_interval_ms).max(1);
    let max_interval = (interval * 5).min(2000).max(interval);
    while Instant::now() < deadline {
        exec.check_stopped()?;
        exec.interruptible_sleep(interval)?;
        if retry(exec)? {
            return Ok(());
        }
        if interval < max_interval {
            interval = (interval * 2).min(max_interval);
        }
    }
    Ok(())
}

pub(super) fn maybe_wait_until_found(
    exec: &mut Executor<'_>,
    wait: &WaitTilFoundConfig,
    hit: bool,
    default_interval_ms: i32,
    retry: impl FnMut(&mut Executor<'_>) -> Result<bool>,
) -> Result<()> {
    if wait.wait_until_found_active() && !hit {
        retry_while_not_found(exec, wait, default_interval_ms, retry)?;
    }
    Ok(())
}

/// When `wait` is repeat-while-found, run `iteration` until it returns false or limits hit.
///
/// `iteration(exec, refresh)` — `refresh` is false on the first pass (caller already captured)
/// and true after each sleep. If `wait_til_found_seconds > 0`, that value is also used as a
/// wall-clock deadline (image-search behaviour).
///
/// Returns `Ok(true)` when the repeat loop ran, `Ok(false)` when repeat mode is inactive.
pub(super) fn maybe_repeat_while_found(
    exec: &mut Executor<'_>,
    wait: &WaitTilFoundConfig,
    default_interval_ms: i32,
    mut iteration: impl FnMut(&mut Executor<'_>, bool) -> Result<bool>,
) -> Result<bool> {
    if !wait.is_repeat_while_found() {
        return Ok(false);
    }

    let max_iter = wait.effective_max_iterations();
    let interval = wait.effective_interval_ms(default_interval_ms).max(1);
    let deadline = if wait.wait_til_found_seconds > 0 {
        Some(Instant::now() + Duration::from_secs(wait.wait_til_found_seconds.max(0) as u64))
    } else {
        None
    };
    for i in 0..max_iter {
        exec.check_stopped()?;
        let refresh = i > 0;
        if refresh {
            exec.interruptible_sleep(interval)?;
            if deadline.is_some_and(|d| Instant::now() >= d) {
                break;
            }
        }
        if !iteration(exec, refresh)? {
            break;
        }
    }
    Ok(true)
}
