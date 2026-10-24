//! Find-pixel action.

use super::common::{
    apply_detection_hits, capture_search_buf, close_matches_distance, run_detection_shell,
    sort_hits, DetectionCtx, DetectionHit, FrameMemo,
};
use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::{action_type_label, Action, ActionKind, Macro};
use sqyre_ports::{DesktopRect, FindContext, LogFind, LogImage};
use sqyre_vision::find_pixels_clustered;
use std::sync::Arc;
use std::time::Instant;

/// Cap on interactive find markers per Finds entry — clustered hits can number
/// in the hundreds and the chip row must stay usable.
const MAX_LOG_FINDS: usize = 64;
/// Half-extent of a pixel hit's marker box so the desktop outline is visible.
const FIND_BOX_HALF: i32 = 8;

pub(crate) fn execute_find_pixel(
    exec: &mut Executor<'_>,
    action: &Action,
    macro_: &mut Macro,
) -> Result<()> {
    let ActionKind::FindPixel {
        search_area,
        target_color,
        color_tolerance,
        detection,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not find pixel".into()));
    };

    let action_id = action.id;
    let label = action_type_label(action.type_key());
    let ctx = DetectionCtx::new(action_id, label, search_area, &[], detection);
    let mut memo = FrameMemo::new();
    run_detection_shell(
        exec,
        macro_,
        &ctx,
        |exec, macro_, fresh| {
            Ok(try_find_pixels(
                exec,
                &ctx,
                target_color,
                *color_tolerance,
                macro_,
                fresh,
                &mut memo,
            ))
        },
        |state| !state.hits.is_empty(),
        |exec, macro_, state, pass| {
            let hits = &state.hits;
            if hits.is_empty() {
                exec.log(action_id, format!("{label}: pixel not found"));
            } else {
                let mut text = if hits.len() == 1 {
                    format!(
                        "{label}: found matching pixel at screen ({}, {})",
                        hits[0].screen_x, hits[0].screen_y
                    )
                } else {
                    format!(
                        "{label}: {} clustered match(es); first at ({}, {})",
                        hits.len(),
                        hits[0].screen_x,
                        hits[0].screen_y
                    )
                };
                if hits.len() > MAX_LOG_FINDS {
                    text = format!("{text} · markers truncated to {MAX_LOG_FINDS}");
                }
                let finds = hits
                    .iter()
                    .take(MAX_LOG_FINDS)
                    .map(|h| {
                        let bx = h.screen_x - state.origin.x;
                        let by = h.screen_y - state.origin.y;
                        LogFind {
                            label: format!("({},{})", h.screen_x, h.screen_y),
                            detail: state
                                .colors
                                .iter()
                                .find(|(c, _)| *c == (h.screen_x, h.screen_y))
                                .map(|(_, [r, g, b])| format!("#{r:02x}{g:02x}{b:02x}")),
                            screen: (
                                h.screen_x - FIND_BOX_HALF,
                                h.screen_y - FIND_BOX_HALF,
                                h.screen_x + FIND_BOX_HALF + 1,
                                h.screen_y + FIND_BOX_HALF + 1,
                            ),
                            context: state.capture.as_ref().map(|image| FindContext::Image {
                                image: Arc::clone(image),
                                rect: crate::log_draw::scale_rect_for_log(
                                    (
                                        bx - FIND_BOX_HALF,
                                        by - FIND_BOX_HALF,
                                        bx + FIND_BOX_HALF + 1,
                                        by + FIND_BOX_HALF + 1,
                                    ),
                                    state.src_w,
                                    state.src_h,
                                ),
                            }),
                        }
                    })
                    .collect();
                exec.log_finds(action_id, text, finds);
            }
            apply_detection_hits(exec, &ctx, hits, macro_, pass)
        },
    )
}

/// One pixel-scan pass: hits plus the capture they were found in, so the Logs
/// window can mark where each hit landed.
#[derive(Clone)]
struct PixelPass {
    hits: Vec<DetectionHit>,
    origin: DesktopRect,
    /// Capture dims before log downscale (context rect space for scale_rect_for_log).
    src_w: usize,
    src_h: usize,
    capture: Option<Arc<LogImage>>,
    /// `(screen_x, screen_y) → RGB` of each raw scan hit — the color actually
    /// found at that pixel, for the find chip's inspect tooltip.
    colors: Vec<((i32, i32), [u8; 3])>,
}

fn try_find_pixels(
    exec: &mut Executor<'_>,
    ctx: &DetectionCtx<'_>,
    target_color: &str,
    color_tolerance: i32,
    macro_: &Macro,
    fresh: bool,
    memo: &mut FrameMemo<PixelPass>,
) -> PixelPass {
    let action_id = ctx.action_id;
    let empty = |origin: DesktopRect, w: usize, h: usize| PixelPass {
        hits: Vec::new(),
        origin,
        src_w: w,
        src_h: h,
        capture: None,
        colors: Vec::new(),
    };
    let Some((buf, origin, frame_token)) =
        capture_search_buf(exec, ctx, macro_, fresh, |_, _, _, _, _| {})
    else {
        return empty(DesktopRect::default(), 0, 0);
    };
    if let Some(state) = memo.get(frame_token, origin) {
        let label = ctx.label;
        let hit_count = state.hits.len();
        exec.log_verbose(action_id, || {
            format!("{label}: frame unchanged (token {frame_token:#x}); reusing {hit_count} hit(s)")
        });
        return state.clone();
    }
    let scan_started = Instant::now();
    let clustered = find_pixels_clustered(
        &buf,
        target_color,
        color_tolerance,
        close_matches_distance(exec),
    );
    exec.log_timing(action_id, "scan", scan_started.elapsed());
    // Live-slot label: retries update the same capture image in the Logs window.
    let capture = exec
        .log_image(action_id, "1. Capture (search area)", &buf)
        .map(Arc::new);
    let mut hits: Vec<DetectionHit> = Vec::with_capacity(clustered.len());
    let mut colors: Vec<((i32, i32), [u8; 3])> = Vec::with_capacity(clustered.len());
    for p in clustered {
        let (sx, sy) = (p.x + origin.x, p.y + origin.y);
        if buf.channels == 3 {
            let o = (p.y as usize * buf.width + p.x as usize) * 3;
            colors.push(((sx, sy), [buf.data[o], buf.data[o + 1], buf.data[o + 2]]));
        }
        hits.push(DetectionHit::plain(sx, sy, ""));
    }
    sort_hits(&mut hits, &ctx.branch.order);
    let state = PixelPass {
        hits,
        origin,
        src_w: buf.width,
        src_h: buf.height,
        capture,
        colors,
    };
    memo.put(frame_token, origin, state.clone());
    state
}
