//! Image search action: capture → match template variants → run children per hit.

use super::common::{
    apply_detection_hits, capture_search_buf, close_matches_distance, run_detection_shell,
    sort_hits, DetectionCtx, DetectionExtras, DetectionHit,
};
use crate::backends::{CollectionArea, DesktopRect, ItemMeta};
use crate::error::{ExecError, FlowSignal, Result, SearchError};
use crate::log_draw::{crop_match_preview, draw_rect_rgb};
use crate::run::Executor;
use rayon::prelude::*;
use sqyre_domain::{
    action_type_label, expand_image_search_targets, grid_item_placements, ordered_item_targets,
    variant_name_from_path, Action, ActionKind, CatalogItemRef, GridPlacement, ItemSortBy,
    ItemSortInfo, ItemSortThen, Macro, MatchMethod, PROGRAM_DELIMITER,
};
use sqyre_match::{
    blur_image_owned, cluster_points, find_template_matches_preblurred_with_prepared,
    prepare_search, search_blur_kernel, ImageBuf, MatchError, Point, SearchPrep,
};
use sqyre_ports::{highlight_clear, highlight_fill};
use sqyre_vision::{
    get_cached_blurred_template, get_cached_image_mask, get_cached_prepared_template,
    load_rgb_image,
};
use std::collections::{HashMap, HashSet};
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
        target_tags,
        search_area,
        tolerance,
        blur,
        match_method,
        sort_by,
        sort_then,
        tag_priority,
        detection,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not image search".into()));
    };
    highlight_fill(exec.deps.highlighter, &macro_.name, action.id, 0.0);
    let action_id = action.id;
    let label = action_type_label(action.type_key());
    let catalog_refs: Vec<CatalogItemRef> = exec
        .deps
        .icons
        .map(|icons| {
            icons
                .catalog_item_refs()
                .into_iter()
                .map(|(target, meta)| CatalogItemRef {
                    target,
                    tags: meta.tags,
                })
                .collect()
        })
        .unwrap_or_default();
    let expanded = expand_image_search_targets(targets, target_tags, &catalog_refs);
    let sorted_targets =
        ordered_search_targets(exec, &expanded, *sort_by, *sort_then, tag_priority);
    let ctx = DetectionCtx::new(action_id, label, search_area, &sorted_targets, detection);
    let wait = &detection.wait;
    let macro_name = macro_.name.clone();
    let result = (|| {
        // Log wait intent once before the shared shell arms retries.
        let results0 =
            capture_and_match(exec, &ctx, *tolerance, *blur, *match_method, false, macro_)?;
        if wait.wait_until_found_active() && results0.is_empty() {
            exec.log(
                action_id,
                format!(
                    "{label}: waiting up to {}s until found",
                    wait.wait_til_found_seconds
                ),
            );
        } else if wait.wait_while_found_active() && !results0.is_empty() {
            exec.log(
                action_id,
                format!(
                    "{label}: waiting up to {}s while found",
                    wait.wait_til_found_seconds
                ),
            );
        }

        let mut initial = Some(results0);
        run_detection_shell(
            exec,
            macro_,
            &ctx,
            |exec, macro_, force_fresh| {
                if let Some(first) = initial.take() {
                    return Ok(first);
                }
                capture_and_match(
                    exec,
                    &ctx,
                    *tolerance,
                    *blur,
                    *match_method,
                    force_fresh,
                    macro_,
                )
            },
            |results| !results.is_empty(),
            |exec, macro_, results, pass| apply_detection_hits(exec, &ctx, results, macro_, pass),
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
    variant_name: String,
}

struct CollectionLayout {
    sel_r1: i32,
    sel_c1: i32,
    sel_r2: i32,
    sel_c2: i32,
    area: CollectionArea,
}

struct VariantSpec {
    variant_name: String,
    path: PathBuf,
}

/// One Image Search target expanded to variants (+ optional collection placements).
struct TargetSearch {
    target: String,
    meta: Option<ItemMeta>,
    mask_path: Option<PathBuf>,
    variants: Vec<VariantSpec>,
    /// Empty means non-collection: search the full capture once.
    placements: Vec<GridPlacement>,
}

struct VariantMatchOutcome {
    target: String,
    variant_name: String,
    meta: Option<ItemMeta>,
    tmpl_w: usize,
    tmpl_h: usize,
    /// Unblurred template — only populated when pipeline logging is enabled.
    template_raw: Option<ImageBuf>,
    /// Blurred template used for matching (for pipeline steps).
    template_blurred: Arc<ImageBuf>,
    /// Mask bytes (kept as Arc until logging builds a preview ImageBuf).
    mask_bytes: Option<Arc<Vec<u8>>>,
    matches: std::result::Result<Vec<Point>, SearchError>,
    match_ms: f64,
    placements: usize,
}

fn capture_and_match(
    exec: &mut Executor<'_>,
    ctx: &DetectionCtx<'_>,
    tolerance: f64,
    blur: i32,
    match_method: MatchMethod,
    force_fresh: bool,
    macro_: &Macro,
) -> Result<Vec<DetectionHit>> {
    let action_id = ctx.action_id;
    let label = ctx.label;
    let targets = ctx.targets;
    let order = &ctx.branch.order;
    // Capture/resolve/blur failures are logged as misses so wait-until-found can retry
    // instead of aborting the macro (same policy as OCR / Find Pixel).
    exec.check_stopped()?;
    let Some(icons) = exec.deps.icons else {
        exec.log(action_id, format!("{label}: missing IconStore"));
        return Ok(Vec::new());
    };

    let fresh = exec.take_capture_fresh(force_fresh);
    let capture_started = Instant::now();
    let Some((search, origin)) = capture_search_buf(
        exec,
        ctx,
        macro_,
        fresh,
        |exec, lx, ty, rx, by| {
            let w = (rx - lx).max(0);
            let h = (by - ty).max(0);
            exec.log(
                action_id,
                format!(
                    "{label}: searching {targets:?} in X1:{lx} Y1:{ty} X2:{rx} Y2:{by}, width:{w} height:{h}"
                ),
            );
        },
    ) else {
        return Ok(Vec::new());
    };
    exec.log_image(action_id, "1. Capture (search area)", &search);
    let kernel = search_blur_kernel(blur);
    let want_pipeline = exec.log_images_enabled();
    // Keep an unblurred copy only for diagnostics overlays / match crops.
    let search_raw = want_pipeline.then(|| search.clone());
    let search_blurred = match blur_image_owned(search, kernel) {
        Ok(b) => b,
        Err(e) => {
            exec.log(action_id, format!("{label}: blur: {e}"));
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
    exec.check_stopped()?;

    let collection_layout = match ctx.search_area.cell_range() {
        None => None,
        Some((sel_r1, sel_c1, sel_r2, sel_c2)) => match exec.deps.resolver {
            Some(resolver) => match resolver.collection_area(ctx.search_area, macro_) {
                Ok(area) => Some(CollectionLayout {
                    sel_r1,
                    sel_c1,
                    sel_r2,
                    sel_c2,
                    area,
                }),
                Err(e) => {
                    exec.log(action_id, format!("{label}: collection layout: {e}"));
                    return Ok(Vec::new());
                }
            },
            None => {
                exec.log(action_id, format!("{label}: missing CoordinateResolver"));
                return Ok(Vec::new());
            }
        },
    };

    let mut targets_search = Vec::new();
    for target in targets {
        let paths = icons.variant_paths(target);
        if paths.is_empty() {
            exec.log(action_id, format!("{label}: no icon variants for {target}"));
            continue;
        }
        let meta = icons.item_meta(target);
        let mask_path = icons.mask_path(target);
        let item = target
            .split_once(PROGRAM_DELIMITER)
            .map(|(_, item)| item)
            .unwrap_or(target.as_str());
        let (item_rows, item_cols) = item_footprint(&meta);
        let placements = collection_layout
            .as_ref()
            .map(|layout| {
                let rects = grid_item_placements(
                    layout.area.bounds(),
                    (layout.area.rows, layout.area.cols),
                    (layout.sel_r1, layout.sel_c1, layout.sel_r2, layout.sel_c2),
                    (item_rows, item_cols),
                );
                exec.log(
                    action_id,
                    format!(
                        "{label}: {target} footprint {item_rows}x{item_cols} → {} placement(s)",
                        rects.len()
                    ),
                );
                rects
            })
            .unwrap_or_default();
        if collection_layout.is_some() && placements.is_empty() {
            continue;
        }
        let variants = paths
            .into_iter()
            .map(|path| VariantSpec {
                variant_name: variant_name_from_path(&path, item),
                path,
            })
            .collect();
        targets_search.push(TargetSearch {
            target: target.clone(),
            meta,
            mask_path,
            variants,
            placements,
        });
    }

    let threshold = tolerance as f32;
    let close_dist = close_matches_distance(exec);
    let match_started = Instant::now();
    let stop_flag: Option<&AtomicBool> = exec.deps.stop_flag;
    let search_prep = if collection_layout.is_none() {
        Some(Arc::new(prepare_search(&search_blurred)))
    } else {
        None
    };
    let method = match_method;

    // Load templates before the parallel match. Inflight cache gates taken inside
    // `into_par_iter` deadlock when nested FFT work needs those same rayon workers
    // (stop then sticks at "Stop requested…"). Warm uses rayon only for independent
    // disk/blur/prepare work (no nested FFT).
    warm_variant_caches(exec, &targets_search, kernel, method)?;
    if let Some(ref prep) = search_prep {
        warm_search_ffts(prep, &search_blurred, &targets_search, kernel);
    }

    // Search order comes from sort_by/sort_then (via DetectionCtx targets). Collection
    // searches still claim occupied cells as hits are accepted below.

    let match_ctx = MatchCtx {
        kernel,
        method,
        threshold,
        close_dist,
        variant_exit_early: exec.deps.variant_exit_early,
        want_pipeline,
        stop_flag,
    };

    let mut outcomes = Vec::new();
    let mut occupied_cells: HashSet<(i32, i32)> = HashSet::new();

    if collection_layout.is_some() {
        for target in &targets_search {
            exec.check_stopped()?;
            let free: Vec<GridPlacement> = target
                .placements
                .iter()
                .copied()
                .filter(|p| !p.overlaps_occupied(&occupied_cells))
                .collect();
            let pruned = target.placements.len() - free.len();
            if pruned > 0 {
                exec.log(
                    action_id,
                    format!(
                        "{label}: {} — skipped {pruned} placement(s) (occupied cells)",
                        target.target
                    ),
                );
            }
            if free.is_empty() {
                continue;
            }

            let free_rects: Vec<(i32, i32, i32, i32)> = free.iter().map(|p| p.rect).collect();
            let crop_preps = prepare_crops(&search_blurred, origin, &free_rects);

            // Outer placement `par_iter` only: match_direct / match_fft stay
            // serial on rows when already on a Rayon worker (avoids nested pool
            // oversubscription). Single-variant callers off-pool still parallelize.
            let wave: Vec<Vec<VariantMatchOutcome>> = free
                .par_iter()
                .map(|placement| {
                    if match_ctx
                        .stop_flag
                        .is_some_and(|f| f.load(Ordering::SeqCst))
                    {
                        return Vec::new();
                    }
                    let cropped = crop_preps.get(&placement.rect).cloned().flatten();
                    let Some(crop) = cropped else {
                        return Vec::new();
                    };
                    match_variants_until_hit(
                        target,
                        &crop.img,
                        Some(&crop.prep),
                        crop.ox,
                        crop.oy,
                        &match_ctx,
                    )
                })
                .collect();

            // Claim in stable placement order so overlapping sibling footprints
            // of the same item do not both keep hits.
            for (placement, tried) in free.into_iter().zip(wave) {
                if placement.overlaps_occupied(&occupied_cells) {
                    continue;
                }
                if tried
                    .iter()
                    .any(|o| matches!(&o.matches, Ok(pts) if !pts.is_empty()))
                {
                    placement.claim_into(&mut occupied_cells);
                }
                outcomes.extend(tried);
            }
        }
    } else {
        // Non-collection: independent targets in parallel; variants may early-exit
        // within each target (one full-frame search) when variant_exit_early is on.
        // Match kernels detect Rayon workers and skip inner row/channel par_iter.
        let wave: Vec<Vec<VariantMatchOutcome>> = targets_search
            .par_iter()
            .map(|target| {
                if match_ctx
                    .stop_flag
                    .is_some_and(|f| f.load(Ordering::SeqCst))
                {
                    return Vec::new();
                }
                match_variants_until_hit(
                    target,
                    &search_blurred,
                    search_prep.as_deref(),
                    0,
                    0,
                    &match_ctx,
                )
            })
            .collect();
        for tried in wave {
            outcomes.extend(tried);
        }
    }
    if exec.check_stopped().is_err() {
        exec.log(
            action_id,
            format!("{label}: stop requested, aborting match"),
        );
        return Err(FlowSignal::Stopped.into());
    }
    let outcomes = merge_variant_outcomes(outcomes, close_dist);

    let mut out = Vec::new();
    for outcome in outcomes {
        let variant_label = variant_log_label(&outcome.target, &outcome.variant_name);

        if outcome.tmpl_w == 0 {
            if let Err(e) = &outcome.matches {
                exec.log(action_id, format!("{label}: {e}"));
            }
            continue;
        }

        exec.log(
            action_id,
            format!(
                "{label}: matching {variant_label} ({}x{}) against {}x{}{}",
                outcome.tmpl_w,
                outcome.tmpl_h,
                search_blurred.width,
                search_blurred.height,
                if outcome.placements > 1 {
                    format!(" ({} placements)", outcome.placements)
                } else {
                    String::new()
                }
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
                exec.log(action_id, format!("{label} match: {e}"));
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
                "{label}: {variant_label} → {} match(es) in {:.0}ms",
                matches.len(),
                outcome.match_ms
            ),
        );

        let half_w = (outcome.tmpl_w / 2) as i32;
        let half_h = (outcome.tmpl_h / 2) as i32;
        let tw = outcome.tmpl_w as i32;
        let th = outcome.tmpl_h as i32;

        // Binding the capture here ties the pipeline branch to it actually
        // existing, instead of re-deriving that from `want_pipeline`.
        if let Some(search_raw) = search_raw.as_ref() {
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
            let mut item_overlay = search_raw.clone();
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
                        crop_match_preview(search_raw, local_tl_x, local_tl_y, tw, th, 12)
                    {
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
                    name: outcome.target.clone(),
                    variant_name: outcome.variant_name.clone(),
                    point: p,
                    origin,
                    meta: outcome.meta.clone(),
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
                    name: outcome.target.clone(),
                    variant_name: outcome.variant_name.clone(),
                    point: p,
                    origin,
                    meta: outcome.meta.clone(),
                    tmpl_w: tw,
                    tmpl_h: th,
                });
            }
        }
    }

    exec.log(
        action_id,
        format!(
            "{label}: capture+match done in {:.0}ms ({} raw hit(s))",
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
                variant_name: np.variant_name,
            },
        })
        .collect();
    sort_hits(&mut hits, order);
    Ok(hits)
}

fn variant_log_label(target: &str, variant_name: &str) -> String {
    if variant_name.is_empty() {
        target.to_string()
    } else {
        format!("{target}{PROGRAM_DELIMITER}{variant_name}")
    }
}

fn item_footprint(meta: &Option<ItemMeta>) -> (i32, i32) {
    let rows = meta.as_ref().map(|m| m.rows).unwrap_or(0).max(1);
    let cols = meta.as_ref().map(|m| m.cols).unwrap_or(0).max(1);
    (rows, cols)
}

fn ordered_search_targets(
    exec: &Executor<'_>,
    targets: &[String],
    sort_by: ItemSortBy,
    sort_then: ItemSortThen,
    tag_priority: &[String],
) -> Vec<String> {
    let infos: Vec<ItemSortInfo> = targets
        .iter()
        .map(|target| {
            let meta = exec.deps.icons.and_then(|icons| icons.item_meta(target));
            let (name, rows, cols, tags) = match meta {
                Some(m) => (m.name, m.rows, m.cols, m.tags),
                None => (String::new(), 1, 1, Vec::new()),
            };
            ItemSortInfo::from_parts(target.clone(), name, rows, cols, tags)
        })
        .collect();
    ordered_item_targets(&infos, sort_by, sort_then, tag_priority)
}

fn warm_variant_caches(
    exec: &Executor<'_>,
    targets: &[TargetSearch],
    kernel: i32,
    method: MatchMethod,
) -> Result<()> {
    let mut jobs: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let mut seen: HashSet<(PathBuf, Option<PathBuf>)> = HashSet::new();
    for target in targets {
        for variant in &target.variants {
            if seen.insert((variant.path.clone(), target.mask_path.clone())) {
                jobs.push((variant.path.clone(), target.mask_path.clone()));
            }
        }
    }
    // Stop check before spawning parallel warm work.
    exec.check_stopped()?;
    let stop = exec.deps.stop_flag;
    jobs.into_par_iter().for_each(|(path, mask_path)| {
        if stop.is_some_and(|f| f.load(Ordering::SeqCst)) {
            return;
        }
        let Ok(template_blurred) = get_cached_blurred_template(&path, kernel) else {
            return;
        };
        let mask_bytes = mask_path.as_ref().and_then(|p| {
            get_cached_image_mask(p, template_blurred.height, template_blurred.width)
        });
        let _ = get_cached_prepared_template(
            &path,
            kernel,
            template_blurred.as_ref(),
            mask_path.as_deref(),
            mask_bytes.as_deref().map(|m| m.as_slice()),
            method,
        );
    });
    if stop.is_some_and(|f| f.load(Ordering::SeqCst)) {
        return Err(FlowSignal::Stopped.into());
    }
    Ok(())
}

/// Precompute search-frame FFTs for template sizes that will take the DFT path.
fn warm_search_ffts(prep: &SearchPrep, search: &ImageBuf, targets: &[TargetSearch], kernel: i32) {
    let mut seen_sizes: HashSet<(usize, usize)> = HashSet::new();
    for target in targets {
        // Masked matches use the direct correlator — skip FFT warm.
        if target.mask_path.is_some() {
            continue;
        }
        for variant in &target.variants {
            let Ok(tmpl) = get_cached_blurred_template(&variant.path, kernel) else {
                continue;
            };
            let tw = tmpl.width;
            let th = tmpl.height;
            if !seen_sizes.insert((tw, th)) {
                continue;
            }
            prep.warm_fft_for_template(search, tw, th);
        }
    }
}

struct MatchCtx<'a> {
    kernel: i32,
    method: MatchMethod,
    threshold: f32,
    close_dist: i32,
    variant_exit_early: bool,
    want_pipeline: bool,
    stop_flag: Option<&'a AtomicBool>,
}

/// Try each variant on one search image.
///
/// When [`MatchCtx::variant_exit_early`] is set, stop after the first hit.
/// Missed variants are still returned (for logging). Later variants of the same
/// item remain eligible on *other* placements — early-exit is per search only.
fn match_variants_until_hit(
    target: &TargetSearch,
    search_img: &ImageBuf,
    prep: Option<&SearchPrep>,
    ox: i32,
    oy: i32,
    ctx: &MatchCtx<'_>,
) -> Vec<VariantMatchOutcome> {
    let mut out = Vec::new();
    for variant in &target.variants {
        if ctx.stop_flag.is_some_and(|f| f.load(Ordering::SeqCst)) {
            break;
        }
        let outcome = match_one_variant(target, variant, search_img, prep, ox, oy, ctx);
        let hit = matches!(&outcome.matches, Ok(pts) if !pts.is_empty());
        out.push(outcome);
        if hit && ctx.variant_exit_early {
            break;
        }
    }
    out
}

fn match_one_variant(
    target: &TargetSearch,
    variant: &VariantSpec,
    search_img: &ImageBuf,
    prep: Option<&SearchPrep>,
    ox: i32,
    oy: i32,
    ctx: &MatchCtx<'_>,
) -> VariantMatchOutcome {
    let template_blurred = match get_cached_blurred_template(&variant.path, ctx.kernel) {
        Ok(t) => t,
        Err(e) => {
            return skipped_outcome(
                target,
                variant,
                Err(SearchError::Template(format!(
                    "load {:?}: {e}",
                    variant.path
                ))),
            );
        }
    };
    let tmpl_w = template_blurred.width;
    let tmpl_h = template_blurred.height;

    let mask_bytes = target
        .mask_path
        .as_ref()
        .and_then(|p| get_cached_image_mask(p, tmpl_h, tmpl_w));

    let template_raw = if ctx.want_pipeline {
        load_rgb_image(&variant.path).ok()
    } else {
        None
    };

    let t0 = Instant::now();
    let matches = get_cached_prepared_template(
        &variant.path,
        ctx.kernel,
        template_blurred.as_ref(),
        target.mask_path.as_deref(),
        mask_bytes.as_deref().map(|m| m.as_slice()),
        ctx.method,
    )
    .map_err(|e| SearchError::Template(format!("prepare {:?}: {e}", variant.path)))
    .and_then(|prepared| {
        find_template_matches_preblurred_with_prepared(
            search_img,
            template_blurred.as_ref(),
            &prepared,
            ctx.threshold,
            ctx.close_dist,
            ctx.method,
            prep,
        )
        .map_err(SearchError::from)
    });
    let matches = match matches {
        Err(SearchError::Match(MatchError::TemplateTooLarge { .. })) => Ok(Vec::new()),
        Ok(mut pts) => {
            for p in &mut pts {
                p.x += ox;
                p.y += oy;
            }
            Ok(pts)
        }
        other => other,
    };
    let match_ms = t0.elapsed().as_secs_f64() * 1000.0;

    VariantMatchOutcome {
        target: target.target.clone(),
        variant_name: variant.variant_name.clone(),
        meta: target.meta.clone(),
        tmpl_w,
        tmpl_h,
        template_raw,
        template_blurred,
        mask_bytes,
        matches,
        match_ms,
        placements: 1,
    }
}

fn skipped_outcome(
    target: &TargetSearch,
    variant: &VariantSpec,
    matches: std::result::Result<Vec<Point>, SearchError>,
) -> VariantMatchOutcome {
    VariantMatchOutcome {
        target: target.target.clone(),
        variant_name: variant.variant_name.clone(),
        meta: target.meta.clone(),
        tmpl_w: 0,
        tmpl_h: 0,
        template_raw: None,
        template_blurred: Arc::new(ImageBuf::new(1, 1, 3, 0)),
        mask_bytes: None,
        matches,
        match_ms: 0.0,
        placements: 1,
    }
}

/// One collection cell cropped out of the capture, with its search preparation.
///
/// Every variant searching the same cell shares this: `prepare_search` builds
/// integral images and a planar `f32` copy, which is wasted work if each
/// variant rebuilds it for the same rect.
struct CropPrep {
    img: ImageBuf,
    prep: SearchPrep,
    /// Offset of the crop within the capture, added back to match coordinates.
    ox: i32,
    oy: i32,
}

/// Prepare each distinct placement rect once. Missing entries mean the rect
/// fell outside the capture (callers treat that as no-match).
fn prepare_crops(
    search: &ImageBuf,
    origin: DesktopRect,
    rects: &[(i32, i32, i32, i32)],
) -> HashMap<(i32, i32, i32, i32), Option<Arc<CropPrep>>> {
    let unique: Vec<(i32, i32, i32, i32)> = rects
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    unique
        .into_par_iter()
        .map(|rect| {
            let prepped = crop_placement(search, origin, rect).map(|(img, ox, oy)| {
                let prep = prepare_search(&img);
                Arc::new(CropPrep { img, prep, ox, oy })
            });
            (rect, prepped)
        })
        .collect()
}

fn crop_placement(
    search: &ImageBuf,
    origin: DesktopRect,
    (slx, sty, srx, sby): (i32, i32, i32, i32),
) -> Option<(ImageBuf, i32, i32)> {
    let x = (slx - origin.x).max(0);
    let y = (sty - origin.y).max(0);
    let x1 = (srx - origin.x).min(search.width as i32).max(0);
    let y1 = (sby - origin.y).min(search.height as i32).max(0);
    if x1 <= x || y1 <= y {
        return None;
    }
    let crop = search.crop(x as usize, y as usize, (x1 - x) as usize, (y1 - y) as usize)?;
    Some((crop, x, y))
}

fn merge_variant_outcomes(
    outcomes: Vec<VariantMatchOutcome>,
    close_dist: i32,
) -> Vec<VariantMatchOutcome> {
    let mut order: Vec<(String, String)> = Vec::new();
    let mut map: HashMap<(String, String), VariantMatchOutcome> = HashMap::new();
    for o in outcomes {
        let key = (o.target.clone(), o.variant_name.clone());
        if let Some(acc) = map.get_mut(&key) {
            acc.placements += o.placements;
            acc.match_ms += o.match_ms;
            if acc.tmpl_w == 0 && o.tmpl_w > 0 {
                acc.tmpl_w = o.tmpl_w;
                acc.tmpl_h = o.tmpl_h;
                acc.template_raw = o.template_raw;
                acc.template_blurred = o.template_blurred;
                acc.mask_bytes = o.mask_bytes;
            }
            match (&mut acc.matches, o.matches) {
                (Ok(dst), Ok(src)) => dst.extend(src),
                (Ok(_), Err(_)) => {}
                (Err(_), Ok(src)) => acc.matches = Ok(src),
                (Err(_), Err(_)) => {}
            }
        } else {
            order.push(key.clone());
            map.insert(key, o);
        }
    }
    order
        .into_iter()
        .filter_map(|k| {
            let mut o = map.remove(&k)?;
            if let Ok(pts) = &mut o.matches {
                *pts = cluster_points(pts, close_dist);
            }
            Some(o)
        })
        .collect()
}
