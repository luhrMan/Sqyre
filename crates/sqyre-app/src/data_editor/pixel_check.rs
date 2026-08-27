//! PixelCheck: live template-match heatmap over a search-area preview.

#[cfg(feature = "native-runtime")]
#[allow(clippy::too_many_arguments)]
mod inner {
    use crate::data_editor_preview::variant_display_label;
    use crate::icon_variants::variant_path;
    use crate::image_view::{self, ImageViewTransform};
    use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
    use image::RgbaImage;
    use sqyre_capture::shared_capturer;
    use sqyre_domain::MatchMethod;
    use sqyre_match::{
        blur_image_owned, find_peaks_for_method, match_template_with_prepared, prepare_search,
        prepare_template, search_blur_kernel, MatchMap,
    };
    use sqyre_persist::ProgramCatalog;
    use sqyre_ports::DesktopRect;
    use sqyre_vision::{get_cached_blurred_template, get_cached_image_mask, rgba_to_rgb_buf};
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;
    use std::thread;
    use web_time::{Duration, Instant};

    const PANEL_MAX_DIM: u32 = 1600;
    const MIN_CAPTURE_SIZE: i32 = 320;
    const CAPTURE_PADDING: i32 = 48;
    const DEBOUNCE: Duration = Duration::from_millis(200);
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

    pub(crate) struct PixelCheckCache {
        pub fingerprint: String,
        pub match_map: MatchMap,
        pub image_w: usize,
        pub image_h: usize,
        pub tmpl_w: usize,
        pub tmpl_h: usize,
        pub summary: MatchSummary,
        pub tolerance_matches: Vec<ToleranceMatch>,
        pub heatmap: TextureHandle,
    }

    pub(crate) struct PixelCheckResult {
        pub fingerprint: String,
        pub match_map: MatchMap,
        pub image_w: usize,
        pub image_h: usize,
        pub tmpl_w: usize,
        pub tmpl_h: usize,
        pub summary: MatchSummary,
        pub tolerance_matches: Vec<ToleranceMatch>,
        pub heatmap_rgba: Vec<u8>,
    }

    pub(crate) struct PixelCheckHover {
        pub x: i32,
        pub y: i32,
        pub score: f32,
        /// 0–100 display closeness for the hovered image pixel.
        pub closeness_pct: f32,
        pub passes: bool,
    }

    /// Session-local match settings for PixelCheck (not persisted).
    pub(crate) struct PixelCheckSettings {
        pub tolerance: f64,
        pub blur: i32,
        pub match_method: MatchMethod,
        pub variant: String,
        pub refresh_gen: u64,
        pub last_inputs: String,
        pub last_request: Option<Instant>,
        /// When true, debounced refresh and retries are suppressed until inputs change or ↻.
        pub paused: bool,
        /// Opt-in paint for match boxes when count exceeds [`MANY_MATCH_BOX_THRESHOLD`].
        pub show_many_match_boxes: bool,
    }

    impl Default for PixelCheckSettings {
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
            }
        }
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

    pub(crate) fn can_compute_pixel_check(
        catalog: &ProgramCatalog,
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
        resolve_template(catalog, prog, item, variant).is_file()
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

    fn preview_bounds_for_search_area(
        lx: i32,
        ty: i32,
        rx: i32,
        by: i32,
        vb: DesktopRect,
    ) -> DesktopRect {
        let aw = (rx - lx).max(0);
        let ah = (by - ty).max(0);
        let pad_x = CAPTURE_PADDING.max(aw / 4);
        let pad_y = CAPTURE_PADDING.max(ah / 4);
        let desired = expand_to_min(
            DesktopRect {
                x: lx - pad_x,
                y: ty - pad_y,
                w: aw + pad_x * 2,
                h: ah + pad_y * 2,
            },
            MIN_CAPTURE_SIZE,
            MIN_CAPTURE_SIZE,
        );
        shift_into_virtual(desired, vb)
    }

    fn expand_to_min(r: DesktopRect, min_w: i32, min_h: i32) -> DesktopRect {
        if r.is_empty() {
            return r;
        }
        let w = r.w.max(min_w);
        let h = r.h.max(min_h);
        let cx = r.x + r.w / 2;
        let cy = r.y + r.h / 2;
        DesktopRect {
            x: cx - w / 2,
            y: cy - h / 2,
            w,
            h,
        }
    }

    fn shift_into_virtual(desired: DesktopRect, vb: DesktopRect) -> DesktopRect {
        if desired.is_empty() || vb.is_empty() {
            return DesktopRect::from_corners(
                desired.x.max(vb.x),
                desired.y.max(vb.y),
                (desired.x + desired.w).min(vb.x + vb.w),
                (desired.y + desired.h).min(vb.y + vb.h),
            );
        }
        let mut w = desired.w;
        let mut h = desired.h;
        if w <= 0 || h <= 0 {
            return DesktopRect::default();
        }
        if w >= vb.w && h >= vb.h {
            return vb;
        }
        let mut x0 = desired.x;
        let mut y0 = desired.y;
        if w > vb.w {
            x0 = vb.x;
            w = vb.w;
        } else {
            if x0 < vb.x {
                x0 = vb.x;
            }
            if x0 + w > vb.x + vb.w {
                x0 = vb.x + vb.w - w;
            }
        }
        if h > vb.h {
            y0 = vb.y;
            h = vb.h;
        } else {
            if y0 < vb.y {
                y0 = vb.y;
            }
            if y0 + h > vb.y + vb.h {
                y0 = vb.y + vb.h - h;
            }
        }
        DesktopRect { x: x0, y: y0, w, h }
    }

    fn downscale_max_dim(img: RgbaImage, max_dim: u32) -> RgbaImage {
        let (w, h) = img.dimensions();
        if w <= max_dim && h <= max_dim {
            return img;
        }
        let longest = w.max(h).max(1);
        let nw = ((w as u64 * max_dim as u64) / longest as u64).max(1) as u32;
        let nh = ((h as u64 * max_dim as u64) / longest as u64).max(1) as u32;
        let filter = if max_dim >= PANEL_MAX_DIM {
            image::imageops::FilterType::CatmullRom
        } else {
            image::imageops::FilterType::Triangle
        };
        image::imageops::resize(&img, nw, nh, filter)
    }

    fn capture_search_area(lx: i32, ty: i32, rx: i32, by: i32) -> Result<RgbaImage, String> {
        let capturer = shared_capturer().map_err(|e| e.to_string())?;
        let vb = capturer.virtual_bounds_ref().map_err(|e| e.to_string())?;
        let (lx, ty, rx, by) = DesktopRect::normalize_corners(lx, ty, rx, by);
        if rx <= lx || by <= ty {
            return Err("invalid search area bounds".into());
        }
        let bounds = preview_bounds_for_search_area(lx, ty, rx, by, vb);
        let img = capturer
            .capture_rect_ref(bounds)
            .map_err(|e| e.to_string())?;
        Ok(downscale_max_dim(img, PANEL_MAX_DIM))
    }

    fn compute_match(
        template_path: &Path,
        mask_path: Option<&Path>,
        lx: i32,
        ty: i32,
        rx: i32,
        by: i32,
        blur: i32,
        method: MatchMethod,
    ) -> Result<(MatchMap, usize, usize, usize, usize), String> {
        let img = capture_search_area(lx, ty, rx, by)?;
        let image_w = img.width() as usize;
        let image_h = img.height() as usize;
        let search = rgba_to_rgb_buf(&img);
        let kernel = search_blur_kernel(blur);
        let search_blurred = if kernel > 0 {
            blur_image_owned(search, kernel).map_err(|e| e.to_string())?
        } else {
            search
        };
        let template_blurred = get_cached_blurred_template(template_path, kernel)
            .map_err(|e| format!("template: {e}"))?;
        let tmpl_w = template_blurred.width;
        let tmpl_h = template_blurred.height;
        let mask_bytes = mask_path.and_then(|p| get_cached_image_mask(p, tmpl_h, tmpl_w));
        let mask_ref = mask_bytes.as_deref().map(|m| m.as_slice());
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
        Ok((map, image_w, image_h, tmpl_w, tmpl_h))
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

    /// Build RGBA heatmap: template-sized blocks, max-pooled scores, graded colormap.
    pub(crate) fn scores_to_heatmap_rgba(
        map: &MatchMap,
        image_w: usize,
        image_h: usize,
        tmpl_w: usize,
        tmpl_h: usize,
        method: MatchMethod,
        tolerance: f32,
    ) -> (Vec<u8>, MatchSummary) {
        let summary = compute_match_summary(map, method, tolerance);
        let mut rgba = vec![0u8; image_w * image_h * 4];
        if map.width == 0 || map.height == 0 || image_w == 0 || image_h == 0 {
            return (rgba, summary);
        }

        // Max-pool template footprints so each pixel shows the best overlapping match.
        let init = if method.higher_is_better() {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
        let mut pooled = vec![init; image_w * image_h];
        let tw = tmpl_w.min(image_w);
        let th = tmpl_h.min(image_h);
        for y in 0..map.height {
            for x in 0..map.width {
                let score = map.scores[y * map.width + x];
                if !score_pools_in_heatmap(score, method, tolerance) {
                    continue;
                }
                let x1 = (x + tw).min(image_w);
                let y1 = (y + th).min(image_h);
                for py in y..y1 {
                    for px in x..x1 {
                        let idx = py * image_w + px;
                        if method.higher_is_better() {
                            if score > pooled[idx] {
                                pooled[idx] = score;
                            }
                        } else if score < pooled[idx] {
                            pooled[idx] = score;
                        }
                    }
                }
            }
        }

        for py in 0..image_h {
            for px in 0..image_w {
                let score = pooled[py * image_w + px];
                if !score.is_finite() {
                    continue;
                }
                let norm = score_norm_for_display(score, method, &summary);
                let px_rgba = pixel_rgba(score, norm, method, tolerance);
                if px_rgba[3] == 0 {
                    continue;
                }
                let i = (py * image_w + px) * 4;
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

    /// Best template-match score covering image pixel `(px, py)` among strict tolerance hits.
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
                if !score_passes(score, method, tolerance) {
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
    ) -> Option<PixelCheckHover> {
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
        Some(PixelCheckHover {
            x: ix,
            y: iy,
            score,
            closeness_pct: closeness_percent(score, method, summary),
            passes,
        })
    }

    pub(crate) fn spawn_compute(
        fingerprint: String,
        template_path: PathBuf,
        mask_path: Option<PathBuf>,
        lx: i32,
        ty: i32,
        rx: i32,
        by: i32,
        blur: i32,
        method: MatchMethod,
        tolerance: f32,
        close_matches_distance: i32,
        image_w_hint: usize,
        image_h_hint: usize,
    ) -> mpsc::Receiver<Result<PixelCheckResult, String>> {
        let (tx, job_rx) = mpsc::channel();
        thread::spawn(move || {
            let result = (|| {
                let (map, image_w, image_h, tmpl_w, tmpl_h) = compute_match(
                    &template_path,
                    mask_path.as_deref(),
                    lx,
                    ty,
                    rx,
                    by,
                    blur,
                    method,
                )?;
                let (heatmap_rgba, summary) = scores_to_heatmap_rgba(
                    &map,
                    image_w.max(image_w_hint),
                    image_h.max(image_h_hint),
                    tmpl_w,
                    tmpl_h,
                    method,
                    tolerance,
                );
                let tolerance_matches =
                    collect_tolerance_matches(&map, tolerance, close_matches_distance, method);
                Ok(PixelCheckResult {
                    fingerprint,
                    match_map: map,
                    image_w,
                    image_h,
                    tmpl_w,
                    tmpl_h,
                    summary,
                    tolerance_matches,
                    heatmap_rgba,
                })
            })();
            let _ = tx.send(result);
        });
        job_rx
    }

    pub(crate) fn finish_cache(ctx: &egui::Context, result: PixelCheckResult) -> PixelCheckCache {
        let size = [result.image_w, result.image_h];
        let color = ColorImage::from_rgba_unmultiplied(size, &result.heatmap_rgba);
        let opts = TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear));
        let heatmap = ctx.load_texture(
            format!("pixel_check_heatmap_{}", result.fingerprint),
            color,
            opts,
        );
        PixelCheckCache {
            fingerprint: result.fingerprint,
            match_map: result.match_map,
            image_w: result.image_w,
            image_h: result.image_h,
            tmpl_w: result.tmpl_w,
            tmpl_h: result.tmpl_h,
            summary: result.summary,
            tolerance_matches: result.tolerance_matches,
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
            let color = Color32::from_rgb(255, 200, 60);
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
                Color32::from_rgb(80, 255, 120)
            } else {
                Color32::from_rgb(120, 230, 180)
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
        cache: &PixelCheckCache,
        hover: &mut Option<PixelCheckHover>,
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
            &cache.summary,
            &cache.tolerance_matches,
            cache.tmpl_w,
            cache.tmpl_h,
            method,
            tolerance,
            show_match_boxes,
        );

        *hover = None;
        let hover_resp = ui.interact(
            content,
            ui.id().with("pixel_check_hover"),
            egui::Sense::hover(),
        );
        if let Some(pos) = hover_resp.hover_pos() {
            let scale_x = image_size.x / content.width().max(1.0);
            let scale_y = image_size.y / content.height().max(1.0);
            let ix = ((pos.x - content.min.x) * scale_x).floor() as i32;
            let iy = ((pos.y - content.min.y) * scale_y).floor() as i32;
            *hover = hover_at_image(
                &cache.match_map,
                &cache.summary,
                ix,
                iy,
                cache.tmpl_w,
                cache.tmpl_h,
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
        h: &PixelCheckHover,
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
            Color32::from_rgb(100, 230, 130)
        } else {
            Color32::from_rgb(255, 200, 80)
        };
        let status = if h.passes { "pass" } else { "below tolerance" };

        egui::Area::new(egui::Id::new("pixel_check_mouse_tooltip"))
            .fixed_pos(pos)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.1}% match", h.closeness_pct))
                                .size(24.0)
                                .strong()
                                .color(pct_color),
                        );
                        ui.label(
                            egui::RichText::new(format!("@ ({}, {}) — {status}", h.x, h.y))
                                .size(15.0),
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
        hover: Option<&PixelCheckHover>,
        tolerance: f32,
        tmpl_w: usize,
        tmpl_h: usize,
    ) {
        ui.horizontal_wrapped(|ui| {
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
                ui.label(format!(
                    "Best {:.4} @ ({}, {}) ({label})",
                    summary.best_score, summary.best_x, summary.best_y
                ));
                ui.weak(format!(
                    "range {:.4}–{:.4}",
                    summary.score_min, summary.score_max
                ));
                ui.weak(format!("tolerance {:.3}", tolerance));
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
                ui.colored_label(Color32::from_rgb(80, 220, 80), "■ best match");
            } else if summary.best_score.is_finite() {
                ui.colored_label(Color32::from_rgb(255, 200, 60), "■ best (below tolerance)");
            }
            ui.colored_label(Color32::from_rgb(120, 230, 180), "□ within tolerance");
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
        catalog: &ProgramCatalog,
        prog: &str,
        item: &str,
        variant: &str,
    ) -> PathBuf {
        variant_path(catalog, prog, item, variant)
    }

    pub(crate) fn default_variant(catalog: &ProgramCatalog, prog: &str, item: &str) -> String {
        crate::icon_variants::variant_names(catalog, prog, item)
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    pub(crate) fn variant_options(
        catalog: &ProgramCatalog,
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
        settings: &PixelCheckSettings,
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
        pub(crate) fn request_pixel_check_match(
            &mut self,
            catalog: &ProgramCatalog,
            left: Option<i32>,
            top: Option<i32>,
            right: Option<i32>,
            bottom: Option<i32>,
            force: bool,
            close_matches_distance: i32,
        ) {
            if force {
                self.pixel_check.refresh_gen = self.pixel_check.refresh_gen.wrapping_add(1);
                self.pixel_check.paused = false;
                self.invalidate_pixel_check();
            }
            let (Some(prog), Some(item)) = (
                self.selected_program.as_deref(),
                self.selected_entity.as_deref(),
            ) else {
                self.stop_pixel_check_compute();
                return;
            };
            if !can_compute_pixel_check(
                catalog,
                prog,
                item,
                &self.pixel_check.variant,
                left,
                top,
                right,
                bottom,
            ) {
                self.stop_pixel_check_compute();
                return;
            }
            let inputs = inputs_key(
                prog,
                item,
                &self.pixel_check.variant,
                left,
                top,
                right,
                bottom,
                self.pixel_check.blur,
                self.pixel_check.match_method,
                self.pixel_check.tolerance,
                self.pixel_check.refresh_gen,
                close_matches_distance,
            );
            if inputs.is_empty() {
                self.stop_pixel_check_compute();
                return;
            }
            if inputs != self.pixel_check.last_inputs {
                self.pixel_check.paused = false;
            }
            if self.pixel_check.paused {
                return;
            }
            if self
                .pixel_check_cache
                .as_ref()
                .is_some_and(|c| c.fingerprint == inputs)
            {
                return;
            }
            let now = Instant::now();
            if !should_request(
                &self.pixel_check,
                &inputs,
                self.pixel_check_pending.is_some(),
                now,
            ) {
                return;
            }
            let template_path = resolve_template(catalog, prog, item, &self.pixel_check.variant);
            if !template_path.is_file() {
                self.stop_pixel_check_compute();
                return;
            }
            let (lx, ty, rx, by) = match (left, top, right, bottom) {
                (Some(lx), Some(ty), Some(rx), Some(by)) => (lx, ty, rx, by),
                _ => return,
            };
            let mask_path =
                catalog.mask_path(&format!("{prog}{}{item}", sqyre_domain::PROGRAM_DELIMITER));
            let tolerance = self.pixel_check.tolerance as f32;
            let job_rx = spawn_compute(
                inputs.clone(),
                template_path,
                mask_path,
                lx,
                ty,
                rx,
                by,
                self.pixel_check.blur,
                self.pixel_check.match_method,
                tolerance,
                close_matches_distance,
                0,
                0,
            );
            self.pixel_check.last_inputs = inputs;
            self.pixel_check.last_request = Some(now);
            self.pixel_check_pending = Some(job_rx);
        }
    }
}

#[cfg(feature = "native-runtime")]
pub(crate) use inner::*;

#[cfg(all(test, feature = "native-runtime"))]
mod tests {
    use super::inner::{
        closeness_percent, collect_tolerance_matches, coords_displayable, hover_at_image,
        scores_to_heatmap_rgba, should_paint_match_boxes, MatchSummary, MANY_MATCH_BOX_THRESHOLD,
    };
    use sqyre_domain::MatchMethod;
    use sqyre_match::MatchMap;

    #[test]
    fn heatmap_normed_higher_is_better() {
        let map = MatchMap {
            width: 2,
            height: 1,
            scores: vec![0.5, 1.0],
        };
        let (rgba, summary) =
            scores_to_heatmap_rgba(&map, 2, 1, 1, 1, MatchMethod::CcoeffNormed, 0.9);
        assert_eq!(rgba.len(), 8);
        // Higher score → greener (index 1 and 5 are G channels)
        assert!(rgba[5] > rgba[1]);
        assert!(summary.best_score > 0.99);
        assert!(summary.best_x == 1);
    }

    #[test]
    fn heatmap_sqdiff_lower_is_better() {
        let map = MatchMap {
            width: 2,
            height: 1,
            scores: vec![0.1, 0.9],
        };
        let (rgba, summary) =
            scores_to_heatmap_rgba(&map, 2, 1, 1, 1, MatchMethod::SqdiffNormed, 0.5);
        assert!(summary.best_score < 0.2);
        assert_eq!(summary.best_x, 0);
        assert!(rgba[3] > rgba[7], "better match should be more opaque");
    }

    #[test]
    fn heatmap_fades_weak_scores() {
        let map = MatchMap {
            width: 1,
            height: 1,
            scores: vec![0.1],
        };
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
        let map = MatchMap {
            width: 4,
            height: 1,
            scores: vec![0.38, 0.39, 0.40, 0.41],
        };
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
        let map = MatchMap {
            width: 2,
            height: 2,
            scores: vec![0.5, 0.6, 0.7, 0.8],
        };
        let summary = MatchSummary {
            score_min: 0.5,
            score_max: 0.8,
            ..Default::default()
        };
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
        let map = MatchMap {
            width: 2,
            height: 1,
            scores: vec![0.5, 0.95],
        };
        let summary = MatchSummary {
            score_min: 0.5,
            score_max: 0.95,
            ..Default::default()
        };
        // Pixel inside the high-score template block should inherit 0.95.
        let h = hover_at_image(&map, &summary, 1, 0, 2, 1, MatchMethod::CcoeffNormed, 0.9).unwrap();
        assert!((h.closeness_pct - 95.0).abs() < 0.1);
    }

    #[test]
    fn hover_strict_skips_below_tolerance() {
        let map = MatchMap {
            width: 2,
            height: 1,
            scores: vec![0.5, 0.95],
        };
        let summary = MatchSummary {
            score_min: 0.5,
            score_max: 0.95,
            ..Default::default()
        };
        assert!(
            hover_at_image(&map, &summary, 0, 0, 2, 1, MatchMethod::CcoeffNormed, 0.9).is_none()
        );
    }

    #[test]
    fn collect_tolerance_matches_filters_weak_peaks() {
        let map = MatchMap {
            width: 4,
            height: 1,
            scores: vec![0.38, 0.39, 0.96, 0.41],
        };
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
}
