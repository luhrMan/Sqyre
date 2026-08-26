//! Live template-match overlays for Image Search action tooltips.

use crate::demo_icons;
use crate::icon_cache::IconCache;
use eframe::egui::{self, Color32};
use sqyre_capture::shared_capturer;
use sqyre_domain::{CoordinateRef, Macro, MatchMethod};
use sqyre_match::{
    blur_image_owned, find_peaks_for_method, match_template_with_prepared, prepare_search,
    prepare_template, search_blur_kernel, MatchMap,
};
use sqyre_persist::ProgramCatalog;
use sqyre_ports::DesktopRect;
use sqyre_vision::{get_cached_blurred_template, get_cached_image_mask, rgba_to_rgb_buf};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use web_time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(250);
const TOOLTIP_MAX_DIM: u32 = 640;
const MIN_CAPTURE_SIZE: i32 = 320;
const CAPTURE_PADDING: i32 = 48;
/// Above this count, match boxes are hidden (paint cost in a small tooltip).
const MANY_MATCH_BOX_THRESHOLD: usize = 100;

const TARGET_COLORS: &[Color32] = &[
    Color32::from_rgb(80, 255, 120),
    Color32::from_rgb(120, 180, 255),
    Color32::from_rgb(255, 200, 80),
    Color32::from_rgb(255, 120, 180),
    Color32::from_rgb(180, 255, 120),
    Color32::from_rgb(255, 140, 100),
];

#[derive(Clone, Debug)]
pub(crate) struct ToleranceMatch {
    pub x: i32,
    pub y: i32,
    pub score: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct VariantMatches {
    pub target_index: usize,
    pub variant_index: usize,
    pub tmpl_w: usize,
    pub tmpl_h: usize,
    pub matches: Vec<ToleranceMatch>,
}

#[derive(Clone, Debug)]
pub(crate) struct ImageSearchPreviewResult {
    pub fingerprint: String,
    pub image_w: usize,
    pub image_h: usize,
    pub variants: Vec<VariantMatches>,
}

#[derive(Default)]
pub(crate) struct ImageSearchPreviewCache {
    pub close_matches_distance: i32,
    pending: Option<Receiver<Result<ImageSearchPreviewResult, String>>>,
    cache: Option<ImageSearchPreviewResult>,
    last_inputs: String,
    last_request: Option<Instant>,
    refresh_gen: u64,
}

impl ImageSearchPreviewCache {
    pub fn clear(&mut self) {
        self.pending = None;
        self.cache = None;
        self.last_inputs.clear();
        self.last_request = None;
    }

    pub fn invalidate(&mut self) {
        self.clear();
        self.refresh_gen = self.refresh_gen.wrapping_add(1);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn paint_overlays(
        &mut self,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        macro_: &Macro,
        search_area: &CoordinateRef,
        targets: &[String],
        tolerance: f64,
        blur: i32,
        match_method: MatchMethod,
        force: bool,
        preview: Option<(egui::Rect, egui::Vec2)>,
    ) {
        if search_area.is_empty() || targets.is_empty() {
            return;
        }

        let Ok((lx, ty, rx, by)) = catalog.resolve_search_area(search_area, macro_) else {
            return;
        };

        if force {
            self.invalidate();
        }

        let inputs = fingerprint(
            targets,
            lx,
            ty,
            rx,
            by,
            blur,
            match_method,
            tolerance,
            self.refresh_gen,
            self.close_matches_distance,
        );

        let Some((image_rect, image_size)) = preview else {
            paint_params(ui, tolerance, blur, match_method);
            paint_target_legend(ui, egui_ctx, catalog, icons, targets, &[]);
            return;
        };

        self.poll(egui_ctx, &inputs);

        if self.should_request(&inputs) {
            self.last_inputs = inputs.clone();
            self.last_request = Some(Instant::now());
            self.pending = Some(spawn_compute(
                inputs,
                catalog,
                targets,
                lx,
                ty,
                rx,
                by,
                blur,
                match_method,
                tolerance as f32,
                self.close_matches_distance,
            ));
            egui_ctx.request_repaint();
        }

        if self.pending.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak("Finding matches…");
            });
        }

        let Some(cache) = self.cache.as_ref() else {
            paint_params(ui, tolerance, blur, match_method);
            paint_target_legend(ui, egui_ctx, catalog, icons, targets, &[]);
            return;
        };

        if cache.fingerprint != inputs {
            return;
        }

        let total_matches: usize = cache.variants.iter().map(|v| v.matches.len()).sum();
        let show_boxes = total_matches <= MANY_MATCH_BOX_THRESHOLD;
        if show_boxes {
            paint_match_boxes(
                ui,
                image_rect,
                image_size,
                &cache.variants,
                match_method,
                tolerance as f32,
            );
        } else {
            ui.weak(format!(
                "{total_matches} matches (too many to draw — lower tolerance or search area)"
            ));
        }

        paint_params(ui, tolerance, blur, match_method);
        let counts = target_match_counts(targets.len(), &cache.variants);
        paint_target_legend(ui, egui_ctx, catalog, icons, targets, &counts);
    }

    fn poll(&mut self, ctx: &egui::Context, inputs: &str) {
        let Some(rx) = self.pending.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(result)) if result.fingerprint == *inputs => {
                self.pending = None;
                self.cache = Some(result);
            }
            Ok(Ok(_)) => {
                self.pending = None;
            }
            Ok(Err(_)) => {
                self.pending = None;
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint();
            }
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
            }
        }
    }

    fn should_request(&self, inputs: &str) -> bool {
        if inputs.is_empty() || self.pending.is_some() {
            return false;
        }
        if inputs != self.last_inputs {
            return true;
        }
        self.last_request
            .is_none_or(|t| Instant::now().duration_since(t) >= DEBOUNCE)
    }
}

fn target_match_counts(target_len: usize, variants: &[VariantMatches]) -> Vec<usize> {
    let mut counts = vec![0usize; target_len];
    for v in variants {
        if v.target_index < counts.len() {
            counts[v.target_index] += v.matches.len();
        }
    }
    counts
}

fn paint_params(ui: &mut egui::Ui, tolerance: f64, blur: i32, match_method: MatchMethod) {
    let tol = format!("{tolerance:.3}");
    ui.weak(format!(
        "Tolerance {tol} · Blur {blur} · {}",
        match_method.label()
    ));
}

fn paint_target_legend(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    targets: &[String],
    counts: &[usize],
) {
    ui.add_space(2.0);
    for (i, target) in targets.iter().enumerate() {
        let count = counts.get(i).copied().unwrap_or(0);
        ui.horizontal(|ui| {
            let tex = icons.for_target_or_fallback(ctx, catalog, target);
            let side = 18.0;
            ui.add(
                egui::Image::new((tex.id(), egui::vec2(side, side)))
                    .fit_to_exact_size(egui::vec2(side, side))
                    .maintain_aspect_ratio(true),
            );
            let short = target
                .split('~')
                .next_back()
                .unwrap_or(target.as_str());
            if counts.is_empty() {
                ui.label(short);
            } else {
                ui.label(format!("{short} — {count} match{}", if count == 1 { "" } else { "es" }));
            }
        });
    }
}

fn paint_match_boxes(
    ui: &egui::Ui,
    image_rect: egui::Rect,
    image_size: egui::Vec2,
    variants: &[VariantMatches],
    method: MatchMethod,
    tolerance: f32,
) {
    let scale_x = image_rect.width() / image_size.x.max(1.0);
    let scale_y = image_rect.height() / image_size.y.max(1.0);
    let painter = ui.painter();
    for v in variants {
        if v.tmpl_w == 0 || v.tmpl_h == 0 {
            continue;
        }
        let color = TARGET_COLORS[v.target_index % TARGET_COLORS.len()];
        let box_size = egui::vec2(v.tmpl_w as f32 * scale_x, v.tmpl_h as f32 * scale_y);
        for m in &v.matches {
            if !score_passes(m.score, method, tolerance) {
                continue;
            }
            let top_left = egui::pos2(
                image_rect.min.x + m.x as f32 * scale_x,
                image_rect.min.y + m.y as f32 * scale_y,
            );
            let rect = egui::Rect::from_min_size(top_left, box_size);
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.5, color),
                egui::StrokeKind::Outside,
            );
        }
    }
}

fn fingerprint(
    targets: &[String],
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
        "{}\0{lx},{ty},{rx},{by}\0{blur}\0{method:?}\0{tolerance}\0{refresh_gen}\0{close_matches_distance}",
        targets.join("\x1f"),
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_compute(
    fingerprint: String,
    catalog: &ProgramCatalog,
    targets: &[String],
    lx: i32,
    ty: i32,
    right: i32,
    bottom: i32,
    blur: i32,
    method: MatchMethod,
    tolerance: f32,
    close_matches_distance: i32,
) -> Receiver<Result<ImageSearchPreviewResult, String>> {
    let targets = targets.to_vec();
    let catalog = catalog.clone();
    let (tx, job_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = compute_all(
            &fingerprint,
            &catalog,
            &targets,
            lx,
            ty,
            right,
            bottom,
            blur,
            method,
            tolerance,
            close_matches_distance,
        );
        let _ = tx.send(result);
    });
    job_rx
}

#[allow(clippy::too_many_arguments)]
fn compute_all(
    fingerprint: &str,
    catalog: &ProgramCatalog,
    targets: &[String],
    lx: i32,
    ty: i32,
    rx: i32,
    by: i32,
    blur: i32,
    method: MatchMethod,
    tolerance: f32,
    close_matches_distance: i32,
) -> Result<ImageSearchPreviewResult, String> {
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
    let search_prep = prepare_search(&search_blurred);

    let mut variants = Vec::new();
    for (target_index, target) in targets.iter().enumerate() {
        let paths = demo_icons::merged_variant_paths(catalog, target);
        if paths.is_empty() {
            continue;
        }
        let mask_path = catalog.mask_path(target);
        for (variant_index, path) in paths.into_iter().enumerate() {
            let Some(matches) = match_variant(
                &path,
                mask_path.as_deref(),
                &search_blurred,
                &search_prep,
                kernel,
                method,
                tolerance,
                close_matches_distance,
            ) else {
                continue;
            };
            variants.push(VariantMatches {
                target_index,
                variant_index,
                tmpl_w: matches.0,
                tmpl_h: matches.1,
                matches: matches.2,
            });
        }
    }

    Ok(ImageSearchPreviewResult {
        fingerprint: fingerprint.to_string(),
        image_w,
        image_h,
        variants,
    })
}

fn match_variant(
    template_path: &Path,
    mask_path: Option<&Path>,
    search_blurred: &sqyre_match::ImageBuf,
    search_prep: &sqyre_match::SearchPrep,
    kernel: i32,
    method: MatchMethod,
    tolerance: f32,
    close_matches_distance: i32,
) -> Option<(usize, usize, Vec<ToleranceMatch>)> {
    let template_blurred = get_cached_blurred_template(template_path, kernel).ok()?;
    let tmpl_w = template_blurred.width;
    let tmpl_h = template_blurred.height;
    let mask_bytes = mask_path.and_then(|p| get_cached_image_mask(p, tmpl_h, tmpl_w));
    let mask_ref = mask_bytes.as_deref().map(|m| m.as_slice());
    let prepared = prepare_template(&template_blurred, mask_ref, method).ok()?;
    let map = match_template_with_prepared(
        search_blurred,
        &template_blurred,
        &prepared,
        Some(search_prep),
    )
    .ok()?;
    let matches = collect_tolerance_matches(&map, tolerance, close_matches_distance, method);
    Some((tmpl_w, tmpl_h, matches))
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

fn collect_tolerance_matches(
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

fn capture_search_area(lx: i32, ty: i32, rx: i32, by: i32) -> Result<image::RgbaImage, String> {
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
    Ok(downscale_max_dim(img, TOOLTIP_MAX_DIM))
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

fn downscale_max_dim(img: image::RgbaImage, max_dim: u32) -> image::RgbaImage {
    let (w, h) = img.dimensions();
    if w <= max_dim && h <= max_dim {
        return img;
    }
    let longest = w.max(h).max(1);
    let nw = ((w as u64 * max_dim as u64) / longest as u64).max(1) as u32;
    let nh = ((h as u64 * max_dim as u64) / longest as u64).max(1) as u32;
    image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_match_counts_sums_variants() {
        let variants = vec![
            VariantMatches {
                target_index: 0,
                variant_index: 0,
                tmpl_w: 1,
                tmpl_h: 1,
                matches: vec![
                    ToleranceMatch { x: 0, y: 0, score: 1.0 },
                    ToleranceMatch { x: 1, y: 0, score: 1.0 },
                ],
            },
            VariantMatches {
                target_index: 1,
                variant_index: 0,
                tmpl_w: 1,
                tmpl_h: 1,
                matches: vec![ToleranceMatch { x: 0, y: 0, score: 1.0 }],
            },
        ];
        assert_eq!(target_match_counts(2, &variants), vec![2, 1]);
    }
}
