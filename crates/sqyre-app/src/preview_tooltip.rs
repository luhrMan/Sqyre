//! Hover tooltips showing a live screen capture around a point or search area.

use crate::image_view::{self, ImageViewTransform};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Vec2};
use image::{Rgba, RgbaImage};
use sqyre_capture::{shared_capturer, OsCapturer};
use sqyre_domain::{Action, ActionKind, CoordinateRef, Macro, ScalarValue};
use sqyre_executor::DesktopRect;
use sqyre_persist::{ProgramCatalog, ProgramPoint, ProgramSearchArea};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;
use web_time::{Duration, Instant};

const MIN_CAPTURE_SIZE: i32 = 320;
const CAPTURE_PADDING: i32 = 48;
/// Hover/action tooltip previews — small on screen, keep textures light.
const TOOLTIP_MAX_DIM: u32 = 640;
/// Data-editor panel previews stretch with the window and support zoom.
const PANEL_MAX_DIM: u32 = 1600;
const DISPLAY_MAX_W: f32 = 260.0;
const DISPLAY_MAX_H: f32 = 195.0;
const PANEL_MIN_W: f32 = 160.0;
const PANEL_MIN_H: f32 = 120.0;
/// Slack so filling remaining height does not trip ScrollArea overflow (rounding / bar hysteresis).
const PANEL_FILL_SLACK: f32 = 1.0;
const CACHE_MAX: usize = 24;
/// How long to remember a failed capture before trying again (manual ↻ clears sooner).
const FAIL_CACHE_TTL: Duration = Duration::from_secs(60);
const OVERLAY: Rgba<u8> = Rgba([255, 0, 0, 255]);
const LITERAL_COORDS_MSG: &str = "Preview needs literal coordinates";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Point,
    SearchArea,
}

struct CacheEntry {
    texture: TextureHandle,
    caption: String,
}

struct FailureEntry {
    error: String,
    expires: Instant,
}

struct PendingCapture {
    caption: String,
    rx: Receiver<Result<RgbaImage, String>>,
}

/// Lazy capturer + LRU texture cache for coordinate preview tooltips.
#[derive(Default)]
pub struct PreviewTooltipCache {
    capturer: Option<Arc<OsCapturer>>,
    capturer_failed: bool,
    entries: HashMap<String, CacheEntry>,
    order: Vec<String>,
    /// In-flight captures keyed like cache entries; polled on the UI frame.
    pending: HashMap<String, PendingCapture>,
    /// Failed captures — avoids respawning on every repaint for permanent errors.
    failures: HashMap<String, FailureEntry>,
}

impl PreviewTooltipCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn invalidate_entity(&mut self, name: &str) {
        if name.is_empty() {
            self.clear();
            return;
        }
        let prefix_pt = format!("pt:{name}:");
        let prefix_sa = format!("sa:{name}:");
        let drop = |k: &str| k.starts_with(&prefix_pt) || k.starts_with(&prefix_sa);
        self.entries.retain(|k, _| !drop(k));
        self.order.retain(|k| !drop(k));
        self.pending.retain(|k, _| !drop(k));
        self.failures.retain(|k, _| !drop(k));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.pending.clear();
        self.failures.clear();
    }

    /// Paint an egui hover tooltip for a program entity list row.
    pub fn show_for_entity(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        catalog: &ProgramCatalog,
        program: &str,
        name: &str,
        kind: PreviewKind,
    ) {
        if !response.hovered() {
            return;
        }
        match entity_preview_spec(catalog, program, name, kind) {
            Ok((key, caption, coords)) => {
                let preview =
                    self.texture_for(ui.ctx(), &key, &caption, coords, false, TOOLTIP_MAX_DIM);
                response.clone().on_hover_ui(|ui| match &preview {
                    Ok((tex, cap)) => paint_preview(ui, tex, cap),
                    Err(err) => {
                        ui.label(caption.as_str());
                        ui.colored_label(crate::theme::error_fg(), err);
                    }
                });
            }
            Err(EntityPreviewError::NonLiteral) => {
                response.clone().on_hover_ui(|ui| {
                    ui.label(format!("{program}~{name}"));
                    ui.colored_label(crate::theme::error_fg(), LITERAL_COORDS_MSG);
                });
            }
            Err(EntityPreviewError::Missing) => {
                response.clone().on_hover_text(format!("{program}~{name}"));
            }
        }
    }

    /// Screen capture preview for a macro coordinate ref (action tooltips).
    pub fn paint_for_coordinate_ref(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        coord_ref: &CoordinateRef,
        kind: PreviewKind,
        force: bool,
    ) {
        let preview = match ref_preview_spec(catalog, coord_ref, kind) {
            Ok((key, caption, coords)) => {
                self.texture_for(ui.ctx(), &key, &caption, coords, force, TOOLTIP_MAX_DIM)
            }
            Err(err) => Err(err),
        };
        match preview {
            // Tip/edit surface is capped around tip_max_width (~280); use the smaller
            // display fit so preview doesn't force the tooltip wider than view mode.
            Ok((tex, cap)) => paint_preview(ui, &tex, &cap),
            Err(err) => {
                ui.colored_label(crate::theme::error_fg(), err);
            }
        }
    }

    /// Embedded form-panel preview for a point (uses form field coords).
    /// Returns the viewport rect for cardinal coord overlays.
    /// Pass `None` for a coordinate that is a variable or non-literal expression.
    pub fn paint_point_panel(
        &mut self,
        ui: &mut egui::Ui,
        x: Option<i32>,
        y: Option<i32>,
        force: bool,
        view: &mut ImageViewTransform,
    ) -> egui::Rect {
        let (Some(x), Some(y)) = (x, y) else {
            return paint_preview_panel_placeholder(ui, LITERAL_COORDS_MSG, view);
        };
        let key = format!("panel:pt:{x}:{y}");
        let caption = format!("X: {x}, Y: {y}");
        let preview = self.texture_for(
            ui.ctx(),
            &key,
            &caption,
            PreviewCoords::Point { x, y },
            force,
            PANEL_MAX_DIM,
        );
        match preview {
            Ok((tex, _)) => paint_preview_panel_image(ui, &tex, view),
            Err(err) => paint_preview_panel_placeholder(ui, &err, view),
        }
    }

    /// Embedded form-panel preview for a search area (uses form field coords).
    /// Returns the viewport rect for cardinal coord overlays.
    /// Pass `None` for a coordinate that is a variable or non-literal expression.
    pub fn paint_search_area_panel(
        &mut self,
        ui: &mut egui::Ui,
        left: Option<i32>,
        top: Option<i32>,
        right: Option<i32>,
        bottom: Option<i32>,
        force: bool,
        view: &mut ImageViewTransform,
    ) -> egui::Rect {
        let (Some(left), Some(top), Some(right), Some(bottom)) = (left, top, right, bottom) else {
            return paint_preview_panel_placeholder(ui, LITERAL_COORDS_MSG, view);
        };
        let key = format!("panel:sa:{left}:{top}:{right}:{bottom}");
        let caption = format!("Left: {left}, Top: {top}, Right: {right}, Bottom: {bottom}");
        let preview = self.texture_for(
            ui.ctx(),
            &key,
            &caption,
            PreviewCoords::SearchArea {
                left,
                top,
                right,
                bottom,
            },
            force,
            PANEL_MAX_DIM,
        );
        match preview {
            Ok((tex, _)) => paint_preview_panel_image(ui, &tex, view),
            Err(err) => paint_preview_panel_placeholder(ui, &err, view),
        }
    }

    fn texture_for(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        caption: &str,
        coords: PreviewCoords,
        force: bool,
        max_dim: u32,
    ) -> Result<(TextureHandle, String), String> {
        if force {
            self.entries.remove(key);
            self.order.retain(|k| k != key);
            self.pending.remove(key);
            self.failures.remove(key);
        }
        if let Some(entry) = self.entries.get(key) {
            let tex = entry.texture.clone();
            let caption = entry.caption.clone();
            self.touch(key);
            return Ok((tex, caption));
        }

        let now = Instant::now();
        if let Some(fail) = self.failures.get(key) {
            if now < fail.expires {
                return Err(fail.error.clone());
            }
            self.failures.remove(key);
        }

        if self.pending.contains_key(key) {
            let recv = self.pending[key].rx.try_recv();
            match recv {
                Ok(Ok(img)) => {
                    let caption = self
                        .pending
                        .remove(key)
                        .map(|p| p.caption)
                        .unwrap_or_else(|| caption.to_string());
                    self.failures.remove(key);
                    return self.finish_texture(ctx, key, &caption, img, max_dim);
                }
                Ok(Err(e)) => {
                    self.pending.remove(key);
                    self.remember_failure(key, e.clone(), now);
                    return Err(e);
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint();
                    return Err("Capturing…".into());
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending.remove(key);
                    let e = "capture failed".to_string();
                    self.remember_failure(key, e.clone(), now);
                    return Err(e);
                }
            }
        }

        self.ensure_capturer()?;
        let capturer = Arc::clone(self.capturer.as_ref().unwrap());
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(capture_preview(capturer.as_ref(), coords, max_dim));
        });
        self.pending.insert(
            key.to_string(),
            PendingCapture {
                caption: caption.to_string(),
                rx,
            },
        );
        ctx.request_repaint();
        Err("Capturing…".into())
    }

    fn remember_failure(&mut self, key: &str, error: String, now: Instant) {
        self.failures.insert(
            key.to_string(),
            FailureEntry {
                error,
                expires: now + FAIL_CACHE_TTL,
            },
        );
    }

    fn finish_texture(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        caption: &str,
        img: RgbaImage,
        max_dim: u32,
    ) -> Result<(TextureHandle, String), String> {
        let size = [img.width() as usize, img.height() as usize];
        let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        // Mipmaps help panel zoom; tooltips stay cheap without them.
        let opts = if max_dim >= PANEL_MAX_DIM {
            TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear))
        } else {
            TextureOptions::LINEAR
        };
        let tex = ctx.load_texture(key.to_string(), color, opts);
        self.insert(
            key.to_string(),
            CacheEntry {
                texture: tex.clone(),
                caption: caption.to_string(),
            },
        );
        Ok((tex, caption.to_string()))
    }

    fn ensure_capturer(&mut self) -> Result<(), String> {
        if self.capturer_failed && self.capturer.is_none() {
            return Err("screen capture unavailable".into());
        }
        if self.capturer.is_some() {
            return Ok(());
        }
        match shared_capturer() {
            Ok(c) => {
                self.capturer = Some(c);
                Ok(())
            }
            Err(e) => {
                self.capturer_failed = true;
                Err(e)
            }
        }
    }

    fn insert(&mut self, key: String, entry: CacheEntry) {
        if !self.entries.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.entries.insert(key.clone(), entry);
        self.touch(&key);
        while self.order.len() > CACHE_MAX {
            let evict = self.order.remove(0);
            self.entries.remove(&evict);
        }
    }

    fn touch(&mut self, key: &str) {
        if let Some(i) = self.order.iter().position(|k| k == key) {
            let k = self.order.remove(i);
            self.order.push(k);
        }
    }
}

#[derive(Clone, Copy)]
enum PreviewCoords {
    Point {
        x: i32,
        y: i32,
    },
    SearchArea {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    },
}

fn capture_preview(
    capturer: &OsCapturer,
    coords: PreviewCoords,
    max_dim: u32,
) -> Result<RgbaImage, String> {
    let vb = capturer.virtual_bounds_ref()?;
    match coords {
        PreviewCoords::Point { x, y } => {
            if x < vb.x || y < vb.y || x > vb.x + vb.w || y > vb.y + vb.h {
                return Err(format!(
                    "point outside desktop ({},{})..({},{}), got ({x},{y})",
                    vb.x,
                    vb.y,
                    vb.x + vb.w,
                    vb.y + vb.h
                ));
            }
            let bounds = preview_bounds_for_point(x, y, vb);
            let mut img = capturer.capture_rect_ref(bounds)?;
            draw_point_marker(&mut img, x - bounds.x, y - bounds.y, OVERLAY, 2);
            Ok(downscale_max_dim(img, max_dim))
        }
        PreviewCoords::SearchArea {
            left,
            top,
            right,
            bottom,
        } => {
            let (lx, ty, rx, by) = normalize_rect(left, top, right, bottom);
            if rx <= lx || by <= ty {
                return Err("empty search area".into());
            }
            let bounds = preview_bounds_for_search_area(lx, ty, rx, by, vb);
            let mut img = capturer.capture_rect_ref(bounds)?;
            draw_rect_outline(
                &mut img,
                lx - bounds.x,
                ty - bounds.y,
                rx - bounds.x,
                by - bounds.y,
                OVERLAY,
                2,
            );
            Ok(downscale_max_dim(img, max_dim))
        }
    }
}

/// Coordinate ref + preview kind for actions that show point/search-area captures.
pub fn coordinate_ref_for_preview(action: &Action) -> Option<(CoordinateRef, PreviewKind)> {
    match &action.kind {
        ActionKind::Move { point, .. } if !point.is_empty() => {
            Some((point.clone(), PreviewKind::Point))
        }
        ActionKind::ImageSearch { search_area, .. }
        | ActionKind::Ocr { search_area, .. }
        | ActionKind::FindPixel { search_area, .. }
            if !search_area.is_empty() =>
        {
            Some((search_area.clone(), PreviewKind::SearchArea))
        }
        _ => None,
    }
}

fn ref_preview_spec(
    catalog: &ProgramCatalog,
    coord_ref: &CoordinateRef,
    kind: PreviewKind,
) -> Result<(String, String, PreviewCoords), String> {
    let macro_ = Macro::new("", 0, vec![]);
    match kind {
        PreviewKind::Point => {
            let (x, y) = catalog.resolve_point(coord_ref, &macro_)?;
            let coords = PreviewCoords::Point { x, y };
            Ok((
                cache_key_ref(coord_ref, coords),
                format!("X: {x}, Y: {y}"),
                coords,
            ))
        }
        PreviewKind::SearchArea => {
            let (left, top, right, bottom) = catalog.resolve_search_area(coord_ref, &macro_)?;
            let coords = PreviewCoords::SearchArea {
                left,
                top,
                right,
                bottom,
            };
            Ok((
                cache_key_ref(coord_ref, coords),
                format!("Left: {left}, Top: {top}, Right: {right}, Bottom: {bottom}"),
                coords,
            ))
        }
    }
}

fn cache_key_ref(coord_ref: &CoordinateRef, coords: PreviewCoords) -> String {
    match coords {
        PreviewCoords::Point { x, y } => format!("ref:pt:{}:{x}:{y}", coord_ref.as_str()),
        PreviewCoords::SearchArea {
            left,
            top,
            right,
            bottom,
        } => format!(
            "ref:sa:{}:{left}:{top}:{right}:{bottom}",
            coord_ref.as_str()
        ),
    }
}

enum EntityPreviewError {
    Missing,
    NonLiteral,
}

fn entity_preview_spec(
    catalog: &ProgramCatalog,
    program: &str,
    name: &str,
    kind: PreviewKind,
) -> Result<(String, String, PreviewCoords), EntityPreviewError> {
    let pdata = catalog.get(program).ok_or(EntityPreviewError::Missing)?;
    let res = catalog.resolution_key();
    match kind {
        PreviewKind::Point => {
            let pt = pdata
                .points
                .get(res)
                .or_else(|| pdata.points.values().next())
                .and_then(|m| m.get(name))
                .ok_or(EntityPreviewError::Missing)?;
            let x = coord_to_literal(&pt.x).ok_or(EntityPreviewError::NonLiteral)?;
            let y = coord_to_literal(&pt.y).ok_or(EntityPreviewError::NonLiteral)?;
            Ok((
                cache_key_point(pt),
                point_caption(pt),
                PreviewCoords::Point { x, y },
            ))
        }
        PreviewKind::SearchArea => {
            let sa = pdata
                .search_areas
                .get(res)
                .or_else(|| pdata.search_areas.values().next())
                .and_then(|m| m.get(name))
                .ok_or(EntityPreviewError::Missing)?;
            let left = coord_to_literal(&sa.left_x).ok_or(EntityPreviewError::NonLiteral)?;
            let top = coord_to_literal(&sa.top_y).ok_or(EntityPreviewError::NonLiteral)?;
            let right = coord_to_literal(&sa.right_x).ok_or(EntityPreviewError::NonLiteral)?;
            let bottom = coord_to_literal(&sa.bottom_y).ok_or(EntityPreviewError::NonLiteral)?;
            Ok((
                cache_key_search_area(sa),
                search_area_caption(sa),
                PreviewCoords::SearchArea {
                    left,
                    top,
                    right,
                    bottom,
                },
            ))
        }
    }
}

fn cache_key_point(pt: &ProgramPoint) -> String {
    format!("pt:{}:{}:{}", pt.name, pt.x.as_display(), pt.y.as_display())
}

fn cache_key_search_area(sa: &ProgramSearchArea) -> String {
    format!(
        "sa:{}:{}:{}:{}:{}",
        sa.name,
        sa.left_x.as_display(),
        sa.top_y.as_display(),
        sa.right_x.as_display(),
        sa.bottom_y.as_display()
    )
}

fn point_caption(pt: &ProgramPoint) -> String {
    format!("X: {}, Y: {}", pt.x.as_display(), pt.y.as_display())
}

fn search_area_caption(sa: &ProgramSearchArea) -> String {
    format!(
        "Left: {}, Top: {}, Right: {}, Bottom: {}",
        sa.left_x.as_display(),
        sa.top_y.as_display(),
        sa.right_x.as_display(),
        sa.bottom_y.as_display()
    )
}

/// Literal numeric coordinate suitable for a live screen preview.
/// Returns `None` for variable refs and other non-numeric expressions.
fn coord_to_literal(v: &ScalarValue) -> Option<i32> {
    match v {
        ScalarValue::Int(i) => Some(*i as i32),
        ScalarValue::Float(f) => Some(*f as i32),
        ScalarValue::Bool(b) => Some(if *b { 1 } else { 0 }),
        ScalarValue::String(s) => {
            let s = s.trim();
            if s.is_empty() || sqyre_varref::contains(s) {
                return None;
            }
            s.parse::<i32>()
                .ok()
                .or_else(|| s.parse::<f64>().ok().map(|f| f as i32))
        }
        ScalarValue::Null => None,
    }
}

fn paint_preview(ui: &mut egui::Ui, tex: &TextureHandle, caption: &str) {
    let [tw, th] = tex.size();
    let size = fit_display(tw as f32, th as f32);
    ui.add(egui::Image::new((tex.id(), size)));
    ui.label(caption);
}

fn panel_viewport_size(ui: &egui::Ui) -> Vec2 {
    let w = ui.available_width().max(PANEL_MIN_W);
    let avail_h = ui.available_height();
    // Fill remaining height; only exceed it (scrollbar) when below the minimum.
    let h = if avail_h < PANEL_MIN_H {
        PANEL_MIN_H
    } else {
        (avail_h - PANEL_FILL_SLACK).max(PANEL_MIN_H)
    };
    Vec2::new(w, h)
}

fn paint_preview_panel_image(
    ui: &mut egui::Ui,
    tex: &TextureHandle,
    view: &mut ImageViewTransform,
) -> egui::Rect {
    let [tw, th] = tex.size();
    let image_size = Vec2::new(tw as f32, th as f32);
    let desired = panel_viewport_size(ui);
    let (viewport, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
    image_view::handle_scroll_zoom(ui, viewport, image_size, view, resp.hovered());
    let content = image_view::image_content_rect(viewport, image_size, view.zoom, view.pan);
    {
        let painter = ui.painter_at(viewport);
        painter.rect_filled(viewport, 0.0, egui::Color32::from_gray(20));
        painter.image(
            tex.id(),
            content,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    let _ = image_view::handle_pan_drag(&resp, viewport, image_size, view);
    viewport
}

fn paint_preview_panel_placeholder(
    ui: &mut egui::Ui,
    err: &str,
    _view: &mut ImageViewTransform,
) -> egui::Rect {
    let desired = panel_viewport_size(ui);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 4.0, egui::Color32::from_gray(28));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        err,
        egui::FontId::proportional(13.0),
        crate::theme::error_fg(),
    );
    rect
}

fn fit_display(w: f32, h: f32) -> Vec2 {
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (DISPLAY_MAX_W / w).min(DISPLAY_MAX_H / h).min(1.0);
    Vec2::new(w * scale, h * scale)
}

fn normalize_rect(lx: i32, ty: i32, rx: i32, by: i32) -> (i32, i32, i32, i32) {
    DesktopRect::normalize_corners(lx, ty, rx, by)
}

fn preview_bounds_for_point(px: i32, py: i32, vb: DesktopRect) -> DesktopRect {
    let half = MIN_CAPTURE_SIZE / 2;
    let desired = DesktopRect {
        x: px - half,
        y: py - half,
        w: MIN_CAPTURE_SIZE,
        h: MIN_CAPTURE_SIZE,
    };
    shift_into_virtual(desired, vb)
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

fn put_pixel_safe(img: &mut RgbaImage, x: i32, y: i32, c: Rgba<u8>) {
    if x < 0 || y < 0 {
        return;
    }
    let (w, h) = img.dimensions();
    if (x as u32) < w && (y as u32) < h {
        img.put_pixel(x as u32, y as u32, c);
    }
}

fn draw_hline(img: &mut RgbaImage, y: i32, x0: i32, x1: i32, c: Rgba<u8>, thick: i32) {
    let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    for t in 0..thick {
        for x in x0..=x1 {
            put_pixel_safe(img, x, y + t, c);
        }
    }
}

fn draw_vline(img: &mut RgbaImage, x: i32, y0: i32, y1: i32, c: Rgba<u8>, thick: i32) {
    let (y0, y1) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    for t in 0..thick {
        for y in y0..=y1 {
            put_pixel_safe(img, x + t, y, c);
        }
    }
}

fn draw_rect_outline(
    img: &mut RgbaImage,
    lx: i32,
    ty: i32,
    rx: i32,
    by: i32,
    c: Rgba<u8>,
    thick: i32,
) {
    let (lx, ty, rx, by) = normalize_rect(lx, ty, rx, by);
    if rx <= lx || by <= ty {
        return;
    }
    // Inclusive max edge for rectangle stroke on image coords.
    let x1 = rx - 1;
    let y1 = by - 1;
    draw_hline(img, ty, lx, x1, c, thick);
    draw_hline(img, y1, lx, x1, c, thick);
    draw_vline(img, lx, ty, y1, c, thick);
    draw_vline(img, x1, ty, y1, c, thick);
}

fn draw_point_marker(img: &mut RgbaImage, cx: i32, cy: i32, c: Rgba<u8>, thick: i32) {
    // Circle radius 8 (approx) + crosshair arms ±15.
    for dy in -8..=8 {
        for dx in -8..=8 {
            if dx * dx + dy * dy >= 49 && dx * dx + dy * dy <= 64 {
                put_pixel_safe(img, cx + dx, cy + dy, c);
            }
        }
    }
    draw_hline(img, cy, cx - 15, cx + 15, c, thick);
    draw_vline(img, cx, cy - 15, cy + 15, c, thick);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_bounds_centered_and_clamped() {
        let vb = DesktopRect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let b = preview_bounds_for_point(100, 100, vb);
        assert_eq!(b.w, MIN_CAPTURE_SIZE);
        assert_eq!(b.h, MIN_CAPTURE_SIZE);
        assert!(b.x >= 0 && b.y >= 0);
        assert!(b.x + b.w <= vb.w);
        assert!(b.y + b.h <= vb.h);
    }

    #[test]
    fn search_area_bounds_include_padding() {
        let vb = DesktopRect {
            x: 0,
            y: 0,
            w: 2560,
            h: 1440,
        };
        let b = preview_bounds_for_search_area(100, 100, 200, 180, vb);
        assert!(b.w >= MIN_CAPTURE_SIZE);
        assert!(b.h >= MIN_CAPTURE_SIZE);
        assert!(b.x <= 100);
        assert!(b.y <= 100);
        assert!(b.x + b.w >= 200);
        assert!(b.y + b.h >= 180);
    }

    #[test]
    fn coordinate_ref_for_preview_matches_action_kinds() {
        use sqyre_domain::{ActionId, ActionKind, CoordinateRef, DetectionBranch};

        assert!(coordinate_ref_for_preview(&Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        })
        .is_none());

        let mv = Action {
            id: ActionId::new(),
            kind: ActionKind::Move {
                point: CoordinateRef("P~Spot".into()),
                smooth: false,
                smooth_low: 0.05,
                smooth_high: 0.2,
                smooth_delay_ms: 1,
            },
        };
        assert_eq!(
            coordinate_ref_for_preview(&mv),
            Some((CoordinateRef("P~Spot".into()), PreviewKind::Point))
        );

        let search = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec![],
                search_area: CoordinateRef("P~Box".into()),
                tolerance: 0.9,
                blur: 0,
                match_method: Default::default(),
                detection: DetectionBranch::default(),
            },
        };
        assert_eq!(
            coordinate_ref_for_preview(&search),
            Some((CoordinateRef("P~Box".into()), PreviewKind::SearchArea))
        );
    }

    #[test]
    fn captions_use_expected_format() {
        let pt = ProgramPoint {
            name: "Spot".into(),
            x: ScalarValue::Int(100),
            y: ScalarValue::Int(200),
        };
        assert_eq!(point_caption(&pt), "X: 100, Y: 200");
        let sa = ProgramSearchArea {
            name: "Box".into(),
            left_x: ScalarValue::Int(10),
            top_y: ScalarValue::Int(20),
            right_x: ScalarValue::Int(110),
            bottom_y: ScalarValue::Int(80),
        };
        assert_eq!(
            search_area_caption(&sa),
            "Left: 10, Top: 20, Right: 110, Bottom: 80"
        );
    }

    #[test]
    fn coord_to_literal_rejects_variable_refs() {
        assert_eq!(coord_to_literal(&ScalarValue::Int(12)), Some(12));
        assert_eq!(
            coord_to_literal(&ScalarValue::String("40".into())),
            Some(40)
        );
        assert_eq!(coord_to_literal(&ScalarValue::String("${x}".into())), None);
        assert_eq!(coord_to_literal(&ScalarValue::Null), None);
    }
}
