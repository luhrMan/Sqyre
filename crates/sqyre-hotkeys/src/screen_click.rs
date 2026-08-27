//! Global mouse-click capture for Data Editor record buttons.
//! Armed by the UI; delivered via the hotkey rdev listener when hooks are enabled.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type AbsolutePosFn = Arc<dyn Fn() -> Option<(i32, i32)> + Send + Sync>;

/// Fast path for OS hooks: skip mouse-move work unless a recording is armed.
/// Windows uses this to avoid flooding WH_MOUSE_LL; Linux evdev grab uses it so
/// pointer motion is not serialized on bridge mutexes (that stalls the cursor).
static HOOK_WANTS_MOVES: AtomicBool = AtomicBool::new(false);

pub(crate) fn hook_wants_mouse_moves() -> bool {
    HOOK_WANTS_MOVES.load(Ordering::Relaxed)
}

fn sync_hook_wants_moves(armed: bool) {
    HOOK_WANTS_MOVES.store(armed, Ordering::Relaxed);
}

#[derive(Debug, Clone)]
enum Armed {
    Point,
    /// Single click → screen coords for Find Pixel color sampling.
    Color,
    SearchArea {
        first: Option<(i32, i32)>,
    },
}

#[derive(Default)]
struct Inner {
    armed: Option<Armed>,
    last_pos: (i32, i32),
    /// Completed capture: Point (x,y) or SearchArea (lx,ty,rx,by).
    point: Option<(i32, i32)>,
    /// Completed color-sample click (sampled by UI via 1×1 capture).
    color_point: Option<(i32, i32)>,
    search_area: Option<(i32, i32, i32, i32)>,
    cancelled: bool,
    /// When true, the fullscreen [`SelectionGrab`] owns mouse/Esc — hotkey hooks
    /// must not also deliver those events (would double-count clicks).
    grab_owns_input: bool,
    /// When false, hooks still deliver left-clicks even if [`Self::grab_owns_input`].
    block_hook_clicks: bool,
    /// Compositor-absolute pointer (portal cursor). Used on every click so the
    /// first corner is Wayland-accurate even over XWayland windows.
    absolute_pos: Option<AbsolutePosFn>,
}

fn normalize_rect(ax: i32, ay: i32, bx: i32, by: i32) -> (i32, i32, i32, i32) {
    // Keep local: hotkeys cannot depend on sqyre-executor (cycle).
    let (lx, rx) = if ax <= bx { (ax, bx) } else { (bx, ax) };
    let (ty, by) = if ay <= by { (ay, by) } else { (by, ay) };
    (lx, ty, rx, by)
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
        self.arm_with(Armed::Point);
    }

    pub fn arm_color(&self) {
        self.arm_with(Armed::Color);
    }

    pub fn arm_search_area(&self) {
        self.arm_with(Armed::SearchArea { first: None });
    }

    fn arm_with(&self, armed: Armed) {
        let mut g = self.inner.lock();
        let last_pos = g.last_pos;
        let absolute_pos = g.absolute_pos.clone();
        *g = Inner {
            armed: Some(armed),
            last_pos,
            absolute_pos,
            ..Inner::default()
        };
        sync_hook_wants_moves(true);
    }

    /// Compositor-absolute pointer sampled on each click (portal cursor on Wayland).
    pub fn set_absolute_pos(&self, f: impl Fn() -> Option<(i32, i32)> + Send + Sync + 'static) {
        self.inner.lock().absolute_pos = Some(Arc::new(f));
    }

    pub fn disarm(&self) {
        let mut g = self.inner.lock();
        g.armed = None;
        sync_hook_wants_moves(false);
    }

    /// When the fullscreen selection grab is active, hooks skip mouse/Esc delivery.
    pub fn set_grab_owns_input(&self, owns: bool) {
        let mut g = self.inner.lock();
        g.grab_owns_input = owns;
        g.block_hook_clicks = owns;
    }

    /// Keep hook clicks while blocking absolute hook moves (Wayland XQueryPointer).
    pub fn allow_hook_clicks(&self) {
        self.inner.lock().block_hook_clicks = false;
    }

    pub fn grab_owns_input(&self) -> bool {
        self.inner.lock().grab_owns_input
    }

    pub fn block_hook_clicks(&self) -> bool {
        self.inner.lock().block_hook_clicks
    }

    pub fn is_armed(&self) -> bool {
        self.inner.lock().armed.is_some()
    }

    /// Last known pointer position (updated by the hotkey thread).
    pub fn last_pos(&self) -> (i32, i32) {
        self.inner.lock().last_pos
    }

    pub fn status_label(&self) -> Option<String> {
        let g = self.inner.lock();
        let (x, y) = g.last_pos;
        match &g.armed {
            Some(Armed::Point) => Some(format!(
                "Recording point — ({x}, {y}) — left-click to capture, Esc to cancel"
            )),
            Some(Armed::Color) => Some(format!(
                "Recording color — ({x}, {y}) — left-click to sample, Esc to cancel"
            )),
            Some(Armed::SearchArea { first: None }) => Some(format!(
                "Recording search area — click first corner ({x}, {y}), Esc to cancel"
            )),
            Some(Armed::SearchArea {
                first: Some((lx, ty)),
            }) => {
                let (l, t, r, b) = normalize_rect(*lx, *ty, x, y);
                Some(format!(
                    "Recording search area — ({l},{t})–({r},{b}) — click opposite corner, Esc to cancel"
                ))
            }
            None => None,
        }
    }

    /// Live cursor while a point recording is armed.
    pub fn peek_point_draft(&self) -> Option<(i32, i32)> {
        let g = self.inner.lock();
        match g.armed {
            Some(Armed::Point) => Some(g.last_pos),
            _ => None,
        }
    }

    /// Live search-area corners while armed.
    ///
    /// Before the first click this is a degenerate rect at the cursor so the form
    /// can show the pending corner. After the first click it spans first→cursor.
    pub fn peek_search_area_draft(&self) -> Option<(i32, i32, i32, i32)> {
        let g = self.inner.lock();
        let (x, y) = g.last_pos;
        match &g.armed {
            Some(Armed::SearchArea { first: None }) => Some((x, y, x, y)),
            Some(Armed::SearchArea {
                first: Some((lx, ty)),
            }) => Some(normalize_rect(*lx, *ty, x, y)),
            _ => None,
        }
    }

    /// Selection rectangle for the recording overlay.
    ///
    /// Only after the first corner click — before that the rect is cleared so nothing
    /// is drawn while waiting for the first corner.
    pub fn peek_search_area_selection(&self) -> Option<(i32, i32, i32, i32)> {
        let g = self.inner.lock();
        let (x, y) = g.last_pos;
        match &g.armed {
            Some(Armed::SearchArea {
                first: Some((lx, ty)),
            }) => Some(normalize_rect(*lx, *ty, x, y)),
            _ => None,
        }
    }

    /// Hotkey thread: track pointer.
    pub fn on_mouse_move(&self, x: i32, y: i32) {
        self.inner.lock().last_pos = (x, y);
    }

    /// Hotkey thread: left button press while armed.
    pub fn on_left_click(&self) {
        let sample = self.inner.lock().absolute_pos.clone();
        let sampled = sample.as_ref().and_then(|f| f());
        let mut g = self.inner.lock();
        if let Some(pos) = sampled {
            g.last_pos = pos;
        }
        apply_left_click(&mut g);
    }

    /// Overlay/grab click at a known desktop point (do not sample portal cursor).
    pub fn on_left_click_at(&self, x: i32, y: i32) {
        let mut g = self.inner.lock();
        g.last_pos = (x, y);
        apply_left_click(&mut g);
    }

    /// Hotkey thread: Esc while armed cancels.
    pub fn on_escape(&self) -> bool {
        let mut g = self.inner.lock();
        if g.armed.is_some() {
            g.armed = None;
            g.cancelled = true;
            sync_hook_wants_moves(false);
            true
        } else {
            false
        }
    }

    pub fn take_point(&self) -> Option<(i32, i32)> {
        self.inner.lock().point.take()
    }

    pub fn take_color_point(&self) -> Option<(i32, i32)> {
        self.inner.lock().color_point.take()
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
        sync_hook_wants_moves(false);
    }
}

fn apply_left_click(g: &mut Inner) {
    let pos = g.last_pos;
    match g.armed.clone() {
        Some(Armed::Point) => {
            g.point = Some(pos);
            g.armed = None;
        }
        Some(Armed::Color) => {
            g.color_point = Some(pos);
            g.armed = None;
        }
        Some(Armed::SearchArea { first: None }) => {
            g.armed = Some(Armed::SearchArea { first: Some(pos) });
        }
        Some(Armed::SearchArea {
            first: Some((lx, ty)),
        }) => {
            g.search_area = Some(normalize_rect(lx, ty, pos.0, pos.1));
            g.armed = None;
        }
        None => {}
    }
    sync_hook_wants_moves(g.armed.is_some());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_label_includes_live_point_coords() {
        let b = ScreenClickBridge::new();
        b.arm_point();
        b.on_mouse_move(12, 34);
        let msg = b.status_label().expect("armed");
        assert!(msg.contains("(12, 34)"), "{msg}");
    }

    #[test]
    fn status_label_includes_search_area_rect_while_selecting() {
        let b = ScreenClickBridge::new();
        b.arm_search_area();
        b.on_mouse_move(10, 20);
        let first = b.status_label().expect("armed");
        assert!(first.contains("(10, 20)"), "{first}");

        b.on_left_click();
        b.on_mouse_move(5, 40);
        let second = b.status_label().expect("armed");
        // Normalized: (5,20)–(10,40)
        assert!(second.contains("(5,20)–(10,40)"), "{second}");
    }

    #[test]
    fn peek_search_area_draft_tracks_cursor() {
        let b = ScreenClickBridge::new();
        b.arm_search_area();
        b.on_mouse_move(100, 200);
        assert_eq!(b.peek_search_area_draft(), Some((100, 200, 100, 200)));
        assert!(b.peek_search_area_selection().is_none());

        b.on_left_click();
        b.on_mouse_move(50, 250);
        assert_eq!(b.peek_search_area_draft(), Some((50, 200, 100, 250)));
        assert_eq!(b.peek_search_area_selection(), Some((50, 200, 100, 250)));
    }

    #[test]
    fn completed_search_area_clears_draft() {
        let b = ScreenClickBridge::new();
        b.arm_search_area();
        b.on_mouse_move(0, 0);
        b.on_left_click();
        b.on_mouse_move(30, 40);
        b.on_left_click();
        assert_eq!(b.take_search_area(), Some((0, 0, 30, 40)));
        assert!(b.peek_search_area_draft().is_none());
        assert!(b.status_label().is_none());
    }

    #[test]
    fn color_click_does_not_fill_point() {
        let b = ScreenClickBridge::new();
        b.arm_color();
        b.on_mouse_move(7, 9);
        let msg = b.status_label().expect("armed");
        assert!(msg.contains("Recording color"), "{msg}");
        assert!(msg.contains("(7, 9)"), "{msg}");
        b.on_left_click();
        assert_eq!(b.take_color_point(), Some((7, 9)));
        assert!(b.take_point().is_none());
        assert!(b.status_label().is_none());
    }

    #[test]
    fn grab_owns_input_flag_roundtrip() {
        let b = ScreenClickBridge::new();
        assert!(!b.grab_owns_input());
        b.set_grab_owns_input(true);
        assert!(b.grab_owns_input());
        b.arm_point();
        // Arming resets bridge state including the grab flag.
        assert!(!b.grab_owns_input());
        b.set_grab_owns_input(true);
        assert!(b.grab_owns_input());
        b.set_grab_owns_input(false);
        assert!(!b.grab_owns_input());
    }

    #[test]
    fn allow_hook_clicks_keeps_grab_flag() {
        let b = ScreenClickBridge::new();
        b.arm_search_area();
        b.set_grab_owns_input(true);
        assert!(b.block_hook_clicks());
        b.allow_hook_clicks();
        assert!(!b.block_hook_clicks());
        assert!(b.grab_owns_input());
    }

    #[test]
    fn left_click_uses_absolute_pos_even_after_arm() {
        let b = ScreenClickBridge::new();
        b.set_absolute_pos(|| Some((3212, 528)));
        b.on_mouse_move(10, 10);
        b.arm_search_area();
        b.on_left_click();
        assert_eq!(b.peek_search_area_draft(), Some((3212, 528, 3212, 528)));
    }

    #[test]
    fn left_click_at_ignores_absolute_pos() {
        let b = ScreenClickBridge::new();
        b.set_absolute_pos(|| Some((3212, 528)));
        b.arm_search_area();
        b.on_left_click_at(40, 50);
        assert_eq!(b.peek_search_area_draft(), Some((40, 50, 40, 50)));
    }
}
