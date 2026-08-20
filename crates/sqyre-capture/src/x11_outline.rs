//! Live search-area outline via direct X11 windows (no desktop snapshot).
//!
//! Positions override-redirect X11 windows with `ConfigureWindow` and paints a
//! stroked rectangle selection layer. We skip the snapshot
//! background and draw only that rectangle as four thin override-redirect edge
//! windows so the desktop stays visible underneath.

use std::os::raw::{c_int, c_uint, c_ulong};
use std::ptr;
use x11::xlib::{
    Button1Mask, CWBackPixel, CWBorderPixel, CWHeight, CWOverrideRedirect, CWWidth, Display,
    InputOutput, True, Window, XAllocColor, XCloseDisplay, XColor, XConfigureWindow, XCreateWindow,
    XDefaultColormap, XDefaultDepth, XDefaultRootWindow, XDefaultScreen, XDefaultVisual,
    XDestroyWindow, XFlush, XMapRaised, XOpenDisplay, XQueryPointer, XSetWindowAttributes,
    XUnmapWindow, XWindowChanges, _XDisplay, CWX, CWY,
};

use crate::outline_geometry::{edge_placements, outline_should_clear};
pub use crate::outline_rect::OutlineRect;
use crate::CaptureError;

/// Selection stroke color (gold).
const STROKE_R: u16 = 255;
const STROKE_G: u16 = 200;
const STROKE_B: u16 = 0;

/// Four edge windows forming a hollow rectangle on the virtual desktop.
pub struct SelectionOutline {
    display: *mut _XDisplay,
    edges: [Window; 4],
    mapped: bool,
    last: Option<OutlineRect>,
    last_edges: Option<[(i32, i32, i32, i32); 4]>,
}

// SAFETY: the raw display pointer is owned exclusively by this struct — only
// `&mut self` methods and `Drop` touch it, so it is never used from two threads
// at once even after the value is moved to another thread.
unsafe impl Send for SelectionOutline {}

impl SelectionOutline {
    pub fn open() -> Result<Self, CaptureError> {
        // SAFETY: `XOpenDisplay(null)` connects to the default display and its
        // result is null-checked before any other Xlib call; every early return
        // destroys the windows created so far and closes the connection.
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(CaptureError::Message(
                    "XOpenDisplay failed (need X11)".into(),
                ));
            }
            crate::x11_secondary::register(display);
            let screen = XDefaultScreen(display);
            let root = XDefaultRootWindow(display);
            let pixel = match alloc_stroke_pixel(display, screen) {
                Ok(p) => p,
                Err(e) => {
                    crate::x11_secondary::unregister(display);
                    XCloseDisplay(display);
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
                        crate::x11_secondary::unregister(display);
                        XCloseDisplay(display);
                        return Err(CaptureError::Message(e));
                    }
                }
            }
            Ok(Self {
                display,
                edges,
                mapped: false,
                last: None,
                last_edges: None,
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
            }
            XFlush(self.display);
        }
        self.mapped = true;
        self.last = Some(rect);
        self.last_edges = Some(placements);
    }

    /// Root-window pointer in the same coordinate space as [`Self::set_rect`].
    pub fn query_pointer(&self) -> Option<(i32, i32, bool)> {
        if self.display.is_null() {
            return None;
        }
        // SAFETY: `self.display` is live since `open`; out-params are stack locals.
        unsafe {
            let root = XDefaultRootWindow(self.display);
            let mut root_ret: Window = 0;
            let mut child: Window = 0;
            let mut root_x = 0;
            let mut root_y = 0;
            let mut win_x = 0;
            let mut win_y = 0;
            let mut mask = 0u32;
            if XQueryPointer(
                self.display,
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
        let fallback = self
            .root_size()
            .map(|(w, h)| sqyre_ports::DesktopRect { x: 0, y: 0, w, h })
            .unwrap_or_default();
        crate::x11_capture::xinerama_monitor_rects_on(self.display, fallback)
    }

    pub fn clear(&mut self) {
        if !self.mapped && self.last.is_none() {
            return;
        }
        // SAFETY: `self.display` is a live connection (non-null since `open`
        // succeeded) and `self.edges` were created on it.
        unsafe {
            for &w in &self.edges {
                XUnmapWindow(self.display, w);
            }
            XFlush(self.display);
        }
        self.mapped = false;
        self.last = None;
        self.last_edges = None;
    }
}

impl Drop for SelectionOutline {
    fn drop(&mut self) {
        // SAFETY: the edges were created on `self.display`, which is still live
        // here; it is closed last and nulled so nothing can reuse it.
        unsafe {
            for &w in &self.edges {
                if w != 0 {
                    XDestroyWindow(self.display, w);
                }
            }
            if !self.display.is_null() {
                crate::x11_secondary::unregister(self.display);
                XCloseDisplay(self.display);
                self.display = ptr::null_mut();
            }
        }
    }
}

// SAFETY: callers must pass a live, non-null Xlib `display` connection and a
// screen index valid on it; `color` is a stack local that outlives `XAllocColor`.
unsafe fn alloc_stroke_pixel(display: *mut Display, screen: c_int) -> Result<c_ulong, String> {
    let mut color = XColor {
        pixel: 0,
        red: STROKE_R << 8,
        green: STROKE_G << 8,
        blue: STROKE_B << 8,
        flags: 0,
        pad: 0,
    };
    let cmap = XDefaultColormap(display, screen);
    if XAllocColor(display, cmap, &mut color) == 0 {
        return Err("XAllocColor failed for selection outline".into());
    }
    Ok(color.pixel)
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
