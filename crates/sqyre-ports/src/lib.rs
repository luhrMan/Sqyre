//! OS-facing port traits (capture, automation, focus) and execution telemetry
//! (action log, highlight, runtime vars) shared by adapters and the macro executor.

mod action_log;
mod automation_error;
mod capture_error;
mod domain_ports;
mod highlight;
mod port_error;
mod portal_remote;
mod runtime_vars;

pub use action_log::{
    lines_for, ActionLogEntry, ActionLogger, LogImage, SharedActionLog, MAX_ENTRIES_PER_ACTION,
};
pub use automation_error::AutomationError;
pub use capture_error::CaptureError;
pub use domain_ports::{ContinueKeyWaiter, CoordinateResolver, IconStore, MacroLookup};
pub use highlight::{
    clear_highlights, highlight_clear, highlight_cursor, highlight_fill, ActionHighlighter,
    HighlightEvent, HighlightKind, HighlightSnapshot, SharedHighlighter,
};
pub use port_error::PortError;
pub use portal_remote::PortalRemoteInput;
pub use runtime_vars::{RuntimeVarSink, SharedRuntimeVars};

use image::RgbaImage;
use rayon::prelude::*;

/// Mouse move options.
#[derive(Debug, Clone, Copy, Default)]
pub struct MoveOptions {
    pub smooth: bool,
    pub low: f64,
    pub high: f64,
    pub delay_ms: i32,
}

/// Absolute virtual-desktop rectangle (inclusive left/top, exclusive right/bottom
/// when used as x,y,w,h via helpers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DesktopRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl DesktopRect {
    /// Normalize two corners so left≤right and top≤bottom.
    pub fn normalize_corners(ax: i32, ay: i32, bx: i32, by: i32) -> (i32, i32, i32, i32) {
        let (left, right) = if ax <= bx { (ax, bx) } else { (bx, ax) };
        let (top, bottom) = if ay <= by { (ay, by) } else { (by, ay) };
        (left, top, right, bottom)
    }

    pub fn from_corners(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        let (left, top, right, bottom) = Self::normalize_corners(left, top, right, bottom);
        Self {
            x: left,
            y: top,
            w: (right - left).max(0),
            h: (bottom - top).max(0),
        }
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0 || self.h <= 0
    }
}

/// Packed RGB capture (no alpha) for search / OCR / find-pixel hot paths.
#[derive(Debug, Clone)]
pub struct RgbCapture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl RgbCapture {
    /// Strip alpha from an RGBA capture.
    pub fn from_rgba(img: &RgbaImage) -> Self {
        let width = img.width();
        let height = img.height();
        let mut data = vec![0u8; width as usize * height as usize * 3];
        data.par_chunks_exact_mut(3)
            .zip(img.as_raw().par_chunks_exact(4))
            .for_each(|(dst, src)| dst.copy_from_slice(&src[..3]));
        Self {
            width,
            height,
            data,
        }
    }
}

/// Clamp a search-area box to optional virtual-desktop bounds.
pub fn clamp_search_rect(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    vb: Option<DesktopRect>,
) -> Result<DesktopRect, CaptureError> {
    let mut rect = DesktopRect::from_corners(left, top, right, bottom);
    if rect.is_empty() {
        return Err(CaptureError::EmptySearchArea {
            left,
            top,
            right,
            bottom,
        });
    }
    if let Some(vb) = vb {
        let lx = left.max(vb.x);
        let ty = top.max(vb.y);
        let rx = right.min(vb.x + vb.w);
        let by = bottom.min(vb.y + vb.h);
        rect = DesktopRect::from_corners(lx, ty, rx, by);
        if rect.is_empty() {
            return Err(CaptureError::OutsideVirtualDesktop);
        }
    }
    Ok(rect)
}

/// Mouse / keyboard / timing / clipboard.
pub trait AutomationBackend {
    fn milli_sleep(&mut self, ms: i32);
    fn move_to(&mut self, x: i32, y: i32, opts: MoveOptions);
    fn click(&mut self, button: &str, down: bool) -> Result<(), AutomationError>;
    fn scroll(&mut self, up: bool) -> Result<(), AutomationError>;
    fn key_down(&mut self, key: &str) -> Result<(), AutomationError>;
    fn key_up(&mut self, key: &str) -> Result<(), AutomationError>;
    fn type_char(&mut self, ch: char);
    fn write_clipboard(&mut self, s: &str) -> Result<(), AutomationError>;
}

/// Screen capture in absolute virtual-desktop coordinates.
pub trait ScreenCapturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, CaptureError>;
    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, CaptureError>;
    fn virtual_bounds(&mut self) -> Result<DesktopRect, CaptureError>;

    /// Per-monitor (width, height) in display order.
    /// Default: one entry from [`Self::virtual_bounds`].
    fn monitor_sizes(&mut self) -> Result<Vec<(i32, i32)>, CaptureError> {
        let vb = self.virtual_bounds()?;
        Ok(vec![(vb.w, vb.h)])
    }

    /// Capture RGB (no alpha). Default: RGBA capture then strip alpha.
    fn capture_rect_rgb(&mut self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        Ok(RgbCapture::from_rgba(&self.capture_rect(rect)?))
    }

    /// RGB capture that waits for a newer compositor frame when the backend
    /// caches buffers (portal + PipeWire). Used for image search and wait/repeat
    /// retries — not the default one-shot OCR / find-pixel crop.
    /// Default: same as [`Self::capture_rect_rgb`].
    fn capture_rect_rgb_fresh(&mut self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        self.capture_rect_rgb(rect)
    }

    /// Capture a search-area rectangle after basic size checks.
    fn capture_search_area(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Result<(RgbaImage, DesktopRect), CaptureError> {
        let vb = self.virtual_bounds().ok();
        let rect = clamp_search_rect(left, top, right, bottom, vb)?;
        let img = self.capture_rect(rect)?;
        Ok((img, rect))
    }

    /// RGB search-area capture (preferred for image/OCR/pixel matching).
    ///
    /// `fresh` waits for a newer compositor frame on caching backends (wait/repeat
    /// retries and image search). Other one-shot searches crop the latest cache.
    fn capture_search_area_rgb(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        fresh: bool,
    ) -> Result<(RgbCapture, DesktopRect), CaptureError> {
        let vb = self.virtual_bounds().ok();
        let rect = clamp_search_rect(left, top, right, bottom, vb)?;
        let img = if fresh {
            self.capture_rect_rgb_fresh(rect)?
        } else {
            self.capture_rect_rgb(rect)?
        };
        Ok((img, rect))
    }
}

/// Bring a window to the front by executable path + title.
pub trait WindowFocuser: Send + Sync {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), AutomationError>;
}

/// Catalog item metadata (name, stack size, grid dimensions) for icon lookups.
#[derive(Debug, Clone, Default)]
pub struct ItemMeta {
    pub name: String,
    pub stack_max: i32,
    pub cols: i32,
    pub rows: i32,
}
