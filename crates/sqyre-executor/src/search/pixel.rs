//! Find-pixel action.

use super::common::{
    apply_detection_hits, capture_search_buf, close_matches_distance, run_detection_shell,
    sort_hits, DetectionHit,
};
use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::{Action, ActionKind, Macro, MatchOrder};
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
    let sqyre_domain::DetectionBranch {
        wait,
        coords,
        order,
        subactions,
        else_actions,
    } = detection;

    let action_id = action.id;
    let order = order.clone();
    let targets: &[String] = &[];
    run_detection_shell(
        exec,
        macro_,
        wait,
        100,
        100,
        |exec, macro_| {
            Ok(try_find_pixels(
                exec,
                action_id,
                search_area,
                target_color,
                *color_tolerance,
                &order,
                macro_,
            ))
        },
        |hits| !hits.is_empty(),
        |exec, macro_, hits, pass| {
            if hits.is_empty() {
                exec.log(action_id, "FindPixel: pixel not found");
            } else if hits.len() == 1 {
                exec.log(
                    action_id,
                    format!(
                        "FindPixel: found matching pixel at screen ({}, {})",
                        hits[0].screen_x, hits[0].screen_y
                    ),
                );
            } else {
                exec.log(
                    action_id,
                    format!(
                        "FindPixel: {} clustered match(es); first at ({}, {})",
                        hits.len(),
                        hits[0].screen_x,
                        hits[0].screen_y
                    ),
                );
            }
            apply_detection_hits(
                exec,
                action_id,
                targets,
                hits,
                coords,
                subactions,
                else_actions,
                macro_,
                pass,
            )
        },
    )
}

fn try_find_pixels(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    search_area: &sqyre_domain::CoordinateRef,
    target_color: &str,
    color_tolerance: i32,
    order: &MatchOrder,
    macro_: &Macro,
) -> Vec<DetectionHit> {
    let Some((buf, origin)) = capture_search_buf(
        exec,
        action_id,
        "FindPixel",
        search_area,
        macro_,
        |_, _, _, _, _| {},
    ) else {
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
    sort_hits(&mut hits, order);
    hits
}
