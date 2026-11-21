//! Native-runtime MatchCheck tool state and UI wiring.

mod heatmap;
use heatmap::*;
pub(crate) use heatmap::{
    closeness_percent, heatmap_hover_allowed, hover_at_image, scores_to_heatmap_rgba,
};

use crate::data_editor::EditorTab;
use crate::data_editor_preview::variant_display_label;
use crate::icon_variants::variant_path;
use crate::image_view::{self, ImageViewTransform};
use crate::preview_tooltip::PreviewTooltipCache;
use crate::theme::{match_fail_fg, match_pass_fg, match_within_fg};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use image::RgbaImage;
use sqyre_domain::MatchMethod;
use sqyre_match::ImageBuf;
use sqyre_match::{
    blur_image_owned, find_peaks_for_method, match_template_with_prepared, prepare_search,
    prepare_template, search_blur_kernel, MatchMap,
};
use sqyre_persist::{CatalogView, ProgramCatalog};
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
    pub plane_flip: sqyre_domain::PlaneFlip,
    pub variant: String,
    pub refresh_gen: u64,
    pub last_inputs: String,
    pub last_request: Option<Instant>,
    /// When true, live recapture/rematch and debounced retries are suppressed until ↻ or unpause.
    pub paused: bool,
    /// Opt-in paint for match boxes when count exceeds [`MANY_MATCH_BOX_THRESHOLD`].
    pub show_many_match_boxes: bool,
    /// When the last live cycle was armed (gates [`LIVE_INTERVAL`]).
    pub last_live_at: Option<Instant>,
    /// One-shot soft panel recapture (keeps the last frame visible).
    pub capture_once: bool,
    /// Wait for the soft panel capture to finish before rematching.
    pub awaiting_panel: bool,
    /// Keep retrying a forced rematch until a worker is spawned.
    pub rematch_once: bool,
}

impl Default for MatchCheckSettings {
    fn default() -> Self {
        Self {
            tolerance: 0.95,
            blur: 0,
            match_method: MatchMethod::CcoeffNormed,
            plane_flip: sqyre_domain::PlaneFlip::None,
            variant: String::new(),
            refresh_gen: 0,
            last_inputs: String::new(),
            last_request: None,
            paused: false,
            show_many_match_boxes: false,
            last_live_at: None,
            capture_once: false,
            awaiting_panel: false,
            rematch_once: false,
        }
    }
}

/// MatchCheck tool state: settings, background match job, uploaded heatmap.
#[derive(Default)]
pub(crate) struct MatchCheckTool {
    pub settings: MatchCheckSettings,
    pub pending: Option<mpsc::Receiver<Result<MatchCheckResult, String>>>,
    pub cache: Option<MatchCheckCache>,
}

impl MatchCheckTool {
    /// Drop the current result so the next frame rematches.
    pub(crate) fn invalidate(&mut self) {
        self.cache = None;
        self.pending = None;
        self.settings.show_many_match_boxes = false;
    }

    /// Stop live matching and pause until the user resumes.
    pub(crate) fn stop_compute(&mut self) {
        self.pending = None;
        self.cache = None;
        self.settings.last_inputs.clear();
        self.settings.paused = true;
        self.settings.capture_once = false;
        self.settings.awaiting_panel = false;
        self.settings.rematch_once = false;
    }
}

/// Whether a live recapture+rematch should start now.
pub(crate) fn live_refresh_due(settings: &MatchCheckSettings, pending: bool, now: Instant) -> bool {
    if settings.paused
        || pending
        || settings.rematch_once
        || settings.capture_once
        || settings.awaiting_panel
    {
        return false;
    }
    settings
        .last_live_at
        .is_none_or(|t| now.duration_since(t) >= LIVE_INTERVAL)
}

/// Arm a single live cycle: rate-limit, soft-recapture, rematch after the new frame.
pub(crate) fn arm_live_cycle(settings: &mut MatchCheckSettings, now: Instant) {
    settings.last_live_at = Some(now);
    settings.capture_once = true;
    settings.awaiting_panel = true;
    settings.rematch_once = false;
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
    plane_flip: sqyre_domain::PlaneFlip,
    tolerance: f64,
    refresh_gen: u64,
    close_matches_distance: i32,
) -> String {
    format!(
        "{prog}\0{item}\0{variant}\0{lx},{ty},{rx},{by}\0{blur}\0{method:?}\0{plane_flip:?}\0{tolerance}\0{refresh_gen}\0{close_matches_distance}"
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
    plane_flip: sqyre_domain::PlaneFlip,
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
    let template_full = get_cached_blurred_template(template_path, kernel, plane_flip)
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
    let need_scale = (scale_x - 1.0).abs() > f32::EPSILON || (scale_y - 1.0).abs() > f32::EPSILON;
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
        let full = get_cached_image_mask(p, template_full.height, template_full.width, plane_flip)?;
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

/// Soft disk radius (image pixels) around each match origin in the heatmap.

pub(crate) fn spawn_compute(
    fingerprint: String,
    display: Arc<RgbaImage>,
    src_w: u32,
    src_h: u32,
    template_path: PathBuf,
    mask_path: Option<PathBuf>,
    blur: i32,
    method: MatchMethod,
    plane_flip: sqyre_domain::PlaneFlip,
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
                plane_flip,
            )?;
            let (heatmap_rgba, summary) =
                scores_to_heatmap_rgba(&map, image_w, image_h, tmpl_w, tmpl_h, method, tolerance);
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

fn image_to_content(pos: egui::Pos2, content: egui::Rect, image_size: egui::Vec2) -> egui::Pos2 {
    let scale_x = content.width() / image_size.x.max(1.0);
    let scale_y = content.height() / image_size.y.max(1.0);
    egui::pos2(
        content.min.x + pos.x * scale_x,
        content.min.y + pos.y * scale_y,
    )
}

/// Caption for the best-peak box when nothing clears tolerance.
pub(crate) fn no_match_box_caption(closeness_pct: f32) -> String {
    format!("{closeness_pct:.1}% · no match")
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

pub(super) fn paint_match_box_caption(
    painter: &egui::Painter,
    content: egui::Rect,
    box_rect: egui::Rect,
    text: &str,
    color: Color32,
) {
    let font = egui::FontId::proportional(12.0);
    let galley = painter.layout_no_wrap(text.to_owned(), font, color);
    let pad = egui::vec2(5.0, 2.0);
    let size = galley.size() + pad * 2.0;
    // Prefer above the box; fall inside the top edge if that would clip.
    let mut origin = egui::pos2(
        box_rect.center().x - size.x * 0.5,
        box_rect.min.y - size.y - 4.0,
    );
    if origin.y < content.min.y {
        origin.y = box_rect.min.y + 2.0;
    }
    origin.x = origin
        .x
        .clamp(content.min.x, (content.max.x - size.x).max(content.min.x));
    let badge = egui::Rect::from_min_size(origin, size);
    painter.rect_filled(badge, 3.0, crate::theme::preview_scrim());
    painter.galley(badge.min + pad, galley, color);
}

pub(crate) fn paint_tolerance_match_boxes(
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
        let pct = closeness_percent(summary.best_score, method, summary);
        paint_match_box_caption(painter, content, rect, &no_match_box_caption(pct), color);
        return;
    }
    for m in tolerance_matches {
        if !score_passes(m.score, method, tolerance) {
            continue;
        }
        let is_best = m.x == summary.best_x && m.y == summary.best_y;
        let top_left = image_to_content(egui::pos2(m.x as f32, m.y as f32), content, image_size);
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
            let pct = closeness_percent(m.score, method, summary);
            paint_match_box_caption(painter, content, rect, &format!("{pct:.1}%"), color);
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
    // Read the pointer without `ui.interact` — a second interactable over the
    // panel's click_and_drag rect steals the drag and interrupts pan/scroll.
    let pointer_down = ui.input(|i| i.pointer.any_down());
    let hover_pos = heatmap_hover_allowed(pointer_down, view.pan_drag.is_some())
        .then(|| ui.input(|i| i.pointer.hover_pos()))
        .flatten()
        .filter(|p| content.contains(*p));
    if let Some(pos) = hover_pos {
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
    if let (Some(h), Some(pointer)) = (hover.as_ref(), hover_pos) {
        let tol_pct = if method.is_normed() {
            Some(tolerance * 100.0)
        } else {
            None
        };
        paint_mouse_follow_tooltip(ui.ctx(), pointer, viewport, h, tol_pct);
        ui.ctx().request_repaint();
    }
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
            } else {
                ui.weak("0 matches");
                if summary.peaks_above > 0 {
                    ui.weak(format!("{} ≥ tolerance (cells)", summary.peaks_above));
                }
            }
        }
        if tolerance_match_count > MANY_MATCH_BOX_THRESHOLD {
            let hiding = !should_paint_match_boxes(tolerance_match_count, *show_many_match_boxes);
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
    plane_flip: sqyre_domain::PlaneFlip,
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
        plane_flip,
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
            self.match_check.settings.paused = false;
        }
        let (Some(prog), Some(item)) = (
            self.nav.selected_program.as_deref(),
            self.nav.selected_entity.as_deref(),
        ) else {
            self.match_check.stop_compute();
            return;
        };
        if !can_compute_match_check(
            catalog,
            prog,
            item,
            &self.match_check.settings.variant,
            left,
            top,
            right,
            bottom,
        ) {
            self.match_check.stop_compute();
            return;
        }
        let refresh_gen = if force {
            self.match_check.settings.refresh_gen.wrapping_add(1)
        } else {
            self.match_check.settings.refresh_gen
        };
        let inputs = inputs_key(
            prog,
            item,
            &self.match_check.settings.variant,
            left,
            top,
            right,
            bottom,
            self.match_check.settings.blur,
            self.match_check.settings.match_method,
            self.match_check.settings.plane_flip,
            self.match_check.settings.tolerance,
            refresh_gen,
            close_matches_distance,
        );
        if inputs.is_empty() {
            self.match_check.stop_compute();
            return;
        }
        if inputs != self.match_check.settings.last_inputs {
            self.match_check.settings.paused = false;
        }
        if self.match_check.settings.paused {
            return;
        }
        if !force
            && self
                .match_check
                .cache
                .as_ref()
                .is_some_and(|c| c.data.fingerprint == inputs)
        {
            return;
        }
        let now = Instant::now();
        if !force
            && !should_request(
                &self.match_check.settings,
                &inputs,
                self.match_check.pending.is_some(),
                now,
            )
        {
            return;
        }
        let Ok(template_path) =
            resolve_template(catalog, prog, item, &self.match_check.settings.variant)
        else {
            self.match_check.stop_compute();
            return;
        };
        if !template_path.is_file() {
            self.match_check.stop_compute();
            return;
        }
        let Some((display, src_w, src_h)) = display else {
            // Panel capture not ready yet — retry next frame once underlay lands.
            return;
        };
        // Commit the forced fingerprint only once capture is ready to spawn.
        if force {
            self.match_check.settings.refresh_gen = refresh_gen;
        }
        let mask_path =
            catalog.mask_path(&format!("{prog}{}{item}", sqyre_domain::PROGRAM_DELIMITER));
        let tolerance = self.match_check.settings.tolerance as f32;
        let job_rx = spawn_compute(
            inputs.clone(),
            display,
            src_w,
            src_h,
            template_path,
            mask_path,
            self.match_check.settings.blur,
            self.match_check.settings.match_method,
            self.match_check.settings.plane_flip,
            tolerance,
            close_matches_distance,
        );
        self.match_check.settings.last_inputs = inputs;
        self.match_check.settings.last_request = Some(now);
        self.match_check.pending = Some(job_rx);
    }

    pub(crate) fn poll_match_check(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        previews: &mut PreviewTooltipCache,
    ) {
        if !matches!(self.nav.tab, EditorTab::MatchCheck) {
            if self.match_check.pending.is_some() {
                self.match_check.stop_compute();
            }
            return;
        }
        use crate::data_editor::helpers::form_desktop_area;
        let (lx, ty, rx, by) = form_desktop_area(
            CatalogView::from(catalog),
            self.drafts.monitor,
            &self.drafts.rect.left,
            &self.drafts.rect.top,
            &self.drafts.rect.right,
            &self.drafts.rect.bottom,
        );
        let coords_ok = match (
            self.nav.selected_program.as_deref(),
            self.nav.selected_entity.as_deref(),
        ) {
            (Some(prog), Some(item)) => can_compute_match_check(
                CatalogView::from(catalog),
                prog,
                item,
                &self.match_check.settings.variant,
                lx,
                ty,
                rx,
                by,
            ),
            _ => false,
        };
        if !coords_ok && self.match_check.pending.is_some() {
            self.match_check.stop_compute();
        }
        // Applying a rematch mid-drag swaps the heatmap/legend and can end the
        // user's gesture early across the whole window.
        let hold_result = self.previews.coord_panning() || ctx.input(|i| i.pointer.any_down());
        if let Some(rx) = self.match_check.pending.as_ref() {
            if hold_result {
                ctx.request_repaint();
            } else {
                match rx.try_recv() {
                    Ok(Ok(result)) => {
                        self.match_check.pending = None;
                        if result.data.fingerprint == self.match_check.settings.last_inputs {
                            if result.data.tolerance_matches.len() > MANY_MATCH_BOX_THRESHOLD {
                                self.match_check.settings.show_many_match_boxes = false;
                            }
                            self.match_check.cache = Some(finish_cache(ctx, result));
                            self.match_check.settings.last_live_at = Some(web_time::Instant::now());
                            if !self.match_check.settings.paused {
                                ctx.request_repaint_after(LIVE_INTERVAL);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        self.match_check.pending = None;
                        self.match_check.settings.paused = true;
                        self.set_err(format!(
                            "{}: {}",
                            EditorTab::MatchCheck.label(),
                            crate::status_banner::user_error_detail(&e)
                        ));
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        ctx.request_repaint();
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.match_check.pending = None;
                        self.match_check.settings.paused = true;
                        self.set_err(format!("{}: match failed", EditorTab::MatchCheck.label()));
                    }
                }
            }
        }
        let _ = previews;
        let _ = catalog;
    }
}
