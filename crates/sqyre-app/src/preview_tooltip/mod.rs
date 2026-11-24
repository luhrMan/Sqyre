//! Hover tooltips showing a live screen capture around a point or search area.

mod capture;
mod geometry;
mod paint;
mod spec;

pub use spec::coordinate_ref_for_preview;

use crate::image_view::ImageViewTransform;
use eframe::egui::{self, TextureHandle};
use geometry::{desktop_outline_rect, same_capture_region, PreviewCoords};
use image::RgbaImage;
use paint::{
    log_hover_outline_transition, paint_preview, paint_preview_panel_image,
    paint_preview_panel_placeholder, paint_preview_status, show_preview_hover_ui, PreviewMarker,
};
use spec::{entity_preview_spec, ref_preview_spec, EntityPreviewError};
use sqyre_capture::OsCapturer;
use sqyre_domain::{CoordinateRef, Macro};
use sqyre_persist::CatalogView;
use sqyre_ports::CaptureError;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use web_time::{Duration, Instant};

/// Hover/action tooltip previews — small on screen, keep textures light.
const TOOLTIP_MAX_DIM: u32 = 640;
/// Data-editor panel previews stretch with the window and support zoom.
const PANEL_MAX_DIM: u32 = 1600;
pub(crate) const DISPLAY_MAX_W: f32 = 260.0;
const DISPLAY_MAX_H: f32 = 195.0;
const CACHE_MAX: usize = 24;
/// How long to remember a failed capture before trying again (manual ↻ clears sooner).
const FAIL_CACHE_TTL: Duration = Duration::from_secs(60);
/// Transient `NotReady` errors (stream restarting, first frame pending) retry
/// after this much shorter window — the shared capturer may already be back.
const RETRY_FAIL_TTL: Duration = Duration::from_millis(500);
const LITERAL_COORDS_MSG: &str = "Preview needs literal coordinates";

/// In-flight preview capture. A slept PipeWire stream shows a spinner instead.
const CAPTURE_PENDING: &str = "Capturing…";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Point,
    SearchArea,
    Collection,
}

/// Full-res Match Check inputs plus the downscaled underlay they paint onto.
#[derive(Clone)]
pub struct PanelMatchSource {
    /// Exclusive search-area crop at capture resolution (Image Search parity).
    pub region: Arc<RgbaImage>,
    /// Downscaled panel texture pixels for heatmap alignment.
    pub display: Arc<RgbaImage>,
    /// Exclusive area origin inside the pre-downscale capture (`PreviewMarker::Area`).
    pub area_x0: f32,
    pub area_y0: f32,
    pub src_w: u32,
    pub src_h: u32,
}

/// How a panel/tooltip capture should refresh.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PreviewRefresh {
    /// Reuse the cached texture when present.
    #[default]
    None,
    /// Drop cached pixels and show Capturing… until the new frame lands.
    Hard,
    /// Keep showing the last frame while a recapture runs in the background.
    Soft,
}

impl PreviewRefresh {
    /// Only manual refresh waits for a new portal frame. That wait pulses the
    /// compositor kick, which takes activation on Mutter and cancels any drag
    /// on the desktop. Live (soft) refresh crops the latest streamed frame.
    fn waits_for_fresh_frame(self) -> bool {
        self == Self::Hard
    }
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
    /// Downscaled panel display pixels (same buffer as the texture); Match Check source.
    display: Option<Arc<RgbaImage>>,
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
    /// When set, keep showing the cached frame and do not apply finished captures.
    ///
    /// Live Match Check soft-refresh must not swap textures mid-drag — a size
    /// change ends egui's drag target and feels like the gesture "finished early".
    hold_pending_swaps: bool,
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
    shot: paint::PreviewShot,
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

    /// Freeze finished capture swaps (and soft-refresh starts) while the user drags.
    pub fn set_hold_pending_swaps(&mut self, hold: bool) {
        self.hold_pending_swaps = hold;
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

    /// Full-res exclusive search-area crop plus underlay mapping for Match Check.
    ///
    /// Matching uses [`PanelMatchSource::region`] (same pixels as Image Search).
    /// Heatmap/boxes map onto the downscaled [`PanelMatchSource::display`] underlay.
    pub fn panel_match_source(
        &self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Option<PanelMatchSource> {
        let key = format!("panel:sa:{left}:{top}:{right}:{bottom}");
        let entry = self.entries.get(&key)?;
        let display = entry.display.as_ref().map(Arc::clone)?;
        let region = entry.region.as_ref().map(Arc::clone)?;
        let PreviewMarker::Area {
            x0,
            y0,
            src_w,
            src_h,
            ..
        } = entry.marker
        else {
            return None;
        };
        Some(PanelMatchSource {
            region,
            display,
            area_x0: x0,
            area_y0: y0,
            src_w,
            src_h,
        })
    }

    /// True while a soft/hard panel search-area capture is still in flight.
    pub fn panel_search_area_pending(&self, left: i32, top: i32, right: i32, bottom: i32) -> bool {
        let key = format!("panel:sa:{left}:{top}:{right}:{bottom}");
        self.pending.contains_key(&key)
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
        catalog: CatalogView<'_>,
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
                let preview = self.texture_for(
                    ui.ctx(),
                    &key,
                    &caption,
                    coords,
                    PreviewRefresh::None,
                    TOOLTIP_MAX_DIM,
                );
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
        catalog: CatalogView<'_>,
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
                let refresh = if force {
                    PreviewRefresh::Hard
                } else {
                    PreviewRefresh::None
                };
                let preview =
                    self.texture_for(ui.ctx(), &key, &caption, coords, refresh, TOOLTIP_MAX_DIM);
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
        catalog: CatalogView<'_>,
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
                match self.texture_for(
                    ui.ctx(),
                    &key,
                    &caption,
                    coords,
                    PreviewRefresh::None,
                    TOOLTIP_MAX_DIM,
                ) {
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
        let refresh = if force {
            PreviewRefresh::Hard
        } else {
            PreviewRefresh::None
        };
        let preview = self.texture_for(
            ui.ctx(),
            &key,
            &caption,
            PreviewCoords::Point { x, y },
            refresh,
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
    ///
    /// `grid` paints blinking rows×cols lines over the area (Collections); it is
    /// paint-only and does not affect the capture cache key or ScreenCap crop.
    #[expect(
        clippy::too_many_arguments,
        reason = "search-area panel: optional LTRB literals, grid, refresh, and view transform"
    )]
    pub fn paint_search_area_panel(
        &mut self,
        ui: &mut egui::Ui,
        left: Option<i32>,
        top: Option<i32>,
        right: Option<i32>,
        bottom: Option<i32>,
        grid: Option<(i32, i32)>,
        refresh: PreviewRefresh,
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
            refresh,
            PANEL_MAX_DIM,
        );
        match preview {
            Ok(ready) => {
                let [tw, th] = ready.texture.size();
                let marker = match ready.marker {
                    PreviewMarker::Area {
                        x0,
                        y0,
                        x1,
                        y1,
                        src_w,
                        src_h,
                        ..
                    } => PreviewMarker::Area {
                        x0,
                        y0,
                        x1,
                        y1,
                        grid,
                        src_w,
                        src_h,
                    },
                    other => other,
                };
                let rect = paint_preview_panel_image(ui, &ready.texture, view, marker);
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
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn only_hard_refresh_waits_for_a_fresh_frame() {
        assert!(PreviewRefresh::Hard.waits_for_fresh_frame());
        assert!(!PreviewRefresh::Soft.waits_for_fresh_frame());
        assert!(!PreviewRefresh::None.waits_for_fresh_frame());
    }
}
