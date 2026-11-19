//! MatchCheck: live template-match heatmap over a search-area preview.
//!
//! Legend / marker colors use app theme viz tokens (`match_pass_fg`,
//! `match_fail_fg`, `match_within_fg`) — not status `error_fg` / `warn_fg` /
//! `ok_fg`, which are reserved for chrome and banners.

#[cfg(feature = "native-runtime")]
#[expect(
    clippy::too_many_arguments,
    reason = "pixel-check helpers share LTRB, match settings, and cache payloads"
)]
mod inner {
    use crate::data_editor_preview::variant_display_label;
    use crate::icon_variants::variant_path;
    use crate::image_view::{self, ImageViewTransform};
    use crate::theme::{match_fail_fg, match_pass_fg, match_within_fg};
    use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
    use image::RgbaImage;
    use sqyre_domain::MatchMethod;
    use sqyre_match::ImageBuf;
    use sqyre_match::{
        blur_image_owned, find_peaks_for_method, match_template_with_prepared, prepare_search,
        prepare_template, search_blur_kernel, MatchMap,
    };
    use sqyre_persist::CatalogView;
    use sqyre_vision::{get_cached_blurred_template, get_cached_image_mask, rgba_to_rgb_buf};
    use std::path::{Path, PathBuf};
    use std::sync::{mpsc, Arc};
    use std::thread;
    use web_time::{Duration, Instant};

    const DEBOUNCE: Duration = Duration::from_millis(200);
    /// Wall-clock gap between live recapture+rematch cycles (~5 Hz).
    pub(crate) const LIVE_INTERVAL: Duration = Duration::from_millis(200);
    /// Above this count, match boxes are hidden until the user opts in (paint cost).
    pub(crate) const MANY_MATCH_BOX_THRESHOLD: usize = 100;

    /// Peak match stats shown in the legend and best-match marker.
    #[derive(Clone, Copy, Debug, Default)]
    pub(crate) struct MatchSummary {
        pub best_score: f32,
        pub best_x: i32,
        pub best_y: i32,
        pub best_passes: bool,
        pub score_min: f32,
        pub score_max: f32,
        pub peaks_above: usize,
    }

    /// Spatially clustered template match at or above tolerance (strict).
    #[derive(Clone, Debug)]
    pub(crate) struct ToleranceMatch {
        pub x: i32,
        pub y: i32,
        pub score: f32,
    }

    /// Shared match output carried by both the worker result and the
    /// uploaded-texture cache.
    pub(crate) struct MatchCheckData {
        pub fingerprint: String,
        pub match_map: MatchMap,
        pub image_w: usize,
        pub image_h: usize,
        pub tmpl_w: usize,
        pub tmpl_h: usize,
        pub summary: MatchSummary,
        pub tolerance_matches: Vec<ToleranceMatch>,
        /// A mask was configured but unreadable — the preview ran unmasked.
        pub mask_missing: bool,
    }

    pub(crate) struct MatchCheckCache {
        pub data: MatchCheckData,
        pub heatmap: TextureHandle,
    }

    pub(crate) struct MatchCheckResult {
        pub data: MatchCheckData,
        pub heatmap_rgba: Vec<u8>,
    }

    pub(crate) struct MatchCheckHover {
        pub x: i32,
        pub y: i32,
        pub score: f32,
        /// 0–100 display closeness for the hovered image pixel.
        pub closeness_pct: f32,
        pub passes: bool,
    }

    /// Session-local match settings for MatchCheck (not persisted).
    pub(crate) struct MatchCheckSettings {
        pub tolerance: f64,
        pub blur: i32,
        pub match_method: MatchMethod,
        pub variant: String,
        pub refresh_gen: u64,
        pub last_inputs: String,
        pub last_request: Option<Instant>,
        /// When true, live recapture/rematch and debounced retries are suppressed until ↻ or unpause.
        pub paused: bool,
        /// Opt-in paint for match boxes when count exceeds [`MANY_MATCH_BOX_THRESHOLD`].
        pub show_many_match_boxes: bool,
        /// When the last live match cycle finished (gates [`LIVE_INTERVAL`]).
        pub last_live_at: Option<Instant>,
    }

    impl Default for MatchCheckSettings {
        fn default() -> Self {
            Self {
                tolerance: 0.95,
                blur: 0,
                match_method: MatchMethod::CcoeffNormed,
                variant: String::new(),
                refresh_gen: 0,
                last_inputs: String::new(),
                last_request: None,
                paused: false,
                show_many_match_boxes: false,
                last_live_at: None,
            }
        }
    }

    /// Whether a live recapture+rematch should start now.
    pub(crate) fn live_refresh_due(
        settings: &MatchCheckSettings,
        pending: bool,
        now: Instant,
    ) -> bool {
        if settings.paused || pending {
            return false;
        }
        settings
            .last_live_at
            .is_none_or(|t| now.duration_since(t) >= LIVE_INTERVAL)
    }

    pub(crate) fn should_paint_match_boxes(match_count: usize, show_many: bool) -> bool {
        match_count <= MANY_MATCH_BOX_THRESHOLD || show_many
    }

    /// All bounds are literal integers forming a non-empty rectangle.
    pub(crate) fn coords_displayable(
        left: Option<i32>,
        top: Option<i32>,
        right: Option<i32>,
        bottom: Option<i32>,
    ) -> bool {
        match (left, top, right, bottom) {
            (Some(lx), Some(ty), Some(rx), Some(by)) => rx > lx && by > ty,
            _ => false,
        }
    }

    pub(crate) fn can_compute_match_check(
        catalog: CatalogView<'_>,
        prog: &str,
        item: &str,
        variant: &str,
        left: Option<i32>,
        top: Option<i32>,
        right: Option<i32>,
        bottom: Option<i32>,
    ) -> bool {
        if !coords_displayable(left, top, right, bottom) {
            return false;
        }
        resolve_template(catalog, prog, item, variant)
            .map(|p| p.is_file())
            .unwrap_or(false)
    }

    pub(crate) fn fingerprint(
        prog: &str,
        item: &str,
        variant: &str,
        lx: i32,
        ty: i32,
        rx: i32,
        by: i32,
        blur: i32,
        method: MatchMethod,
        tolerance: f64,
        refresh_gen: u64,
        close_matches_distance: i32,
    ) -> String {
        format!(
            "{prog}\0{item}\0{variant}\0{lx},{ty},{rx},{by}\0{blur}\0{method:?}\0{tolerance}\0{refresh_gen}\0{close_matches_distance}"
        )
    }

    /// Nearest-neighbor scale of an RGB `ImageBuf` to `(nw, nh)`.
    pub(crate) fn scale_image_buf(img: &ImageBuf, nw: usize, nh: usize) -> ImageBuf {
        if img.width == nw && img.height == nh {
            return img.clone();
        }
        let nw = nw.max(1);
        let nh = nh.max(1);
        let mut out = ImageBuf::new(nw, nh, img.channels, 0);
        for y in 0..nh {
            let sy = y * img.height / nh;
            for x in 0..nw {
                let sx = x * img.width / nw;
                let si = img.pixel_offset(sx, sy);
                let di = out.pixel_offset(x, y);
                out.data[di..di + img.channels].copy_from_slice(&img.data[si..si + img.channels]);
            }
        }
        out
    }

    /// Scale a single-channel mask to template size (nearest-neighbor).
    fn scale_mask(mask: &[u8], src_w: usize, src_h: usize, nw: usize, nh: usize) -> Vec<u8> {
        if src_w == nw && src_h == nh && mask.len() == nw * nh {
            return mask.to_vec();
        }
        let nw = nw.max(1);
        let nh = nh.max(1);
        let mut out = vec![0u8; nw * nh];
        for y in 0..nh {
            let sy = y * src_h / nh;
            for x in 0..nw {
                let sx = x * src_w / nw;
                out[y * nw + x] = mask[sy * src_w + sx];
            }
        }
        out
    }

    /// Match the template against a shared panel display capture.
    ///
    /// When the display was downscaled from `src_w`×`src_h`, the template (and
    /// mask) are scaled by the same factors so sizes stay consistent.
    fn compute_match(
        display: &RgbaImage,
        src_w: u32,
        src_h: u32,
        template_path: &Path,
        mask_path: Option<&Path>,
        blur: i32,
        method: MatchMethod,
    ) -> Result<(MatchMap, usize, usize, usize, usize, bool), String> {
        let image_w = display.width() as usize;
        let image_h = display.height() as usize;
        if image_w == 0 || image_h == 0 {
            return Err("empty display image".into());
        }
        let search = rgba_to_rgb_buf(display);
        let kernel = search_blur_kernel(blur);
        let search_blurred = if kernel > 0 {
            blur_image_owned(search, kernel).map_err(|e| e.to_string())?
        } else {
            search
        };
        let template_full = get_cached_blurred_template(template_path, kernel)
            .map_err(|e| format!("template: {e}"))?;
        let scale_x = if src_w > 0 {
            image_w as f32 / src_w as f32
        } else {
            1.0
        };
        let scale_y = if src_h > 0 {
            image_h as f32 / src_h as f32
        } else {
            1.0
        };
        let need_scale =
            (scale_x - 1.0).abs() > f32::EPSILON || (scale_y - 1.0).abs() > f32::EPSILON;
        let template_blurred = if need_scale {
            let nw = ((template_full.width as f32) * scale_x).round().max(1.0) as usize;
            let nh = ((template_full.height as f32) * scale_y).round().max(1.0) as usize;
            Arc::new(scale_image_buf(&template_full, nw, nh))
        } else {
            Arc::clone(&template_full)
        };
        let tmpl_w = template_blurred.width;
        let tmpl_h = template_blurred.height;
        let mask_owned = mask_path.and_then(|p| {
            let full = get_cached_image_mask(p, template_full.height, template_full.width)?;
            if need_scale {
                Some(scale_mask(
                    &full,
                    template_full.width,
                    template_full.height,
                    tmpl_w,
                    tmpl_h,
                ))
            } else {
                Some(full.as_ref().clone())
            }
        });
        let mask_missing = mask_path.is_some() && mask_owned.is_none();
        let mask_ref = mask_owned.as_deref();
        let prepared =
            prepare_template(&template_blurred, mask_ref, method).map_err(|e| e.to_string())?;
        let search_prep = prepare_search(&search_blurred);
        let map = match_template_with_prepared(
            &search_blurred,
            &template_blurred,
            &prepared,
            Some(&search_prep),
        )
        .map_err(|e| e.to_string())?;
        Ok((map, image_w, image_h, tmpl_w, tmpl_h, mask_missing))
    }

    fn score_passes(score: f32, method: MatchMethod, tolerance: f32) -> bool {
        if !score.is_finite() {
            return false;
        }
        if method.higher_is_better() {
            score >= tolerance
        } else {
            score <= tolerance
        }
    }

    fn match_score_at(map: &MatchMap, x: i32, y: i32) -> Option<f32> {
        if x < 0 || y < 0 {
            return None;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= map.width || y >= map.height {
            return None;
        }
        let score = map.scores[y * map.width + x];
        score.is_finite().then_some(score)
    }

    pub(crate) fn collect_tolerance_matches(
        map: &MatchMap,
        tolerance: f32,
        close_matches_distance: i32,
        method: MatchMethod,
    ) -> Vec<ToleranceMatch> {
        find_peaks_for_method(map, tolerance, close_matches_distance, method)
            .into_iter()
            .filter_map(|pt| {
                let score = match_score_at(map, pt.x, pt.y)?;
                if score_passes(score, method, tolerance) {
                    Some(ToleranceMatch {
                        x: pt.x,
                        y: pt.y,
                        score,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// 0–1 display strength for a raw score (matches hover closeness semantics).
    fn score_norm_for_display(score: f32, method: MatchMethod, summary: &MatchSummary) -> f32 {
        if !score.is_finite() {
            return 0.0;
        }
        if method.is_normed() {
            if method.higher_is_better() {
                score.clamp(0.0, 1.0)
            } else {
                1.0 - score.clamp(0.0, 1.0)
            }
        } else {
            let span = (summary.score_max - summary.score_min).max(f32::EPSILON);
            if method.higher_is_better() {
                ((score - summary.score_min) / span).clamp(0.0, 1.0)
            } else {
                ((summary.score_max - score) / span).clamp(0.0, 1.0)
            }
        }
    }

    const HEATMAP_VIZ_MARGIN: f32 = 0.12;

    /// Whether a match position should contribute to the pooled heatmap overlay.
    fn score_pools_in_heatmap(score: f32, method: MatchMethod, tolerance: f32) -> bool {
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

    fn compute_match_summary(map: &MatchMap, method: MatchMethod, tolerance: f32) -> MatchSummary {
        let mut summary = MatchSummary::default();
        if map.width == 0 || map.height == 0 {
            return summary;
        }
        let mut best_score = if method.higher_is_better() {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for y in 0..map.height {
            for x in 0..map.width {
                let score = map.scores[y * map.width + x];
                if !score.is_finite() {
                    continue;
                }
                min = min.min(score);
                max = max.max(score);
                let better = if method.higher_is_better() {
                    score > best_score
                } else {
                    score < best_score
                };
                if better {
                    best_score = score;
                    summary.best_x = x as i32;
                    summary.best_y = y as i32;
                }
                if score_passes(score, method, tolerance) {
                    summary.peaks_above += 1;
                }
            }
        }
        if min.is_finite() && max.is_finite() {
            summary.score_min = min;
            summary.score_max = max;
            summary.best_score = best_score;
            summary.best_passes = score_passes(best_score, method, tolerance);
        }
        summary
    }

    #[inline]
    fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
        (a as f32 + (b as f32 - a as f32) * t).round() as u8
    }

    /// Blue (weak) → yellow (mid) → green (strong) on normalized similarity.
    fn similarity_rgb(norm: f32) -> (u8, u8, u8) {
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

    fn pixel_rgba(score: f32, norm: f32, method: MatchMethod, tolerance: f32) -> [u8; 4] {
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

    /// Soft disk radius (image pixels) around each match origin in the heatmap.
    const HEATMAP_PEAK_RADIUS: i32 = 4;

    /// Whether `a` is a better raw match score than `b` for `method`.
    fn score_better(a: f32, b: f32, method: MatchMethod) -> bool {
        if method.higher_is_better() {
            a > b
        } else {
            a < b
        }
    }

    /// Stamp a compact soft kernel at match origin `(ox, oy)`.
    /// `pooled` keeps the best raw score; `weight` keeps max radial falloff for that score.
    fn stamp_peak_kernel(
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
    pub(crate) fn closeness_percent(
        score: f32,
        method: MatchMethod,
        summary: &MatchSummary,
    ) -> f32 {
        score_norm_for_display(score, method, summary) * 100.0
    }

    /// Best template-match score covering image pixel `(px, py)` among heatmap-pooled hits
    /// (strict passes and normed near-misses within [`HEATMAP_VIZ_MARGIN`]).
    fn pooled_score_at(
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

    pub(crate) fn spawn_compute(
        fingerprint: String,
        display: Arc<RgbaImage>,
        src_w: u32,
        src_h: u32,
        template_path: PathBuf,
        mask_path: Option<PathBuf>,
        blur: i32,
        method: MatchMethod,
        tolerance: f32,
        close_matches_distance: i32,
    ) -> mpsc::Receiver<Result<MatchCheckResult, String>> {
        let (tx, job_rx) = mpsc::channel();
        thread::spawn(move || {
            let result = (|| {
                let (map, image_w, image_h, tmpl_w, tmpl_h, mask_missing) = compute_match(
                    &display,
                    src_w,
                    src_h,
                    &template_path,
                    mask_path.as_deref(),
                    blur,
                    method,
                )?;
                let (heatmap_rgba, summary) = scores_to_heatmap_rgba(
                    &map, image_w, image_h, tmpl_w, tmpl_h, method, tolerance,
                );
                let tolerance_matches =
                    collect_tolerance_matches(&map, tolerance, close_matches_distance, method);
                Ok(MatchCheckResult {
                    data: MatchCheckData {
                        fingerprint,
                        match_map: map,
                        image_w,
                        image_h,
                        tmpl_w,
                        tmpl_h,
                        summary,
                        tolerance_matches,
                        mask_missing,
                    },
                    heatmap_rgba,
                })
            })();
            let _ = tx.send(result);
        });
        job_rx
    }

    pub(crate) fn finish_cache(ctx: &egui::Context, result: MatchCheckResult) -> MatchCheckCache {
        let size = [result.data.image_w, result.data.image_h];
        let color = ColorImage::from_rgba_unmultiplied(size, &result.heatmap_rgba);
        let opts = TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear));
        let heatmap = ctx.load_texture(
            format!("match_check_heatmap_{}", result.data.fingerprint),
            color,
            opts,
        );
        MatchCheckCache {
            data: result.data,
            heatmap,
        }
    }

    fn image_to_content(
        pos: egui::Pos2,
        content: egui::Rect,
        image_size: egui::Vec2,
    ) -> egui::Pos2 {
        let scale_x = content.width() / image_size.x.max(1.0);
        let scale_y = content.height() / image_size.y.max(1.0);
        egui::pos2(
            content.min.x + pos.x * scale_x,
            content.min.y + pos.y * scale_y,
        )
    }

    fn paint_tolerance_match_boxes(
        painter: &egui::Painter,
        content: egui::Rect,
        image_size: egui::Vec2,
        summary: &MatchSummary,
        tolerance_matches: &[ToleranceMatch],
        tmpl_w: usize,
        tmpl_h: usize,
        method: MatchMethod,
        tolerance: f32,
        show_match_boxes: bool,
    ) {
        if !show_match_boxes {
            return;
        }
        if tmpl_w == 0 || tmpl_h == 0 || !summary.best_score.is_finite() {
            return;
        }
        let scale_x = content.width() / image_size.x.max(1.0);
        let scale_y = content.height() / image_size.y.max(1.0);
        let box_size = egui::vec2(tmpl_w as f32 * scale_x, tmpl_h as f32 * scale_y);
        if tolerance_matches.is_empty() {
            if score_passes(summary.best_score, method, tolerance) {
                return;
            }
            let top_left = image_to_content(
                egui::pos2(summary.best_x as f32, summary.best_y as f32),
                content,
                image_size,
            );
            let rect = egui::Rect::from_min_size(top_left, box_size);
            let color = match_fail_fg();
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Outside,
            );
            painter.circle_filled(egui::pos2(rect.center().x, rect.min.y - 6.0), 4.0, color);
            return;
        }
        for m in tolerance_matches {
            if !score_passes(m.score, method, tolerance) {
                continue;
            }
            let is_best = m.x == summary.best_x && m.y == summary.best_y;
            let top_left =
                image_to_content(egui::pos2(m.x as f32, m.y as f32), content, image_size);
            let rect = egui::Rect::from_min_size(top_left, box_size);
            let color = if is_best {
                match_pass_fg()
            } else {
                match_within_fg()
            };
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(if is_best { 2.0 } else { 1.5 }, color),
                egui::StrokeKind::Outside,
            );
            if is_best {
                painter.circle_filled(egui::pos2(rect.center().x, rect.min.y - 6.0), 4.0, color);
            }
        }
    }

    pub(crate) fn paint_heatmap_overlay(
        ui: &mut egui::Ui,
        viewport: egui::Rect,
        image_size: egui::Vec2,
        view: &ImageViewTransform,
        cache: &MatchCheckCache,
        hover: &mut Option<MatchCheckHover>,
        method: MatchMethod,
        tolerance: f32,
        show_match_boxes: bool,
    ) {
        let content = image_view::image_content_rect(viewport, image_size, view.zoom, view.pan);
        let painter = ui.painter_at(viewport);
        painter.image(
            cache.heatmap.id(),
            content,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        paint_tolerance_match_boxes(
            &painter,
            content,
            image_size,
            &cache.data.summary,
            &cache.data.tolerance_matches,
            cache.data.tmpl_w,
            cache.data.tmpl_h,
            method,
            tolerance,
            show_match_boxes,
        );

        *hover = None;
        let hover_resp = ui.interact(
            content,
            ui.id().with("match_check_hover"),
            egui::Sense::hover(),
        );
        hover_resp.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Image,
                true,
                "Pixel match map — hover a pixel to inspect its match closeness",
            )
        });
        if let Some(pos) = hover_resp.hover_pos() {
            let scale_x = image_size.x / content.width().max(1.0);
            let scale_y = image_size.y / content.height().max(1.0);
            let ix = ((pos.x - content.min.x) * scale_x).floor() as i32;
            let iy = ((pos.y - content.min.y) * scale_y).floor() as i32;
            *hover = hover_at_image(
                &cache.data.match_map,
                &cache.data.summary,
                ix,
                iy,
                cache.data.tmpl_w,
                cache.data.tmpl_h,
                method,
                tolerance,
            );
        }
        if let Some(h) = hover {
            let tol_pct = if method.is_normed() {
                Some(tolerance * 100.0)
            } else {
                None
            };
            if let Some(pointer) = hover_resp.hover_pos() {
                paint_mouse_follow_tooltip(ui.ctx(), pointer, viewport, h, tol_pct);
                ui.ctx().request_repaint();
            }
        }
    }

    fn paint_mouse_follow_tooltip(
        ctx: &egui::Context,
        pointer: egui::Pos2,
        viewport: egui::Rect,
        h: &MatchCheckHover,
        tol_pct: Option<f32>,
    ) {
        const OFFSET: egui::Vec2 = egui::vec2(20.0, 20.0);
        const EST_W: f32 = 200.0;
        const EST_H: f32 = 88.0;
        let flip_x = pointer.x + OFFSET.x + EST_W > viewport.max.x;
        let flip_y = pointer.y + OFFSET.y + EST_H > viewport.max.y;
        let offset = egui::vec2(
            if flip_x { -EST_W - 12.0 } else { OFFSET.x },
            if flip_y { -EST_H - 12.0 } else { OFFSET.y },
        );
        let mut pos = pointer + offset;
        pos.x = pos.x.clamp(
            viewport.min.x + 4.0,
            (viewport.max.x - EST_W - 4.0).max(viewport.min.x),
        );
        pos.y = pos.y.clamp(
            viewport.min.y + 4.0,
            (viewport.max.y - EST_H - 4.0).max(viewport.min.y),
        );

        let pct_color = if h.passes {
            match_pass_fg()
        } else {
            match_fail_fg()
        };
        let status = if h.passes { "pass" } else { "below tolerance" };

        egui::Area::new(egui::Id::new("match_check_mouse_tooltip"))
            .fixed_pos(pos)
            .interactable(false)
            .show(ctx, |ui| {
                crate::theme::popup_frame(ui).show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("{:.1}% match", h.closeness_pct))
                            .size(24.0)
                            .strong()
                            .color(pct_color),
                    );
                    ui.label(
                        egui::RichText::new(format!("@ ({}, {}) — {status}", h.x, h.y)).size(15.0),
                    );
                    if let Some(tol) = tol_pct {
                        ui.label(
                            egui::RichText::new(format!("tolerance {:.0}%", tol))
                                .size(14.0)
                                .weak(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!("raw score {:.4}", h.score))
                                .size(14.0)
                                .weak(),
                        );
                    }
                });
            });
    }

    pub(crate) fn paint_legend(
        ui: &mut egui::Ui,
        summary: &MatchSummary,
        tolerance_match_count: usize,
        show_many_match_boxes: &mut bool,
        hover: Option<&MatchCheckHover>,
        method: MatchMethod,
        tolerance: f32,
        tmpl_w: usize,
        tmpl_h: usize,
    ) {
        ui.horizontal(|ui| {
            // Color scale bar
            let (bar, _) = ui.allocate_exact_size(egui::vec2(120.0, 10.0), egui::Sense::hover());
            if ui.is_rect_visible(bar) {
                let n = 24usize;
                let w = bar.width() / n as f32;
                for i in 0..n {
                    let t = i as f32 / (n - 1) as f32;
                    let (r, g, b) = similarity_rgb(t);
                    let seg = egui::Rect::from_min_size(
                        egui::pos2(bar.min.x + w * i as f32, bar.min.y),
                        egui::vec2(w.ceil(), bar.height()),
                    );
                    ui.painter()
                        .rect_filled(seg, 0.0, Color32::from_rgb(r, g, b));
                }
            }
            ui.weak("weak → strong");
        });
        ui.horizontal_wrapped(|ui| {
            if summary.best_score.is_finite() {
                let label = if summary.best_passes { "pass" } else { "fail" };
                let best_pct = closeness_percent(summary.best_score, method, summary);
                if method.is_normed() {
                    ui.label(format!(
                        "Best {best_pct:.1}% @ ({}, {}) ({label})",
                        summary.best_x, summary.best_y
                    ));
                    ui.weak(format!("raw {:.4}", summary.best_score));
                    ui.weak(format!("tolerance {:.0}%", tolerance * 100.0));
                } else {
                    ui.label(format!(
                        "Best {best_pct:.1}% @ ({}, {}) ({label})",
                        summary.best_x, summary.best_y
                    ));
                    ui.weak(format!("raw {:.4}", summary.best_score));
                    ui.weak(format!("tolerance {:.3}", tolerance));
                }
                if tolerance_match_count > 0 {
                    ui.weak(format!("{tolerance_match_count} match(es)"));
                } else if summary.peaks_above > 0 {
                    ui.weak(format!("{} ≥ tolerance (cells)", summary.peaks_above));
                }
            }
            if tolerance_match_count > MANY_MATCH_BOX_THRESHOLD {
                let hiding =
                    !should_paint_match_boxes(tolerance_match_count, *show_many_match_boxes);
                let btn = if *show_many_match_boxes {
                    "Hide match boxes".to_string()
                } else {
                    format!("Show {tolerance_match_count} match boxes")
                };
                if ui.small_button(btn).clicked() {
                    *show_many_match_boxes = !*show_many_match_boxes;
                }
                if hiding {
                    ui.weak("match boxes hidden (too many to paint by default)");
                }
            }
            if let Some(h) = hover {
                let label = if h.passes { "pass" } else { "fail" };
                ui.label(format!(
                    "Hover ({},{}) {:.1}% ({label})",
                    h.x, h.y, h.closeness_pct
                ));
            }
            if tmpl_w > 0 && tmpl_h > 0 {
                ui.weak(format!("template {tmpl_w}×{tmpl_h}"));
            }
            if summary.best_passes {
                ui.colored_label(match_pass_fg(), "■ best match");
            } else if summary.best_score.is_finite() {
                ui.colored_label(match_fail_fg(), "■ best (below tolerance)");
            }
            ui.colored_label(match_within_fg(), "□ within tolerance");
        });
    }

    pub(crate) fn inputs_key(
        prog: &str,
        item: &str,
        variant: &str,
        left: Option<i32>,
        top: Option<i32>,
        right: Option<i32>,
        bottom: Option<i32>,
        blur: i32,
        method: MatchMethod,
        tolerance: f64,
        refresh_gen: u64,
        close_matches_distance: i32,
    ) -> String {
        let (lx, ty, rx, by) = match (left, top, right, bottom) {
            (Some(lx), Some(ty), Some(rx), Some(by)) => (lx, ty, rx, by),
            _ => return String::new(),
        };
        fingerprint(
            prog,
            item,
            variant,
            lx,
            ty,
            rx,
            by,
            blur,
            method,
            tolerance,
            refresh_gen,
            close_matches_distance,
        )
    }

    pub(crate) fn resolve_template(
        catalog: CatalogView<'_>,
        prog: &str,
        item: &str,
        variant: &str,
    ) -> Result<PathBuf, String> {
        variant_path(catalog, prog, item, variant)
    }

    pub(crate) fn default_variant(catalog: CatalogView<'_>, prog: &str, item: &str) -> String {
        crate::icon_variants::variant_names(catalog, prog, item)
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    pub(crate) fn variant_options(
        catalog: CatalogView<'_>,
        prog: &str,
        item: &str,
    ) -> Vec<(String, String)> {
        crate::icon_variants::variant_names(catalog, prog, item)
            .into_iter()
            .map(|name| {
                let label = variant_display_label(&name).to_string();
                (name, label)
            })
            .collect()
    }

    pub(crate) fn should_request(
        settings: &MatchCheckSettings,
        inputs: &str,
        pending: bool,
        now: Instant,
    ) -> bool {
        if inputs.is_empty() || pending || settings.paused {
            return false;
        }
        if inputs != settings.last_inputs {
            return true;
        }
        settings
            .last_request
            .is_none_or(|t| now.duration_since(t) >= DEBOUNCE)
    }

    impl crate::data_editor::DataEditor {
        pub(crate) fn request_match_check(
            &mut self,
            catalog: CatalogView<'_>,
            left: Option<i32>,
            top: Option<i32>,
            right: Option<i32>,
            bottom: Option<i32>,
            force: bool,
            close_matches_distance: i32,
            display: Option<(Arc<RgbaImage>, u32, u32)>,
        ) {
            if force {
                // Unpause and prepare a new fingerprint, but keep the prior heatmap
                // until spawn succeeds so live refresh does not flicker.
                self.match_check.paused = false;
            }
            let (Some(prog), Some(item)) = (
                self.selected_program.as_deref(),
                self.selected_entity.as_deref(),
            ) else {
                self.stop_match_check_compute();
                return;
            };
            if !can_compute_match_check(
                catalog,
                prog,
                item,
                &self.match_check.variant,
                left,
                top,
                right,
                bottom,
            ) {
                self.stop_match_check_compute();
                return;
            }
            let refresh_gen = if force {
                self.match_check.refresh_gen.wrapping_add(1)
            } else {
                self.match_check.refresh_gen
            };
            let inputs = inputs_key(
                prog,
                item,
                &self.match_check.variant,
                left,
                top,
                right,
                bottom,
                self.match_check.blur,
                self.match_check.match_method,
                self.match_check.tolerance,
                refresh_gen,
                close_matches_distance,
            );
            if inputs.is_empty() {
                self.stop_match_check_compute();
                return;
            }
            if inputs != self.match_check.last_inputs {
                self.match_check.paused = false;
            }
            if self.match_check.paused {
                return;
            }
            if !force
                && self
                    .match_check_cache
                    .as_ref()
                    .is_some_and(|c| c.data.fingerprint == inputs)
            {
                return;
            }
            let now = Instant::now();
            if !force
                && !should_request(
                    &self.match_check,
                    &inputs,
                    self.match_check_pending.is_some(),
                    now,
                )
            {
                return;
            }
            let Ok(template_path) =
                resolve_template(catalog, prog, item, &self.match_check.variant)
            else {
                self.stop_match_check_compute();
                return;
            };
            if !template_path.is_file() {
                self.stop_match_check_compute();
                return;
            }
            let Some((display, src_w, src_h)) = display else {
                // Panel capture not ready yet — retry next frame once underlay lands.
                return;
            };
            // Commit the forced fingerprint only once capture is ready to spawn.
            if force {
                self.match_check.refresh_gen = refresh_gen;
            }
            let mask_path =
                catalog.mask_path(&format!("{prog}{}{item}", sqyre_domain::PROGRAM_DELIMITER));
            let tolerance = self.match_check.tolerance as f32;
            let job_rx = spawn_compute(
                inputs.clone(),
                display,
                src_w,
                src_h,
                template_path,
                mask_path,
                self.match_check.blur,
                self.match_check.match_method,
                tolerance,
                close_matches_distance,
            );
            self.match_check.last_inputs = inputs;
            self.match_check.last_request = Some(now);
            self.match_check_pending = Some(job_rx);
        }
    }
}

#[cfg(feature = "native-runtime")]
pub(crate) use inner::*;

#[cfg(all(test, feature = "native-runtime"))]
mod tests {
    use super::inner::{
        closeness_percent, collect_tolerance_matches, coords_displayable, hover_at_image,
        live_refresh_due, scale_image_buf, scores_to_heatmap_rgba, should_paint_match_boxes,
        MatchCheckSettings, MatchSummary, LIVE_INTERVAL, MANY_MATCH_BOX_THRESHOLD,
    };
    use sqyre_domain::MatchMethod;
    use sqyre_match::{ImageBuf, MatchMap};
    use web_time::{Duration, Instant};

    fn map(w: usize, h: usize, scores: &[f32]) -> MatchMap {
        MatchMap {
            width: w,
            height: h,
            scores: scores.to_vec(),
        }
    }

    fn summary(min: f32, max: f32) -> MatchSummary {
        MatchSummary {
            score_min: min,
            score_max: max,
            ..Default::default()
        }
    }

    #[test]
    fn heatmap_normed_higher_is_better() {
        // Wide image so peak kernels at x=0 and x=10 do not overlap.
        let mut scores = vec![0.0f32; 11];
        scores[0] = 0.5;
        scores[10] = 1.0;
        let map = map(11, 1, &scores);
        let (rgba, summary) =
            scores_to_heatmap_rgba(&map, 11, 1, 1, 1, MatchMethod::CcoeffNormed, 0.9);
        assert_eq!(rgba.len(), 11 * 4);
        // 0.5 is below viz margin — origin stays unpainted; peak at 10 paints.
        assert_eq!(rgba[3], 0, "weak origin should not paint");
        assert!(rgba[10 * 4 + 3] > 0, "strong peak should paint");
        assert!(summary.best_score > 0.99);
        assert_eq!(summary.best_x, 10);
    }

    #[test]
    fn heatmap_sqdiff_lower_is_better() {
        let mut scores = vec![1.0f32; 11];
        scores[0] = 0.1;
        scores[10] = 0.9;
        let map = map(11, 1, &scores);
        let (rgba, summary) =
            scores_to_heatmap_rgba(&map, 11, 1, 1, 1, MatchMethod::SqdiffNormed, 0.5);
        assert!(summary.best_score < 0.2);
        assert_eq!(summary.best_x, 0);
        assert!(rgba[3] > 0, "better match peak should paint");
        assert_eq!(
            rgba[10 * 4 + 3],
            0,
            "worse score outside tolerance should not paint"
        );
    }

    #[test]
    fn scale_image_buf_halves_dims() {
        let mut img = ImageBuf::new(4, 2, 3, 0);
        img.data[0] = 10;
        let corner = img.pixel_offset(3, 1);
        img.data[corner] = 20;
        let out = scale_image_buf(&img, 2, 1);
        assert_eq!(out.width, 2);
        assert_eq!(out.height, 1);
        assert_eq!(out.data[0], 10);
    }

    #[test]
    fn heatmap_peak_kernel_stays_compact() {
        let map = map(1, 1, &[1.0]);
        let (rgba, _) = scores_to_heatmap_rgba(&map, 20, 20, 8, 8, MatchMethod::CcoeffNormed, 0.9);
        let painted = rgba.chunks(4).filter(|p| p[3] > 0).count();
        // Soft disk radius 4 → at most (2*4+1)^2 = 81 cells, far below an 8×8 footprint.
        assert!(painted > 0 && painted <= 81, "painted={painted}");
    }

    #[test]
    fn heatmap_fades_weak_scores() {
        let map = map(1, 1, &[0.1]);
        let (rgba, _) = scores_to_heatmap_rgba(&map, 4, 4, 2, 2, MatchMethod::CcoeffNormed, 0.95);
        // Far below tolerance — should not paint any heat.
        let alphas: Vec<u8> = rgba.chunks(4).map(|p| p[3]).collect();
        assert!(
            alphas.iter().all(|&a| a == 0),
            "weak match should not paint: {alphas:?}"
        );
    }

    #[test]
    fn heatmap_suppresses_clustered_weak_scores() {
        // Tight score range that used to normalize to full-span yellow streaks.
        let map = map(4, 1, &[0.38, 0.39, 0.40, 0.41]);
        let (rgba, _) = scores_to_heatmap_rgba(&map, 8, 2, 2, 1, MatchMethod::CcoeffNormed, 0.95);
        let painted = rgba.chunks(4).filter(|p| p[3] > 0).count();
        assert_eq!(painted, 0, "clustered weak scores should not paint");
    }

    #[test]
    fn coords_displayable_rejects_invalid_bounds() {
        assert!(!coords_displayable(None, Some(0), Some(10), Some(10)));
        assert!(!coords_displayable(Some(0), Some(0), Some(0), Some(10)));
        assert!(!coords_displayable(Some(10), Some(0), Some(0), Some(10)));
        assert!(coords_displayable(Some(0), Some(0), Some(10), Some(10)));
    }

    #[test]
    fn hover_out_of_bounds() {
        let map = map(2, 2, &[0.5, 0.6, 0.7, 0.8]);
        let summary = summary(0.5, 0.8);
        assert!(
            hover_at_image(&map, &summary, -1, 0, 1, 1, MatchMethod::CcoeffNormed, 0.5).is_none()
        );
        let h = hover_at_image(&map, &summary, 0, 0, 1, 1, MatchMethod::CcoeffNormed, 0.4).unwrap();
        assert!((h.closeness_pct - 50.0).abs() < 0.1);
        assert!(h.passes);
    }

    #[test]
    fn closeness_normed_is_absolute_percent() {
        let summary = MatchSummary::default();
        assert!(
            (closeness_percent(0.873, MatchMethod::CcoeffNormed, &summary) - 87.3).abs() < 0.01
        );
    }

    #[test]
    fn hover_pooled_across_template_footprint() {
        let map = map(2, 1, &[0.5, 0.95]);
        let summary = summary(0.5, 0.95);
        // Pixel inside the high-score template block should inherit 0.95.
        let h = hover_at_image(&map, &summary, 1, 0, 2, 1, MatchMethod::CcoeffNormed, 0.9).unwrap();
        assert!((h.closeness_pct - 95.0).abs() < 0.1);
    }

    #[test]
    fn hover_includes_heatmap_near_miss() {
        // 0.85 is below tolerance 0.95 but within HEATMAP_VIZ_MARGIN (0.12).
        let map = map(2, 1, &[0.85, 0.95]);
        let summary = summary(0.85, 0.95);
        let h = hover_at_image(&map, &summary, 0, 0, 1, 1, MatchMethod::CcoeffNormed, 0.95)
            .expect("near-miss should be hoverable");
        assert!(!h.passes);
        assert!((h.closeness_pct - 85.0).abs() < 0.1);
    }

    #[test]
    fn hover_skips_far_below_heatmap_margin() {
        let map = map(2, 1, &[0.5, 0.95]);
        let summary = summary(0.5, 0.95);
        assert!(
            hover_at_image(&map, &summary, 0, 0, 1, 1, MatchMethod::CcoeffNormed, 0.95).is_none()
        );
    }

    #[test]
    fn collect_tolerance_matches_filters_weak_peaks() {
        let map = map(4, 1, &[0.38, 0.39, 0.96, 0.41]);
        let matches = collect_tolerance_matches(&map, 0.95, 0, MatchMethod::CcoeffNormed);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].x, 2);
        assert!(matches[0].score >= 0.95);
    }

    #[test]
    fn should_paint_match_boxes_threshold() {
        assert!(should_paint_match_boxes(MANY_MATCH_BOX_THRESHOLD, false));
        assert!(should_paint_match_boxes(50, false));
        assert!(!should_paint_match_boxes(
            MANY_MATCH_BOX_THRESHOLD + 1,
            false
        ));
        assert!(should_paint_match_boxes(MANY_MATCH_BOX_THRESHOLD + 1, true));
    }

    #[test]
    fn live_refresh_due_respects_pause_pending_and_interval() {
        let now = Instant::now();
        let mut settings = MatchCheckSettings::default();
        assert!(live_refresh_due(&settings, false, now));
        assert!(!live_refresh_due(&settings, true, now));
        settings.paused = true;
        assert!(!live_refresh_due(&settings, false, now));
        settings.paused = false;
        settings.last_live_at = Some(now);
        assert!(!live_refresh_due(&settings, false, now));
        assert!(live_refresh_due(
            &settings,
            false,
            now + LIVE_INTERVAL + Duration::from_millis(1)
        ));
    }
}
