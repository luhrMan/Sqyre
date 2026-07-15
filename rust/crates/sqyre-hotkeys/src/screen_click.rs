//! Global mouse-click capture for Data Editor record buttons.
//! Armed by the UI; delivered via the hotkey rdev listener when hooks are enabled.

use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
enum Armed {
    Point,
    SearchArea { first: Option<(i32, i32)> },
}

#[derive(Debug, Default)]
struct Inner {
    armed: Option<Armed>,
    last_pos: (i32, i32),
    /// Completed capture: Point (x,y) or SearchArea (lx,ty,rx,by).
    point: Option<(i32, i32)>,
    search_area: Option<(i32, i32, i32, i32)>,
    cancelled: bool,
}

/// Shared bridge between the hotkey thread and the UI.
#[derive(Clone, Default)]
pub struct ScreenClickBridge {
    inner: Arc<Mutex<Inner>>,
}

impl ScreenClickBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arm_point(&self) {
        let mut g = self.inner.lock();
        *g = Inner {
            armed: Some(Armed::Point),
            last_pos: g.last_pos,
            ..Inner::default()
        };
    }

    pub fn arm_search_area(&self) {
        let mut g = self.inner.lock();
        *g = Inner {
            armed: Some(Armed::SearchArea { first: None }),
            last_pos: g.last_pos,
            ..Inner::default()
        };
    }

    pub fn disarm(&self) {
        let mut g = self.inner.lock();
        g.armed = None;
    }

    pub fn is_armed(&self) -> bool {
        self.inner.lock().armed.is_some()
    }

    pub fn status_label(&self) -> Option<String> {
        match &self.inner.lock().armed {
            Some(Armed::Point) => Some("Recording point — left-click to capture, Esc to cancel".into()),
            Some(Armed::SearchArea { first: None }) => {
                Some("Recording search area — click first corner, Esc to cancel".into())
            }
            Some(Armed::SearchArea { first: Some(_) }) => {
                Some("Recording search area — click opposite corner, Esc to cancel".into())
            }
            None => None,
        }
    }

    /// Hotkey thread: track pointer.
    pub fn on_mouse_move(&self, x: i32, y: i32) {
        self.inner.lock().last_pos = (x, y);
    }

    /// Hotkey thread: left button press while armed.
    pub fn on_left_click(&self) {
        let mut g = self.inner.lock();
        let pos = g.last_pos;
        match g.armed.clone() {
            Some(Armed::Point) => {
                g.point = Some(pos);
                g.armed = None;
            }
            Some(Armed::SearchArea { first: None }) => {
                g.armed = Some(Armed::SearchArea { first: Some(pos) });
            }
            Some(Armed::SearchArea {
                first: Some((lx, ty)),
            }) => {
                let (rx, by) = pos;
                let (lx, rx) = if lx <= rx { (lx, rx) } else { (rx, lx) };
                let (ty, by) = if ty <= by { (ty, by) } else { (by, ty) };
                g.search_area = Some((lx, ty, rx, by));
                g.armed = None;
            }
            None => {}
        }
    }

    /// Hotkey thread: Esc while armed cancels.
    pub fn on_escape(&self) -> bool {
        let mut g = self.inner.lock();
        if g.armed.is_some() {
            g.armed = None;
            g.cancelled = true;
            true
        } else {
            false
        }
    }

    pub fn take_point(&self) -> Option<(i32, i32)> {
        self.inner.lock().point.take()
    }

    pub fn take_search_area(&self) -> Option<(i32, i32, i32, i32)> {
        self.inner.lock().search_area.take()
    }

    pub fn take_cancelled(&self) -> bool {
        let mut g = self.inner.lock();
        let c = g.cancelled;
        g.cancelled = false;
        c
    }

    /// Fallback when hooks are disabled: capture current last_pos / injected pos.
    pub fn capture_point_now(&self, x: i32, y: i32) {
        let mut g = self.inner.lock();
        g.point = Some((x, y));
        g.armed = None;
    }
}
