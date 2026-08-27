//! Live search-area outline via direct X11 windows (no desktop snapshot).
//!
//! Positions override-redirect X11 windows with `ConfigureWindow` and paints a
//! stroked rectangle selection layer. We skip the snapshot
//! background and draw only that rectangle as four thin override-redirect edge
//! windows so the desktop stays visible underneath.

use std::os::raw::{c_int, c_uint, c_ulong};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use x11::xfixes::{
    XFixesCreateRegion, XFixesDestroyRegion, XFixesQueryExtension, XFixesQueryVersion,
    XFixesSetWindowShapeRegion,
};
use x11::xlib::{
    Button1Mask, CWBackPixel, CWBorderPixel, CWHeight, CWOverrideRedirect, CWWidth, Display,
    InputOutput, True, Window, XAllocColor, XCloseDisplay, XColor, XConfigureWindow, XCreateWindow,
    XDefaultColormap, XDefaultDepth, XDefaultRootWindow, XDefaultScreen, XDefaultVisual,
    XDestroyWindow, XEvent, XFlush, XMapRaised, XMapWindow, XNextEvent, XOpenDisplay, XPending,
    XQueryPointer, XRaiseWindow, XSetWindowAttributes, XWindowChanges, _XDisplay, CWX, CWY,
};

use crate::outline_geometry::{
    edge_placements, outline_should_clear, STROKE_B, STROKE_G, STROKE_R,
};
pub use crate::outline_rect::OutlineRect;
use crate::{cap_log, event_log, CaptureError};

/// `ShapeInput` from `X11/extensions/shapeconst.h` — empty region = click-through.
const SHAPE_INPUT: c_int = 2;

static LOGGED_SLOW_SET_RECT: AtomicBool = AtomicBool::new(false);

/// Four edge windows forming a hollow rectangle on the virtual desktop.
pub struct SelectionOutline {
    display: *mut _XDisplay,
    /// Quiet connection used only for `XQueryPointer` / layout. ConfigureWindow on
    /// `display` must not share this round-trip (that froze the rubber-band).
    ptr_display: *mut _XDisplay,
    edges: [Window; 4],
    mapped: bool,
    last: Option<OutlineRect>,
    last_edges: Option<[(i32, i32, i32, i32); 4]>,
    /// Empty `ShapeInput` so the cursor is not inside a window we resize every move.
    input_passthrough: bool,
    cached_rects: Vec<sqyre_ports::DesktopRect>,
}

// SAFETY: the raw display pointer is owned exclusively by this struct — only
// `&mut self` methods and `Drop` touch it, so it is never used from two threads
// at once even after the value is moved to another thread.
unsafe impl Send for SelectionOutline {}

impl SelectionOutline {
    pub fn open() -> Result<Self, CaptureError> {
        // SAFETY: `XOpenDisplay(null)` connects to the default display and its
        // result is null-checked before any other Xlib call; every early return
        // destroys the windows created so far and closes the connection(s).
        unsafe {
            let display = open_x_display()?;
            let ptr_display = match open_x_display() {
                Ok(d) => d,
                Err(_) => ptr::null_mut(),
            };
            let screen = XDefaultScreen(display);
            let root = XDefaultRootWindow(display);
            // Do not XQueryPointer here. Over a fullscreen XWayland game that
            // round-trip stalls open() for seconds; recording uses portal cursor
            // metadata instead.
            let cached_root = (
                x11::xlib::XDisplayWidth(display, screen),
                x11::xlib::XDisplayHeight(display, screen),
            );
            let cached_rects = crate::x11_capture::xinerama_monitor_rects_on(
                display,
                sqyre_ports::DesktopRect {
                    x: 0,
                    y: 0,
                    w: cached_root.0,
                    h: cached_root.1,
                },
            );
            let pixel = match alloc_stroke_pixel(display, screen) {
                Ok(p) => p,
                Err(e) => {
                    close_x_display(ptr_display);
                    close_x_display(display);
                    return Err(CaptureError::Message(e));
                }
            };
            let mut edges = [0 as Window; 4];
            for edge in &mut edges {
                match create_edge(display, root, screen, pixel) {
                    Ok(w) => *edge = w,
                    Err(e) => {
                        for &w in edges.iter() {
                            if w != 0 {
                                XDestroyWindow(display, w);
                            }
                        }
                        close_x_display(ptr_display);
                        close_x_display(display);
                        return Err(CaptureError::Message(e));
                    }
                }
            }
            let input_passthrough = apply_input_passthrough(display, &edges);
            // Map 1×1 off-origin so the first rubber-band `set_rect` only
            // ConfigureWindow. XMapRaised over a fullscreen XWayland game is
            // what delayed the gold box by seconds.
            for &w in &edges {
                configure(display, w, 0, 0, 1, 1);
                XMapWindow(display, w);
            }
            XFlush(display);
            drain_x_events(display);
            event_log(
                "SQYRE_OUTLINE",
                &[
                    (
                        "input",
                        if input_passthrough {
                            "passthrough"
                        } else {
                            "opaque"
                        },
                    ),
                    ("ptr", "portal"),
                    (
                        "ptr_conn",
                        if ptr_display.is_null() {
                            "shared"
                        } else {
                            "separate"
                        },
                    ),
                    ("root", &format!("{}x{}", cached_root.0, cached_root.1)),
                    ("x11_outputs", &cached_rects.len().to_string()),
                ],
            );
            Ok(Self {
                display,
                ptr_display,
                edges,
                mapped: true,
                last: None,
                last_edges: None,
                input_passthrough,
                cached_rects,
            })
        }
    }

    /// Show/update the outline for absolute desktop corners.
    pub fn set_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        let rect = OutlineRect::normalize(left, top, right, bottom);
        if outline_should_clear(rect) {
            self.clear();
            return;
        }
        if self.last == Some(rect) && self.mapped {
            return;
        }
        let placements = edge_placements(rect);
        let coming_from_clear = self.last.is_none();
        if coming_from_clear {
            event_log(
                "SQYRE_OUTLINE",
                &[
                    ("op", "show"),
                    (
                        "rect",
                        &format!("{},{}-{},{}", rect.left, rect.top, rect.right, rect.bottom),
                    ),
                    (
                        "top",
                        &format!(
                            "{},{},{},{}",
                            placements[0].0, placements[0].1, placements[0].2, placements[0].3
                        ),
                    ),
                    (
                        "sides",
                        &format!("y={} h={}", placements[2].1, placements[2].3),
                    ),
                ],
            );
        }
        let start = Instant::now();
        // SAFETY: `self.display` is a live connection (non-null since `open`
        // succeeded) and `self.edges` were created on it.
        unsafe {
            let prev = if self.mapped { self.last_edges } else { None };
            for (i, (&win, &place)) in self.edges.iter().zip(placements.iter()).enumerate() {
                if prev.is_some_and(|p| p[i] == place) {
                    continue;
                }
                configure(self.display, win, place.0, place.1, place.2, place.3);
            }
            if !self.mapped {
                for &w in &self.edges {
                    XMapRaised(self.display, w);
                }
            } else if coming_from_clear {
                // Parked 1×1 windows sit at the bottom of the stack. Raise once when
                // showing a tooltip/static rect — not on every rubber-band mouse-move.
                for &w in &self.edges {
                    XRaiseWindow(self.display, w);
                }
            }
            XFlush(self.display);
            drain_x_events(self.display);
        }
        let ms = start.elapsed().as_millis();
        if ms >= 16
            && LOGGED_SLOW_SET_RECT
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            cap_log("OUTLINE", "slow", &format!("set_rect_ms={ms}"));
        }
        self.mapped = true;
        self.last = Some(rect);
        self.last_edges = Some(placements);
    }

    /// True when the edge windows have an empty input shape (cursor is not inside them).
    pub fn input_passthrough(&self) -> bool {
        self.input_passthrough
    }

    /// True when pointer samples use a Display that never sees `ConfigureWindow`.
    pub fn has_separate_pointer_conn(&self) -> bool {
        !self.ptr_display.is_null()
    }

    /// Root-window pointer in the same coordinate space as [`Self::set_rect`].
    ///
    /// Uses [`Self::ptr_display`] when available so this round-trip does not wait
    /// for pending edge `ConfigureWindow` requests.
    pub fn query_pointer(&self) -> Option<(i32, i32, bool)> {
        let dpy = if !self.ptr_display.is_null() {
            self.ptr_display
        } else {
            self.display
        };
        if dpy.is_null() {
            return None;
        }
        // SAFETY: `dpy` is a live connection from `open`.
        unsafe { query_pointer_on(dpy) }
    }

    /// X11 root size for this outline connection (for diag).
    pub fn root_size(&self) -> Option<(i32, i32)> {
        if self.display.is_null() {
            return None;
        }
        // SAFETY: live display from `open`.
        unsafe {
            let screen = XDefaultScreen(self.display);
            Some((
                x11::xlib::XDisplayWidth(self.display, screen),
                x11::xlib::XDisplayHeight(self.display, screen),
            ))
        }
    }

    /// Xinerama outputs on this connection (may span multiple monitors).
    pub fn virtual_rects(&self) -> Vec<sqyre_ports::DesktopRect> {
        if !self.cached_rects.is_empty() {
            return self.cached_rects.clone();
        }
        let fallback = self
            .root_size()
            .map(|(w, h)| sqyre_ports::DesktopRect { x: 0, y: 0, w, h })
            .unwrap_or_default();
        crate::x11_capture::xinerama_monitor_rects_on(self.display, fallback)
    }

    pub fn clear(&mut self) {
        // Already parked (1×1): do not re-Configure/Flush every UI frame — that
        // X11 round-trip hitch under a fullscreen XWayland game makes overlay
        // buttons feel frozen whenever the game holds focus.
        if self.last.is_none() {
            return;
        }
        // Park 1×1 instead of unmapping so the next rubber-band does not
        // XMapRaised over a fullscreen XWayland game.
        // SAFETY: `self.display` is a live connection (non-null since `open`
        // succeeded) and `self.edges` were created on it.
        unsafe {
            for &w in &self.edges {
                configure(self.display, w, 0, 0, 1, 1);
            }
            XFlush(self.display);
            drain_x_events(self.display);
        }
        self.mapped = true;
        self.last = None;
        self.last_edges = None;
    }

    /// True while a non-parked rectangle is shown.
    pub fn is_active(&self) -> bool {
        self.last.is_some()
    }
}

impl Drop for SelectionOutline {
    fn drop(&mut self) {
        crate::mark_site("outline:drop:start");
        let t0 = std::time::Instant::now();
        // SAFETY: edges were created on `self.display`. We destroy windows but do
        // **not** XFlush / XCloseDisplay — both block for seconds under a busy
        // fullscreen XWayland client (Proton games). The kernel reclaims the
        // connection fds when the process exits; mid-session we leak at most two
        // Display connections per outline lifetime.
        unsafe {
            if !self.display.is_null() {
                for &w in &self.edges {
                    if w != 0 {
                        XDestroyWindow(self.display, w);
                    }
                }
                crate::x11_secondary::unregister(self.display);
                self.display = ptr::null_mut();
            }
            if !self.ptr_display.is_null() {
                crate::x11_secondary::unregister(self.ptr_display);
                self.ptr_display = ptr::null_mut();
            }
        }
        crate::cap_log(
            "OUTLINE",
            "drop",
            &format!("ms={}", t0.elapsed().as_millis()),
        );
        crate::mark_site("outline:drop:done");
    }
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// screen index valid on it; `color` is a stack local that outlives `XAllocColor`.
unsafe fn alloc_stroke_pixel(display: *mut Display, screen: c_int) -> Result<c_ulong, String> {
    let mut color = XColor {
        pixel: 0,
        red: u16::from(STROKE_R) << 8,
        green: u16::from(STROKE_G) << 8,
        blue: u16::from(STROKE_B) << 8,
        flags: 0,
        pad: 0,
    };
    let cmap = XDefaultColormap(display, screen);
    if XAllocColor(display, cmap, &mut color) == 0 {
        return Err("XAllocColor failed for selection outline".into());
    }
    Ok(color.pixel)
}

fn open_x_display() -> Result<*mut Display, CaptureError> {
    // SAFETY: null-checked before return; registered so winit's error hook skips it.
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err(CaptureError::Message(
                "XOpenDisplay failed (need X11)".into(),
            ));
        }
        crate::x11_secondary::register(display);
        Ok(display)
    }
}

fn close_x_display(display: *mut Display) {
    if display.is_null() {
        return;
    }
    // SAFETY: `display` was opened and registered by `open_x_display`.
    unsafe {
        crate::x11_secondary::unregister(display);
        XCloseDisplay(display);
    }
}

/// Root pointer on `display`. Round-trip — do not call while `ConfigureWindow` is pending
/// on this same connection.
unsafe fn query_pointer_on(display: *mut Display) -> Option<(i32, i32, bool)> {
    let root = XDefaultRootWindow(display);
    let mut root_ret: Window = 0;
    let mut child: Window = 0;
    let mut root_x = 0;
    let mut root_y = 0;
    let mut win_x = 0;
    let mut win_y = 0;
    let mut mask = 0u32;
    if XQueryPointer(
        display,
        root,
        &mut root_ret,
        &mut child,
        &mut root_x,
        &mut root_y,
        &mut win_x,
        &mut win_y,
        &mut mask,
    ) == 0
    {
        return None;
    }
    Some((root_x, root_y, mask & Button1Mask != 0))
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection plus a
// `root`/`screen`/`pixel` valid on it; `attrs` is zeroed before the fields named
// by `mask` are set, and it outlives the `XCreateWindow` call that reads it.
unsafe fn create_edge(
    display: *mut Display,
    root: Window,
    screen: c_int,
    pixel: c_ulong,
) -> Result<Window, String> {
    let mut attrs: XSetWindowAttributes = std::mem::zeroed();
    attrs.background_pixel = pixel;
    attrs.border_pixel = pixel;
    attrs.override_redirect = True;
    let mask = CWBackPixel | CWBorderPixel | CWOverrideRedirect;
    let win = XCreateWindow(
        display,
        root,
        0,
        0,
        1,
        1,
        0,
        XDefaultDepth(display, screen),
        InputOutput as c_uint,
        XDefaultVisual(display, screen),
        mask,
        &mut attrs,
    );
    if win == 0 {
        return Err("XCreateWindow failed for selection edge".into());
    }
    Ok(win)
}

/// Empty `ShapeInput` so XWayland/Mutter does not treat the cursor as inside
/// an override-redirect window we resize every mouse-move (that freezes tracking).
/// Windows already uses `WS_EX_TRANSPARENT` / `HTTRANSPARENT` for the same reason.
fn apply_input_passthrough(display: *mut Display, edges: &[Window; 4]) -> bool {
    // SAFETY: `display` is the live connection from `open`; regions are created
    // and destroyed on it before return.
    unsafe {
        let mut event_base = 0;
        let mut error_base = 0;
        if XFixesQueryExtension(display, &mut event_base, &mut error_base) == 0 {
            return false;
        }
        let mut major: c_int = 4;
        let mut minor: c_int = 0;
        if XFixesQueryVersion(
            display,
            &mut major,
            &mut minor as *mut c_int as *const c_int,
        ) == 0
        {
            return false;
        }
        for &win in edges {
            let region = XFixesCreateRegion(display, ptr::null_mut(), 0);
            if region == 0 {
                return false;
            }
            XFixesSetWindowShapeRegion(display, win, SHAPE_INPUT, 0, 0, region);
            XFixesDestroyRegion(display, region);
        }
        XFlush(display);
        true
    }
}

// SAFETY: `display` is a live connection; unread events are discarded so the
// Xlib input buffer cannot stall later `XFlush` / `XConfigureWindow` calls.
unsafe fn drain_x_events(display: *mut Display) {
    while XPending(display) != 0 {
        let mut event: XEvent = std::mem::zeroed();
        XNextEvent(display, &mut event);
    }
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// `win` created on it; `changes` is a stack local that outlives
// `XConfigureWindow`, and every field named by `mask` is initialized.
unsafe fn configure(display: *mut Display, win: Window, x: i32, y: i32, w: i32, h: i32) {
    let mut changes = XWindowChanges {
        x,
        y,
        width: w.max(1),
        height: h.max(1),
        border_width: 0,
        sibling: 0,
        stack_mode: 0,
    };
    // Do not raise on every mouse-move: MapRaised + CWStackMode floods XWayland
    // and the outline stops tracking once the request queue backs up.
    let mask = (CWX | CWY | CWWidth | CWHeight) as c_uint;
    XConfigureWindow(display, win, mask, &mut changes);
}

#[cfg(test)]
mod tests {
    use super::SelectionOutline;

    #[test]
    fn open_or_skip() {
        let _ = SelectionOutline::open();
    }
}
