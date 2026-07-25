//! Domain-coupled executor ports and test doubles.
//!
//! OS-facing traits live in [`sqyre_ports`] and are re-exported here so executor
//! consumers keep a single import path.

use image::RgbaImage;
use sqyre_domain::{CoordinateRef, Macro};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub use sqyre_ports::{
    clamp_search_rect, AutomationBackend, AutomationError, CaptureError, DesktopRect, MoveOptions,
    RgbCapture, ScreenCapturer, WindowFocuser,
};

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

    /// Member Collection names for `program` + atlas name.
    fn atlas_members(&self, program: &str, atlas: &str) -> Result<Vec<String>, String> {
        let _ = (program, atlas);
        Err("atlas lookup not configured".into())
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
    fn click(&mut self, button: &str, down: bool) -> Result<(), AutomationError> {
        self.log.push(format!(
            "click:{button}:{}",
            if down { "down" } else { "up" }
        ));
        Ok(())
    }
    fn scroll(&mut self, up: bool) -> Result<(), AutomationError> {
        self.log
            .push(format!("scroll:{}", if up { "up" } else { "down" }));
        Ok(())
    }
    fn key_down(&mut self, key: &str) -> Result<(), AutomationError> {
        self.log.push(format!("keydown:{key}"));
        Ok(())
    }
    fn key_up(&mut self, key: &str) -> Result<(), AutomationError> {
        self.log.push(format!("keyup:{key}"));
        Ok(())
    }
    fn type_char(&mut self, ch: char) {
        self.log.push(format!("type:{ch}"));
    }
    fn write_clipboard(&mut self, s: &str) -> Result<(), AutomationError> {
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
    fn take_image(&mut self) -> Result<RgbaImage, crate::CaptureError> {
        if !self.queue.is_empty() {
            return Ok(self.queue.remove(0));
        }
        self.next
            .clone()
            .ok_or_else(|| crate::CaptureError::Message("RecordingCapturer: no image".into()))
    }
}

impl ScreenCapturer for RecordingCapturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, crate::CaptureError> {
        self.log.push(format!("monitor:{display_index}"));
        self.take_image()
    }
    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, crate::CaptureError> {
        self.log
            .push(format!("rect:{},{},{},{}", rect.x, rect.y, rect.w, rect.h));
        self.take_image()
    }
    fn virtual_bounds(&mut self) -> Result<DesktopRect, crate::CaptureError> {
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
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), AutomationError> {
        if let Ok(mut g) = self.log.lock() {
            g.push(format!("focus:{process_path}:{window_title}"));
        }
        Ok(())
    }
}
