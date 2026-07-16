//! Hover tooltips showing a live screen capture around a point or search area.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Vec2};
use image::{Rgba, RgbaImage};
use sqyre_capture::X11Capturer;
use sqyre_domain::{Action, ActionKind, CoordinateRef, Macro, ScalarValue};
use sqyre_executor::{DesktopRect, ScreenCapturer};
use sqyre_persist::{ProgramCatalog, ProgramPoint, ProgramSearchArea};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MIN_CAPTURE_SIZE: i32 = 320;
const CAPTURE_PADDING: i32 = 48;
const TOOLTIP_MAX_DIM: u32 = 640;
const DISPLAY_MAX_W: f32 = 260.0;
const DISPLAY_MAX_H: f32 = 195.0;
const PANEL_MAX_W: f32 = 340.0;
const PANEL_MAX_H: f32 = 240.0;
const CACHE_MAX: usize = 24;
const CACHE_TTL: Duration = Duration::from_secs(30);
const OVERLAY: Rgba<u8> = Rgba([255, 0, 0, 255]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Point,
    SearchArea,
}

struct CacheEntry {
    texture: TextureHandle,
    caption: String,
    expires: Instant,
}

/// Lazy capturer + LRU texture cache for coordinate preview tooltips.
#[derive(Default)]
pub struct PreviewTooltipCache {
    capturer: Option<X11Capturer>,
    capturer_failed: bool,
    entries: HashMap<String, CacheEntry>,
    order: Vec<String>,
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
        self.entries.retain(|k, _| {
            !(k.starts_with(&prefix_pt) || k.starts_with(&prefix_sa))
        });
        self.order
            .retain(|k| !(k.starts_with(&prefix_pt) || k.starts_with(&prefix_sa)));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
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
        let Some((key, caption, coords)) = entity_preview_spec(catalog, program, name, kind) else {
            response.clone().on_hover_text(format!("{program}~{name}"));
            return;
        };
        let preview = self.texture_for(ui.ctx(), &key, &caption, coords, false);
        response.clone().on_hover_ui(|ui| match &preview {
            Ok((tex, cap)) => paint_preview(ui, tex, cap, false),
            Err(err) => {
                ui.label(caption.as_str());
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
            }
        });
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
                self.texture_for(ui.ctx(), &key, &caption, coords, force)
            }
            Err(err) => Err(err),
        };
        match preview {
            // Tip/edit surface is capped around tip_max_width (~280); use the smaller
            // display fit so preview doesn't force the tooltip wider than view mode.
            Ok((tex, cap)) => paint_preview(ui, &tex, &cap, false),
            Err(err) => {
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
            }
        }
    }

    /// Embedded form-panel preview for a point (uses form field coords).
    /// Returns the image/placeholder rect for cardinal coord overlays.
    pub fn paint_point_panel(
        &mut self,
        ui: &mut egui::Ui,
        x: i32,
        y: i32,
        force: bool,
    ) -> egui::Rect {
        let key = format!("panel:pt:{x}:{y}");
        let caption = format!("X: {x}, Y: {y}");
        let preview = self.texture_for(
            ui.ctx(),
            &key,
            &caption,
            PreviewCoords::Point { x, y },
            force,
        );
        match preview {
            Ok((tex, _)) => paint_preview_panel_image(ui, &tex),
            Err(err) => paint_preview_panel_placeholder(ui, &err),
        }
    }

    /// Embedded form-panel preview for a search area (uses form field coords).
    /// Returns the image/placeholder rect for cardinal coord overlays.
    pub fn paint_search_area_panel(
        &mut self,
        ui: &mut egui::Ui,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        force: bool,
    ) -> egui::Rect {
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
        );
        match preview {
            Ok((tex, _)) => paint_preview_panel_image(ui, &tex),
            Err(err) => paint_preview_panel_placeholder(ui, &err),
        }
    }

    fn texture_for(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        caption: &str,
        coords: PreviewCoords,
        force: bool,
    ) -> Result<(TextureHandle, String), String> {
        if force {
            self.entries.remove(key);
            self.order.retain(|k| k != key);
        }
        let now = Instant::now();
        if let Some(entry) = self.entries.get(key) {
            if now < entry.expires {
                let tex = entry.texture.clone();
                let caption = entry.caption.clone();
                self.touch(key);
                return Ok((tex, caption));
            }
        }
        self.entries.remove(key);
        self.order.retain(|k| k != key);

        let img = self.capture(coords)?;
        let size = [img.width() as usize, img.height() as usize];
        let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        let tex = ctx.load_texture(key.to_string(), color, TextureOptions::LINEAR);
        self.insert(
            key.to_string(),
            CacheEntry {
                texture: tex.clone(),
                caption: caption.to_string(),
                expires: now + CACHE_TTL,
            },
        );
        Ok((tex, caption.to_string()))
    }

    fn capture(&mut self, coords: PreviewCoords) -> Result<RgbaImage, String> {
        if self.capturer_failed && self.capturer.is_none() {
            return Err("screen capture unavailable".into());
        }
        if self.capturer.is_none() {
            match X11Capturer::open() {
                Ok(c) => self.capturer = Some(c),
                Err(e) => {
                    self.capturer_failed = true;
                    return Err(e);
                }
            }
        }
        let capturer = self.capturer.as_mut().unwrap();
        let vb = capturer.virtual_bounds()?;
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
                let mut img = capturer.capture_rect(bounds)?;
                draw_point_marker(
                    &mut img,
                    x - bounds.x,
                    y - bounds.y,
                    OVERLAY,
                    2,
                );
                Ok(downscale_max_dim(img, TOOLTIP_MAX_DIM))
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
                let mut img = capturer.capture_rect(bounds)?;
                draw_rect_outline(
                    &mut img,
                    lx - bounds.x,
                    ty - bounds.y,
                    rx - bounds.x,
                    by - bounds.y,
                    OVERLAY,
                    2,
                );
                Ok(downscale_max_dim(img, TOOLTIP_MAX_DIM))
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

/// Coordinate ref + preview kind for actions that show point/search-area captures.
pub fn coordinate_ref_for_preview(action: &Action) -> Option<(CoordinateRef, PreviewKind)> {
    match &action.kind {
        ActionKind::Move { point, .. } if !point.is_empty() => {
            Some((point.clone(), PreviewKind::Point))
        }
        ActionKind::ImageSearch { search_area, .. }
        | ActionKind::Ocr { search_area, .. }
        | ActionKind::FindPixel { search_area, .. } if !search_area.is_empty() => {
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

fn entity_preview_spec(
    catalog: &ProgramCatalog,
    program: &str,
    name: &str,
    kind: PreviewKind,
) -> Option<(String, String, PreviewCoords)> {
    let pdata = catalog.get(program)?;
    let res = catalog.resolution_key();
    match kind {
        PreviewKind::Point => {
            let pt = pdata
                .points
                .get(res)
                .or_else(|| pdata.points.values().next())?
                .get(name)?;
            Some((
                cache_key_point(pt),
                point_caption(pt),
                PreviewCoords::Point {
                    x: coord_to_int(&pt.x),
                    y: coord_to_int(&pt.y),
                },
            ))
        }
        PreviewKind::SearchArea => {
            let sa = pdata
                .search_areas
                .get(res)
                .or_else(|| pdata.search_areas.values().next())?
                .get(name)?;
            Some((
                cache_key_search_area(sa),
                search_area_caption(sa),
                PreviewCoords::SearchArea {
                    left: coord_to_int(&sa.left_x),
                    top: coord_to_int(&sa.top_y),
                    right: coord_to_int(&sa.right_x),
                    bottom: coord_to_int(&sa.bottom_y),
                },
            ))
        }
    }
}

fn cache_key_point(pt: &ProgramPoint) -> String {
    format!(
        "pt:{}:{}:{}",
        pt.name,
        pt.x.as_display(),
        pt.y.as_display()
    )
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

fn coord_to_int(v: &ScalarValue) -> i32 {
    match v {
        ScalarValue::Int(i) => *i as i32,
        ScalarValue::Float(f) => *f as i32,
        ScalarValue::Bool(b) => {
            if *b {
                1
            } else {
                0
            }
        }
        ScalarValue::String(s) => s.trim().parse().unwrap_or(0),
        ScalarValue::Null => 0,
    }
}

fn paint_preview(ui: &mut egui::Ui, tex: &TextureHandle, caption: &str, panel: bool) {
    let [tw, th] = tex.size();
    let size = if panel {
        fit_display_panel(tw as f32, th as f32)
    } else {
        fit_display(tw as f32, th as f32)
    };
    ui.add(egui::Image::new((tex.id(), size)));
    ui.label(caption);
}

fn paint_preview_panel_image(ui: &mut egui::Ui, tex: &TextureHandle) -> egui::Rect {
    let [tw, th] = tex.size();
    let size = fit_display_panel(tw as f32, th as f32);
    ui.add(egui::Image::new((tex.id(), size))).rect
}

fn paint_preview_panel_placeholder(ui: &mut egui::Ui, err: &str) -> egui::Rect {
    let size = Vec2::new(PANEL_MAX_W, PANEL_MAX_H * 0.65);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 4.0, egui::Color32::from_gray(28));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        err,
        egui::FontId::proportional(13.0),
        egui::Color32::from_rgb(220, 80, 80),
    );
    rect
}

fn fit_display(w: f32, h: f32) -> Vec2 {
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (DISPLAY_MAX_W / w).min(DISPLAY_MAX_H / h).min(1.0);
    Vec2::new(w * scale, h * scale)
}

fn fit_display_panel(w: f32, h: f32) -> Vec2 {
    let w = w.max(1.0);
    let h = h.max(1.0);
    let scale = (PANEL_MAX_W / w).min(PANEL_MAX_H / h).min(1.0);
    Vec2::new(w * scale, h * scale)
}

fn normalize_rect(mut lx: i32, mut ty: i32, mut rx: i32, mut by: i32) -> (i32, i32, i32, i32) {
    if lx > rx {
        std::mem::swap(&mut lx, &mut rx);
    }
    if ty > by {
        std::mem::swap(&mut ty, &mut by);
    }
    (lx, ty, rx, by)
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
    DesktopRect {
        x: x0,
        y: y0,
        w,
        h,
    }
}

fn downscale_max_dim(img: RgbaImage, max_dim: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w <= max_dim && h <= max_dim {
        return img;
    }
    let longest = w.max(h).max(1);
    let nw = ((w as u64 * max_dim as u64) / longest as u64).max(1) as u32;
    let nh = ((h as u64 * max_dim as u64) / longest as u64).max(1) as u32;
    image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
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
        use sqyre_domain::{ActionId, ActionKind, CoordinateRef};

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
                wait: Default::default(),
                coords: Default::default(),
                run_branch_on_no_find: false,
                order: Default::default(),
                subactions: vec![],
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
}
