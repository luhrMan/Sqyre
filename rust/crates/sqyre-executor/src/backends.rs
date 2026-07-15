use image::RgbaImage;
use sqyre_domain::{CoordinateRef, Macro};
use sqyre_match::{ImageBuf, MatchError, Point};
use std::sync::atomic::AtomicBool;

/// Mouse move options (Go `MoveOptions`).
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
    pub fn from_corners(left: i32, top: i32, right: i32, bottom: i32) -> Self {
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

/// Mouse / keyboard / timing / clipboard (Go `AutomationBackend`).
pub trait AutomationBackend {
    fn milli_sleep(&mut self, ms: i32);
    fn move_to(&mut self, x: i32, y: i32, opts: MoveOptions);
    fn click(&mut self, button: &str, down: bool) -> Result<(), String>;
    fn scroll(&mut self, up: bool) -> Result<(), String>;
    fn key_down(&mut self, key: &str) -> Result<(), String>;
    fn key_up(&mut self, key: &str) -> Result<(), String>;
    fn type_char(&mut self, s: &str);
    fn write_clipboard(&mut self, s: &str) -> Result<(), String>;
}

/// Screen capture in absolute virtual-desktop coordinates (Go `capture` package).
pub trait ScreenCapturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, String>;
    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, String>;
    fn virtual_bounds(&mut self) -> Result<DesktopRect, String>;

    /// Capture a search-area rectangle after basic size checks.
    fn capture_search_area(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Result<(RgbaImage, DesktopRect), String> {
        let mut rect = DesktopRect::from_corners(left, top, right, bottom);
        if rect.is_empty() {
            return Err(format!(
                "empty search area {left},{top},{right},{bottom}"
            ));
        }
        // Clamp to virtual bounds when available.
        if let Ok(vb) = self.virtual_bounds() {
            let lx = left.max(vb.x);
            let ty = top.max(vb.y);
            let rx = right.min(vb.x + vb.w);
            let by = bottom.min(vb.y + vb.h);
            rect = DesktopRect::from_corners(lx, ty, rx, by);
            if rect.is_empty() {
                return Err("search area outside virtual desktop".into());
            }
        }
        let img = self.capture_rect(rect)?;
        Ok((img, rect))
    }
}

/// Template matching façade over `sqyre-match`.
pub trait TemplateMatcher {
    fn find_matches(
        &self,
        search: &ImageBuf,
        template: &ImageBuf,
        mask: Option<&ImageBuf>,
        threshold: f32,
        blur: i32,
    ) -> Result<Vec<Point>, MatchError>;
}

/// Resolve `program~point` / search-area refs using the loaded program catalog.
pub trait CoordinateResolver {
    fn resolve_point(&self, r: &CoordinateRef, macro_: &Macro) -> Result<(i32, i32), String>;
    fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), String>;
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

/// Look up another macro by name (Go `repositories.MacroRepo().Get`).
pub trait MacroLookup: Send + Sync {
    fn get(&self, name: &str) -> Option<Macro>;
}

/// Block until the user presses a continue chord (Go `WaitForContinueKey`).
pub trait ContinueKeyWaiter: Send + Sync {
    fn wait_for_continue(
        &self,
        keys: &[String],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<(), String>;
}

/// Bring a window to the front by executable path + title (Go `RunFocusWindow`).
pub trait WindowFocuser: Send + Sync {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), String>;
}

/// OCR recognition result (Go gosseract word boxes + joined text).
#[derive(Debug, Clone, Default)]
pub struct OcrResult {
    pub text: String,
    pub words: Vec<sqyre_vision::OcrWordBox>,
}

/// Run OCR on a preprocessed image buffer (Go `ocrMatWithBoxes`).
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
            g.push(format!("ocr:{}x{}c{}", image.width, image.height, image.channels));
        }
        Ok(self.result.clone())
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
    fn type_char(&mut self, s: &str) {
        self.log.push(format!("type:{s}"));
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
    pub next: Option<RgbaImage>,
    pub bounds: DesktopRect,
}

impl ScreenCapturer for RecordingCapturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, String> {
        self.log.push(format!("monitor:{display_index}"));
        self.next
            .clone()
            .ok_or_else(|| "RecordingCapturer: no image".into())
    }
    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, String> {
        self.log
            .push(format!("rect:{},{},{},{}", rect.x, rect.y, rect.w, rect.h));
        self.next
            .clone()
            .ok_or_else(|| "RecordingCapturer: no image".into())
    }
    fn virtual_bounds(&mut self) -> Result<DesktopRect, String> {
        Ok(self.bounds)
    }
}

/// In-memory macro catalog for tests.
#[derive(Debug, Default)]
pub struct MapMacroLookup {
    pub macros: std::collections::BTreeMap<String, Macro>,
}

impl MacroLookup for MapMacroLookup {
    fn get(&self, name: &str) -> Option<Macro> {
        self.macros.get(name).cloned()
    }
}

/// Test waiter that returns immediately (does not block).
#[derive(Debug, Default)]
pub struct ImmediateContinueWaiter {
    pub log: std::sync::Mutex<Vec<String>>,
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
