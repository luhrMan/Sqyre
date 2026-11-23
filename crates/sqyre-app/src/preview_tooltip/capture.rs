//! Capture side of the preview cache: texture lookup/refresh, worker spawn,
//! capturer lifecycle, and the LRU entry store.

use super::geometry::{
    downscale_max_dim, normalize_rect, preview_bounds_for_point, preview_bounds_for_search_area,
    PreviewCoords,
};
use super::paint::PreviewMarker;
#[cfg(any(not(target_os = "windows"), test))]
use super::PendingCapture;
use super::{
    CacheEntry, CapturedPreview, FailureEntry, PreviewRefresh, PreviewTooltipCache, ReadyPreview,
    CACHE_MAX, CAPTURE_PENDING, FAIL_CACHE_TTL, PANEL_MAX_DIM, RETRY_FAIL_TTL,
};
use eframe::egui::{self, ColorImage, TextureOptions};
use image::RgbaImage;
use sqyre_capture::{shared_capturer_nonblocking, OsCapturer};
use sqyre_ports::{clamp_search_rect, mark_site, CaptureError, DesktopRect};
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;
#[cfg(not(target_os = "windows"))]
use std::thread;
use web_time::{Duration, Instant};

impl PreviewTooltipCache {
    pub(super) fn texture_for(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        caption: &str,
        coords: PreviewCoords,
        refresh: PreviewRefresh,
        max_dim: u32,
    ) -> Result<ReadyPreview, String> {
        #[cfg(target_os = "android")]
        self.poll_app_shot(ctx);
        #[cfg(target_os = "android")]
        self.drop_stale_backdrop_previews();
        let keep_region = max_dim >= PANEL_MAX_DIM
            && matches!(coords, PreviewCoords::SearchArea { grid: None, .. });
        if refresh == PreviewRefresh::Hard {
            self.invalidate_coords(coords);
        }

        // Always poll in-flight work before returning a cached entry so soft
        // refreshes can swap in without dropping the visible frame — unless a
        // drag is holding swaps so the underlay size stays stable.
        if self.pending.contains_key(key) {
            if self.hold_pending_swaps {
                if let Some(ready) = self.ready_from_entry(key) {
                    ctx.request_repaint();
                    return Ok(ready);
                }
                // First capture (no cached frame): fall through and poll.
            }
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
                    self.remember_failure(
                        key,
                        e.clone(),
                        no_saved,
                        Instant::now(),
                        fail_coords,
                        ttl,
                    );
                    if let Some(ready) = self.ready_from_entry(key) {
                        return Ok(ready);
                    }
                    return Err(e);
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint();
                    if let Some(ready) = self.ready_from_entry(key) {
                        return Ok(ready);
                    }
                    return Err(CAPTURE_PENDING.into());
                }
                Err(TryRecvError::Disconnected) => {
                    let pending = self.pending.remove(key);
                    let e = "capture failed".to_string();
                    let fail_coords = pending.map(|p| p.coords).unwrap_or(coords);
                    self.remember_failure(
                        key,
                        e.clone(),
                        false,
                        Instant::now(),
                        fail_coords,
                        FAIL_CACHE_TTL,
                    );
                    if let Some(ready) = self.ready_from_entry(key) {
                        return Ok(ready);
                    }
                    return Err(e);
                }
            }
        }

        if let Some(ready) = self.ready_from_entry(key) {
            if refresh == PreviewRefresh::Soft && !self.hold_pending_swaps {
                // Recapture without dropping the visible frame. Sync platforms may
                // finish immediately — prefer that result; otherwise keep `ready`.
                let fresh = refresh.waits_for_fresh_frame();
                match self.spawn_capture(ctx, key, caption, coords, max_dim, keep_region, fresh) {
                    Ok(fresh) => return Ok(fresh),
                    Err(_) => return Ok(ready),
                }
            }
            return Ok(ready);
        }

        let now = Instant::now();
        if let Some(fail) = self.failures.get(key) {
            if now < fail.expires {
                return Err(fail.error.clone());
            }
            self.failures.remove(key);
        }

        let fresh = refresh.waits_for_fresh_frame();
        self.spawn_capture(ctx, key, caption, coords, max_dim, keep_region, fresh)
    }

    fn ready_from_entry(&mut self, key: &str) -> Option<ReadyPreview> {
        let entry = self.entries.get(key)?;
        let ready = ReadyPreview {
            texture: entry.texture.clone(),
            caption: entry.caption.clone(),
            marker: entry.marker,
        };
        self.touch(key);
        Some(ready)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "capture spawn needs key/caption/coords plus dim and freshness flags"
    )]
    fn spawn_capture(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        caption: &str,
        coords: PreviewCoords,
        max_dim: u32,
        keep_region: bool,
        fresh: bool,
    ) -> Result<ReadyPreview, String> {
        if self.pending.contains_key(key) {
            ctx.request_repaint();
            return Err(CAPTURE_PENDING.into());
        }
        self.ensure_capturer()?;
        let capturer = Arc::clone(self.capturer.as_ref().unwrap());

        // Windows + glow: GDI BitBlt off the UI thread races the GL context and can
        // hard-abort the process when the result is uploaded as a texture (no Rust
        // panic hook / crash.log). Capture synchronously on this thread instead.
        #[cfg(target_os = "windows")]
        {
            mark_site("preview:capture_sync");
            match capture_preview(capturer.as_ref(), coords, max_dim, fresh, keep_region) {
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

    pub(super) fn failure_is_no_saved_screen(&self, key: &str) -> bool {
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
        let img = Arc::new(captured.display);
        let size = [img.width() as usize, img.height() as usize];
        mark_site(&format!("preview:finish_texture:{}x{}", size[0], size[1]));
        let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        // Mipmaps help panel zoom; tooltips stay cheap without them.
        let is_panel = key.starts_with("panel:");
        let opts = if is_panel {
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
        let display = is_panel.then(|| Arc::clone(&img));
        let marker = captured.marker;
        self.insert(
            key.to_string(),
            CacheEntry {
                texture: tex.clone(),
                caption: caption.to_string(),
                coords,
                marker,
                region,
                display,
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

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
    fn hold_pending_swaps_keeps_stale_frame_without_consuming_ready() {
        let ctx = egui::Context::default();
        let mut cache = PreviewTooltipCache::new();
        let coords = PreviewCoords::SearchArea {
            left: 0,
            top: 0,
            right: 10,
            bottom: 10,
            grid: None,
        };
        let key = "panel:sa:0:0:10:10";
        let color = ColorImage::new([4, 4], vec![egui::Color32::GREEN; 16]);
        let tex = ctx.load_texture(key, color, TextureOptions::LINEAR);
        cache.insert(
            key.into(),
            CacheEntry {
                texture: tex,
                caption: "stale".into(),
                coords,
                marker: PreviewMarker::Area {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 10.0,
                    y1: 10.0,
                    grid: None,
                    src_w: 10,
                    src_h: 10,
                },
                region: None,
                display: None,
            },
        );
        let (tx, rx) = std::sync::mpsc::channel();
        // Finished capture waiting to be applied.
        tx.send(Ok(CapturedPreview {
            display: image::RgbaImage::new(4, 4),
            region: None,
            marker: PreviewMarker::Area {
                x0: 0.0,
                y0: 0.0,
                x1: 10.0,
                y1: 10.0,
                grid: None,
                src_w: 10,
                src_h: 10,
            },
        }))
        .unwrap();
        cache.pending.insert(
            key.into(),
            PendingCapture {
                caption: "fresh".into(),
                coords,
                rx,
            },
        );
        cache.set_hold_pending_swaps(true);
        let ready = cache
            .texture_for(
                &ctx,
                key,
                "fresh",
                coords,
                PreviewRefresh::None,
                PANEL_MAX_DIM,
            )
            .expect("hold must keep the stale frame");
        assert_eq!(ready.caption, "stale");
        assert!(
            cache.pending.contains_key(key),
            "ready capture must stay queued until the drag ends"
        );
    }

    #[test]
    fn soft_refresh_keeps_cached_frame_while_pending() {
        let ctx = egui::Context::default();
        let mut cache = PreviewTooltipCache::new();
        let coords = PreviewCoords::SearchArea {
            left: 0,
            top: 0,
            right: 10,
            bottom: 10,
            grid: None,
        };
        let key = "panel:sa:0:0:10:10";
        let color = ColorImage::new([4, 4], vec![egui::Color32::GREEN; 16]);
        let tex = ctx.load_texture(key, color, TextureOptions::LINEAR);
        cache.insert(
            key.into(),
            CacheEntry {
                texture: tex.clone(),
                caption: "stale".into(),
                coords,
                marker: PreviewMarker::Area {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 10.0,
                    y1: 10.0,
                    grid: None,
                    src_w: 10,
                    src_h: 10,
                },
                region: None,
                display: None,
            },
        );
        let (_tx, rx) = std::sync::mpsc::channel();
        cache.pending.insert(
            key.into(),
            PendingCapture {
                caption: "refresh".into(),
                coords,
                rx,
            },
        );

        let ready = cache
            .texture_for(
                &ctx,
                key,
                "refresh",
                coords,
                PreviewRefresh::None,
                PANEL_MAX_DIM,
            )
            .expect("stale frame must stay visible while soft capture is in flight");
        assert_eq!(ready.caption, "stale");
        assert!(cache.panel_search_area_pending(0, 0, 10, 10));
    }
}
