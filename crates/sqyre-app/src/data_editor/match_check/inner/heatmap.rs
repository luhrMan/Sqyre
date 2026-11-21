//! Heatmap score pooling and RGBA conversion for MatchCheck.

use super::{
    compute_match_summary, score_norm_for_display, score_passes, MatchCheckHover, MatchSummary,
};
use sqyre_domain::MatchMethod;
use sqyre_match::MatchMap;

pub(super) const HEATMAP_VIZ_MARGIN: f32 = 0.12;

/// Whether a match position should contribute to the pooled heatmap overlay.
pub(super) fn score_pools_in_heatmap(score: f32, method: MatchMethod, tolerance: f32) -> bool {
    if !score.is_finite() {
        return false;
    }
    if score_passes(score, method, tolerance) {
        return true;
    }
    if !method.is_normed() {
        return false;
    }
    if method.higher_is_better() {
        score >= tolerance - HEATMAP_VIZ_MARGIN
    } else {
        score <= tolerance + HEATMAP_VIZ_MARGIN
    }
}

#[inline]
pub(super) fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

/// Blue (weak) → yellow (mid) → green (strong) on normalized similarity.
pub(super) fn similarity_rgb(norm: f32) -> (u8, u8, u8) {
    let t = norm.clamp(0.0, 1.0);
    if t < 0.5 {
        let u = t * 2.0;
        (
            lerp_u8(20, 230, u),
            lerp_u8(40, 210, u),
            lerp_u8(180, 40, u),
        )
    } else {
        let u = (t - 0.5) * 2.0;
        (
            lerp_u8(230, 30, u),
            lerp_u8(210, 230, u),
            lerp_u8(40, 70, u),
        )
    }
}

pub(super) fn pixel_rgba(score: f32, norm: f32, method: MatchMethod, tolerance: f32) -> [u8; 4] {
    let (r, g, b) = similarity_rgb(norm);
    let mut alpha = (20.0 + norm * norm * 180.0) as u8;
    if score_passes(score, method, tolerance) {
        alpha = alpha.max(150);
    } else if method.is_normed() {
        // Fade near-misses so the user can see "almost" matches.
        let margin = if method.higher_is_better() {
            (score - (tolerance - 0.15)).max(0.0) / 0.15
        } else {
            ((tolerance + 0.15) - score).max(0.0) / 0.15
        };
        alpha = alpha.max((margin * 90.0) as u8);
    }
    [r, g, b, alpha]
}

pub(super) const HEATMAP_PEAK_RADIUS: i32 = 4;

/// Whether `a` is a better raw match score than `b` for `method`.
pub(super) fn score_better(a: f32, b: f32, method: MatchMethod) -> bool {
    if method.higher_is_better() {
        a > b
    } else {
        a < b
    }
}

/// Stamp a compact soft kernel at match origin `(ox, oy)`.
/// `pooled` keeps the best raw score; `weight` keeps max radial falloff for that score.
pub(super) fn stamp_peak_kernel(
    pooled: &mut [f32],
    weight: &mut [f32],
    image_w: usize,
    image_h: usize,
    ox: i32,
    oy: i32,
    score: f32,
    method: MatchMethod,
) {
    let r = HEATMAP_PEAK_RADIUS;
    let r2 = (r * r) as f32;
    for dy in -r..=r {
        for dx in -r..=r {
            let dist2 = (dx * dx + dy * dy) as f32;
            if dist2 > r2 {
                continue;
            }
            let px = ox + dx;
            let py = oy + dy;
            if px < 0 || py < 0 || px as usize >= image_w || py as usize >= image_h {
                continue;
            }
            let falloff = 1.0 - (dist2 / r2).sqrt();
            let idx = py as usize * image_w + px as usize;
            let cur = pooled[idx];
            if !cur.is_finite() || score_better(score, cur, method) {
                pooled[idx] = score;
                weight[idx] = falloff;
            } else if (score - cur).abs() <= f32::EPSILON {
                weight[idx] = weight[idx].max(falloff);
            }
        }
    }
}

/// Build RGBA heatmap: compact peak kernels at match origins, graded colormap.
pub(crate) fn scores_to_heatmap_rgba(
    map: &MatchMap,
    image_w: usize,
    image_h: usize,
    _tmpl_w: usize,
    _tmpl_h: usize,
    method: MatchMethod,
    tolerance: f32,
) -> (Vec<u8>, MatchSummary) {
    let summary = compute_match_summary(map, method, tolerance);
    let mut rgba = vec![0u8; image_w * image_h * 4];
    if map.width == 0 || map.height == 0 || image_w == 0 || image_h == 0 {
        return (rgba, summary);
    }

    let init = if method.higher_is_better() {
        f32::NEG_INFINITY
    } else {
        f32::INFINITY
    };
    let mut pooled = vec![init; image_w * image_h];
    let mut weight = vec![0.0f32; image_w * image_h];
    for y in 0..map.height {
        for x in 0..map.width {
            let score = map.scores[y * map.width + x];
            if !score_pools_in_heatmap(score, method, tolerance) {
                continue;
            }
            stamp_peak_kernel(
                &mut pooled,
                &mut weight,
                image_w,
                image_h,
                x as i32,
                y as i32,
                score,
                method,
            );
        }
    }

    for py in 0..image_h {
        for px in 0..image_w {
            let idx = py * image_w + px;
            let score = pooled[idx];
            if !score.is_finite() {
                continue;
            }
            let norm = score_norm_for_display(score, method, &summary);
            let mut px_rgba = pixel_rgba(score, norm, method, tolerance);
            px_rgba[3] = ((px_rgba[3] as f32) * weight[idx].clamp(0.0, 1.0)).round() as u8;
            if px_rgba[3] == 0 {
                continue;
            }
            let i = idx * 4;
            rgba[i..i + 4].copy_from_slice(&px_rgba);
        }
    }
    (rgba, summary)
}

/// User-facing closeness percentage for a raw match score.
pub(crate) fn closeness_percent(score: f32, method: MatchMethod, summary: &MatchSummary) -> f32 {
    score_norm_for_display(score, method, summary) * 100.0
}

/// Best template-match score covering image pixel `(px, py)` among heatmap-pooled hits
/// (strict passes and normed near-misses within [`HEATMAP_VIZ_MARGIN`]).
pub(super) fn pooled_score_at(
    map: &MatchMap,
    px: usize,
    py: usize,
    tmpl_w: usize,
    tmpl_h: usize,
    method: MatchMethod,
    tolerance: f32,
) -> Option<f32> {
    let mut best: Option<f32> = None;
    for my in 0..map.height {
        for mx in 0..map.width {
            if px < mx || py < my || px >= mx + tmpl_w || py >= my + tmpl_h {
                continue;
            }
            let score = map.scores[my * map.width + mx];
            if !score_pools_in_heatmap(score, method, tolerance) {
                continue;
            }
            best = Some(match best {
                None => score,
                Some(b) if method.higher_is_better() => b.max(score),
                Some(b) => b.min(score),
            });
        }
    }
    best
}

/// Hover probe at an image-space pixel (anywhere under the cursor).
pub(crate) fn hover_at_image(
    map: &MatchMap,
    summary: &MatchSummary,
    ix: i32,
    iy: i32,
    tmpl_w: usize,
    tmpl_h: usize,
    method: MatchMethod,
    tolerance: f32,
) -> Option<MatchCheckHover> {
    if ix < 0 || iy < 0 {
        return None;
    }
    let score = pooled_score_at(
        map,
        ix as usize,
        iy as usize,
        tmpl_w,
        tmpl_h,
        method,
        tolerance,
    )?;
    let passes = score_passes(score, method, tolerance);
    Some(MatchCheckHover {
        x: ix,
        y: iy,
        score,
        closeness_pct: closeness_percent(score, method, summary),
        passes,
    })
}

/// Hover inspection must not run while a drag is active (pan or click).
pub(crate) fn heatmap_hover_allowed(pointer_down: bool, pan_dragging: bool) -> bool {
    !pointer_down && !pan_dragging
}
