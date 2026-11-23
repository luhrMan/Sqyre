//! Hover tooltips showing a live screen capture around a point or search area.

use crate::image_view::{self, ImageViewTransform};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Vec2};
use image::RgbaImage;
use sqyre_capture::{mark_site, shared_capturer_nonblocking, OsCapturer};
use sqyre_domain::{Action, ActionKind, CoordinateRef, Macro, ScalarValue, PROGRAM_DELIMITER};
use sqyre_persist::{ProgramCatalog, ProgramPoint, ProgramSearchArea};
use sqyre_ports::{clamp_search_rect, CaptureError, DesktopRect};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;
#[cfg(not(target_os = "windows"))]
use std::thread;
use web_time::{Duration, Instant};

const MIN_CAPTURE_SIZE: i32 = 320;
const CAPTURE_PADDING: i32 = 48;
/// Hover/action tooltip previews — small on screen, keep textures light.
const TOOLTIP_MAX_DIM: u32 = 640;
/// Data-editor panel previews stretch with the window and support zoom.
const PANEL_MAX_DIM: u32 = 1600;
pub(crate) const DISPLAY_MAX_W: f32 = 260.0;
const DISPLAY_MAX_H: f32 = 195.0;
const PANEL_MIN_W: f32 = 160.0;
const PANEL_MIN_H: f32 = 120.0;
/// Slack so filling remaining height does not trip ScrollArea overflow (rounding / bar hysteresis).
const PANEL_FILL_SLACK: f32 = 1.0;
const CACHE_MAX: usize = 24;
/// How long to remember a failed capture before trying again (manual ↻ clears sooner).
const FAIL_CACHE_TTL: Duration = Duration::from_secs(60);
/// Transient `NotReady` errors (stream restarting, first frame pending) retry
/// after this much shorter window — the shared capturer may already be back.
const RETRY_FAIL_TTL: Duration = Duration::from_millis(500);
const LITERAL_COORDS_MSG: &str = "Preview needs literal coordinates";
/// Point-marker ring radius in capture pixels (the raster ring sat near r = 7–8).
const POINT_RING_RADIUS: f32 = 7.5;
/// Point-marker crosshair half-length in capture pixels.
const POINT_ARM: f32 = 15.0;
/// Crosshair thickness in capture pixels.
const POINT_STROKE_PX: f32 = 2.0;
/// Search-area outline and grid thickness in capture pixels.
const AREA_STROKE_PX: f32 = 1.0;
/// Screen-pixel floor so a fitted preview still shows a blinking mark.
const MARKER_MIN_STROKE: f32 = 1.5;
/// In-flight preview capture. A slept PipeWire stream shows a spinner instead.
const CAPTURE_PENDING: &str = "Capturing…";
/// Half-size of the desktop outline box drawn around a point (px).
const POINT_OUTLINE_HALF: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Point,
    SearchArea,
    Collection,
}

struct CacheEntry {
    texture: TextureHandle,
    caption: String,
    /// Absolute desktop region this texture was captured for (manual ↻ clears matches).
    coords: PreviewCoords,
    /// Location mark in capture pixels. Painted each frame so it can blink.
    marker: PreviewMarker,
    /// Exclusive search-area crop (panel previews only); ScreenCap Save source.
    region: Option<Arc<RgbaImage>>,
}

struct ReadyPreview {
    texture: TextureHandle,
    caption: String,
    marker: PreviewMarker,
}

struct FailureEntry {
    error: String,
    /// [`CaptureError::NoSavedScreen`]: the preview offers an app dropdown instead.
    no_saved_screen: bool,
    expires: Instant,
    coords: PreviewCoords,
}

struct PendingCapture {
    caption: String,
    coords: PreviewCoords,
    /// Panel search-area captures also return the exclusive region crop for ScreenCap.
    rx: Receiver<Result<CapturedPreview, CaptureError>>,
}

struct CapturedPreview {
    display: RgbaImage,
    /// Exclusive `[left,right)×[top,bottom)` pixels before downscale.
    region: Option<RgbaImage>,
    /// Mark in pre-downscale capture pixels. Not baked into `display`.
    marker: PreviewMarker,
}

/// Lazy capturer + LRU texture cache for coordinate preview tooltips.
#[derive(Default)]
pub struct PreviewTooltipCache {
    capturer: Option<Arc<OsCapturer>>,
    /// Docs and kittest never open the real screen capturer.
    capture_disabled: bool,
    entries: HashMap<String, CacheEntry>,
    order: Vec<String>,
    /// In-flight captures keyed like cache entries; polled on the UI frame.
    pending: HashMap<String, PendingCapture>,
    /// Failed captures — avoids respawning on every repaint for permanent errors.
    failures: HashMap<String, FailureEntry>,
    /// Desktop outline from an embedded preview (action tooltip body, edit form).
    desktop_outline: Option<(i32, i32, i32, i32)>,
    /// Desktop outline from a hovered list row / coord label; wins over embedded.
    desktop_outline_hover: Option<(i32, i32, i32, i32)>,
    /// Settings → Appearance toggle: preview tooltips may request the gold
    /// desktop outline. Logs window pin/hover requests bypass this flag.
    desktop_outlines_enabled: bool,
    #[cfg(target_os = "android")]
    backdrop_generation: u64,
    /// App dropdown shown when Android has no saved screen yet.
    #[cfg(target_os = "android")]
    shot: PreviewShot,
}

impl PreviewTooltipCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Never opens the screen capturer (headless docs/kittest: no real-desktop pixels).
    pub fn without_capture() -> Self {
        Self {
            capture_disabled: true,
            ..Self::default()
        }
    }

    pub fn invalidate_entity(&mut self, name: &str) {
        if name.is_empty() {
            self.clear();
            return;
        }
        let prefix_pt = format!("pt:{name}:");
        let prefix_sa = format!("sa:{name}:");
        let prefix_col = format!("col:{name}:");
        let drop = |k: &str| {
            k.starts_with(&prefix_pt) || k.starts_with(&prefix_sa) || k.starts_with(&prefix_col)
        };
        self.entries.retain(|k, _| !drop(k));
        self.order.retain(|k| !drop(k));
        self.pending.retain(|k, _| !drop(k));
        self.failures.retain(|k, _| !drop(k));
    }

    /// Drop every cached / pending / failed preview for the same absolute desktop region.
    ///
    /// Panel, list-hover, and action-tooltip keys for one point or search area must all
    /// refresh together when the user hits ↻ — otherwise a stale tooltip can outlive the
    /// panel capture they just updated.
    fn invalidate_coords(&mut self, coords: PreviewCoords) {
        let related = |c: PreviewCoords| same_capture_region(c, coords);
        self.entries.retain(|_, e| !related(e.coords));
        self.order.retain(|k| self.entries.contains_key(k));
        self.pending.retain(|_, p| !related(p.coords));
        self.failures.retain(|_, f| !related(f.coords));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.pending.clear();
        self.failures.clear();
        self.desktop_outline = None;
        self.desktop_outline_hover = None;
    }

    /// Full-res pixels inside the panel preview outline for these bounds, if cached.
    ///
    /// ScreenCap Save uses this so the PNG matches the screenshot the user is looking at
    /// instead of grabbing the live desktop (which can include Sqyre and disrupt the flow).
    pub fn screen_cap_image(
        &self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Option<Arc<RgbaImage>> {
        let key = format!("panel:sa:{left}:{top}:{right}:{bottom}");
        self.entries
            .get(&key)
            .and_then(|e| e.region.as_ref().map(Arc::clone))
    }

    /// Absolute desktop corners requested by a tooltip preview last frame, if any.
    ///
    /// Consumed by the recording overlay so the gold selection outline is drawn on
    /// the real desktop at the point / search-area location while the tip is open.
    ///
    /// Android has no desktop outline (Sqyre covers the screen), so it always gets `None`.
    pub fn take_desktop_outline(&mut self) -> Option<(i32, i32, i32, i32)> {
        let hover = self.desktop_outline_hover.take();
        let embedded = self.desktop_outline.take();
        #[cfg(target_os = "android")]
        {
            let _ = (hover, embedded);
            None
        }
        #[cfg(not(target_os = "android"))]
        hover.or(embedded)
    }

    /// Android previews crop the backdrop; drop them all when a newer one arrives.
    #[cfg(target_os = "android")]
    fn drop_stale_backdrop_previews(&mut self) {
        let generation = sqyre_android::frames().backdrop_generation();
        if generation != self.backdrop_generation {
            self.backdrop_generation = generation;
            self.entries.clear();
            self.order.clear();
            self.pending.clear();
            self.failures.clear();
        }
    }

    /// Sync from `UserSettings::preview_desktop_outline`; turning it off also
    /// drops a tooltip request queued this frame so nothing stale paints.
    pub fn set_desktop_outlines_enabled(&mut self, enabled: bool) {
        self.desktop_outlines_enabled = enabled;
        if !enabled {
            self.desktop_outline = None;
            self.desktop_outline_hover = None;
        }
    }

    fn request_desktop_outline_embedded(&mut self, coords: PreviewCoords) {
        if self.desktop_outlines_enabled {
            self.desktop_outline = Some(desktop_outline_rect(coords));
        }
    }

    fn request_desktop_outline_hover(&mut self, coords: PreviewCoords) {
        if self.desktop_outlines_enabled {
            self.desktop_outline_hover = Some(desktop_outline_rect(coords));
        }
    }

    /// Whether hovered/pinned rects can be outlined on the real desktop.
    /// (Always true natively except Android; the WASM stub returns false so the
    /// Logs window can show a "requires the desktop app" hint.)
    pub fn desktop_outlines_supported(&self) -> bool {
        !cfg!(target_os = "android")
    }

    /// Pin a desktop outline at an absolute `(l,t,r,b)` rect — Logs window find
    /// markers on click. Re-request each frame while pinned; consumed by
    /// [`Self::take_desktop_outline`].
    pub fn request_outline_abs(&mut self, rect: (i32, i32, i32, i32)) {
        self.desktop_outline = Some(rect);
    }

    /// Hover variant of [`Self::request_outline_abs`]; wins over the pinned
    /// request for the frame, same as list-row hovers.
    pub fn request_outline_abs_hover(&mut self, rect: (i32, i32, i32, i32)) {
        self.desktop_outline_hover = Some(rect);
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
        // Keep preparing while the tip is already open so a large preview that briefly
        // covers the row does not drop capture / outline and restart a flash loop.
        let tip_open = response.is_tooltip_open();
        if !response.hovered() && !tip_open {
            return;
        }
        match entity_preview_spec(catalog, program, name, kind) {
            Ok((key, caption, coords)) => {
                self.request_desktop_outline_hover(coords);
                if self.desktop_outlines_enabled {
                    log_hover_outline_transition(kind, program, name, coords);
                }
                let preview =
                    self.texture_for(ui.ctx(), &key, &caption, coords, false, TOOLTIP_MAX_DIM);
                let show_size = matches!(kind, PreviewKind::SearchArea | PreviewKind::Collection);
                show_preview_hover_ui(response, |ui| match &preview {
                    Ok(ready) => {
                        paint_preview(ui, &ready.texture, &ready.caption, show_size, ready.marker)
                    }
                    Err(err) => paint_preview_status(ui, caption.as_str(), err),
                });
            }
            Err(EntityPreviewError::NonLiteral) => {
                show_preview_hover_ui(response, |ui| {
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
    /// `interactive` is whether the host surface accepts hover: the edit window
    /// does, so errors collapse to an icon + tooltip; the view tip is a
    /// non-interactive [`egui::Area`] where hover never fires, so it keeps text.
    pub fn paint_for_coordinate_ref(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        coord_ref: &CoordinateRef,
        kind: PreviewKind,
        force: bool,
        interactive: bool,
    ) {
        let macro_ = Macro::new("", 0, vec![]);
        let mut choose_app = false;
        let preview = match ref_preview_spec(catalog, &macro_, coord_ref, kind) {
            Ok((key, caption, coords)) => {
                self.request_desktop_outline_embedded(coords);
                let preview =
                    self.texture_for(ui.ctx(), &key, &caption, coords, force, TOOLTIP_MAX_DIM);
                choose_app =
                    interactive && preview.is_err() && self.failure_is_no_saved_screen(&key);
                preview
            }
            Err(err) => Err(err),
        };
        match preview {
            // Tip/edit surface is viewport-relative (see action_tooltip::tip_max_width);
            // keep the smaller display fit so preview doesn't force the tip wider.
            Ok(ready) => paint_preview(
                ui,
                &ready.texture,
                &ready.caption,
                matches!(kind, PreviewKind::SearchArea | PreviewKind::Collection),
                ready.marker,
            ),
            Err(err) if choose_app => self.paint_no_saved_screen_inline(ui, &err),
            Err(err) => {
                if interactive {
                    crate::widgets::error_icon(ui, err.as_str());
                } else {
                    ui.colored_label(crate::theme::error_fg(), err);
                }
            }
        }
    }

    /// Hover tooltip variant of [`Self::paint_for_coordinate_ref`]. `help` is the
    /// labeled field's description, shown under the ref name in the tooltip.
    pub fn show_for_coordinate_ref(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        catalog: &ProgramCatalog,
        coord_ref: &CoordinateRef,
        kind: PreviewKind,
        help: &str,
    ) {
        if coord_ref.is_empty() {
            return;
        }
        let tip_open = response.is_tooltip_open();
        if !response.hovered() && !tip_open {
            return;
        }
        let label = coord_ref.as_str().to_string();
        let macro_ = Macro::new("", 0, vec![]);
        let preview = match ref_preview_spec(catalog, &macro_, coord_ref, kind) {
            Ok((key, caption, coords)) => {
                self.request_desktop_outline_hover(coords);
                match self.texture_for(ui.ctx(), &key, &caption, coords, false, TOOLTIP_MAX_DIM) {
                    Ok(ready) => Ok(ready),
                    Err(err) => Err((caption, err)),
                }
            }
            Err(err) => Err((label.clone(), err)),
        };
        let show_size = matches!(kind, PreviewKind::SearchArea | PreviewKind::Collection);
        show_preview_hover_ui(response, |ui| {
            ui.add(egui::Label::new(label.as_str()).wrap());
            if !help.is_empty() {
                ui.add(egui::Label::new(egui::RichText::new(help).weak()).wrap());
            }
            match &preview {
                Ok(ready) => {
                    paint_preview(ui, &ready.texture, &ready.caption, show_size, ready.marker)
                }
                Err((cap, err)) => paint_preview_status(ui, cap, err),
            }
        });
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
            Ok(ready) => paint_preview_panel_image(ui, &ready.texture, view, ready.marker),
            Err(err) if self.failure_is_no_saved_screen(&key) => {
                self.paint_no_saved_screen_panel(ui, &err, view)
            }
            Err(err) => paint_preview_panel_placeholder(ui, &err, view),
        }
    }

    /// Embedded form-panel preview for a search area (uses form field coords).
    /// Returns the viewport rect and native image size (for heatmap alignment).
    /// Pass `None` for a coordinate that is a variable or non-literal expression.
    #[expect(
        clippy::too_many_arguments,
        reason = "search-area panel: optional LTRB literals, force refresh, and view transform"
    )]
    pub fn paint_search_area_panel(
        &mut self,
        ui: &mut egui::Ui,
        left: Option<i32>,
        top: Option<i32>,
        right: Option<i32>,
        bottom: Option<i32>,
        force: bool,
        view: &mut ImageViewTransform,
    ) -> (egui::Rect, egui::Vec2) {
        let (Some(left), Some(top), Some(right), Some(bottom)) = (left, top, right, bottom) else {
            return (
                paint_preview_panel_placeholder(ui, LITERAL_COORDS_MSG, view),
                egui::Vec2::ZERO,
            );
        };
        let key = format!("panel:sa:{left}:{top}:{right}:{bottom}");
        let caption =
            match crate::data_editor_preview::exclusive_pixel_size(left, top, right, bottom) {
                Some((w, h)) => format!(
                    "Left: {left}, Top: {top}, Right: {right}, Bottom: {bottom}\n· area {}",
                    crate::data_editor_preview::pixel_size_text(w, h)
                ),
                None => format!("Left: {left}, Top: {top}, Right: {right}, Bottom: {bottom}"),
            };
        let preview = self.texture_for(
            ui.ctx(),
            &key,
            &caption,
            PreviewCoords::SearchArea {
                left,
                top,
                right,
                bottom,
                grid: None,
            },
            force,
            PANEL_MAX_DIM,
        );
        match preview {
            Ok(ready) => {
                let [tw, th] = ready.texture.size();
                let rect = paint_preview_panel_image(ui, &ready.texture, view, ready.marker);
                (rect, egui::vec2(tw as f32, th as f32))
            }
            Err(err) if self.failure_is_no_saved_screen(&key) => (
                self.paint_no_saved_screen_panel(ui, &err, view),
                egui::Vec2::ZERO,
            ),
            Err(err) => (
                paint_preview_panel_placeholder(ui, &err, view),
                egui::Vec2::ZERO,
            ),
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
    ) -> Result<ReadyPreview, String> {
        #[cfg(target_os = "android")]
        self.poll_app_shot(ctx);
        #[cfg(target_os = "android")]
        self.drop_stale_backdrop_previews();
        let keep_region = max_dim >= PANEL_MAX_DIM
            && matches!(coords, PreviewCoords::SearchArea { grid: None, .. });
        if force {
            self.invalidate_coords(coords);
        }
        if let Some(entry) = self.entries.get(key) {
            let ready = ReadyPreview {
                texture: entry.texture.clone(),
                caption: entry.caption.clone(),
                marker: entry.marker,
            };
            self.touch(key);
            return Ok(ready);
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
                Ok(Ok(captured)) => {
                    let pending = self.pending.remove(key);
                    let (caption, finish_coords) = match pending {
                        Some(p) => (p.caption, p.coords),
                        None => (caption.to_string(), coords),
                    };
                    self.failures.remove(key);
                    return self.finish_texture(
                        ctx,
                        key,
                        &caption,
                        finish_coords,
                        captured,
                        keep_region,
                    );
                }
                Ok(Err(e)) => {
                    let pending = self.pending.remove(key);
                    let no_saved = matches!(e, CaptureError::NoSavedScreen);
                    let ttl = if e.is_retryable() {
                        RETRY_FAIL_TTL
                    } else {
                        FAIL_CACHE_TTL
                    };
                    let e = e.to_string();
                    let fail_coords = pending.map(|p| p.coords).unwrap_or(coords);
                    self.remember_failure(key, e.clone(), no_saved, now, fail_coords, ttl);
                    return Err(e);
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint();
                    return Err(CAPTURE_PENDING.into());
                }
                Err(TryRecvError::Disconnected) => {
                    let pending = self.pending.remove(key);
                    let e = "capture failed".to_string();
                    let fail_coords = pending.map(|p| p.coords).unwrap_or(coords);
                    self.remember_failure(key, e.clone(), false, now, fail_coords, FAIL_CACHE_TTL);
                    return Err(e);
                }
            }
        }

        self.ensure_capturer()?;
        let capturer = Arc::clone(self.capturer.as_ref().unwrap());

        // Windows + glow: GDI BitBlt off the UI thread races the GL context and can
        // hard-abort the process when the result is uploaded as a texture (no Rust
        // panic hook / crash.log). Capture synchronously on this thread instead.
        #[cfg(target_os = "windows")]
        {
            mark_site("preview:capture_sync");
            match capture_preview(capturer.as_ref(), coords, max_dim, force, keep_region) {
                Ok(captured) => {
                    self.failures.remove(key);
                    self.finish_texture(ctx, key, caption, coords, captured, keep_region)
                }
                Err(e) => {
                    let no_saved = matches!(e, CaptureError::NoSavedScreen);
                    let ttl = if e.is_retryable() {
                        RETRY_FAIL_TTL
                    } else {
                        FAIL_CACHE_TTL
                    };
                    let e = e.to_string();
                    self.remember_failure(key, e.clone(), no_saved, Instant::now(), coords, ttl);
                    Err(e)
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            let fresh = force;
            thread::spawn(move || {
                let _ = tx.send(capture_preview(
                    capturer.as_ref(),
                    coords,
                    max_dim,
                    fresh,
                    keep_region,
                ));
            });
            self.pending.insert(
                key.to_string(),
                PendingCapture {
                    caption: caption.to_string(),
                    coords,
                    rx,
                },
            );
            ctx.request_repaint();
            Err(CAPTURE_PENDING.into())
        }
    }

    fn failure_is_no_saved_screen(&self, key: &str) -> bool {
        self.failures
            .get(key)
            .is_some_and(|fail| fail.no_saved_screen)
    }

    fn remember_failure(
        &mut self,
        key: &str,
        error: String,
        no_saved_screen: bool,
        now: Instant,
        coords: PreviewCoords,
        ttl: Duration,
    ) {
        self.failures.insert(
            key.to_string(),
            FailureEntry {
                error,
                no_saved_screen,
                expires: now + ttl,
                coords,
            },
        );
    }

    fn finish_texture(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        caption: &str,
        coords: PreviewCoords,
        captured: CapturedPreview,
        keep_region: bool,
    ) -> Result<ReadyPreview, String> {
        let img = captured.display;
        let size = [img.width() as usize, img.height() as usize];
        mark_site(&format!("preview:finish_texture:{}x{}", size[0], size[1]));
        let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        // Mipmaps help panel zoom; tooltips stay cheap without them.
        let opts = if key.starts_with("panel:") {
            TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear))
        } else {
            TextureOptions::LINEAR
        };
        let tex = ctx.load_texture(key.to_string(), color, opts);
        let region = if keep_region {
            captured.region.map(Arc::new)
        } else {
            None
        };
        let marker = captured.marker;
        self.insert(
            key.to_string(),
            CacheEntry {
                texture: tex.clone(),
                caption: caption.to_string(),
                coords,
                marker,
                region,
            },
        );
        mark_site("preview:finish_texture:done");
        Ok(ReadyPreview {
            texture: tex,
            caption: caption.to_string(),
            marker,
        })
    }

    fn ensure_capturer(&mut self) -> Result<(), String> {
        if self.capture_disabled {
            return Err("screen capture unavailable".into());
        }
        if let Some(c) = &self.capturer {
            if !c.is_dead() {
                return Ok(());
            }
            // Portal stream ended — take the reopened shared capturer if there is
            // one. The last Arc's Drop joins the pw thread — keep that off the UI.
            if let Some(c) = self.capturer.take() {
                let _ = std::thread::Builder::new()
                    .name("sqyre-cap-drop".into())
                    .spawn(move || drop(c));
            }
        }
        match shared_capturer_nonblocking() {
            Ok(c) => {
                // A reopened session serves new frames (and maybe different
                // monitor coverage) — previews and failures captured against
                // the old capturer are stale.
                self.entries.clear();
                self.order.clear();
                self.pending.clear();
                self.failures.clear();
                self.capturer = Some(c);
                Ok(())
            }
            // Open in flight / stream restarting / picker pending all resolve on
            // a later frame — never latch a permanent failure on the cache.
            Err(e) => Err(e.to_string()),
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

    #[cfg(target_os = "android")]
    fn poll_app_shot(&mut self, ctx: &egui::Context) {
        self.shot.list.poll(ctx);
        let Some(rx) = self.shot.capture.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(_)) => {
                self.shot.capture = None;
                self.shot.status.clear();
            }
            Ok(Err(err)) => {
                self.shot.status = err.to_string();
                self.shot.capture = None;
            }
            Err(TryRecvError::Empty) => ctx.request_repaint(),
            Err(TryRecvError::Disconnected) => {
                self.shot.status = "Taking the picture failed.".into();
                self.shot.capture = None;
            }
        }
    }

    fn paint_no_saved_screen_inline(&mut self, ui: &mut egui::Ui, err: &str) {
        #[cfg(target_os = "android")]
        {
            let _ = err;
            self.paint_app_shot_inline(ui);
        }
        #[cfg(not(target_os = "android"))]
        crate::widgets::error_icon(ui, err);
    }

    fn paint_no_saved_screen_panel(
        &mut self,
        ui: &mut egui::Ui,
        err: &str,
        view: &mut ImageViewTransform,
    ) -> egui::Rect {
        #[cfg(target_os = "android")]
        {
            let _ = err;
            self.paint_app_shot_panel(ui, view)
        }
        #[cfg(not(target_os = "android"))]
        paint_preview_panel_placeholder(ui, err, view)
    }

    #[cfg(target_os = "android")]
    fn paint_app_shot_inline(&mut self, ui: &mut egui::Ui) {
        self.prepare_app_shot(ui.ctx());
        self.paint_app_shot_status(ui);
        self.start_app_shot_if_chosen(ui, "inline_app_shot", None);
    }

    #[cfg(target_os = "android")]
    fn paint_app_shot_panel(
        &mut self,
        ui: &mut egui::Ui,
        _view: &mut ImageViewTransform,
    ) -> egui::Rect {
        let desired = panel_viewport_size(ui);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 4.0, crate::theme::preview_status_fill());
        let body = rect.shrink(12.0);
        let combo_w = (body.width() - 8.0).clamp(120.0, 360.0);
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            ui.set_max_width(combo_w);
            ui.add_space((body.height() * 0.3).min(72.0));
            if self.shot.status.is_empty() && self.shot.capture.is_none() {
                ui.label(crate::app_shot::CHOOSE_HEADING);
            } else {
                self.paint_app_shot_status(ui);
            }
            self.prepare_app_shot(ui.ctx());
            self.start_app_shot_if_chosen(ui, "panel_app_shot", Some(combo_w));
        });
        crate::data_editor_preview::paint_preview_frame(ui.painter(), rect);
        rect
    }

    #[cfg(target_os = "android")]
    fn prepare_app_shot(&mut self, ctx: &egui::Context) {
        self.shot.list.ensure_loading(ctx);
        self.shot.list.poll(ctx);
    }

    #[cfg(target_os = "android")]
    fn paint_app_shot_status(&self, ui: &mut egui::Ui) {
        if self.shot.capture.is_some() {
            ui.label(self.shot.status.as_str());
        } else if !self.shot.status.is_empty() {
            ui.colored_label(crate::theme::error_fg(), self.shot.status.as_str());
        }
    }

    #[cfg(target_os = "android")]
    fn start_app_shot_if_chosen(&mut self, ui: &mut egui::Ui, id_salt: &str, width: Option<f32>) {
        let Some(app) = self.shot.list.show(ui, id_salt, width) else {
            return;
        };
        if self.shot.capture.is_some() {
            return;
        }
        self.shot.status = crate::app_shot::taking_picture_of(app.shown());
        self.shot.capture = Some(crate::screen_pick::spawn_app_shot(
            ui.ctx(),
            app.package,
            app.label,
        ));
    }
}

/// Android: app dropdown and the in-flight screenshot it starts.
#[cfg(target_os = "android")]
#[derive(Default)]
struct PreviewShot {
    list: crate::app_shot::AppList,
    capture: Option<crate::screen_pick::ShotRx>,
    status: String,
}

/// Location mark in pre-downscale capture pixels. Painted over the image so
/// it can blink; the cached texture stays unmarked.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PreviewMarker {
    Point {
        x: f32,
        y: f32,
        src_w: u32,
        src_h: u32,
    },
    Area {
        /// Exclusive rect. The outline is stroked just outside it.
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        grid: Option<(i32, i32)>,
        src_w: u32,
        src_h: u32,
    },
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
        grid: Option<(i32, i32)>,
    },
}

/// True when both describe the same absolute desktop region (grid overlay ignored).
fn same_capture_region(a: PreviewCoords, b: PreviewCoords) -> bool {
    match (a, b) {
        (PreviewCoords::Point { x: x1, y: y1 }, PreviewCoords::Point { x: x2, y: y2 }) => {
            x1 == x2 && y1 == y2
        }
        (
            PreviewCoords::SearchArea {
                left: l1,
                top: t1,
                right: r1,
                bottom: b1,
                ..
            },
            PreviewCoords::SearchArea {
                left: l2,
                top: t2,
                right: r2,
                bottom: b2,
                ..
            },
        ) => l1 == l2 && t1 == t2 && r1 == r2 && b1 == b2,
        _ => false,
    }
}

fn capture_preview(
    capturer: &OsCapturer,
    coords: PreviewCoords,
    max_dim: u32,
    fresh: bool,
    keep_region: bool,
) -> Result<CapturedPreview, CaptureError> {
    // A live Android frame shows Sqyre itself, so previews use the saved backdrop.
    #[cfg(target_os = "android")]
    let capture_rect = |bounds: DesktopRect| {
        let _ = fresh;
        capturer.capture_backdrop_rect_ref(bounds)
    };
    #[cfg(not(target_os = "android"))]
    let capture_rect = |bounds: DesktopRect| {
        if fresh {
            capturer.capture_rect_fresh_ref(bounds)
        } else {
            capturer.capture_rect_ref(bounds)
        }
    };
    let vb = capturer.virtual_bounds_ref()?;
    match coords {
        PreviewCoords::Point { x, y } => {
            if x < vb.x || y < vb.y || x > vb.x + vb.w || y > vb.y + vb.h {
                return Err(CaptureError::Message(format!(
                    "point outside desktop ({},{})..({},{}), got ({x},{y})",
                    vb.x,
                    vb.y,
                    vb.x + vb.w,
                    vb.y + vb.h
                )));
            }
            let bounds = preview_bounds_for_point(x, y, vb);
            let img = capture_rect(bounds)?;
            let (src_w, src_h) = img.dimensions();
            Ok(CapturedPreview {
                marker: PreviewMarker::Point {
                    x: (x - bounds.x) as f32,
                    y: (y - bounds.y) as f32,
                    src_w,
                    src_h,
                },
                display: downscale_max_dim(img, max_dim),
                region: None,
            })
        }
        PreviewCoords::SearchArea {
            left,
            top,
            right,
            bottom,
            grid,
        } => {
            let (lx, ty, rx, by) = normalize_rect(left, top, right, bottom);
            if rx <= lx || by <= ty {
                return Err(CaptureError::EmptySearchArea {
                    left,
                    top,
                    right,
                    bottom,
                });
            }
            // An area fully outside the shared virtual bounds can never produce
            // pixels — surface it like a point outside the desktop instead of
            // silently cropping the wrong monitor and churning recaptures.
            clamp_search_rect(lx, ty, rx, by, Some(vb))?;
            let bounds = preview_bounds_for_search_area(lx, ty, rx, by, vb);
            let img = capture_rect(bounds)?;
            let (src_w, src_h) = img.dimensions();
            let olx = lx - bounds.x;
            let oty = ty - bounds.y;
            let orx = rx - bounds.x;
            let oby = by - bounds.y;
            let region = if keep_region {
                crop_exclusive_region(&img, olx, oty, orx, oby)
            } else {
                None
            };
            Ok(CapturedPreview {
                marker: PreviewMarker::Area {
                    x0: olx as f32,
                    y0: oty as f32,
                    x1: orx as f32,
                    y1: oby as f32,
                    grid,
                    src_w,
                    src_h,
                },
                display: downscale_max_dim(img, max_dim),
                region,
            })
        }
    }
}

/// Exclusive `[lx,rx)×[ty,by)` crop in image space, clamped to the bitmap.
fn crop_exclusive_region(img: &RgbaImage, lx: i32, ty: i32, rx: i32, by: i32) -> Option<RgbaImage> {
    let (lx, ty, rx, by) = normalize_rect(lx, ty, rx, by);
    let (w, h) = img.dimensions();
    let x0 = lx.max(0) as u32;
    let y0 = ty.max(0) as u32;
    let x1 = (rx.max(0) as u32).min(w);
    let y1 = (by.max(0) as u32).min(h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some(image::imageops::crop_imm(img, x0, y0, x1 - x0, y1 - y0).to_image())
}

/// Coordinate ref + preview kind for actions that show point/search-area captures.
pub fn coordinate_ref_for_preview(action: &Action) -> Option<(CoordinateRef, PreviewKind)> {
    match &action.kind {
        ActionKind::Move { point, .. } if !point.is_empty() => {
            Some((point.clone(), PreviewKind::Point))
        }
        ActionKind::ImageSearch(d) if !d.search_area.is_empty() => {
            Some((d.search_area.clone(), PreviewKind::SearchArea))
        }
        ActionKind::Ocr(d) if !d.search_area.is_empty() => {
            Some((d.search_area.clone(), PreviewKind::SearchArea))
        }
        ActionKind::FindPixel(d) if !d.search_area.is_empty() => {
            Some((d.search_area.clone(), PreviewKind::SearchArea))
        }
        ActionKind::ForEachCell { cells, .. } if !cells.is_empty() => {
            Some((cells.clone(), PreviewKind::SearchArea))
        }
        _ => None,
    }
}

fn ref_preview_spec(
    catalog: &ProgramCatalog,
    macro_: &Macro,
    coord_ref: &CoordinateRef,
    kind: PreviewKind,
) -> Result<(String, String, PreviewCoords), String> {
    match kind {
        PreviewKind::Point => {
            let (x, y) = catalog
                .resolve_point(coord_ref, macro_)
                .map_err(|e| e.to_string())?;
            let coords = PreviewCoords::Point { x, y };
            Ok((
                cache_key_ref(coord_ref, coords),
                format!("X: {x}, Y: {y}"),
                coords,
            ))
        }
        PreviewKind::SearchArea | PreviewKind::Collection => {
            let (left, top, right, bottom) = catalog
                .resolve_search_area(coord_ref, macro_)
                .map_err(|e| e.to_string())?;
            let coords = PreviewCoords::SearchArea {
                left,
                top,
                right,
                bottom,
                grid: None,
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
            grid: _,
        } => format!(
            "ref:sa:{}:{left}:{top}:{right}:{bottom}",
            coord_ref.as_str()
        ),
    }
}

#[derive(Debug)]
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
    let empty = Macro::new("", 0, vec![]);
    match kind {
        PreviewKind::Point => {
            let pt = pdata
                .points
                .get(res)
                .or_else(|| pdata.points.values().next())
                .and_then(|m| m.get(name))
                .ok_or(EntityPreviewError::Missing)?;
            let _ = coord_to_literal(&pt.x).ok_or(EntityPreviewError::NonLiteral)?;
            let _ = coord_to_literal(&pt.y).ok_or(EntityPreviewError::NonLiteral)?;
            let coord = CoordinateRef(format!("{program}{PROGRAM_DELIMITER}{name}"));
            let (x, y) = catalog
                .resolve_point(&coord, &empty)
                .map_err(|_| EntityPreviewError::Missing)?;
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
            require_literal_area(sa)?;
            let coord = CoordinateRef(format!("{program}{PROGRAM_DELIMITER}{name}"));
            let (left, top, right, bottom) = catalog
                .resolve_search_area(&coord, &empty)
                .map_err(|_| EntityPreviewError::Missing)?;
            Ok((
                cache_key_search_area(sa),
                search_area_caption(sa),
                PreviewCoords::SearchArea {
                    left,
                    top,
                    right,
                    bottom,
                    grid: None,
                },
            ))
        }
        PreviewKind::Collection => {
            let col = pdata
                .collections
                .get(name)
                .ok_or(EntityPreviewError::Missing)?;
            if col.search_area.is_empty() {
                return Err(EntityPreviewError::Missing);
            }
            let sa = pdata
                .search_areas
                .get(res)
                .or_else(|| pdata.search_areas.values().next())
                .and_then(|m| m.get(&col.search_area))
                .ok_or(EntityPreviewError::Missing)?;
            require_literal_area(sa)?;
            let rows = col.rows.max(1);
            let cols = col.cols.max(1);
            let cell = CoordinateRef::collection(program, name, 1, 1, rows, cols);
            let (left, top, right, bottom) = catalog
                .resolve_search_area(&cell, &empty)
                .map_err(|_| EntityPreviewError::Missing)?;
            Ok((
                cache_key_collection(&col.name, sa, rows, cols),
                collection_caption(sa, rows, cols),
                PreviewCoords::SearchArea {
                    left,
                    top,
                    right,
                    bottom,
                    grid: Some((rows, cols)),
                },
            ))
        }
    }
}

fn require_literal_area(sa: &ProgramSearchArea) -> Result<(), EntityPreviewError> {
    let _ = coord_to_literal(&sa.left_x).ok_or(EntityPreviewError::NonLiteral)?;
    let _ = coord_to_literal(&sa.top_y).ok_or(EntityPreviewError::NonLiteral)?;
    let _ = coord_to_literal(&sa.right_x).ok_or(EntityPreviewError::NonLiteral)?;
    let _ = coord_to_literal(&sa.bottom_y).ok_or(EntityPreviewError::NonLiteral)?;
    Ok(())
}

fn cache_key_point(pt: &ProgramPoint) -> String {
    format!(
        "pt:{}:m{}:{}:{}",
        pt.name,
        pt.monitor,
        pt.x.as_display(),
        pt.y.as_display()
    )
}

fn cache_key_search_area(sa: &ProgramSearchArea) -> String {
    format!(
        "sa:{}:m{}:{}:{}:{}:{}",
        sa.name,
        sa.monitor,
        sa.left_x.as_display(),
        sa.top_y.as_display(),
        sa.right_x.as_display(),
        sa.bottom_y.as_display()
    )
}

fn cache_key_collection(name: &str, sa: &ProgramSearchArea, rows: i32, cols: i32) -> String {
    format!(
        "col:{}:m{}:{}:{}:{}:{}:{}x{}",
        name,
        sa.monitor,
        sa.left_x.as_display(),
        sa.top_y.as_display(),
        sa.right_x.as_display(),
        sa.bottom_y.as_display(),
        rows,
        cols
    )
}

fn point_caption(pt: &ProgramPoint) -> String {
    format!("X: {}, Y: {}", pt.x.as_display(), pt.y.as_display())
}

fn search_area_caption(sa: &ProgramSearchArea) -> String {
    let bounds = format!(
        "Left: {}, Top: {}, Right: {}, Bottom: {}",
        sa.left_x.as_display(),
        sa.top_y.as_display(),
        sa.right_x.as_display(),
        sa.bottom_y.as_display()
    );
    match crate::data_editor_preview::search_area_pixel_size(sa) {
        Some((w, h)) => format!(
            "{bounds}\n· area {}",
            crate::data_editor_preview::pixel_size_text(w, h)
        ),
        None => bounds,
    }
}

fn collection_caption(sa: &ProgramSearchArea, rows: i32, cols: i32) -> String {
    let bounds = search_area_caption(sa);
    match crate::data_editor_preview::search_area_pixel_size(sa)
        .and_then(|(w, h)| crate::data_editor_preview::collection_cell_pixel_size(w, h, rows, cols))
    {
        Some((cw, ch)) => format!(
            "{bounds}\n· cell {}\n· {rows} rows × {cols} cols",
            crate::data_editor_preview::pixel_size_text(cw, ch)
        ),
        None => format!("{bounds}\n· {rows} rows × {cols} cols"),
    }
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

/// List/combo hover tips open to the **right** of the row so a tall collection
/// preview cannot cover the pointer and start an open/close flash loop.
fn show_preview_hover_ui(response: &egui::Response, add_contents: impl FnOnce(&mut egui::Ui)) {
    let mut tip = egui::Tooltip::for_enabled(response);
    tip.popup = tip
        .popup
        .align(egui::RectAlign::RIGHT)
        .align_alternatives(&[
            egui::RectAlign::RIGHT_START,
            egui::RectAlign::RIGHT_END,
            egui::RectAlign::LEFT,
            egui::RectAlign::BOTTOM_START,
        ])
        .gap(10.0);
    tip.show(add_contents);
}

fn paint_preview(
    ui: &mut egui::Ui,
    tex: &TextureHandle,
    caption: &str,
    show_image_size: bool,
    marker: PreviewMarker,
) {
    let [tw, th] = tex.size();
    let size = fit_display(ui, tw as f32, th as f32);
    // Click sense makes the tip "interactive" so egui keeps it open while the
    // pointer is over the image — needed when a tall tip still overlaps the row.
    let image = image_view::add_checkered_image(
        ui,
        tex,
        size,
        egui::Sense::click(),
        0.0,
        image_view::CheckerboardEdge::Outset,
    );
    paint_blinking_marker(&ui.painter_at(image.rect), image.rect, marker);
    ui.add(egui::Label::new(caption).wrap());
    if show_image_size {
        ui.label(
            egui::RichText::new(format!(
                "Preview {}",
                crate::data_editor_preview::pixel_size_text(tw as i32, th as i32)
            ))
            .weak()
            .small(),
        );
    }
}

/// Status line while capture is pending/failed. Reserves the display footprint so
/// the tip does not grow under the cursor when the texture arrives.
///
/// This lives inside a hover tip — a non-interactive surface where a nested
/// hover tooltip can never fire — so the error stays as centered visible text
/// rather than collapsing to an icon like interactive surfaces do.
fn paint_preview_status(ui: &mut egui::Ui, caption: &str, err: &str) {
    paint_preview_status_waking(ui, caption, err, err == CAPTURE_PENDING);
}

fn paint_preview_status_waking(ui: &mut egui::Ui, caption: &str, err: &str, waking: bool) {
    let box_size = image_view::fit_in_box_no_upscale(
        DISPLAY_MAX_W,
        DISPLAY_MAX_H,
        crate::widgets::visible_content_width(ui),
        DISPLAY_MAX_H,
    );
    let (rect, _) = ui.allocate_exact_size(box_size, egui::Sense::click());
    ui.painter()
        .rect_filled(rect, 4.0, crate::theme::preview_status_fill());
    if waking {
        paint_wake_spinner(ui, rect);
    } else {
        paint_preview_message(ui, rect, err, crate::theme::error_fg(), 0.0);
    }
    ui.add(egui::Label::new(caption).wrap());
}

fn paint_wake_spinner(ui: &mut egui::Ui, rect: egui::Rect) {
    let side = ui.spacing().interact_size.y;
    let at = egui::Rect::from_center_size(rect.center(), egui::vec2(side, side));
    let resp = ui.interact(
        at,
        ui.id()
            .with(("pw_wake", at.left().to_bits(), at.top().to_bits())),
        egui::Sense::hover(),
    );
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::ProgressIndicator,
            true,
            crate::help_text::CAPTURE_WAKE,
        )
    });
    resp.on_hover_text(crate::help_text::CAPTURE_WAKE);
    if ui.is_rect_visible(at) {
        let glyph = egui::Rect::from_center_size(at.center(), at.size() * 0.92);
        sqyre_overlay::icons::paint_trace_spinner(ui, glyph, sqyre_ui_theme::accent());
        ui.ctx()
            .request_repaint_after(sqyre_overlay::icons::SPINNER_TICK);
    }
}

/// Centered status/error text inside a preview placeholder, word-wrapped to
/// `rect` minus `side_inset` per side (never narrower than a readable minimum).
fn paint_preview_message(
    ui: &egui::Ui,
    rect: egui::Rect,
    text: &str,
    color: egui::Color32,
    side_inset: f32,
) {
    const MIN_WRAP_W: f32 = 120.0;
    const EDGE_PAD: f32 = 8.0;
    let max_w = (rect.width() - EDGE_PAD * 2.0).max(1.0);
    let wrap_w = (rect.width() - side_inset * 2.0).max(MIN_WRAP_W).min(max_w);
    let mut job = egui::text::LayoutJob::simple(
        text.to_owned(),
        egui::TextStyle::Small.resolve(ui.style()),
        color,
        wrap_w,
    );
    job.halign = egui::Align::Center;
    let painter = ui.painter_at(rect);
    let galley = painter.layout_job(job);
    let pos = egui::pos2(rect.center().x, rect.center().y - galley.size().y * 0.5);
    painter.galley(pos, galley, color);
}

fn log_hover_outline_transition(
    kind: PreviewKind,
    program: &str,
    name: &str,
    coords: PreviewCoords,
) {
    #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
    {
        use std::sync::Mutex;
        type LastOutline = (String, (i32, i32, i32, i32));
        static LAST: Mutex<Option<LastOutline>> = Mutex::new(None);
        let rect = desktop_outline_rect(coords);
        let key = format!("{kind:?}:{program}~{name}");
        let mut guard = LAST.lock().unwrap_or_else(|e| e.into_inner());
        if guard.as_ref() == Some(&(key.clone(), rect)) {
            return;
        }
        *guard = Some((key.clone(), rect));
        sqyre_capture::event_log(
            "SQYRE_PREVIEW",
            &[
                ("op", "hover_outline"),
                ("entity", &key),
                (
                    "rect",
                    &format!("{},{}-{},{}", rect.0, rect.1, rect.2, rect.3),
                ),
            ],
        );
    }
    #[cfg(any(not(feature = "native-runtime"), target_arch = "wasm32"))]
    {
        let _ = (kind, program, name, coords);
    }
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
    marker: PreviewMarker,
) -> egui::Rect {
    let [tw, th] = tex.size();
    let image_size = Vec2::new(tw as f32, th as f32);
    let desired = panel_viewport_size(ui);
    let (viewport, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
    image_view::handle_scroll_zoom(ui, viewport, image_size, view, resp.hovered());
    let content = image_view::image_content_rect(viewport, image_size, view.zoom, view.pan);
    {
        let painter = ui.painter_at(viewport);
        painter.rect_filled(viewport, 0.0, crate::theme::preview_viewport_fill());
        image_view::paint_checkerboard(
            &painter,
            content,
            0.0,
            image_view::CheckerboardEdge::Outset,
        );
        painter.image(
            tex.id(),
            content,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        paint_blinking_marker(&painter, content, marker);
        crate::data_editor_preview::paint_preview_frame(&painter, viewport);
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
    // Hover-only on the panel so a drag still scrolls the form. The icon in
    // the middle is what opens the error (see `attach_preview_error_icon`).
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 4.0, crate::theme::preview_status_fill());
    if err == CAPTURE_PENDING {
        paint_wake_spinner(ui, rect);
    } else {
        crate::data_editor_preview::attach_preview_error_icon(ui, rect, err);
    }
    crate::data_editor_preview::paint_preview_frame(ui.painter(), rect);
    rect
}

fn fit_display(ui: &egui::Ui, w: f32, h: f32) -> Vec2 {
    let w = w.max(1.0);
    let h = h.max(1.0);
    // Cap at the visible bound: tip bodies live in vertical-only scroll areas
    // that do not clip horizontally, so a fixed-size image would paint past
    // the tip edge when the host is narrower than DISPLAY_MAX_W.
    let max_w = DISPLAY_MAX_W.min(crate::widgets::visible_content_width(ui));
    let scale = (max_w / w).min(DISPLAY_MAX_H / h).min(1.0);
    Vec2::new(w * scale, h * scale)
}

fn normalize_rect(lx: i32, ty: i32, rx: i32, by: i32) -> (i32, i32, i32, i32) {
    DesktopRect::normalize_corners(lx, ty, rx, by)
}

fn desktop_outline_rect(coords: PreviewCoords) -> (i32, i32, i32, i32) {
    match coords {
        PreviewCoords::Point { x, y } => {
            let h = POINT_OUTLINE_HALF;
            (x - h, y - h, x + h + 1, y + h + 1)
        }
        PreviewCoords::SearchArea {
            left,
            top,
            right,
            bottom,
            grid: _,
        } => normalize_rect(left, top, right, bottom),
    }
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

/// Paint `marker` over `content` (the image rect, including zoom/pan).
///
/// The mark is not in the texture: it toggles on and off like a text caret so
/// it stays visible on a busy capture. `painter` should already be clipped to
/// the viewport.
fn paint_blinking_marker(painter: &egui::Painter, content: egui::Rect, marker: PreviewMarker) {
    if !content.is_positive() || !marker_in_frame(marker) {
        return;
    }
    if !crate::image_view::preview_marker_blink_on(painter.ctx()) {
        return;
    }
    let color = crate::theme::preview_grid_stroke();
    let (sx, sy) = marker_scale(content, marker);
    let scale = (sx + sy) * 0.5;
    let stroke_px = match marker {
        PreviewMarker::Point { .. } => POINT_STROKE_PX,
        PreviewMarker::Area { .. } => AREA_STROKE_PX,
    };
    let stroke = egui::Stroke::new((stroke_px * scale).max(MARKER_MIN_STROKE), color);
    match marker {
        PreviewMarker::Point { x, y, .. } => {
            let c = egui::pos2(content.left() + x * sx, content.top() + y * sy);
            painter.circle_stroke(c, POINT_RING_RADIUS * scale, stroke);
            painter.line_segment(
                [
                    egui::pos2(c.x - POINT_ARM * sx, c.y),
                    egui::pos2(c.x + POINT_ARM * sx, c.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x, c.y - POINT_ARM * sy),
                    egui::pos2(c.x, c.y + POINT_ARM * sy),
                ],
                stroke,
            );
        }
        PreviewMarker::Area {
            x0,
            y0,
            x1,
            y1,
            grid,
            ..
        } => {
            let rect = egui::Rect::from_min_max(
                egui::pos2(content.left() + x0 * sx, content.top() + y0 * sy),
                egui::pos2(content.left() + x1 * sx, content.top() + y1 * sy),
            );
            if !rect.is_positive() {
                return;
            }
            // Outside the exclusive region so searched pixels stay unpainted.
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
            if let Some((rows, cols)) = grid {
                let rows = rows.max(1);
                let cols = cols.max(1);
                for i in 1..rows {
                    let y = rect.top() + rect.height() * (i as f32) / (rows as f32);
                    painter.hline(rect.x_range(), y, stroke);
                }
                for i in 1..cols {
                    let x = rect.left() + rect.width() * (i as f32) / (cols as f32);
                    painter.vline(x, rect.y_range(), stroke);
                }
            }
        }
    }
}

fn marker_in_frame(marker: PreviewMarker) -> bool {
    match marker {
        PreviewMarker::Point { src_w, src_h, .. } => src_w > 0 && src_h > 0,
        PreviewMarker::Area {
            x0,
            y0,
            x1,
            y1,
            src_w,
            src_h,
            ..
        } => src_w > 0 && src_h > 0 && x1 > x0 && y1 > y0,
    }
}

fn marker_scale(content: egui::Rect, marker: PreviewMarker) -> (f32, f32) {
    let (src_w, src_h) = match marker {
        PreviewMarker::Point { src_w, src_h, .. } | PreviewMarker::Area { src_w, src_h, .. } => {
            (src_w, src_h)
        }
    };
    (
        content.width() / src_w.max(1) as f32,
        content.height() / src_h.max(1) as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn phone_screen() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(360.0, 640.0))
    }

    #[test]
    fn pending_capture_shows_a_spinner_while_the_stream_wakes() {
        use egui_kittest::kittest::Queryable;
        let mut h = egui_kittest::Harness::builder()
            .with_size([320.0, 240.0])
            .build_ui(|ui| {
                paint_preview_status_waking(ui, "point", CAPTURE_PENDING, true);
            });
        h.run_steps(1);
        h.get_by_label(crate::help_text::CAPTURE_WAKE);
    }

    fn input(screen: egui::Rect, time: f64, events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            predicted_dt: 0.1,
            events,
            ..Default::default()
        }
    }

    fn text_on_screen(
        shapes: &[egui::epaint::ClippedShape],
        screen: egui::Rect,
        needle: &str,
    ) -> bool {
        fn walk(shape: &egui::Shape, clip: egui::Rect, screen: egui::Rect, needle: &str) -> bool {
            match shape {
                egui::Shape::Text(text) if text.galley.text().contains(needle) => {
                    let painted =
                        egui::Rect::from_min_size(text.pos, text.galley.size()).intersect(clip);
                    let visible = painted.intersect(screen);
                    visible.width() > 8.0 && visible.height() > 8.0
                }
                egui::Shape::Vec(shapes) => shapes.iter().any(|s| walk(s, clip, screen, needle)),
                _ => false,
            }
        }
        shapes
            .iter()
            .any(|clipped| walk(&clipped.shape, clipped.clip_rect, screen, needle))
    }

    /// The preview fills a phone, so a tip hung off the panel itself sits under
    /// the screen. Hovering the icon must paint the message inside the screen.
    #[test]
    fn preview_error_is_on_screen_when_the_icon_is_hovered() {
        let ctx = egui::Context::default();
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = 0.0;
            style.interaction.show_tooltips_only_when_still = false;
        });
        let screen = phone_screen();
        let mut cache = PreviewTooltipCache::new();
        let mut view = ImageViewTransform::default();
        let mut center = egui::Pos2::ZERO;
        ctx.run_ui(input(screen, 0.0, vec![]), |ui| {
            center = cache
                .paint_point_panel(ui, None, None, false, &mut view)
                .center();
        })
        .drop_without_applying_deltas();

        let mut shown = false;
        for frame in 1..6 {
            let events = if frame == 1 {
                vec![egui::Event::PointerMoved(center)]
            } else {
                Vec::new()
            };
            let out = ctx.run_ui(input(screen, frame as f64 * 0.2, events), |ui| {
                let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
            });
            shown = text_on_screen(&out.shapes, screen, LITERAL_COORDS_MSG);
            out.drop_without_applying_deltas();
        }
        assert!(
            shown,
            "hovering the preview error icon did not paint the message on screen"
        );
    }

    /// A tap is not a hover: the message has to show while the finger is down
    /// and stay after it lifts, without waiting out the hover delay.
    #[test]
    fn preview_error_stays_on_screen_after_tap() {
        let ctx = egui::Context::default();
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = 30.0;
        });
        let screen = phone_screen();
        let mut cache = PreviewTooltipCache::new();
        let mut view = ImageViewTransform::default();
        let mut center = egui::Pos2::ZERO;
        ctx.run_ui(input(screen, 0.0, vec![]), |ui| {
            center = cache
                .paint_point_panel(ui, None, None, false, &mut view)
                .center();
        })
        .drop_without_applying_deltas();

        let press = pointer_button(center, true);
        ctx.run_ui(
            input(screen, 0.2, vec![egui::Event::PointerMoved(center), press]),
            |ui| {
                let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
            },
        )
        .drop_without_applying_deltas();
        // The opening frame is a sizing pass. The next frame, still held, paints.
        let held = ctx.run_ui(input(screen, 0.4, vec![]), |ui| {
            let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
        });
        let held_ok = text_on_screen(&held.shapes, screen, LITERAL_COORDS_MSG);
        held.drop_without_applying_deltas();
        assert!(
            held_ok,
            "holding the preview error icon did not paint the message"
        );

        let release = ctx.run_ui(
            input(
                screen,
                0.6,
                vec![pointer_button(center, false), egui::Event::PointerGone],
            ),
            |ui| {
                let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
            },
        );
        let release_ok = text_on_screen(&release.shapes, screen, LITERAL_COORDS_MSG);
        release.drop_without_applying_deltas();
        assert!(
            release_ok,
            "lifting off the preview error icon hid the message"
        );

        let stay = ctx.run_ui(input(screen, 0.8, vec![]), |ui| {
            let _ = cache.paint_point_panel(ui, None, None, false, &mut view);
        });
        let stay_ok = text_on_screen(&stay.shapes, screen, LITERAL_COORDS_MSG);
        stay.drop_without_applying_deltas();
        assert!(stay_ok, "the preview error disappeared after the tap");
    }

    fn pointer_button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Tip bodies live in vertical-only scroll areas that never clip the X axis.
    /// Fixed-size preview content must shrink to the host bound instead of
    /// expanding the section frame — and the tip — past its right edge.
    #[test]
    fn preview_content_stays_within_narrow_tip_bound() {
        let ctx = egui::Context::default();
        let tex = ctx.load_texture(
            "narrow-bound",
            ColorImage::new([640, 360], vec![egui::Color32::RED; 640 * 360]),
            TextureOptions::LINEAR,
        );
        // FLOATER_MIN_COMPACT is the narrowest tip bound a user can reach.
        let bound = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(220.0, 600.0));
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.scope_builder(egui::UiBuilder::new().max_rect(bound), |ui| {
                crate::theme::framed_section(ui, |ui| {
                    paint_preview(
                        ui,
                        &tex,
                        "X: 1, Y: 2",
                        true,
                        PreviewMarker::Point {
                            x: 320.0,
                            y: 180.0,
                            src_w: 640,
                            src_h: 360,
                        },
                    );
                });
                crate::theme::framed_section(ui, |ui| {
                    paint_preview_status(ui, "cap", "failed");
                });
            });
        });
        for clipped in &out.shapes {
            let right = match &clipped.shape {
                egui::Shape::Mesh(mesh) => mesh
                    .vertices
                    .iter()
                    .map(|v| v.pos.x)
                    .fold(f32::NEG_INFINITY, f32::max),
                egui::Shape::Rect(rect) => rect.rect.right(),
                _ => continue,
            };
            assert!(
                right <= bound.right() + 1.0,
                "tooltip content paints past the tip bound: {right} > {}",
                bound.right()
            );
        }
        out.drop_without_applying_deltas();
    }

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
    fn search_area_outline_sits_outside_and_blinks_off() {
        let marker = PreviewMarker::Area {
            x0: 10.0,
            y0: 20.0,
            x1: 40.0,
            y1: 50.0,
            grid: Some((2, 2)),
            src_w: 100,
            src_h: 100,
        };
        let shapes = paint_marker_at(0.0, marker);
        let color = crate::theme::preview_grid_stroke();
        let outline = shapes.iter().find_map(|c| match &c.shape {
            egui::Shape::Rect(r) if r.stroke.color == color => Some(r),
            _ => None,
        });
        let outline = outline.expect("search-area outline");
        assert_eq!(outline.stroke_kind, egui::StrokeKind::Outside);
        let expected = egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(40.0, 50.0));
        assert!((outline.rect.min.x - expected.min.x).abs() < 0.01);
        assert!((outline.rect.min.y - expected.min.y).abs() < 0.01);
        assert!((outline.rect.max.x - expected.max.x).abs() < 0.01);
        assert!((outline.rect.max.y - expected.max.y).abs() < 0.01);
        // One outline plus the row and column lines.
        assert_eq!(marker_ink(&shapes, color), 3);
        let hidden = paint_marker_at(0.6, marker);
        assert_eq!(marker_ink(&hidden, color), 0);
    }

    #[test]
    fn point_marker_sits_on_the_point_and_blinks_off() {
        let marker = PreviewMarker::Point {
            x: 40.0,
            y: 50.0,
            src_w: 100,
            src_h: 100,
        };
        let content = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 100.0));
        let shapes = paint_marker_in(0.0, content, marker);
        let color = crate::theme::preview_grid_stroke();
        let circle = shapes.iter().find_map(|c| match &c.shape {
            egui::Shape::Circle(circle) if circle.stroke.color == color => Some(circle),
            _ => None,
        });
        let circle = circle.expect("point ring");
        assert!((circle.center.x - 50.0).abs() < 0.01, "{:?}", circle.center);
        assert!((circle.center.y - 70.0).abs() < 0.01, "{:?}", circle.center);
        assert!((circle.radius - POINT_RING_RADIUS).abs() < 0.01);
        // Ring plus the two crosshair arms.
        assert_eq!(marker_ink(&shapes, color), 3);
        let hidden = paint_marker_in(0.6, content, marker);
        assert_eq!(marker_ink(&hidden, color), 0);
    }

    fn paint_marker_at(time: f64, marker: PreviewMarker) -> Vec<egui::epaint::ClippedShape> {
        let content = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 100.0));
        paint_marker_in(time, content, marker)
    }

    fn paint_marker_in(
        time: f64,
        content: egui::Rect,
        marker: PreviewMarker,
    ) -> Vec<egui::epaint::ClippedShape> {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(240.0, 200.0));
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                paint_blinking_marker(&ui.painter_at(screen), content, marker);
            },
        );
        let shapes = out.shapes.clone();
        out.drop_without_applying_deltas();
        shapes
    }

    fn marker_ink(shapes: &[egui::epaint::ClippedShape], color: egui::Color32) -> usize {
        fn walk(shape: &egui::Shape, color: egui::Color32) -> usize {
            match shape {
                egui::Shape::Vec(v) => v.iter().map(|s| walk(s, color)).sum(),
                egui::Shape::LineSegment { stroke, .. } if stroke.color == color => 1,
                egui::Shape::Rect(r) if r.stroke.color == color && r.stroke.width > 0.0 => 1,
                egui::Shape::Circle(c) if c.stroke.color == color && c.stroke.width > 0.0 => 1,
                _ => 0,
            }
        }
        shapes.iter().map(|c| walk(&c.shape, color)).sum()
    }

    #[test]
    fn crop_exclusive_region_matches_outline_interior() {
        let fill = Rgba([10, 20, 30, 255]);
        let mut img = RgbaImage::from_pixel(20, 20, fill);
        // Paint a distinct interior for exclusive [4, 12) × [5, 15).
        for y in 5..15 {
            for x in 4..12 {
                img.put_pixel(x, y, Rgba([1, 2, 3, 255]));
            }
        }
        let crop = crop_exclusive_region(&img, 4, 5, 12, 15).expect("crop");
        assert_eq!(crop.dimensions(), (8, 10));
        assert_eq!(*crop.get_pixel(0, 0), Rgba([1, 2, 3, 255]));
        assert_eq!(*crop.get_pixel(7, 9), Rgba([1, 2, 3, 255]));
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
        use sqyre_domain::{ActionKind, CoordinateRef};

        assert!(coordinate_ref_for_preview(&Action::new(ActionKind::Wait {
            time: ScalarValue::Int(1),
        }))
        .is_none());

        let mv = Action::new(ActionKind::Move {
            point: CoordinateRef("P~Spot".into()),
            smooth: false,
            smooth_low: 0.05,
            smooth_high: 0.2,
            smooth_delay_ms: 1,
        });
        assert_eq!(
            coordinate_ref_for_preview(&mv),
            Some((CoordinateRef("P~Spot".into()), PreviewKind::Point))
        );

        let search = Action::new(ActionKind::ImageSearch(Box::new(
            sqyre_domain::ImageSearchData {
                name: "find".into(),
                search_area: CoordinateRef("P~Box".into()),
                tolerance: 0.9,
                ..Default::default()
            },
        )));
        assert_eq!(
            coordinate_ref_for_preview(&search),
            Some((CoordinateRef("P~Box".into()), PreviewKind::SearchArea))
        );
    }

    #[test]
    fn captions_use_expected_format() {
        let pt = ProgramPoint {
            name: "Spot".into(),
            monitor: 1,
            x: ScalarValue::Int(100),
            y: ScalarValue::Int(200),
        };
        assert_eq!(point_caption(&pt), "X: 100, Y: 200");
        let sa = ProgramSearchArea {
            name: "Box".into(),
            monitor: 1,
            left_x: ScalarValue::Int(10),
            top_y: ScalarValue::Int(20),
            right_x: ScalarValue::Int(110),
            bottom_y: ScalarValue::Int(80),
        };
        assert_eq!(
            search_area_caption(&sa),
            "Left: 10, Top: 20, Right: 110, Bottom: 80\n· area 100×60 px"
        );
        assert_eq!(
            collection_caption(&sa, 2, 2),
            "Left: 10, Top: 20, Right: 110, Bottom: 80\n· area 100×60 px\n· cell 50×30 px\n· 2 rows × 2 cols"
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

    fn catalog_two_monitors() -> ProgramCatalog {
        let mut cat = ProgramCatalog::default();
        cat.set_resolution_key("1920x1080");
        cat.set_monitor_rects(vec![(0, 0, 1920, 1080), (1920, 100, 1920, 1080)]);
        cat.create_program("Game").unwrap();
        cat
    }

    #[test]
    fn entity_preview_point_uses_monitor_origin() {
        let mut cat = catalog_two_monitors();
        cat.upsert_point(
            "Game",
            ProgramPoint {
                name: "Spot".into(),
                monitor: 2,
                x: ScalarValue::Int(10),
                y: ScalarValue::Int(20),
            },
        )
        .unwrap();
        let (_, _, coords) = entity_preview_spec(&cat, "Game", "Spot", PreviewKind::Point).unwrap();
        match coords {
            PreviewCoords::Point { x, y } => assert_eq!((x, y), (1930, 120)),
            _ => panic!("expected point"),
        }
    }

    #[test]
    fn entity_preview_search_area_uses_monitor_origin() {
        let mut cat = catalog_two_monitors();
        cat.upsert_search_area(
            "Game",
            ProgramSearchArea {
                name: "Box".into(),
                monitor: 2,
                left_x: ScalarValue::Int(10),
                top_y: ScalarValue::Int(20),
                right_x: ScalarValue::Int(110),
                bottom_y: ScalarValue::Int(80),
            },
        )
        .unwrap();
        let (_, _, coords) =
            entity_preview_spec(&cat, "Game", "Box", PreviewKind::SearchArea).unwrap();
        match coords {
            PreviewCoords::SearchArea {
                left,
                top,
                right,
                bottom,
                grid,
            } => {
                assert_eq!((left, top, right, bottom), (1930, 120, 2030, 180));
                assert_eq!(grid, None);
            }
            _ => panic!("expected search area"),
        }
    }

    #[test]
    fn entity_preview_collection_uses_linked_search_area_origin() {
        let mut cat = catalog_two_monitors();
        cat.upsert_search_area(
            "Game",
            ProgramSearchArea {
                name: "Box".into(),
                monitor: 2,
                left_x: ScalarValue::Int(0),
                top_y: ScalarValue::Int(0),
                right_x: ScalarValue::Int(100),
                bottom_y: ScalarValue::Int(100),
            },
        )
        .unwrap();
        cat.upsert_collection(
            "Game",
            sqyre_persist::ProgramCollection {
                name: "Bag".into(),
                search_area: "Box".into(),
                rows: 2,
                cols: 2,
            },
        )
        .unwrap();
        let (_, _, coords) =
            entity_preview_spec(&cat, "Game", "Bag", PreviewKind::Collection).unwrap();
        match coords {
            PreviewCoords::SearchArea {
                left,
                top,
                right,
                bottom,
                grid,
            } => {
                assert_eq!((left, top, right, bottom), (1920, 100, 2020, 200));
                assert_eq!(grid, Some((2, 2)));
            }
            _ => panic!("expected collection area"),
        }
    }

    #[test]
    fn same_capture_region_ignores_grid_overlay() {
        let a = PreviewCoords::SearchArea {
            left: 1,
            top: 2,
            right: 10,
            bottom: 20,
            grid: None,
        };
        let b = PreviewCoords::SearchArea {
            left: 1,
            top: 2,
            right: 10,
            bottom: 20,
            grid: Some((3, 4)),
        };
        let c = PreviewCoords::SearchArea {
            left: 1,
            top: 2,
            right: 11,
            bottom: 20,
            grid: None,
        };
        assert!(same_capture_region(a, b));
        assert!(!same_capture_region(a, c));
        assert!(!same_capture_region(PreviewCoords::Point { x: 1, y: 2 }, a));
    }

    #[test]
    fn invalidate_coords_clears_related_keys_only() {
        let mut cache = PreviewTooltipCache::new();
        let coords = PreviewCoords::Point { x: 10, y: 20 };
        let other = PreviewCoords::Point { x: 99, y: 99 };
        let now = Instant::now();
        cache.failures.insert(
            "panel:pt:10:20".into(),
            FailureEntry {
                error: "a".into(),
                no_saved_screen: false,
                expires: now + FAIL_CACHE_TTL,
                coords,
            },
        );
        cache.failures.insert(
            "ref:pt:Game~Spot:10:20".into(),
            FailureEntry {
                error: "b".into(),
                no_saved_screen: false,
                expires: now + FAIL_CACHE_TTL,
                coords,
            },
        );
        cache.failures.insert(
            "panel:pt:99:99".into(),
            FailureEntry {
                error: "c".into(),
                no_saved_screen: false,
                expires: now + FAIL_CACHE_TTL,
                coords: other,
            },
        );
        let (_tx, rx) = std::sync::mpsc::channel();
        cache.pending.insert(
            "pt:Spot:m1:5:5".into(),
            PendingCapture {
                caption: "pending".into(),
                coords,
                rx,
            },
        );
        let (_tx2, rx2) = std::sync::mpsc::channel();
        cache.pending.insert(
            "pt:Other:m1:1:1".into(),
            PendingCapture {
                caption: "keep".into(),
                coords: other,
                rx: rx2,
            },
        );

        cache.invalidate_coords(coords);

        assert!(!cache.failures.contains_key("panel:pt:10:20"));
        assert!(!cache.failures.contains_key("ref:pt:Game~Spot:10:20"));
        assert!(cache.failures.contains_key("panel:pt:99:99"));
        assert!(!cache.pending.contains_key("pt:Spot:m1:5:5"));
        assert!(cache.pending.contains_key("pt:Other:m1:1:1"));
    }
}
