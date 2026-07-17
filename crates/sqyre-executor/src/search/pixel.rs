//! Find-pixel action.

use super::common::{
    clear_coord_outputs, maybe_repeat_while_found, maybe_wait_until_found, run_detection_outcome,
    set_coord_outputs,
};
use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::{Action, ActionKind, Macro};
use sqyre_vision::find_pixel;
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
        run_branch_on_no_find,
        subactions,
        ..
    } = detection;

    let action_id = action.id;
    let mut found = try_find_pixel(
        exec,
        action_id,
        search_area,
        target_color,
        *color_tolerance,
        macro_,
    );
    maybe_wait_until_found(exec, wait, found.is_some(), 100, |exec| {
        found = try_find_pixel(
            exec,
            action_id,
            search_area,
            target_color,
            *color_tolerance,
            macro_,
        );
        Ok(found.is_some())
    })?;

    if maybe_repeat_while_found(exec, wait, 200, |exec, refresh| {
        if refresh {
            found = try_find_pixel(
                exec,
                action_id,
                search_area,
                target_color,
                *color_tolerance,
                macro_,
            );
        }
        apply_find_pixel_outputs(exec, action_id, macro_, coords, found);
        run_detection_outcome(
            exec,
            found.is_some(),
            *run_branch_on_no_find,
            subactions,
            macro_,
        )
    })? {
        return Ok(());
    }

    apply_find_pixel_outputs(exec, action_id, macro_, coords, found);
    run_detection_outcome(
        exec,
        found.is_some(),
        *run_branch_on_no_find,
        subactions,
        macro_,
    )
    .map(|_| ())
}

fn apply_find_pixel_outputs(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    macro_: &mut Macro,
    coords: &sqyre_domain::CoordinateOutputs,
    found: Option<(i32, i32)>,
) {
    if let Some((x, y)) = found {
        exec.log(
            action_id,
            format!("FindPixel: found matching pixel at screen ({x}, {y})"),
        );
        set_coord_outputs(macro_, coords, x, y);
    } else {
        exec.log(action_id, "FindPixel: pixel not found");
        clear_coord_outputs(macro_, coords);
    }
}

fn try_find_pixel(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    search_area: &sqyre_domain::CoordinateRef,
    target_color: &str,
    color_tolerance: i32,
    macro_: &Macro,
) -> Option<(i32, i32)> {
    let resolver = exec.deps.resolver?;
    let capturer = exec.deps.capturer.as_mut()?;
    let (lx, ty, rx, by) = resolver.resolve_search_area(search_area, macro_).ok()?;
    let capture_started = Instant::now();
    let (img, origin) = capturer.capture_search_area_rgb(lx, ty, rx, by).ok()?;
    exec.log_timing(action_id, "capture", capture_started.elapsed());
    let scan_started = Instant::now();
    let buf = img.into_image_buf();
    let local = find_pixel(&buf, target_color, color_tolerance);
    exec.log_timing(action_id, "scan", scan_started.elapsed());
    let local = local?;
    Some((local.x + origin.x, local.y + origin.y))
}
