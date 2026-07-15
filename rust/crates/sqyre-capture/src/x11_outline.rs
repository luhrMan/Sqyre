//! Live search-area outline via direct X11 windows (no desktop snapshot).
//!
//! Go's recording overlay positions Fyne windows with X11 `OverrideRedirect` +
//! `ConfigureWindow` (`ui/recording/overlay_position_x11.go`) and paints a stroked
//! rectangle (`overlay.go` selection layer). For Rust we skip the snapshot
//! background and draw only that rectangle as four thin override-redirect edge
//! windows so the desktop stays visible underneath.

use std::os::raw::{c_int, c_uint, c_ulong};
use std::ptr;
use x11::xlib::{
    Above, CWBackPixel, CWBorderPixel, CWHeight, CWOverrideRedirect, CWStackMode, CWWidth, CWX, CWY,
    Display, InputOutput, True, Window, XAllocColor, XCloseDisplay, XColor, XConfigureWindow,
    XCreateWindow, XDefaultColormap, XDefaultDepth, XDefaultRootWindow, XDefaultScreen,
    XDefaultVisual, XDestroyWindow, XFlush, XMapRaised, XOpenDisplay, XSetWindowAttributes,
    XUnmapWindow, XWindowChanges, _XDisplay,
};

const EDGE_PX: i32 = 2;
/// Go selection stroke `NRGBA{255, 200, 0, 255}`.
const STROKE_R: u16 = 255;
const STROKE_G: u16 = 200;
const STROKE_B: u16 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlineRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl OutlineRect {
    pub fn normalize(ax: i32, ay: i32, bx: i32, by: i32) -> Self {
        let (left, right) = if ax <= bx { (ax, bx) } else { (bx, ax) };
        let (top, bottom) = if ay <= by { (ay, by) } else { (by, ay) };
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn is_empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    pub fn width(self) -> i32 {
        (self.right - self.left).max(0)
    }

    pub fn height(self) -> i32 {
        (self.bottom - self.top).max(0)
    }
}

/// Four edge windows forming a hollow rectangle on the virtual desktop.
pub struct SelectionOutline {
    display: *mut _XDisplay,
    edges: [Window; 4],
    mapped: bool,
    last: Option<OutlineRect>,
}

// X11 display pointer: all use stays on the UI / owning thread.
unsafe impl Send for SelectionOutline {}

impl SelectionOutline {
    pub fn open() -> Result<Self, String> {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err("XOpenDisplay failed (need X11)".into());
            }
            let screen = XDefaultScreen(display);
            let root = XDefaultRootWindow(display);
            let pixel = match alloc_stroke_pixel(display, screen) {
                Ok(p) => p,
                Err(e) => {
                    XCloseDisplay(display);
                    return Err(e);
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
                        XCloseDisplay(display);
                        return Err(e);
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
        if rect.is_empty() || rect.width() < EDGE_PX * 2 || rect.height() < EDGE_PX * 2 {
            self.clear();
            return;
        }
        if self.last == Some(rect) && self.mapped {
            return;
        }
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
        unsafe {
            for &w in &self.edges {
                if w != 0 {
                    XDestroyWindow(self.display, w);
                }
            }
            if !self.display.is_null() {
                XCloseDisplay(self.display);
                self.display = ptr::null_mut();
            }
        }
    }
}

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

unsafe fn place_edges(display: *mut Display, edges: &[Window; 4], r: OutlineRect) {
    let w = r.width().max(1);
    let h = r.height().max(1);
    let t = EDGE_PX;
    // top, bottom, left, right
    configure(display, edges[0], r.left, r.top, w, t);
    configure(display, edges[1], r.left, r.bottom - EDGE_PX, w, t);
    configure(display, edges[2], r.left, r.top, t, h);
    configure(display, edges[3], r.right - EDGE_PX, r.top, t, h);
}

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
    use super::*;

    #[test]
    fn normalize_and_empty() {
        let r = OutlineRect::normalize(10, 20, 5, 40);
        assert_eq!(
            r,
            OutlineRect {
                left: 5,
                top: 20,
                right: 10,
                bottom: 40
            }
        );
        assert!(!r.is_empty());
        assert!(OutlineRect::normalize(1, 1, 1, 1).is_empty());
    }

    #[test]
    fn open_or_skip() {
        let _ = SelectionOutline::open();
    }
}
