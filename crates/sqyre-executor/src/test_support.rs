//! Shared test doubles for executor unit/integration tests.

use crate::backends::{
    AutomationBackend, AutomationError, ContinueKeyWaiter, CoordinateResolver, DesktopRect,
    MacroLookup, MoveOptions, OcrEngine, PortError, ScreenCapturer, WindowFocuser,
};
use image::RgbaImage;
use sqyre_domain::{CoordinateRef, Macro};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Collection grid metadata for [`FixedResolver`] atlas tests.
#[derive(Debug, Clone, Copy)]
pub struct FixedCollection {
    pub rows: i32,
    pub cols: i32,
    pub bounds: (i32, i32, i32, i32),
}

/// One atlas member passed to [`FixedResolver::with_atlas`].
#[derive(Debug, Clone)]
pub struct AtlasMemberSpec {
    pub name: String,
    pub collection: FixedCollection,
}

/// Fixed point + search-area resolver; optional collection grid / atlas.
#[derive(Debug, Clone)]
pub struct FixedResolver {
    pub point: (i32, i32),
    pub area: (i32, i32, i32, i32),
    pub grid: Option<(i32, i32)>,
    /// Collection name → grid; `None` for const/simple resolvers.
    pub collections: Option<HashMap<String, FixedCollection>>,
    pub atlas_members: Option<Vec<String>>,
}

impl FixedResolver {
    pub const fn point_area(point: (i32, i32), area: (i32, i32, i32, i32)) -> Self {
        Self {
            point,
            area,
            grid: None,
            collections: None,
            atlas_members: None,
        }
    }

    #[allow(dead_code)]
    pub const fn with_grid(rows: i32, cols: i32) -> Self {
        Self {
            point: (0, 0),
            area: (0, 0, 100, 100),
            grid: Some((rows, cols)),
            collections: None,
            atlas_members: None,
        }
    }

    pub fn with_atlas(collections: Vec<AtlasMemberSpec>, members: Vec<String>) -> Self {
        let mut map = HashMap::new();
        for spec in collections {
            map.insert(spec.name, spec.collection);
        }
        Self {
            point: (0, 0),
            area: (0, 0, 100, 100),
            grid: None,
            collections: Some(map),
            atlas_members: Some(members),
        }
    }
}

/// Default used by most search tests: point (0,0), area (100,200)-(110,210).
pub const SEARCH_FIXED_AREA: FixedResolver =
    FixedResolver::point_area((0, 0), (100, 200, 110, 210));

impl CoordinateResolver for FixedResolver {
    fn resolve_point(&self, r: &CoordinateRef, _macro_: &Macro) -> Result<(i32, i32), PortError> {
        if let Some((r1, c1, _, _)) = r.cell_range() {
            if let Some(cols) = &self.collections {
                if let Some(FixedCollection {
                    rows,
                    cols: cols_n,
                    bounds: (lx, ty, rx, by),
                }) = cols.get(r.name())
                {
                    let width = rx - lx;
                    let height = by - ty;
                    let cx = lx + (c1 - 1) * width / cols_n + width / (cols_n * 2);
                    let cy = ty + (r1 - 1) * height / rows + height / (rows * 2);
                    return Ok((cx, cy));
                }
            }
            if self.grid.is_some() {
                return Ok((c1 * 10, r1 * 10));
            }
        }
        Ok(self.point)
    }

    fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        _macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), PortError> {
        if r.cell_range().is_some() {
            if let Some(cols) = &self.collections {
                if let Some(FixedCollection { bounds, .. }) = cols.get(r.name()) {
                    return Ok(*bounds);
                }
            }
        }
        Ok(self.area)
    }

    fn collection_grid(&self, _program: &str, collection: &str) -> Result<(i32, i32), PortError> {
        if let Some(cols) = &self.collections {
            if let Some(FixedCollection { rows, cols, .. }) = cols.get(collection) {
                return Ok((*rows, *cols));
            }
        }
        self.grid
            .ok_or_else(|| PortError::not_configured("collection grid lookup"))
    }

    fn atlas_members(&self, _program: &str, _atlas: &str) -> Result<Vec<String>, PortError> {
        self.atlas_members
            .clone()
            .ok_or_else(|| PortError::not_configured("atlas lookup"))
    }
}

/// Test OCR engine that returns a fixed result.
#[derive(Debug, Default)]
pub struct FixedOcrEngine {
    pub result: sqyre_vision::OcrRecognition,
    pub log: std::sync::Mutex<Vec<String>>,
}

impl OcrEngine for FixedOcrEngine {
    fn recognize(
        &self,
        image: &sqyre_match::ImageBuf,
    ) -> Result<sqyre_vision::OcrRecognition, PortError> {
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
    pub queue: std::sync::Mutex<Vec<sqyre_vision::OcrRecognition>>,
    pub log: std::sync::Mutex<Vec<String>>,
}

impl OcrEngine for QueuedOcrEngine {
    fn recognize(
        &self,
        image: &sqyre_match::ImageBuf,
    ) -> Result<sqyre_vision::OcrRecognition, PortError> {
        if let Ok(mut g) = self.log.lock() {
            g.push(format!(
                "ocr:{}x{}c{}",
                image.width, image.height, image.channels
            ));
        }
        let mut q = self
            .queue
            .lock()
            .map_err(|_| PortError::Message("QueuedOcrEngine: lock poisoned".into()))?;
        if q.is_empty() {
            return Err(PortError::invalid("QueuedOcrEngine: empty queue"));
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
    ) -> Result<(), PortError> {
        if keys.is_empty() {
            return Err(PortError::invalid("pause: continue key not set"));
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
    ) -> Result<usize, PortError> {
        if chords.is_empty() || chords.iter().all(|c| c.is_empty()) {
            return Err(PortError::invalid("key wait: no chords configured"));
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
