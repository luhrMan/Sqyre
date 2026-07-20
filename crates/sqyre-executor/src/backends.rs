use image::RgbaImage;
use sqyre_domain::{CoordinateRef, Macro};
use sqyre_match::ImageBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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
    /// Strip alpha from an RGBA capture (delegates to [`sqyre_vision::rgba_to_rgb_buf`]).
    pub fn from_rgba(img: &RgbaImage) -> Self {
        let buf = sqyre_vision::rgba_to_rgb_buf(img);
        Self {
            width: buf.width as u32,
            height: buf.height as u32,
            data: buf.data,
        }
    }

    pub fn into_image_buf(self) -> ImageBuf {
        ImageBuf::from_raw(self.width as usize, self.height as usize, 3, self.data)
    }
}

/// Clamp a search-area box to optional virtual-desktop bounds.
pub fn clamp_search_rect(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    vb: Option<DesktopRect>,
) -> Result<DesktopRect, String> {
    let mut rect = DesktopRect::from_corners(left, top, right, bottom);
    if rect.is_empty() {
        return Err(format!("empty search area {left},{top},{right},{bottom}"));
    }
    if let Some(vb) = vb {
        let lx = left.max(vb.x);
        let ty = top.max(vb.y);
        let rx = right.min(vb.x + vb.w);
        let by = bottom.min(vb.y + vb.h);
        rect = DesktopRect::from_corners(lx, ty, rx, by);
        if rect.is_empty() {
            return Err("search area outside virtual desktop".into());
        }
    }
    Ok(rect)
}

/// Mouse / keyboard / timing / clipboard.
pub trait AutomationBackend {
    fn milli_sleep(&mut self, ms: i32);
    fn move_to(&mut self, x: i32, y: i32, opts: MoveOptions);
    fn click(&mut self, button: &str, down: bool) -> Result<(), String>;
    fn scroll(&mut self, up: bool) -> Result<(), String>;
    fn key_down(&mut self, key: &str) -> Result<(), String>;
    fn key_up(&mut self, key: &str) -> Result<(), String>;
    fn type_char(&mut self, ch: char);
    fn write_clipboard(&mut self, s: &str) -> Result<(), String>;
}

/// Screen capture in absolute virtual-desktop coordinates.
pub trait ScreenCapturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, String>;
    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, String>;
    fn virtual_bounds(&mut self) -> Result<DesktopRect, String>;

    /// Per-monitor (width, height) in display order.
    /// Default: one entry from [`Self::virtual_bounds`].
    fn monitor_sizes(&mut self) -> Result<Vec<(i32, i32)>, String> {
        let vb = self.virtual_bounds()?;
        Ok(vec![(vb.w, vb.h)])
    }

    /// Capture RGB (no alpha). Default: RGBA capture then strip alpha.
    fn capture_rect_rgb(&mut self, rect: DesktopRect) -> Result<RgbCapture, String> {
        Ok(RgbCapture::from_rgba(&self.capture_rect(rect)?))
    }

    /// Capture a search-area rectangle after basic size checks.
    fn capture_search_area(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Result<(RgbaImage, DesktopRect), String> {
        let vb = self.virtual_bounds().ok();
        let rect = clamp_search_rect(left, top, right, bottom, vb)?;
        let img = self.capture_rect(rect)?;
        Ok((img, rect))
    }

    /// RGB search-area capture (preferred for image/OCR/pixel matching).
    fn capture_search_area_rgb(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Result<(RgbCapture, DesktopRect), String> {
        let vb = self.virtual_bounds().ok();
        let rect = clamp_search_rect(left, top, right, bottom, vb)?;
        let img = self.capture_rect_rgb(rect)?;
        Ok((img, rect))
    }
}

/// Resolve `program~point` / search-area refs using the loaded program catalog.
pub trait CoordinateResolver {
    fn resolve_point(&self, r: &CoordinateRef, macro_: &Macro) -> Result<(i32, i32), String>;
    fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), String>;

    /// Collection grid size `(rows, cols)` for `program` + collection name.
    fn collection_grid(&self, program: &str, collection: &str) -> Result<(i32, i32), String> {
        let _ = (program, collection);
        Err("collection grid lookup not configured".into())
    }
}

/// Resolve image-search targets to on-disk icon / mask paths.
pub trait IconStore {
    /// Variant icon paths for `program~item` (may be empty).
    fn variant_paths(&self, target: &str) -> Vec<std::path::PathBuf>;
    /// Optional mask PNG for the item (resized by caller).
    fn mask_path(&self, target: &str) -> Option<std::path::PathBuf>;
    fn item_meta(&self, target: &str) -> Option<ItemMeta>;
}

#[derive(Debug, Clone, Default)]
pub struct ItemMeta {
    pub name: String,
    pub stack_max: i32,
    pub cols: i32,
    pub rows: i32,
}

/// Look up another macro by name.
pub trait MacroLookup: Send + Sync {
    fn get(&self, name: &str) -> Option<Arc<Macro>>;
}

/// Block until the user presses a continue chord.
pub trait ContinueKeyWaiter: Send + Sync {
    fn wait_for_continue(
        &self,
        keys: &[String],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<(), String>;

    /// Wait until one of `chords` is pressed. Returns the matched index.
    /// `hold_repeat` is parallel to `chords` (missing = false).
    fn wait_for_any_chord(
        &self,
        chords: &[Vec<String>],
        hold_repeat: &[bool],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<usize, String> {
        let _ = hold_repeat;
        if chords.is_empty() {
            return Err("key wait: no chords configured".into());
        }
        // Default: only the first chord (used by tests / simple waiters).
        self.wait_for_continue(&chords[0], pass_through, stop)?;
        Ok(0)
    }
}

/// Bring a window to the front by executable path + title.
pub trait WindowFocuser: Send + Sync {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), String>;
}

/// OCR recognition result (word boxes + joined text).
#[derive(Debug, Clone, Default)]
pub struct OcrResult {
    pub text: String,
    pub words: Vec<sqyre_vision::OcrWordBox>,
}

/// Run OCR on a preprocessed image buffer.
pub trait OcrEngine: Send + Sync {
    fn recognize(&self, image: &sqyre_match::ImageBuf) -> Result<OcrResult, String>;
}

/// Test OCR engine that returns a fixed result.
#[derive(Debug, Default)]
pub struct FixedOcrEngine {
    pub result: OcrResult,
    pub log: std::sync::Mutex<Vec<String>>,
}

impl OcrEngine for FixedOcrEngine {
    fn recognize(&self, image: &sqyre_match::ImageBuf) -> Result<OcrResult, String> {
        if let Ok(mut g) = self.log.lock() {
            g.push(format!(
                "ocr:{}x{}c{}",
                image.width, image.height, image.channels
            ));
        }
        Ok(self.result.clone())
    }
}

/// Test OCR engine that pops results from a FIFO queue (then repeats the last).
#[derive(Debug, Default)]
pub struct QueuedOcrEngine {
    pub queue: std::sync::Mutex<Vec<OcrResult>>,
    pub log: std::sync::Mutex<Vec<String>>,
}

impl OcrEngine for QueuedOcrEngine {
    fn recognize(&self, image: &sqyre_match::ImageBuf) -> Result<OcrResult, String> {
        if let Ok(mut g) = self.log.lock() {
            g.push(format!(
                "ocr:{}x{}c{}",
                image.width, image.height, image.channels
            ));
        }
        let mut q = self
            .queue
            .lock()
            .map_err(|_| "QueuedOcrEngine: lock poisoned".to_string())?;
        if q.is_empty() {
            return Err("QueuedOcrEngine: empty queue".into());
        }
        if q.len() == 1 {
            return Ok(q[0].clone());
        }
        Ok(q.remove(0))
    }
}

/// Recording backend for unit tests.
#[derive(Debug, Default)]
pub struct RecordingBackend {
    pub log: Vec<String>,
}

impl AutomationBackend for RecordingBackend {
    fn milli_sleep(&mut self, ms: i32) {
        self.log.push(format!("sleep:{ms}"));
    }
    fn move_to(&mut self, x: i32, y: i32, opts: MoveOptions) {
        self.log
            .push(format!("move:{x},{y},smooth={}", opts.smooth));
    }
    fn click(&mut self, button: &str, down: bool) -> Result<(), String> {
        self.log.push(format!(
            "click:{button}:{}",
            if down { "down" } else { "up" }
        ));
        Ok(())
    }
    fn scroll(&mut self, up: bool) -> Result<(), String> {
        self.log
            .push(format!("scroll:{}", if up { "up" } else { "down" }));
        Ok(())
    }
    fn key_down(&mut self, key: &str) -> Result<(), String> {
        self.log.push(format!("keydown:{key}"));
        Ok(())
    }
    fn key_up(&mut self, key: &str) -> Result<(), String> {
        self.log.push(format!("keyup:{key}"));
        Ok(())
    }
    fn type_char(&mut self, ch: char) {
        self.log.push(format!("type:{ch}"));
    }
    fn write_clipboard(&mut self, s: &str) -> Result<(), String> {
        self.log.push(format!("clipboard:{s}"));
        Ok(())
    }
}

/// In-memory capturer for tests.
#[derive(Debug, Default)]
pub struct RecordingCapturer {
    pub log: Vec<String>,
    /// Single image returned when [`Self::queue`] is empty.
    pub next: Option<RgbaImage>,
    /// FIFO images consumed one per capture (then falls back to [`Self::next`]).
    pub queue: Vec<RgbaImage>,
    pub bounds: DesktopRect,
}

impl RecordingCapturer {
    fn take_image(&mut self) -> Result<RgbaImage, String> {
        if !self.queue.is_empty() {
            return Ok(self.queue.remove(0));
        }
        self.next
            .clone()
            .ok_or_else(|| "RecordingCapturer: no image".into())
    }
}

impl ScreenCapturer for RecordingCapturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, String> {
        self.log.push(format!("monitor:{display_index}"));
        self.take_image()
    }
    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, String> {
        self.log
            .push(format!("rect:{},{},{},{}", rect.x, rect.y, rect.w, rect.h));
        self.take_image()
    }
    fn virtual_bounds(&mut self) -> Result<DesktopRect, String> {
        Ok(self.bounds)
    }
}

/// In-memory macro catalog for tests.
#[derive(Debug, Default)]
pub struct MapMacroLookup {
    pub macros: std::collections::BTreeMap<String, Arc<Macro>>,
}

impl MacroLookup for MapMacroLookup {
    fn get(&self, name: &str) -> Option<Arc<Macro>> {
        self.macros.get(name).cloned()
    }
}

/// Test waiter that returns immediately (does not block).
#[derive(Debug, Default)]
pub struct ImmediateContinueWaiter {
    pub log: std::sync::Mutex<Vec<String>>,
    /// Indices returned by successive `wait_for_any_chord` calls (defaults to 0).
    pub any_queue: std::sync::Mutex<Vec<usize>>,
}

impl ContinueKeyWaiter for ImmediateContinueWaiter {
    fn wait_for_continue(
        &self,
        keys: &[String],
        pass_through: bool,
        _stop: &AtomicBool,
    ) -> Result<(), String> {
        if keys.is_empty() {
            return Err("pause: continue key not set".into());
        }
        if let Ok(mut g) = self.log.lock() {
            g.push(format!(
                "continue:{}:passthrough={pass_through}",
                keys.join("+")
            ));
        }
        Ok(())
    }

    fn wait_for_any_chord(
        &self,
        chords: &[Vec<String>],
        hold_repeat: &[bool],
        pass_through: bool,
        _stop: &AtomicBool,
    ) -> Result<usize, String> {
        if chords.is_empty() || chords.iter().all(|c| c.is_empty()) {
            return Err("key wait: no chords configured".into());
        }
        let idx = self
            .any_queue
            .lock()
            .ok()
            .and_then(|mut q| {
                if q.is_empty() {
                    None
                } else {
                    Some(q.remove(0))
                }
            })
            .unwrap_or(0);
        if let Ok(mut g) = self.log.lock() {
            let labels: Vec<String> = chords
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let hold = if hold_repeat.get(i).copied().unwrap_or(false) {
                        "*"
                    } else {
                        ""
                    };
                    format!("{hold}{}", c.join("+"))
                })
                .collect();
            g.push(format!(
                "any:{}:pick={idx}:passthrough={pass_through}",
                labels.join("|")
            ));
        }
        Ok(idx.min(chords.len().saturating_sub(1)))
    }
}

/// Test focuser that records calls.
#[derive(Debug, Default)]
pub struct RecordingWindowFocuser {
    pub log: std::sync::Mutex<Vec<String>>,
}

impl WindowFocuser for RecordingWindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), String> {
        if let Ok(mut g) = self.log.lock() {
            g.push(format!("focus:{process_path}:{window_title}"));
        }
        Ok(())
    }
}
