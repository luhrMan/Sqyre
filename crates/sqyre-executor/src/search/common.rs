//! Helpers shared across the image search, OCR, and find-pixel implementations.

use crate::backends::{DesktopRect, ItemMeta};
use crate::error::{ExecError, FlowSignal, Result};
use crate::run::{run_children, Executor};
use sqyre_domain::{
    Action, ActionId, CoordinateOutputs, CoordinateRef, DetectionBranch, Macro, MatchGrouping,
    MatchOrder, ScalarValue, WaitTilFoundConfig,
};
use sqyre_match::{ImageBuf, DEFAULT_CLOSE_MATCHES_DISTANCE};
use sqyre_ports::{highlight_clear, highlight_fill};
use sqyre_vision::rgb_capture_to_image_buf;
use std::time::{Duration, Instant};

/// Spatial dedup distance for match/pixel peaks, falling back to the library default
/// when the configured value is `0`. Shared by Image Search and Find Pixel.
pub(super) fn close_matches_distance(exec: &Executor<'_>) -> i32 {
    let d = exec.deps.close_matches_distance;
    if d > 0 {
        d
    } else {
        DEFAULT_CLOSE_MATCHES_DISTANCE
    }
}

/// Shared capture / wait / branch wiring for Image Search, OCR, and Find Pixel.
pub(super) struct DetectionCtx<'a> {
    pub action_id: ActionId,
    pub label: &'a str,
    pub search_area: &'a CoordinateRef,
    pub targets: &'a [String],
    pub branch: &'a DetectionBranch,
    pub wait_interval_ms: i32,
    pub repeat_interval_ms: i32,
}

impl<'a> DetectionCtx<'a> {
    pub(super) fn new(
        action_id: ActionId,
        label: &'a str,
        search_area: &'a CoordinateRef,
        targets: &'a [String],
        branch: &'a DetectionBranch,
    ) -> Self {
        Self {
            action_id,
            label,
            search_area,
            targets,
            branch,
            wait_interval_ms: 100,
            repeat_interval_ms: 100,
        }
    }
}

/// Resolve the search area, capture it, and convert to an [`ImageBuf`] — the shared
/// resolve→capture→convert preamble used by Image Search, OCR, and Find Pixel.
///
/// Missing resolver/capturer deps, resolve failures, and capture failures are logged
/// with `label` and treated as a miss (`None`) so the shared wait/repeat shell in
/// [`run_detection_shell`] can retry instead of aborting the macro.
///
/// `fresh` is true on wait/repeat recaptures and image search so caching backends
/// (portal) request a newer frame. OCR and find-pixel one-shot searches crop the cache.
///
/// `on_resolved` runs after a successful resolve but before capture, so callers can
/// log action-specific detail (e.g. targets, dimensions) using the resolved rect.
pub(super) fn capture_search_buf(
    exec: &mut Executor<'_>,
    ctx: &DetectionCtx<'_>,
    macro_: &Macro,
    fresh: bool,
    on_resolved: impl FnOnce(&mut Executor<'_>, i32, i32, i32, i32),
) -> Option<(ImageBuf, DesktopRect)> {
    let action_id = ctx.action_id;
    let label = ctx.label;
    let Some(resolver) = exec.deps.resolver else {
        exec.log(action_id, format!("{label}: missing CoordinateResolver"));
        return None;
    };
    if exec.deps.capturer.is_none() {
        exec.log(action_id, format!("{label}: missing ScreenCapturer"));
        return None;
    }

    let (lx, ty, rx, by) = match resolver.resolve_search_area(ctx.search_area, macro_) {
        Ok(v) => v,
        Err(e) => {
            exec.log(
                action_id,
                format!(
                    "{label}: resolve search area {}: {e}",
                    ctx.search_area.display_label()
                ),
            );
            return None;
        }
    };
    on_resolved(exec, lx, ty, rx, by);

    let capture_started = Instant::now();
    let (img, origin) = match exec
        .deps
        .capturer
        .as_mut()
        .expect("checked Some above")
        .capture_search_area_rgb(lx, ty, rx, by, fresh)
    {
        Ok(v) => v,
        Err(e) => {
            exec.log(action_id, format!("{label}: capture: {e}"));
            return None;
        }
    };
    exec.log_timing(action_id, "capture", capture_started.elapsed());
    let buf = rgb_capture_to_image_buf(img);
    let checksum = capture_checksum(&buf.data);
    exec.log(
        action_id,
        format!(
            "{label}: capture {}×{} checksum={:#x}",
            buf.width, buf.height, checksum
        ),
    );
    Some((buf, origin))
}

fn capture_checksum(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    let mut chunks = bytes.chunks_exact(8);
    for chunk in chunks.by_ref() {
        hash ^= u64::from_le_bytes(chunk.try_into().expect("chunks_exact(8)"));
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &b in chunks.remainder() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

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

/// Quantize a screen axis into ±[`ORDER_BAND_PX`] bands (transitive; safe for `sort_by`).
fn band_axis(v: i32) -> i32 {
    v.saturating_add(ORDER_BAND_PX)
        .div_euclid(ORDER_BAND_PX + 1)
}

/// Sort hits using [`MatchOrder`]. Default grouping is row banding
/// (±5px Y band), left-to-right, top-to-bottom.
pub(super) fn sort_hits(hits: &mut [DetectionHit], order: &MatchOrder) {
    let h_rev = order.horizontal.eq_ignore_ascii_case("right_to_left");
    let v_rev = order.vertical.eq_ignore_ascii_case("bottom_to_top");

    hits.sort_by(|a, b| {
        let cmp_x = if h_rev {
            b.screen_x.cmp(&a.screen_x)
        } else {
            a.screen_x.cmp(&b.screen_x)
        };
        let cmp_y = if v_rev {
            b.screen_y.cmp(&a.screen_y)
        } else {
            a.screen_y.cmp(&b.screen_y)
        };
        let name = a.name.cmp(&b.name);

        match order.grouping {
            MatchGrouping::Column => band_axis(a.screen_x)
                .cmp(&band_axis(b.screen_x))
                .then(cmp_y)
                .then(name),
            MatchGrouping::None => cmp_y.then(cmp_x).then(name),
            MatchGrouping::Row => band_axis(a.screen_y)
                .cmp(&band_axis(b.screen_y))
                .then(cmp_x)
                .then(name),
        }
    });
}

/// Shared per-hit children loop used by Image Search, OCR, and Find Pixel.
pub(super) fn run_matches(
    exec: &mut Executor<'_>,
    ctx: &DetectionCtx<'_>,
    results: &[DetectionHit],
    macro_: &mut Macro,
) -> Result<()> {
    let action_id = ctx.action_id;
    let coords = &ctx.branch.coords;
    let mut found_names: Vec<&str> = results.iter().map(|h| h.name.as_str()).collect();
    found_names.sort_unstable();
    found_names.dedup();
    let not_found: Vec<&str> = ctx
        .targets
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
        if !ctx.branch.else_actions.is_empty() {
            run_children_flow(exec, &ctx.branch.else_actions, macro_)?;
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
        match run_children_flow(exec, &ctx.branch.subactions, macro_) {
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

/// Apply hits for a detection pass: repeat-while miss runs else (if any) and stops;
/// otherwise [`run_matches`] runs the then or else branch.
pub(super) fn apply_detection_hits(
    exec: &mut Executor<'_>,
    ctx: &DetectionCtx<'_>,
    hits: &[DetectionHit],
    macro_: &mut Macro,
    pass: DetectionPass,
) -> Result<bool> {
    // Repeat-while-found stops on miss after running the else branch once.
    if matches!(pass, DetectionPass::RepeatWhile { .. }) && hits.is_empty() {
        clear_coord_outputs(macro_, &ctx.branch.coords);
        if !ctx.branch.else_actions.is_empty() {
            run_children_flow(exec, &ctx.branch.else_actions, macro_)?;
        }
        return Ok(false);
    }
    run_matches(exec, ctx, hits, macro_)?;
    // Repeat-until-found continues while missing; other passes continue while found.
    Ok(match pass {
        DetectionPass::RepeatUntil { .. } => hits.is_empty(),
        _ => !hits.is_empty(),
    })
}

/// Shared wait → repeat → single-shot shell for detection actions.
///
/// `try_once(exec, macro_, fresh)` produces the latest attempt. Image search always
/// fresh-captures; other kinds use `fresh` false on the first pass (crop cache) and
/// true on wait/repeat recaptures.
/// `is_hit` decides whether wait/repeat treat it as found. `on_outcome` applies
/// outputs and runs branch children; its returned bool is the continue flag for
/// the repeat loop (typically the hit flag).
///
/// `macro_` is passed into callbacks so try/outcome do not both capture it.
pub(super) fn run_detection_shell<T>(
    exec: &mut Executor<'_>,
    macro_: &mut Macro,
    ctx: &DetectionCtx<'_>,
    mut try_once: impl FnMut(&mut Executor<'_>, &Macro, bool) -> Result<T>,
    is_hit: impl Fn(&T) -> bool,
    mut on_outcome: impl FnMut(&mut Executor<'_>, &mut Macro, &T, DetectionPass) -> Result<bool>,
) -> Result<()> {
    let wait = &ctx.branch.wait;
    let wait_interval_ms = ctx.wait_interval_ms;
    let repeat_interval_ms = ctx.repeat_interval_ms;
    let mut state = try_once(exec, macro_, false)?;
    let mut wait_timed_out = false;
    if wait.wait_until_found_active() && !is_hit(&state) {
        if !maybe_wait_until_found(exec, wait, is_hit(&state), wait_interval_ms, |exec| {
            state = try_once(exec, macro_, true)?;
            Ok(is_hit(&state))
        })? {
            wait_timed_out = true;
        }
    } else if wait.wait_while_found_active() && is_hit(&state) {
        if !maybe_wait_while_found(exec, wait, is_hit(&state), wait_interval_ms, |exec| {
            state = try_once(exec, macro_, true)?;
            Ok(!is_hit(&state))
        })? {
            wait_timed_out = true;
        }
    }
    if wait_timed_out {
        state = try_once(exec, macro_, true)?;
    }

    if maybe_repeat_while_found(exec, wait, repeat_interval_ms, |exec, refresh| {
        if refresh {
            state = try_once(exec, macro_, true)?;
        }
        on_outcome(exec, macro_, &state, DetectionPass::RepeatWhile { refresh })
    })? {
        return Ok(());
    }

    if maybe_repeat_until_found(exec, wait, repeat_interval_ms, |exec, refresh| {
        if refresh {
            state = try_once(exec, macro_, true)?;
        }
        on_outcome(exec, macro_, &state, DetectionPass::RepeatUntil { refresh })
    })? {
        return Ok(());
    }

    on_outcome(exec, macro_, &state, DetectionPass::Final).map(|_| ())
}

/// Whether `on_outcome` is running inside a repeat loop or as the single-shot after wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DetectionPass {
    RepeatWhile { refresh: bool },
    RepeatUntil { refresh: bool },
    Final,
}

/// Poll until `done` returns true or the wait timeout elapses.
///
/// Returns `Ok(true)` when `done` succeeded, `Ok(false)` on timeout.
pub(super) fn retry_until(
    exec: &mut Executor<'_>,
    wait: &WaitTilFoundConfig,
    default_interval_ms: i32,
    mut done: impl FnMut(&mut Executor<'_>) -> Result<bool>,
) -> Result<bool> {
    let deadline = Instant::now() + wait.timeout().unwrap_or(Duration::ZERO);
    let mut interval = wait.effective_interval_ms(default_interval_ms).max(1);
    let max_interval = (interval * 5).min(2000).max(interval);
    while Instant::now() < deadline {
        exec.check_stopped()?;
        exec.interruptible_sleep(interval)?;
        if done(exec)? {
            return Ok(true);
        }
        if interval < max_interval {
            interval = (interval * 2).min(max_interval);
        }
    }
    Ok(false)
}

/// Returns `Ok(true)` when found (or wait inactive), `Ok(false)` on timeout.
pub(super) fn maybe_wait_until_found(
    exec: &mut Executor<'_>,
    wait: &WaitTilFoundConfig,
    hit: bool,
    default_interval_ms: i32,
    retry: impl FnMut(&mut Executor<'_>) -> Result<bool>,
) -> Result<bool> {
    if wait.wait_until_found_active() && !hit {
        retry_until(exec, wait, default_interval_ms, retry)
    } else {
        Ok(true)
    }
}

/// Returns `Ok(true)` when gone (or wait inactive), `Ok(false)` on timeout.
pub(super) fn maybe_wait_while_found(
    exec: &mut Executor<'_>,
    wait: &WaitTilFoundConfig,
    hit: bool,
    default_interval_ms: i32,
    // `gone` returns true when the target disappeared (stop waiting).
    gone: impl FnMut(&mut Executor<'_>) -> Result<bool>,
) -> Result<bool> {
    if wait.wait_while_found_active() && hit {
        retry_until(exec, wait, default_interval_ms, gone)
    } else {
        Ok(true)
    }
}

/// Shared repeat loop body for while-found / until-found modes.
///
/// `iteration(exec, refresh)` — `refresh` is false on the first pass (caller already captured)
/// and true after each sleep. If `wait_til_found_seconds > 0`, that value is also used as a
/// wall-clock deadline (image-search behaviour).
///
/// Returns `Ok(true)` when the repeat loop ran, `Ok(false)` when the mode is inactive.
fn maybe_repeat_loop(
    exec: &mut Executor<'_>,
    wait: &WaitTilFoundConfig,
    default_interval_ms: i32,
    active: bool,
    mut iteration: impl FnMut(&mut Executor<'_>, bool) -> Result<bool>,
) -> Result<bool> {
    if !active {
        return Ok(false);
    }

    let max_iter = wait.effective_max_iterations();
    let interval = wait.effective_interval_ms(default_interval_ms).max(1);
    let deadline = wait.timeout().map(|d| Instant::now() + d);
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

/// When `wait` is repeat-while-found, run `iteration` until it returns false or limits hit.
pub(super) fn maybe_repeat_while_found(
    exec: &mut Executor<'_>,
    wait: &WaitTilFoundConfig,
    default_interval_ms: i32,
    iteration: impl FnMut(&mut Executor<'_>, bool) -> Result<bool>,
) -> Result<bool> {
    maybe_repeat_loop(
        exec,
        wait,
        default_interval_ms,
        wait.is_repeat_while_found(),
        iteration,
    )
}

/// When `wait` is repeat-until-found, run `iteration` until it returns false or limits hit.
pub(super) fn maybe_repeat_until_found(
    exec: &mut Executor<'_>,
    wait: &WaitTilFoundConfig,
    default_interval_ms: i32,
    iteration: impl FnMut(&mut Executor<'_>, bool) -> Result<bool>,
) -> Result<bool> {
    maybe_repeat_loop(
        exec,
        wait,
        default_interval_ms,
        wait.is_repeat_until_found(),
        iteration,
    )
}
