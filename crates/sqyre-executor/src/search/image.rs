//! Image search action: capture → match template variants → run children per hit.

use super::common::{
    apply_detection_hits, run_detection_shell, sort_hits, DetectionExtras, DetectionHit,
};
use crate::action_log::{crop_match_preview, draw_rect_rgb};
use crate::backends::{DesktopRect, ItemMeta};
use crate::error::{ExecError, Result};
use crate::highlight::{highlight_clear, highlight_fill};
use crate::run::Executor;
use rayon::prelude::*;
use sqyre_domain::{Action, ActionKind, Macro, MatchOrder};
use sqyre_match::{
    blur_image_owned, find_template_matches_preblurred_with_integrals, prepare_search_integrals,
    search_blur_kernel, ImageBuf, MatchMethod, Point, DEFAULT_CLOSE_MATCHES_DISTANCE,
};
use sqyre_vision::{get_cached_blurred_template, get_cached_image_mask, load_rgb_image};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

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
        match_method,
        detection,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not image search".into()));
    };
    let sqyre_domain::DetectionBranch {
        wait,
        coords,
        order,
        subactions,
        else_actions,
    } = detection;

    highlight_fill(exec.deps.highlighter, &macro_.name, action.id, 0.0);
    let action_id = action.id;
    let macro_name = macro_.name.clone();
    let order = order.clone();
    let result = (|| {
        // Log wait intent once before the shared shell arms retries.
        let results0 = capture_and_match(
            exec,
            action_id,
            targets,
            search_area,
            *tolerance,
            *blur,
            *match_method,
            &order,
            macro_,
        )?;
        if wait.wait_until_found_active() && results0.is_empty() {
            exec.log(
                action_id,
                format!(
                    "Image Search: waiting up to {}s until found",
                    wait.wait_til_found_seconds
                ),
            );
        } else if wait.wait_while_found_active() && !results0.is_empty() {
            exec.log(
                action_id,
                format!(
                    "Image Search: waiting up to {}s while found",
                    wait.wait_til_found_seconds
                ),
            );
        }

        let mut initial = Some(results0);
        run_detection_shell(
            exec,
            macro_,
            wait,
            100,
            100,
            |exec, macro_| {
                if let Some(first) = initial.take() {
                    return Ok(first);
                }
                capture_and_match(
                    exec,
                    action_id,
                    targets,
                    search_area,
                    *tolerance,
                    *blur,
                    *match_method,
                    &order,
                    macro_,
                )
            },
            |results| !results.is_empty(),
            |exec, macro_, results, pass| {
                apply_detection_hits(
                    exec,
                    action_id,
                    targets,
                    results,
                    coords,
                    subactions,
                    else_actions,
                    macro_,
                    pass,
                )
            },
        )
    })();
    highlight_clear(exec.deps.highlighter, &macro_name, action_id);
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

struct VariantJob {
    target: String,
    variant_i: usize,
    path: PathBuf,
    meta: Option<ItemMeta>,
    mask_path: Option<PathBuf>,
}

struct VariantMatchOutcome {
    job: VariantJob,
    tmpl_w: usize,
    tmpl_h: usize,
    /// Unblurred template — only populated when pipeline logging is enabled.
    template_raw: Option<ImageBuf>,
    /// Blurred template used for matching (for pipeline steps).
    template_blurred: Arc<ImageBuf>,
    /// Mask bytes (kept as Arc until logging builds a preview ImageBuf).
    mask_bytes: Option<Arc<Vec<u8>>>,
    matches: std::result::Result<Vec<Point>, String>,
    match_ms: f64,
}

fn close_matches_distance(exec: &Executor<'_>) -> i32 {
    let d = exec.deps.close_matches_distance;
    if d > 0 {
        d
    } else {
        DEFAULT_CLOSE_MATCHES_DISTANCE
    }
}

fn to_match_method(m: sqyre_domain::TemplateMatchMethod) -> MatchMethod {
    match m {
        sqyre_domain::TemplateMatchMethod::Sqdiff => MatchMethod::Sqdiff,
        sqyre_domain::TemplateMatchMethod::SqdiffNormed => MatchMethod::SqdiffNormed,
        sqyre_domain::TemplateMatchMethod::Ccorr => MatchMethod::Ccorr,
        sqyre_domain::TemplateMatchMethod::CcorrNormed => MatchMethod::CcorrNormed,
        sqyre_domain::TemplateMatchMethod::Ccoeff => MatchMethod::Ccoeff,
        sqyre_domain::TemplateMatchMethod::CcoeffNormed => MatchMethod::CcoeffNormed,
    }
}

#[allow(clippy::too_many_arguments)]
fn capture_and_match(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    targets: &[String],
    search_area: &sqyre_domain::CoordinateRef,
    tolerance: f64,
    blur: i32,
    match_method: sqyre_domain::TemplateMatchMethod,
    order: &MatchOrder,
    macro_: &Macro,
) -> Result<Vec<DetectionHit>> {
    // Capture/resolve/blur failures are logged as misses so wait-until-found can retry
    // instead of aborting the macro (same policy as OCR / Find Pixel).
    let Some(resolver) = exec.deps.resolver else {
        exec.log(action_id, "Image Search: missing CoordinateResolver");
        return Ok(Vec::new());
    };
    if exec.deps.capturer.is_none() {
        exec.log(action_id, "Image Search: missing ScreenCapturer");
        return Ok(Vec::new());
    }
    let Some(icons) = exec.deps.icons else {
        exec.log(action_id, "Image Search: missing IconStore");
        return Ok(Vec::new());
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
            return Ok(Vec::new());
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

    let capture_started = Instant::now();
    let Some(capturer) = exec.deps.capturer.as_mut() else {
        exec.log(action_id, "Image Search: missing ScreenCapturer");
        return Ok(Vec::new());
    };
    let (img, origin) = match capturer.capture_search_area_rgb(lx, ty, rx, by) {
        Ok(v) => v,
        Err(e) => {
            exec.log(action_id, format!("Image Search: capture: {e}"));
            return Ok(Vec::new());
        }
    };
    let search = img.into_image_buf();
    exec.log_image(action_id, "1. Capture (search area)", &search);
    let kernel = search_blur_kernel(blur);
    let want_pipeline = exec.log_images_enabled();
    // Keep an unblurred copy only for diagnostics overlays / match crops.
    let search_raw = want_pipeline.then(|| search.clone());
    let search_blurred = match blur_image_owned(search, kernel) {
        Ok(b) => b,
        Err(e) => {
            exec.log(action_id, format!("Image Search: blur: {e}"));
            return Ok(Vec::new());
        }
    };
    if blur > 0 {
        exec.log_image(
            action_id,
            format!("2. Preprocess — blur search (amount={blur})"),
            &search_blurred,
        );
    }
    exec.log_timing(action_id, "capture+preprocess", capture_started.elapsed());

    let mut jobs = Vec::new();
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
            jobs.push(VariantJob {
                target: target.clone(),
                variant_i,
                path,
                meta: meta.clone(),
                mask_path: mask_path.clone(),
            });
        }
    }

    let threshold = tolerance as f32;
    let close_dist = close_matches_distance(exec);
    let match_started = Instant::now();
    let stop_flag: Option<&AtomicBool> = exec.deps.stop_flag;
    let search_integrals = std::sync::Arc::new(prepare_search_integrals(&search_blurred));
    let method = to_match_method(match_method);

    let outcomes: Vec<VariantMatchOutcome> = jobs
        .into_par_iter()
        .map(|job| {
            if stop_flag.is_some_and(|f| f.load(Ordering::SeqCst)) {
                return VariantMatchOutcome {
                    tmpl_w: 0,
                    tmpl_h: 0,
                    template_raw: None,
                    template_blurred: Arc::new(ImageBuf::new(1, 1, 3, 0)),
                    mask_bytes: None,
                    matches: Ok(Vec::new()),
                    match_ms: 0.0,
                    job,
                };
            }

            let template_blurred = match get_cached_blurred_template(&job.path, kernel) {
                Ok(t) => t,
                Err(e) => {
                    return VariantMatchOutcome {
                        tmpl_w: 0,
                        tmpl_h: 0,
                        template_raw: None,
                        template_blurred: Arc::new(ImageBuf::new(1, 1, 3, 0)),
                        mask_bytes: None,
                        matches: Err(format!("load {:?}: {e}", job.path)),
                        match_ms: 0.0,
                        job,
                    };
                }
            };
            let tmpl_w = template_blurred.width;
            let tmpl_h = template_blurred.height;

            let mask_bytes = job
                .mask_path
                .as_ref()
                .and_then(|p| get_cached_image_mask(p, tmpl_h, tmpl_w));

            let template_raw = if want_pipeline {
                load_rgb_image(&job.path).ok()
            } else {
                None
            };

            let t0 = Instant::now();
            let matches = find_template_matches_preblurred_with_integrals(
                &search_blurred,
                template_blurred.as_ref(),
                mask_bytes.as_deref().map(|m| m.as_slice()),
                threshold,
                close_dist,
                method,
                Some(search_integrals.as_ref()),
            )
            .map_err(|e| e.to_string());
            let match_ms = t0.elapsed().as_secs_f64() * 1000.0;

            VariantMatchOutcome {
                job,
                tmpl_w,
                tmpl_h,
                template_raw,
                template_blurred,
                mask_bytes,
                matches,
                match_ms,
            }
        })
        .collect();

    let mut out = Vec::new();
    for outcome in outcomes {
        let variant_label = if outcome.job.variant_i == 0 {
            outcome.job.target.clone()
        } else {
            format!(
                "{} variant {}",
                outcome.job.target,
                outcome.job.variant_i + 1
            )
        };

        if outcome.tmpl_w == 0 {
            if let Err(e) = &outcome.matches {
                exec.log(action_id, format!("Image Search: {e}"));
            }
            continue;
        }

        exec.log(
            action_id,
            format!(
                "Image Search: matching {variant_label} ({}x{}) against {}x{}",
                outcome.tmpl_w, outcome.tmpl_h, search_blurred.width, search_blurred.height
            ),
        );

        let thumbnail = outcome
            .template_raw
            .as_ref()
            .unwrap_or(outcome.template_blurred.as_ref());

        let mask_preview = if want_pipeline {
            outcome
                .mask_bytes
                .as_ref()
                .map(|m| ImageBuf::from_raw(outcome.tmpl_w, outcome.tmpl_h, 1, m.as_ref().clone()))
        } else {
            None
        };

        let matches = match outcome.matches {
            Ok(m) => m,
            Err(e) => {
                exec.log(action_id, format!("Image Search match: {e}"));
                if want_pipeline {
                    let mut steps: Vec<(&str, &ImageBuf)> = vec![
                        ("0. Search area (match input)", &search_blurred),
                        ("1. Item template", thumbnail),
                    ];
                    let blur_label;
                    if blur > 0 {
                        blur_label = format!("2. Preprocess — blur item (amount={blur})");
                        steps.push((blur_label.as_str(), outcome.template_blurred.as_ref()));
                    }
                    if let Some(mask) = &mask_preview {
                        steps.push(("3. Mask", mask));
                    }
                    exec.log_item_pipeline(
                        action_id,
                        variant_label,
                        format!("match error: {e}"),
                        thumbnail,
                        &steps,
                        vec![format!("Error: {e}")],
                    );
                }
                continue;
            }
        };

        exec.log(
            action_id,
            format!(
                "Image Search: {variant_label} → {} match(es) in {:.0}ms",
                matches.len(),
                outcome.match_ms
            ),
        );

        let half_w = (outcome.tmpl_w / 2) as i32;
        let half_h = (outcome.tmpl_h / 2) as i32;
        let tw = outcome.tmpl_w as i32;
        let th = outcome.tmpl_h as i32;

        if want_pipeline {
            let blur_label = format!("2. Preprocess — blur item (amount={blur})");
            let mut owned_steps: Vec<(String, ImageBuf)> = Vec::new();
            let mut details = vec![
                format!(
                    "Template {}×{} · search {}×{} · threshold={threshold:.3} · blur={blur}",
                    outcome.tmpl_w, outcome.tmpl_h, search_blurred.width, search_blurred.height
                ),
                format!(
                    "Match time: {:.0}ms · {} hit(s)",
                    outcome.match_ms,
                    matches.len()
                ),
            ];
            let mut item_overlay = search_raw.as_ref().unwrap().clone();
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
                    if let Some(crop) = crop_match_preview(
                        search_raw.as_ref().unwrap(),
                        local_tl_x,
                        local_tl_y,
                        tw,
                        th,
                        12,
                    ) {
                        owned_steps.push((
                            format!("Find #{} — crop around ({local_tl_x},{local_tl_y})", mi + 1),
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
                    name: outcome.job.target.clone(),
                    point: p,
                    origin,
                    meta: outcome.job.meta.clone(),
                    tmpl_w: tw,
                    tmpl_h: th,
                });
            }
            let find_count = details.iter().filter(|d| d.starts_with("Find #")).count();
            if find_count == 0 {
                details.push("No matches found for this item.".into());
            } else {
                owned_steps.push(("Where found (all matches)".into(), item_overlay));
            }
            let summary = if find_count == 0 {
                format!("0 matches · {:.0}ms", outcome.match_ms)
            } else {
                format!("{find_count} match(es) · {:.0}ms", outcome.match_ms)
            };

            let mut steps: Vec<(&str, &ImageBuf)> = vec![
                ("0. Search area (match input)", &search_blurred),
                ("1. Item template", thumbnail),
            ];
            if blur > 0 {
                steps.push((blur_label.as_str(), outcome.template_blurred.as_ref()));
            }
            if let Some(mask) = &mask_preview {
                steps.push(("3. Mask", mask));
            }
            for (label, img) in &owned_steps {
                steps.push((label.as_str(), img));
            }

            exec.log_item_pipeline(
                action_id,
                variant_label,
                summary,
                thumbnail,
                &steps,
                details,
            );
        } else {
            for mut p in matches {
                p.x += half_w;
                p.y += half_h;
                out.push(NamedPoint {
                    name: outcome.job.target.clone(),
                    point: p,
                    origin,
                    meta: outcome.job.meta.clone(),
                    tmpl_w: tw,
                    tmpl_h: th,
                });
            }
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
    exec.log_timing(action_id, "match", match_started.elapsed());
    let mut hits: Vec<DetectionHit> = out
        .into_iter()
        .map(|np| DetectionHit {
            screen_x: np.point.x + np.origin.x,
            screen_y: np.point.y + np.origin.y,
            name: np.name,
            extras: DetectionExtras::Image {
                meta: np.meta,
                tmpl_w: np.tmpl_w,
                tmpl_h: np.tmpl_h,
            },
        })
        .collect();
    sort_hits(&mut hits, order);
    Ok(hits)
}
