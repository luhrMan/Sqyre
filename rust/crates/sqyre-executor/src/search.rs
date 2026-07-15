//! Image search / find-pixel orchestration (Go `executor_search.go`).

use crate::backends::{DesktopRect, ItemMeta, TemplateMatcher};
use crate::error::{ExecError, FlowSignal, Result};
use crate::highlight::{highlight_clear, highlight_fill};
use crate::run::{execute_action, Executor};
use crate::action_log::{crop_match_preview, draw_rect_rgb};
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

    highlight_fill(exec.highlighter, &macro_.name, action.id, 0.0);
    let action_id = action.id;
    let macro_name = macro_.name.clone();
    let result = (|| {
        let mut results =
            capture_and_match(exec, action_id, targets, search_area, *tolerance, *blur, macro_);

        if wait.wait_until_found_active() && results.is_empty() {
            exec.log(
                action_id,
                format!(
                    "Image Search: waiting up to {}s until found",
                    wait.wait_til_found_seconds
                ),
            );
            let _ = retry_while_not_found(exec, wait, 100, |exec| {
                results = capture_and_match(
                    exec,
                    action_id,
                    targets,
                    search_area,
                    *tolerance,
                    *blur,
                    macro_,
                );
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
                    action_id,
                    targets,
                    &results,
                    coords,
                    *run_branch_on_no_find,
                    subactions,
                    macro_,
                )?;
                results = capture_and_match(
                    exec,
                    action_id,
                    targets,
                    search_area,
                    *tolerance,
                    *blur,
                    macro_,
                );
                let interval = wait.effective_interval_ms(100).max(1);
                exec.automation.milli_sleep(interval);
            }
            return Ok(());
        }

        run_matches(
            exec,
            action_id,
            targets,
            &results,
            coords,
            *run_branch_on_no_find,
            subactions,
            macro_,
        )
    })();
    highlight_clear(exec.highlighter, &macro_name, action_id);
    result
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
    action_id: sqyre_domain::ActionId,
    targets: &[String],
    search_area: &sqyre_domain::CoordinateRef,
    tolerance: f64,
    blur: i32,
    macro_: &Macro,
) -> Vec<NamedPoint> {
    let Some(resolver) = exec.resolver else {
        exec.log(action_id, "Image Search: missing CoordinateResolver");
        return Vec::new();
    };
    if exec.capturer.is_none() {
        exec.log(action_id, "Image Search: missing ScreenCapturer");
        return Vec::new();
    }
    let Some(icons) = exec.icons else {
        exec.log(action_id, "Image Search: missing IconStore");
        return Vec::new();
    };

    let (lx, ty, rx, by) = match resolver.resolve_search_area(search_area, macro_) {
        Ok(v) => v,
        Err(e) => {
            exec.log(
                action_id,
                format!(
                    "Image Search: resolve search area {}: {e}",
                    search_area.display_label()
                ),
            );
            return Vec::new();
        }
    };
    let w = (rx - lx).max(0);
    let h = (by - ty).max(0);
    exec.log(
        action_id,
        format!(
            "Image Searching | {targets:?} in X1:{lx} Y1:{ty} X2:{rx} Y2:{by}, width:{w} height:{h}"
        ),
    );

    let (img, origin) = match exec
        .capturer
        .as_mut()
        .unwrap()
        .capture_search_area(lx, ty, rx, by)
    {
        Ok(v) => v,
        Err(e) => {
            exec.log(action_id, format!("Image Search: capture: {e}"));
            return Vec::new();
        }
    };
    let search = rgba_to_rgb_buf(&img);
    exec.log_image(action_id, "1. Capture (search area)", &search);
    let kernel = search_blur_kernel(blur);
    let search_blurred = match blur_image(&search, kernel) {
        Ok(b) => b,
        Err(e) => {
            exec.log(action_id, format!("Image Search: blur: {e}"));
            return Vec::new();
        }
    };
    if blur > 0 {
        exec.log_image(
            action_id,
            &format!("2. Preprocess — blur search (amount={blur})"),
            &search_blurred,
        );
    }

    let threshold = tolerance as f32;
    let mut out = Vec::new();
    let match_started = Instant::now();
    for target in targets {
        let paths = icons.variant_paths(target);
        if paths.is_empty() {
            exec.log(
                action_id,
                format!("Image Search: no icon variants for {target}"),
            );
            continue;
        }
        let meta = icons.item_meta(target);
        let mask_path = icons.mask_path(target);
        for (variant_i, path) in paths.into_iter().enumerate() {
            let template = match load_rgb_image(&path) {
                Ok(t) => t,
                Err(e) => {
                    exec.log(action_id, format!("Image Search: load {path:?}: {e}"));
                    continue;
                }
            };
            let variant_label = if variant_i == 0 {
                target.to_string()
            } else {
                format!("{target} variant {}", variant_i + 1)
            };

            let mut steps: Vec<(String, ImageBuf)> = Vec::new();
            steps.push((
                "0. Search area (match input)".into(),
                search_blurred.clone(),
            ));
            steps.push(("1. Item template".into(), template.clone()));
            let tmpl_kernel = search_blur_kernel(blur);
            if blur > 0 {
                if let Ok(tmpl_blurred) = blur_image(&template, tmpl_kernel) {
                    steps.push((
                        format!("2. Preprocess — blur item (amount={blur})"),
                        tmpl_blurred,
                    ));
                }
            }

            let mask_bytes = mask_path
                .as_ref()
                .and_then(|p| load_rgb_image(p).ok())
                .map(|m| {
                    let resized = resize_mask(&m, template.width, template.height);
                    steps.push(("3. Mask".into(), resized.clone()));
                    mask_as_u8(&resized)
                });

            exec.log(
                action_id,
                format!(
                    "Image Search: matching {variant_label} ({}x{}) against {}x{}",
                    template.width, template.height, search_blurred.width, search_blurred.height
                ),
            );
            let t0 = Instant::now();
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
                    exec.log(action_id, format!("Image Search match: {e}"));
                    exec.log_item_pipeline(
                        action_id,
                        variant_label,
                        format!("match error: {e}"),
                        &template,
                        &steps,
                        vec![format!("Error: {e}")],
                    );
                    continue;
                }
            };
            let match_ms = t0.elapsed().as_secs_f64() * 1000.0;
            exec.log(
                action_id,
                format!(
                    "Image Search: {variant_label} → {} match(es) in {:.0}ms",
                    matches.len(),
                    match_ms
                ),
            );

            let half_w = (template.width / 2) as i32;
            let half_h = (template.height / 2) as i32;
            let tw = template.width as i32;
            let th = template.height as i32;
            let mut details = vec![
                format!(
                    "Template {}×{} · search {}×{} · threshold={threshold:.3} · blur={blur}",
                    template.width, template.height, search_blurred.width, search_blurred.height
                ),
                format!("Match time: {match_ms:.0}ms · {} hit(s)", matches.len()),
            ];
            let mut item_overlay = search.clone();
            const MAX_MATCH_PREVIEWS: usize = 8;
            for (mi, mut p) in matches.into_iter().enumerate() {
                let local_tl_x = p.x;
                let local_tl_y = p.y;
                draw_rect_rgb(
                    &mut item_overlay,
                    local_tl_x,
                    local_tl_y,
                    local_tl_x + tw - 1,
                    local_tl_y + th - 1,
                    [255, 40, 40],
                );
                if mi < MAX_MATCH_PREVIEWS {
                    if let Some(crop) =
                        crop_match_preview(&search, local_tl_x, local_tl_y, tw, th, 12)
                    {
                        steps.push((
                            format!(
                                "Find #{} — crop around ({local_tl_x},{local_tl_y})",
                                mi + 1
                            ),
                            crop,
                        ));
                    }
                }
                p.x += half_w;
                p.y += half_h;
                let screen_x = origin.x + p.x;
                let screen_y = origin.y + p.y;
                details.push(format!(
                    "Find #{}: center local ({}, {}) → screen ({screen_x}, {screen_y}) · box TL ({local_tl_x}, {local_tl_y}) size {tw}×{th}",
                    mi + 1,
                    p.x,
                    p.y,
                ));
                out.push(NamedPoint {
                    name: target.clone(),
                    point: p,
                    origin,
                    meta: meta.clone(),
                    tmpl_w: template.width as i32,
                    tmpl_h: template.height as i32,
                });
            }
            let find_count = details.iter().filter(|d| d.starts_with("Find #")).count();
            if find_count == 0 {
                details.push("No matches found for this item.".into());
            } else {
                steps.push(("Where found (all matches)".into(), item_overlay));
            }
            let summary = if find_count == 0 {
                format!("0 matches · {match_ms:.0}ms")
            } else {
                format!("{find_count} match(es) · {match_ms:.0}ms")
            };

            exec.log_item_pipeline(
                action_id,
                variant_label,
                summary,
                &template,
                &steps,
                details,
            );
        }
    }
    exec.log(
        action_id,
        format!(
            "Image Search: capture+match done in {:.0}ms ({} raw hit(s))",
            match_started.elapsed().as_secs_f64() * 1000.0,
            out.len()
        ),
    );
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
    action_id: sqyre_domain::ActionId,
    targets: &[String],
    results: &[NamedPoint],
    coords: &sqyre_domain::CoordinateOutputs,
    run_branch_on_no_find: bool,
    subactions: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    let mut found_names: Vec<&str> = results.iter().map(|np| np.name.as_str()).collect();
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

    let mut first: Option<(i32, i32)> = None;
    let total = results.len();
    for (count, np) in results.iter().enumerate() {
        if total > 0 {
            highlight_fill(
                exec.highlighter,
                &macro_.name,
                action_id,
                count as f64 / total as f64,
            );
        }
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
        exec.log(
            action.id,
            format!("FindPixel: found matching pixel at screen ({x}, {y})"),
        );
        set_coord_outputs(macro_, coords, x, y);
    } else {
        exec.log(action.id, "FindPixel: pixel not found");
    }
    Ok(())
}

/// OCR leaf action (Go `executeOcr`): capture → preprocess → recognize → write vars.
/// Errors are logged and the macro continues (Go behavior).
pub(crate) fn execute_ocr(
    exec: &mut Executor<'_>,
    action: &Action,
    macro_: &mut Macro,
) -> Result<()> {
    let ActionKind::Ocr {
        target,
        search_area,
        output_variable,
        coords,
        wait,
        blur,
        min_threshold,
        resize,
        grayscale,
        threshold_otsu,
        threshold_invert,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not ocr".into()));
    };

    let action_id = action.id;
    let mut shot = run_ocr_once(
        exec,
        action_id,
        search_area,
        target,
        *blur,
        *min_threshold,
        *resize,
        *grayscale,
        *threshold_otsu,
        *threshold_invert,
        macro_,
    );

    if wait.wait_until_found_active() {
        let target_ok = shot
            .as_ref()
            .is_some_and(|s| s.text.contains(target.as_str()));
        if !target_ok {
            exec.log(
                action_id,
                format!(
                    "OCR: waiting up to {}s until text contains {target:?}",
                    wait.wait_til_found_seconds
                ),
            );
            let _ = retry_while_not_found(exec, wait, 500, |exec| {
                shot = run_ocr_once(
                    exec,
                    action_id,
                    search_area,
                    target,
                    *blur,
                    *min_threshold,
                    *resize,
                    *grayscale,
                    *threshold_otsu,
                    *threshold_invert,
                    macro_,
                );
                Ok(shot
                    .as_ref()
                    .is_some_and(|s| s.text.contains(target.as_str())))
            })?;
        }
    }

    let Some(result) = shot else {
        // Capture/OCR failed — Go continues the macro without writing outputs.
        return Ok(());
    };

    if !output_variable.is_empty() {
        macro_
            .variables
            .set(output_variable, ScalarValue::String(result.text.clone()));
    }
    set_coord_outputs(macro_, coords, result.x, result.y);
    exec.log(
        action_id,
        format!(
            "OCR: {} chars → {:?} at ({}, {})",
            result.text.len(),
            output_variable,
            result.x,
            result.y
        ),
    );
    Ok(())
}

struct OcrShot {
    text: String,
    x: i32,
    y: i32,
}

fn run_ocr_once(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    search_area: &sqyre_domain::CoordinateRef,
    target: &str,
    blur: i32,
    min_threshold: i32,
    resize: f64,
    grayscale: bool,
    threshold_otsu: bool,
    threshold_invert: bool,
    macro_: &Macro,
) -> Option<OcrShot> {
    let Some(resolver) = exec.resolver else {
        exec.log(action_id, "OCR: missing CoordinateResolver");
        return None;
    };
    if exec.capturer.is_none() {
        exec.log(action_id, "OCR: missing ScreenCapturer");
        return None;
    }
    let Some(ocr) = exec.ocr else {
        exec.log(action_id, "OCR: missing OcrEngine");
        return None;
    };

    let (lx, ty, rx, by) = match resolver.resolve_search_area(search_area, macro_) {
        Ok(v) => v,
        Err(e) => {
            exec.log(
                action_id,
                format!(
                    "OCR: resolve search area {}: {e}",
                    search_area.display_label()
                ),
            );
            return None;
        }
    };
    exec.log(
        action_id,
        format!(
            "{target} OCR search | {} in X1:{lx} Y1:{ty} X2:{rx} Y2:{by}",
            search_area.display_label()
        ),
    );

    let (img, origin) = match exec
        .capturer
        .as_mut()
        .unwrap()
        .capture_search_area(lx, ty, rx, by)
    {
        Ok(v) => v,
        Err(e) => {
            exec.log(action_id, format!("OCR: capture: {e}"));
            return None;
        }
    };
    let search_center_x = origin.x + origin.w / 2;
    let search_center_y = origin.y + origin.h / 2;
    let rgb = rgba_to_rgb_buf(&img);
    exec.log_image(action_id, "Capture (raw)", &rgb);
    let opts = sqyre_vision::OcrPreprocessOptions::from_action_fields(
        grayscale,
        blur,
        min_threshold,
        resize,
        threshold_otsu,
        threshold_invert,
    );
    let collect = exec.logger.is_some();
    let (processed, scale, steps) =
        match sqyre_vision::preprocess_for_ocr_with_steps(&rgb, opts, collect) {
            Ok(v) => v,
            Err(e) => {
                exec.log(action_id, format!("OCR: preprocess: {e}"));
                return None;
            }
        };
    for step in &steps {
        exec.log_image(action_id, &step.label, &step.image);
    }
    let recognized = match ocr.recognize(&processed) {
        Ok(v) => v,
        Err(e) => {
            exec.log(action_id, format!("OCR: {e}"));
            return None;
        }
    };

    exec.log(
        action_id,
        format!(
            "OCR full text ({} chars): {}",
            recognized.text.len(),
            recognized.text
        ),
    );
    exec.log(
        action_id,
        format!("OCR words found: {}", recognized.words.len()),
    );
    for (i, w) in recognized.words.iter().enumerate() {
        let ww = (w.right - w.left).max(0);
        let wh = (w.bottom - w.top).max(0);
        exec.log(
            action_id,
            format!(
                "  word[{i}] {:?} box=({},{})-({},{}) size={ww}×{wh}",
                w.word, w.left, w.top, w.right, w.bottom
            ),
        );
    }

    // Overlay word boxes on an RGB copy of the OCR input for the logs UI.
    if !recognized.words.is_empty() {
        let mut overlay = if processed.channels == 1 {
            gray_to_rgb(&processed)
        } else {
            processed.clone()
        };
        for w in &recognized.words {
            draw_rect_rgb(
                &mut overlay,
                w.left,
                w.top,
                w.right.saturating_sub(1),
                w.bottom.saturating_sub(1),
                [40, 220, 80],
            );
        }
        exec.log_image(action_id, "OCR word boxes", &overlay);
    }

    let mut out_x = search_center_x;
    let mut out_y = search_center_y;
    let resize_scale = if scale > 0.0 { scale } else { 1.0 };
    if let Some((bx, by)) = sqyre_vision::find_target_in_boxes(&recognized.words, target) {
        out_x = origin.x + (bx as f64 / resize_scale) as i32;
        out_y = origin.y + (by as f64 / resize_scale) as i32;
        exec.log(
            action_id,
            format!(
                "OCR target {target:?} matched at image ({bx}, {by}) → screen ({out_x}, {out_y}) (scale={resize_scale:.3})"
            ),
        );
    } else {
        exec.log(
            action_id,
            format!("OCR target {target:?} not found among word boxes"),
        );
    }

    Some(OcrShot {
        text: recognized.text,
        x: out_x,
        y: out_y,
    })
}

fn gray_to_rgb(img: &ImageBuf) -> ImageBuf {
    debug_assert_eq!(img.channels, 1);
    let mut data = Vec::with_capacity(img.width * img.height * 3);
    for &v in &img.data {
        data.extend_from_slice(&[v, v, v]);
    }
    ImageBuf::from_raw(img.width, img.height, 3, data)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{
        DesktopRect, IconStore, ItemMeta, RecordingBackend, RecordingCapturer, TemplateMatcher,
    };
    use crate::run::{execute_macro_with, ExecDeps};
    use crate::SharedActionLog;
    use image::{Rgba, RgbaImage};
    use sqyre_domain::{
        root_loop, Action, ActionId, ActionKind, CoordinateOutputs, CoordinateRef, Macro,
        ScalarValue, WaitTilFoundConfig,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    struct FixedArea;

    impl crate::backends::CoordinateResolver for FixedArea {
        fn resolve_point(
            &self,
            _r: &CoordinateRef,
            _macro_: &Macro,
        ) -> std::result::Result<(i32, i32), String> {
            Ok((0, 0))
        }
        fn resolve_search_area(
            &self,
            _r: &CoordinateRef,
            _macro_: &Macro,
        ) -> std::result::Result<(i32, i32, i32, i32), String> {
            Ok((100, 200, 110, 210))
        }
    }

    struct MapIcons {
        paths: HashMap<String, PathBuf>,
        meta: HashMap<String, ItemMeta>,
    }

    impl IconStore for MapIcons {
        fn variant_paths(&self, target: &str) -> Vec<PathBuf> {
            self.paths.get(target).cloned().into_iter().collect()
        }
        fn mask_path(&self, _target: &str) -> Option<PathBuf> {
            None
        }
        fn item_meta(&self, target: &str) -> Option<ItemMeta> {
            self.meta.get(target).cloned()
        }
    }

    fn named(name: &str, x: i32, y: i32, ox: i32, oy: i32) -> NamedPoint {
        NamedPoint {
            point: Point { x, y },
            origin: DesktopRect {
                x: ox,
                y: oy,
                w: 10,
                h: 10,
            },
            meta: None,
            tmpl_w: 1,
            tmpl_h: 1,
            name: name.into(),
        }
    }

    #[test]
    fn sort_points_uses_row_band_then_x() {
        let mut pts = vec![
            named("b", 20, 10, 0, 0),
            named("a", 5, 12, 0, 0), // same band (abs dy <= 5), lower x → first
            named("c", 1, 30, 0, 0), // next row
        ];
        sort_points(&mut pts);
        assert_eq!(
            pts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn set_coord_outputs_writes_variables() {
        let mut macro_ = Macro::new("t", 0, vec![]);
        let coords = CoordinateOutputs {
            output_x_variable: "fx".into(),
            output_y_variable: "fy".into(),
        };
        set_coord_outputs(&mut macro_, &coords, 11, 22);
        assert_eq!(
            macro_.variables.get("fx").map(|v| v.as_display()),
            Some("11".into())
        );
        assert_eq!(
            macro_.variables.get("fy").map(|v| v.as_display()),
            Some("22".into())
        );
    }

    #[test]
    fn find_pixel_uses_collection_cell_search_area() {
        let mut img = RgbaImage::new(4, 4);
        for p in img.pixels_mut() {
            *p = Rgba([0, 0, 0, 255]);
        }
        img.put_pixel(1, 1, Rgba([0, 255, 0, 255]));

        struct CollectionOnly;
        impl crate::backends::CoordinateResolver for CollectionOnly {
            fn resolve_point(
                &self,
                _r: &CoordinateRef,
                _macro_: &Macro,
            ) -> std::result::Result<(i32, i32), String> {
                Err("point".into())
            }
            fn resolve_search_area(
                &self,
                r: &CoordinateRef,
                _macro_: &Macro,
            ) -> std::result::Result<(i32, i32, i32, i32), String> {
                assert!(r.is_collection(), "expected collection ref, got {r:?}");
                assert_eq!(r.as_str(), "Demo~bag@1,2-1,2");
                Ok((50, 60, 54, 64))
            }
        }

        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(img),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = CollectionOnly;
        let logger = SharedActionLog::new();
        let find_id = ActionId::new();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: find_id,
            kind: ActionKind::FindPixel {
                name: "green".into(),
                search_area: CoordinateRef::collection("Demo", "bag", 1, 2, 1, 2),
                target_color: "#00ff00".into(),
                color_tolerance: 0,
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                run_branch_on_no_find: false,
                order: Default::default(),
                subactions: vec![],
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert_eq!(
            macro_.variables.get("foundX").map(|v| v.as_display()),
            Some("51".into())
        );
        assert_eq!(
            macro_.variables.get("foundY").map(|v| v.as_display()),
            Some("61".into())
        );
        assert!(capturer.log.iter().any(|e| e == "rect:50,60,4,4"), "{:?}", capturer.log);
    }

    #[test]
    fn find_pixel_sets_coords_and_logs() {
        let mut img = RgbaImage::new(10, 10);
        for p in img.pixels_mut() {
            *p = Rgba([0, 0, 0, 255]);
        }
        img.put_pixel(3, 5, Rgba([255, 0, 0, 255]));

        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(img),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = FixedArea;
        let logger = SharedActionLog::new();
        let find_id = ActionId::new();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: find_id,
            kind: ActionKind::FindPixel {
                name: "red".into(),
                search_area: CoordinateRef("Prog~Box".into()),
                target_color: "#ff0000".into(),
                color_tolerance: 0,
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                run_branch_on_no_find: false,
                order: Default::default(),
                subactions: vec![],
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert_eq!(
            macro_.variables.get("foundX").map(|v| v.as_display()),
            Some("103".into()) // 100 origin + 3
        );
        assert_eq!(
            macro_.variables.get("foundY").map(|v| v.as_display()),
            Some("205".into()) // 200 origin + 5
        );
        let lines = logger.lines_for(find_id);
        assert!(
            lines.iter().any(|l| l.contains("found matching pixel")),
            "{lines:?}"
        );
        assert!(capturer.log.iter().any(|e| e.starts_with("rect:")));
    }

    #[test]
    fn find_pixel_not_found_logs() {
        let img = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255]));
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(img),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = FixedArea;
        let logger = SharedActionLog::new();
        let find_id = ActionId::new();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: find_id,
            kind: ActionKind::FindPixel {
                name: "red".into(),
                search_area: CoordinateRef("Prog~Box".into()),
                target_color: "#ff0000".into(),
                color_tolerance: 0,
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs::defaults(),
                run_branch_on_no_find: false,
                order: Default::default(),
                subactions: vec![],
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();
        let lines = logger.lines_for(find_id);
        assert!(
            lines.iter().any(|l| l.contains("pixel not found")),
            "{lines:?}"
        );
    }

    #[test]
    fn image_search_no_find_runs_branch() {
        let img = RgbaImage::from_pixel(8, 8, Rgba([10, 20, 30, 255]));
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("tmpl.png");
        // Distinct template that will not match the solid search image well at high threshold.
        let mut tmpl = RgbaImage::new(4, 4);
        for (i, p) in tmpl.pixels_mut().enumerate() {
            *p = Rgba([(i as u8).wrapping_mul(37), 200, 50, 255]);
        }
        tmpl.save(&tmpl_path).unwrap();

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), tmpl_path)]),
            meta: HashMap::from([(
                "Prog~Item".into(),
                ItemMeta {
                    name: "Item".into(),
                    stack_max: 99,
                    cols: 2,
                    rows: 2,
                },
            )]),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(img),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = FixedArea;
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec!["Prog~Item".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                row_split: 0,
                col_split: 0,
                tolerance: 0.99,
                blur: 0,
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs::defaults(),
                run_branch_on_no_find: true,
                order: Default::default(),
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(13),
                    },
                }],
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: Some(&resolver),
                icons: Some(&icons),
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert!(backend.log.iter().any(|e| e == "sleep:13"));
        let lines = logger.lines_for(search_id);
        assert!(
            lines.iter().any(|l| l.contains("Image Searching")),
            "expected search-area log before match: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("matching")),
            "expected per-target match log: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Total # found: 0")),
            "{lines:?}"
        );
        let entries = logger.entries_for(search_id);
        let image_labels: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                crate::ActionLogEntry::Image(img) => Some(img.label.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            image_labels.iter().any(|l| l.contains("Capture")),
            "expected capture image in logs: {image_labels:?}"
        );
        let item_titles: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                crate::ActionLogEntry::ItemPipeline { title, .. } => Some(title.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            item_titles.iter().any(|t| t.contains("Prog~Item") || t.contains("Item")),
            "expected item pipeline card in logs: {item_titles:?}"
        );
    }

    #[test]
    fn image_search_break_stops_match_loop() {
        // Two synthetic matches via run_matches directly.
        let mut backend = RecordingBackend::default();
        let mut exec = crate::run::Executor::new(&mut backend);
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        let results = vec![
            named("a", 1, 1, 0, 0),
            named("b", 2, 2, 0, 0),
        ];
        let subactions = vec![
            Action {
                id: ActionId::new(),
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(4),
                },
            },
            Action {
                id: ActionId::new(),
                kind: ActionKind::Break,
            },
        ];
        run_matches(
            &mut exec,
            ActionId::new(),
            &["a".into(), "b".into()],
            &results,
            &CoordinateOutputs::defaults(),
            false,
            &subactions,
            &mut macro_,
        )
        .unwrap();
        assert_eq!(
            backend
                .log
                .iter()
                .filter(|e| e.as_str() == "sleep:4")
                .count(),
            1,
            "break should stop after first match: {:?}",
            backend.log
        );
    }

    #[test]
    fn match_facade_finds_exact_template() {
        // blur=0 still maps to kernel 5 (Go searchBlurKernel); pattern must survive that.
        let mut tmpl = ImageBuf::new(10, 10, 3, 40);
        for y in 0..10 {
            for x in 0..10 {
                let o = tmpl.pixel_offset(x, y);
                tmpl.data[o] = (x * 17 + y * 9) as u8;
                tmpl.data[o + 1] = (x * 3 + y * 29) as u8;
                tmpl.data[o + 2] = (255 - x * 11) as u8;
            }
        }
        let mut search = ImageBuf::new(50, 50, 3, 30);
        search.stamp(&tmpl, 15, 18);
        let facade = MatchFacade::new();
        let hits = facade
            .find_matches(&search, &tmpl, None, 0.7, 0)
            .unwrap();
        assert!(
            hits.iter()
                .any(|p| (p.x - 15).abs() <= 2 && (p.y - 18).abs() <= 2),
            "expected peak near (15,18), got {hits:?}"
        );
    }

    #[test]
    fn ocr_writes_text_and_target_coords() {
        use crate::backends::{FixedOcrEngine, OcrResult};
        use sqyre_domain::MatchOrder;
        use sqyre_vision::OcrWordBox;

        let img = RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255]));
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(img),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = FixedArea;
        let ocr = FixedOcrEngine {
            result: OcrResult {
                text: "Hello Submit Button".into(),
                words: vec![
                    OcrWordBox {
                        word: "Hello".into(),
                        left: 0,
                        top: 0,
                        right: 40,
                        bottom: 20,
                    },
                    OcrWordBox {
                        word: "Submit".into(),
                        left: 50,
                        top: 0,
                        right: 110,
                        bottom: 20,
                    },
                    OcrWordBox {
                        word: "Button".into(),
                        left: 120,
                        top: 0,
                        right: 180,
                        bottom: 20,
                    },
                ],
            },
            ..Default::default()
        };
        let log = SharedActionLog::new();
        let ocr_id = ActionId::new();

        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: ocr_id,
            kind: ActionKind::Ocr {
                name: "read".into(),
                target: "Submit".into(),
                search_area: CoordinateRef("prog~box".into()),
                output_variable: "ocrText".into(),
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                wait: WaitTilFoundConfig::default(),
                run_branch_on_no_find: false,
                blur: 1,
                min_threshold: 0,
                resize: 1.0,
                grayscale: true,
                threshold_otsu: false,
                threshold_invert: false,
                order: MatchOrder::default(),
                subactions: vec![],
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: Some(&ocr),
                stop_flag: None,
                logger: Some(&log),
                highlighter: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert_eq!(
            macro_.variables.get("ocrText").map(|v| v.as_display()),
            Some("Hello Submit Button".into())
        );
        // FixedArea resolves to (100,200)-(110,210); box center (80,10) + origin
        assert_eq!(
            macro_.variables.get("foundX").map(|v| v.as_display()),
            Some("180".into())
        );
        assert_eq!(
            macro_.variables.get("foundY").map(|v| v.as_display()),
            Some("210".into())
        );
        let entries = log.entries_for(ocr_id);
        let image_labels: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                crate::ActionLogEntry::Image(img) => Some(img.label.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            image_labels.iter().any(|l| l.contains("Capture")),
            "expected capture image: {image_labels:?}"
        );
        assert!(
            image_labels.iter().any(|l| l.contains("Ready for OCR") || l.contains("Grayscale")),
            "expected preprocess step images: {image_labels:?}"
        );
        assert!(
            image_labels.iter().any(|l| l.contains("word boxes")),
            "expected OCR word-box overlay: {image_labels:?}"
        );
        let lines = log.lines_for(ocr_id);
        assert!(
            lines.iter().any(|l| l.contains("OCR full text")),
            "expected full OCR text log: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("word[") && l.contains("Hello")),
            "expected per-word OCR detail: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("word[") && l.contains("Submit")),
            "expected Submit word in OCR detail: {lines:?}"
        );
    }
}
