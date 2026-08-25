//! Find-pixel action.

use super::common::{
    apply_detection_hits, capture_search_buf, close_matches_distance, run_detection_shell,
    sort_hits, DetectionCtx, DetectionHit,
};
use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::{action_type_label, Action, ActionKind, Macro};
use sqyre_match::cluster_points;
use sqyre_vision::find_pixels;
use std::time::Instant;

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
            ))
        },
        |hits| !hits.is_empty(),
        |exec, macro_, hits, pass| {
            if hits.is_empty() {
                exec.log(action_id, format!("{label}: pixel not found"));
            } else if hits.len() == 1 {
                exec.log(
                    action_id,
                    format!(
                        "{label}: found matching pixel at screen ({}, {})",
                        hits[0].screen_x, hits[0].screen_y
                    ),
                );
            } else {
                exec.log(
                    action_id,
                    format!(
                        "{label}: {} clustered match(es); first at ({}, {})",
                        hits.len(),
                        hits[0].screen_x,
                        hits[0].screen_y
                    ),
                );
            }
            apply_detection_hits(exec, &ctx, hits, macro_, pass)
        },
    )
}

fn try_find_pixels(
    exec: &mut Executor<'_>,
    ctx: &DetectionCtx<'_>,
    target_color: &str,
    color_tolerance: i32,
    macro_: &Macro,
    fresh: bool,
) -> Vec<DetectionHit> {
    let action_id = ctx.action_id;
    let Some((buf, origin)) = capture_search_buf(exec, ctx, macro_, fresh, |_, _, _, _, _| {})
    else {
        return Vec::new();
    };
    let scan_started = Instant::now();
    let locals = find_pixels(&buf, target_color, color_tolerance);
    let clustered = cluster_points(&locals, close_matches_distance(exec));
    exec.log_timing(action_id, "scan", scan_started.elapsed());
    let mut hits: Vec<DetectionHit> = clustered
        .into_iter()
        .map(|p| DetectionHit::plain(p.x + origin.x, p.y + origin.y, ""))
        .collect();
    sort_hits(&mut hits, &ctx.branch.order);
    hits
}
