//! Helpers shared across the image search, OCR, and find-pixel implementations.

use crate::backends::ItemMeta;
use crate::error::{ExecError, FlowSignal, Result};
use crate::highlight::{highlight_clear, highlight_fill};
use crate::run::{run_children, Executor};
use sqyre_domain::{
    Action, ActionId, CoordinateOutputs, Macro, MatchOrder, ScalarValue, WaitTilFoundConfig,
};
use std::time::{Duration, Instant};

pub(super) fn run_children_flow(
    exec: &mut Executor<'_>,
    children: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    run_children(exec, children, macro_)
}

pub(super) fn set_coord_outputs(macro_: &mut Macro, coords: &CoordinateOutputs, x: i32, y: i32) {
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

pub(super) fn clear_coord_outputs(macro_: &mut Macro, coords: &CoordinateOutputs) {
    macro_.variables.delete(&coords.output_x_variable);
    macro_.variables.delete(&coords.output_y_variable);
}

/// One detection hit in screen coordinates after kind-specific capture/match.
#[derive(Debug, Clone)]
pub(super) struct DetectionHit {
    pub screen_x: i32,
    pub screen_y: i32,
    pub name: String,
    pub extras: DetectionExtras,
}

#[derive(Debug, Clone, Default)]
pub(super) enum DetectionExtras {
    #[default]
    None,
    Image {
        meta: Option<ItemMeta>,
        tmpl_w: i32,
        tmpl_h: i32,
    },
}

impl DetectionHit {
    pub(super) fn plain(screen_x: i32, screen_y: i32, name: impl Into<String>) -> Self {
        Self {
            screen_x,
            screen_y,
            name: name.into(),
            extras: DetectionExtras::None,
        }
    }
}

const ORDER_BAND_PX: i32 = 5;

/// Sort hits using [`MatchOrder`]. Empty fields keep the historical default:
/// row grouping (±5px Y band), left-to-right, top-to-bottom.
pub(super) fn sort_hits(hits: &mut [DetectionHit], order: &MatchOrder) {
    let grouping = order.grouping.trim().to_ascii_lowercase();
    let h_rev = order.horizontal.eq_ignore_ascii_case("right_to_left");
    let v_rev = order.vertical.eq_ignore_ascii_case("bottom_to_top");

    hits.sort_by(|a, b| {
        let ay = a.screen_y;
        let by = b.screen_y;
        let ax = a.screen_x;
        let bx = b.screen_x;
        let cmp_x = if h_rev { bx.cmp(&ax) } else { ax.cmp(&bx) };
        let cmp_y = if v_rev { by.cmp(&ay) } else { ay.cmp(&by) };
        let name = a.name.cmp(&b.name);

        match grouping.as_str() {
            "column" => {
                if (ax - bx).abs() <= ORDER_BAND_PX {
                    cmp_y.then(name)
                } else {
                    cmp_x.then(cmp_y).then(name)
                }
            }
            "none" => cmp_y.then(cmp_x).then(name),
            // "" | "row" | anything else → row banding (legacy default)
            _ => {
                if (ay - by).abs() <= ORDER_BAND_PX {
                    cmp_x.then(name)
                } else {
                    cmp_y.then(cmp_x).then(name)
                }
            }
        }
    });
}

/// Shared per-hit children loop used by Image Search, OCR, and Find Pixel.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_matches(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    targets: &[String],
    results: &[DetectionHit],
    coords: &CoordinateOutputs,
    run_branch_on_no_find: bool,
    subactions: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    let mut found_names: Vec<&str> = results.iter().map(|h| h.name.as_str()).collect();
    found_names.sort_unstable();
    found_names.dedup();
    let not_found: Vec<&str> = targets
        .iter()
        .map(|t| t.as_str())
        .filter(|t| !found_names.iter().any(|f| f == t))
        .collect();
    exec.log(
        action_id,
        format!(
            "Total # found: {} (found: {:?}; not found: {:?})",
            results.len(),
            found_names,
            not_found
        ),
    );

    if results.is_empty() {
        clear_coord_outputs(macro_, coords);
        if run_branch_on_no_find {
            run_children_flow(exec, subactions, macro_)?;
        }
        return Ok(());
    }

    let mut first: Option<(i32, i32)> = None;
    let total = results.len();
    for (count, hit) in results.iter().enumerate() {
        if total > 0 {
            highlight_fill(
                exec.deps.highlighter,
                &macro_.name,
                action_id,
                count as f64 / total as f64,
            );
        }
        if first.is_none() {
            first = Some((hit.screen_x, hit.screen_y));
        }
        set_coord_outputs(macro_, coords, hit.screen_x, hit.screen_y);
        if let DetectionExtras::Image {
            meta,
            tmpl_w,
            tmpl_h,
        } = &hit.extras
        {
            if let Some(meta) = meta {
                macro_
                    .variables
                    .set("StackMax", ScalarValue::Int(meta.stack_max as i64));
                macro_
                    .variables
                    .set("Cols", ScalarValue::Int(meta.cols as i64));
                macro_
                    .variables
                    .set("Rows", ScalarValue::Int(meta.rows as i64));
                macro_
                    .variables
                    .set("ItemName", ScalarValue::String(meta.name.clone()));
            }
            macro_
                .variables
                .set("ImagePixelWidth", ScalarValue::Int(*tmpl_w as i64));
            macro_
                .variables
                .set("ImagePixelHeight", ScalarValue::Int(*tmpl_h as i64));
        }
        match run_children_flow(exec, subactions, macro_) {
            Err(ExecError::Flow(FlowSignal::Break)) => break,
            Err(ExecError::Flow(FlowSignal::Continue)) => continue,
            Err(e) => {
                highlight_clear(exec.deps.highlighter, &macro_.name, action_id);
                return Err(e);
            }
            Ok(()) => {}
        }
    }
    if let Some((x, y)) = first {
        set_coord_outputs(macro_, coords, x, y);
    }
    Ok(())
}

/// Apply hits for a detection pass: Repeat miss skips children; otherwise sort is
/// already done by the caller and [`run_matches`] runs the branch.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_detection_hits(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    targets: &[String],
    hits: &[DetectionHit],
    coords: &CoordinateOutputs,
    run_branch_on_no_find: bool,
    subactions: &[Action],
    macro_: &mut Macro,
    pass: DetectionPass,
) -> Result<bool> {
    // Repeat-while-found stops on miss without running the no-find branch;
    // the final single-shot still calls run_matches (for run_branch_on_no_find).
    if matches!(pass, DetectionPass::Repeat { .. }) && hits.is_empty() {
        clear_coord_outputs(macro_, coords);
        return Ok(false);
    }
    run_matches(
        exec,
        action_id,
        targets,
        hits,
        coords,
        run_branch_on_no_find,
        subactions,
        macro_,
    )?;
    Ok(!hits.is_empty())
}

/// Shared wait → repeat → single-shot shell for detection actions.
///
/// `try_once` produces the latest attempt state. `is_hit` decides whether wait/repeat
/// treat it as found. `on_outcome` applies outputs and runs branch children; its
/// returned bool is the continue flag for the repeat loop (typically the hit flag).
///
/// `macro_` is passed into callbacks so try/outcome do not both capture it.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_detection_shell<T>(
    exec: &mut Executor<'_>,
    macro_: &mut Macro,
    wait: &WaitTilFoundConfig,
    wait_interval_ms: i32,
    repeat_interval_ms: i32,
    mut try_once: impl FnMut(&mut Executor<'_>, &Macro) -> Result<T>,
    is_hit: impl Fn(&T) -> bool,
    mut on_outcome: impl FnMut(&mut Executor<'_>, &mut Macro, &T, DetectionPass) -> Result<bool>,
) -> Result<()> {
    let mut state = try_once(exec, macro_)?;
    maybe_wait_until_found(exec, wait, is_hit(&state), wait_interval_ms, |exec| {
        state = try_once(exec, macro_)?;
        Ok(is_hit(&state))
    })?;

    if maybe_repeat_while_found(exec, wait, repeat_interval_ms, |exec, refresh| {
        if refresh {
            state = try_once(exec, macro_)?;
        }
        on_outcome(exec, macro_, &state, DetectionPass::Repeat { refresh })
    })? {
        return Ok(());
    }

    on_outcome(exec, macro_, &state, DetectionPass::Final).map(|_| ())
}

/// Whether `on_outcome` is running inside the repeat-while-found loop or as the
/// single-shot after wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DetectionPass {
    Repeat { refresh: bool },
    Final,
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
