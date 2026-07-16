//! Find-pixel action.

use super::common::{clear_coord_outputs, retry_while_not_found, run_detection_children, set_coord_outputs};
use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::{Action, ActionKind, Macro};
use sqyre_vision::{find_pixel, rgba_to_rgb_buf};

pub(crate) fn execute_find_pixel(
    exec: &mut Executor<'_>,
    action: &Action,
    macro_: &mut Macro,
) -> Result<()> {
    let ActionKind::FindPixel {
        search_area,
        target_color,
        color_tolerance,
        wait,
        coords,
        run_branch_on_no_find,
        subactions,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not find pixel".into()));
    };

    let action_id = action.id;
    let mut found = try_find_pixel(exec, search_area, target_color, *color_tolerance, macro_);
    if wait.wait_until_found_active() && found.is_none() {
        let _ = retry_while_not_found(exec, wait, 100, |exec| {
            found = try_find_pixel(exec, search_area, target_color, *color_tolerance, macro_);
            Ok(found.is_some())
        })?;
    }

    if wait.is_repeat_while_found() {
        let max_iter = wait.effective_max_iterations();
        let interval = wait.effective_interval_ms(200).max(1);
        for i in 0..max_iter {
            exec.check_stopped()?;
            if i > 0 {
                exec.interruptible_sleep(interval)?;
                found = try_find_pixel(exec, search_area, target_color, *color_tolerance, macro_);
            }
            if let Some((x, y)) = found {
                exec.log(
                    action_id,
                    format!("FindPixel: found matching pixel at screen ({x}, {y})"),
                );
                set_coord_outputs(macro_, coords, x, y);
                run_detection_children(exec, subactions, macro_)?;
            } else {
                exec.log(action_id, "FindPixel: pixel not found");
                clear_coord_outputs(macro_, coords);
                if *run_branch_on_no_find {
                    run_detection_children(exec, subactions, macro_)?;
                }
                return Ok(());
            }
        }
        return Ok(());
    }

    if let Some((x, y)) = found {
        exec.log(
            action_id,
            format!("FindPixel: found matching pixel at screen ({x}, {y})"),
        );
        set_coord_outputs(macro_, coords, x, y);
        return run_detection_children(exec, subactions, macro_);
    }

    exec.log(action_id, "FindPixel: pixel not found");
    clear_coord_outputs(macro_, coords);
    if *run_branch_on_no_find {
        return run_detection_children(exec, subactions, macro_);
    }
    Ok(())
}

fn try_find_pixel(
    exec: &mut Executor<'_>,
    search_area: &sqyre_domain::CoordinateRef,
    target_color: &str,
    color_tolerance: i32,
    macro_: &Macro,
) -> Option<(i32, i32)> {
    let resolver = exec.resolver?;
    let capturer = exec.capturer.as_mut()?;
    let (lx, ty, rx, by) = resolver.resolve_search_area(search_area, macro_).ok()?;
    let (img, origin) = capturer.capture_search_area(lx, ty, rx, by).ok()?;
    let buf = rgba_to_rgb_buf(&img);
    let local = find_pixel(&buf, target_color, color_tolerance)?;
    Some((local.x + origin.x, local.y + origin.y))
}
