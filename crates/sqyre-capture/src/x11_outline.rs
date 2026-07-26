//! Live search-area outline via direct X11 windows (no desktop snapshot).
//!
//! Positions override-redirect X11 windows with `ConfigureWindow` and paints a
//! stroked rectangle selection layer. We skip the snapshot
//! background and draw only that rectangle as four thin override-redirect edge
//! windows so the desktop stays visible underneath.

use std::os::raw::{c_int, c_uint, c_ulong};
use std::ptr;
use x11::xlib::{
    Above, CWBackPixel, CWBorderPixel, CWHeight, CWOverrideRedirect, CWStackMode, CWWidth, Display,
    InputOutput, True, Window, XAllocColor, XCloseDisplay, XColor, XConfigureWindow, XCreateWindow,
    XDefaultColormap, XDefaultDepth, XDefaultRootWindow, XDefaultScreen, XDefaultVisual,
    XDestroyWindow, XFlush, XMapRaised, XOpenDisplay, XSetWindowAttributes, XUnmapWindow,
    XWindowChanges, _XDisplay, CWX, CWY,
};

use crate::outline_geometry::{edge_placements, outline_should_clear};
use crate::CaptureError;
pub use crate::outline_rect::OutlineRect;

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
        // SAFETY: `self.display` is a live connection (non-null since `open`
        // succeeded) and `self.edges` were created on it.
        unsafe {
            place_edges(self.display, &self.edges, rect);
            for &w in &self.edges {
                XMapRaised(self.display, w);
            }
            XFlush(self.display);
        }
        self.mapped = true;
        self.last = Some(rect);
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

// SAFETY: callers must pass a live, non-null Xlib `display` connection and
// `edges` created on it.
unsafe fn place_edges(display: *mut Display, edges: &[Window; 4], r: OutlineRect) {
    for (&win, &(x, y, w, h)) in edges.iter().zip(edge_placements(r).iter()) {
        configure(display, win, x, y, w, h);
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
        stack_mode: Above,
    };
    let mask = (CWX | CWY | CWWidth | CWHeight | CWStackMode) as c_uint;
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
