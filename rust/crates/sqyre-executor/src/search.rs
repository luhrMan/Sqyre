//! Image search / find-pixel orchestration (Go `executor_search.go`).

use crate::backends::{DesktopRect, ItemMeta, TemplateMatcher};
use crate::error::{ExecError, FlowSignal, Result};
use crate::run::{execute_action, Executor};
use sqyre_domain::{Action, ActionKind, Macro, ScalarValue, REPEAT_WHILE_FOUND};
use sqyre_match::{
    blur_image, find_template_matches, search_blur_kernel, ImageBuf, Point,
    DEFAULT_CLOSE_MATCHES_DISTANCE,
};
use sqyre_vision::{find_pixel, load_rgb_image, mask_as_u8, resize_mask, rgba_to_rgb_buf};
use std::time::{Duration, Instant};

pub(crate) fn execute_image_search(
    exec: &mut Executor<'_>,
    action: &Action,
    macro_: &mut Macro,
) -> Result<()> {
    let ActionKind::ImageSearch {
        targets,
        search_area,
        tolerance,
        blur,
        wait,
        coords,
        run_branch_on_no_find,
        subactions,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not image search".into()));
    };

    let mut results = capture_and_match(exec, targets, search_area, *tolerance, *blur, macro_);

    if wait.wait_until_found_active() && results.is_empty() {
        let _ = retry_while_not_found(exec, wait, 100, |exec| {
            results = capture_and_match(exec, targets, search_area, *tolerance, *blur, macro_);
            Ok(!results.is_empty())
        })?;
    }

    if wait.effective_repeat_mode() == REPEAT_WHILE_FOUND {
        let deadline =
            Instant::now() + Duration::from_secs(wait.wait_til_found_seconds.max(0) as u64);
        while Instant::now() < deadline {
            exec.check_stopped()?;
            if results.is_empty() {
                break;
            }
            run_matches(
                exec,
                &results,
                coords,
                *run_branch_on_no_find,
                subactions,
                macro_,
            )?;
            results = capture_and_match(exec, targets, search_area, *tolerance, *blur, macro_);
            let interval = wait.effective_interval_ms(100).max(1);
            exec.automation.milli_sleep(interval);
        }
        return Ok(());
    }

    run_matches(
        exec,
        &results,
        coords,
        *run_branch_on_no_find,
        subactions,
        macro_,
    )
}

struct NamedPoint {
    point: Point,
    origin: DesktopRect,
    meta: Option<ItemMeta>,
    tmpl_w: i32,
    tmpl_h: i32,
    name: String,
}

fn capture_and_match(
    exec: &mut Executor<'_>,
    targets: &[String],
    search_area: &sqyre_domain::CoordinateRef,
    tolerance: f64,
    blur: i32,
    macro_: &Macro,
) -> Vec<NamedPoint> {
    let Some(resolver) = exec.resolver else {
        eprintln!("Image search: missing CoordinateResolver");
        return Vec::new();
    };
    let Some(capturer) = exec.capturer.as_mut() else {
        eprintln!("Image search: missing ScreenCapturer");
        return Vec::new();
    };
    let Some(icons) = exec.icons else {
        eprintln!("Image search: missing IconStore");
        return Vec::new();
    };

    let (lx, ty, rx, by) = match resolver.resolve_search_area(search_area, macro_) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Image search: resolve search area: {e}");
            return Vec::new();
        }
    };
    let (img, origin) = match capturer.capture_search_area(lx, ty, rx, by) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Image search: capture: {e}");
            return Vec::new();
        }
    };
    let search = rgba_to_rgb_buf(&img);
    let kernel = search_blur_kernel(blur);
    let search_blurred = match blur_image(&search, kernel) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Image search: blur: {e}");
            return Vec::new();
        }
    };

    let threshold = tolerance as f32;
    let mut out = Vec::new();
    for target in targets {
        let paths = icons.variant_paths(target);
        let meta = icons.item_meta(target);
        let mask_path = icons.mask_path(target);
        for path in paths {
            let template = match load_rgb_image(&path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Image search: {e}");
                    continue;
                }
            };
            let mask_bytes = mask_path
                .as_ref()
                .and_then(|p| load_rgb_image(p).ok())
                .map(|m| {
                    let resized = resize_mask(&m, template.width, template.height);
                    mask_as_u8(&resized)
                });
            let matches = match find_template_matches(
                &search_blurred,
                &template,
                mask_bytes.as_deref(),
                threshold,
                blur,
                DEFAULT_CLOSE_MATCHES_DISTANCE,
            ) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Image search match: {e}");
                    continue;
                }
            };
            let half_w = (template.width / 2) as i32;
            let half_h = (template.height / 2) as i32;
            for mut p in matches {
                p.x += half_w;
                p.y += half_h;
                out.push(NamedPoint {
                    name: target.clone(),
                    point: p,
                    origin,
                    meta: meta.clone(),
                    tmpl_w: template.width as i32,
                    tmpl_h: template.height as i32,
                });
            }
        }
    }
    sort_points(&mut out);
    let _ = exec.matcher; // optional override reserved for tests injecting TemplateMatcher
    out
}

fn sort_points(pts: &mut [NamedPoint]) {
    pts.sort_by(|a, b| {
        let ay = a.point.y + a.origin.y;
        let by = b.point.y + b.origin.y;
        let ax = a.point.x + a.origin.x;
        let bx = b.point.x + b.origin.x;
        if (ay - by).abs() <= 5 {
            ax.cmp(&bx).then(a.name.cmp(&b.name))
        } else {
            ay.cmp(&by).then(ax.cmp(&bx))
        }
    });
}

fn run_matches(
    exec: &mut Executor<'_>,
    results: &[NamedPoint],
    coords: &sqyre_domain::CoordinateOutputs,
    run_branch_on_no_find: bool,
    subactions: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    let mut first: Option<(i32, i32)> = None;
    for np in results {
        let x = np.point.x + np.origin.x;
        let y = np.point.y + np.origin.y;
        if first.is_none() {
            first = Some((x, y));
        }
        set_coord_outputs(macro_, coords, x, y);
        if let Some(meta) = &np.meta {
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
            macro_
                .variables
                .set("ImagePixelWidth", ScalarValue::Int(np.tmpl_w as i64));
            macro_
                .variables
                .set("ImagePixelHeight", ScalarValue::Int(np.tmpl_h as i64));
        }
        match run_children_flow(exec, subactions, macro_) {
            Err(ExecError::Flow(FlowSignal::Break)) => break,
            Err(ExecError::Flow(FlowSignal::Continue)) => continue,
            Err(e) => return Err(e),
            Ok(()) => {}
        }
    }
    if results.is_empty() && run_branch_on_no_find {
        run_children_flow(exec, subactions, macro_)?;
    }
    if let Some((x, y)) = first {
        set_coord_outputs(macro_, coords, x, y);
    }
    Ok(())
}

fn run_children_flow(exec: &mut Executor<'_>, children: &[Action], macro_: &mut Macro) -> Result<()> {
    for child in children {
        execute_action(exec, child, macro_)?;
    }
    Ok(())
}

fn set_coord_outputs(macro_: &mut Macro, coords: &sqyre_domain::CoordinateOutputs, x: i32, y: i32) {
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

fn retry_while_not_found(
    exec: &mut Executor<'_>,
    wait: &sqyre_domain::WaitTilFoundConfig,
    default_interval_ms: i32,
    mut retry: impl FnMut(&mut Executor<'_>) -> Result<bool>,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(wait.wait_til_found_seconds.max(0) as u64);
    let mut interval = wait.effective_interval_ms(default_interval_ms).max(1);
    let max_interval = (interval * 5).min(2000).max(interval);
    while Instant::now() < deadline {
        exec.check_stopped()?;
        exec.automation.milli_sleep(interval);
        if retry(exec)? {
            return Ok(());
        }
        if interval < max_interval {
            interval = (interval * 2).min(max_interval);
        }
    }
    Ok(())
}

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
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not find pixel".into()));
    };

    let mut found = try_find_pixel(exec, search_area, target_color, *color_tolerance, macro_);
    if wait.wait_until_found_active() && found.is_none() {
        let _ = retry_while_not_found(exec, wait, 100, |exec| {
            found = try_find_pixel(exec, search_area, target_color, *color_tolerance, macro_);
            Ok(found.is_some())
        })?;
    }
    if let Some((x, y)) = found {
        set_coord_outputs(macro_, coords, x, y);
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

/// `TemplateMatcher` over `sqyre-match` (injectable in tests).
#[derive(Debug, Default)]
pub struct MatchFacade {
    pub close_matches_distance: i32,
}

impl MatchFacade {
    pub fn new() -> Self {
        Self {
            close_matches_distance: DEFAULT_CLOSE_MATCHES_DISTANCE,
        }
    }
}

impl TemplateMatcher for MatchFacade {
    fn find_matches(
        &self,
        search: &ImageBuf,
        template: &ImageBuf,
        mask: Option<&ImageBuf>,
        threshold: f32,
        blur: i32,
    ) -> std::result::Result<Vec<Point>, sqyre_match::MatchError> {
        let mask_bytes = mask.map(|m| {
            if m.channels == 1 {
                m.data.clone()
            } else {
                mask_as_u8(m)
            }
        });
        let kernel = search_blur_kernel(blur);
        let search_blurred = blur_image(search, kernel)?;
        find_template_matches(
            &search_blurred,
            template,
            mask_bytes.as_deref(),
            threshold,
            blur,
            if self.close_matches_distance > 0 {
                self.close_matches_distance
            } else {
                DEFAULT_CLOSE_MATCHES_DISTANCE
            },
        )
    }
}
