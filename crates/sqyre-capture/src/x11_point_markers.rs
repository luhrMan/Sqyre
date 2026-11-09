//! Click-through squares on recorded macro points.
//!
//! Deferred egui viewports cannot be positioned on GNOME/Wayland (the compositor
//! cascades them down the middle of the screen). These are override-redirect X11
//! windows, same as the selection outline, so `ConfigureWindow` is the position.

use std::os::raw::{c_int, c_uint, c_ulong};
use std::ptr;

use x11::xfixes::{
    XFixesCreateRegion, XFixesDestroyRegion, XFixesQueryExtension, XFixesQueryVersion,
    XFixesSetWindowShapeRegion,
};
use x11::xlib::{
    CWBackPixel, CWBorderPixel, CWHeight, CWOverrideRedirect, CWWidth, Display, InputOutput, True,
    Window, XClearWindow, XConfigureWindow, XCreateWindow, XDefaultDepth, XDefaultRootWindow,
    XDefaultScreen, XDefaultVisual, XDestroyWindow, XFlush, XMapWindow, XSetWindowAttributes,
    XWindowChanges, CWX, CWY,
};

use crate::outline_geometry::{marker_window, sample_marker_points};
use crate::x11_outline::{alloc_stroke_pixel, drain_x_events, open_x_display, schedule_x_close};
use crate::CaptureError;

/// `ShapeInput` from `X11/extensions/shapeconst.h` — empty region = click-through.
const SHAPE_INPUT: c_int = 2;

/// One gold square per recorded point, positioned in virtual-desktop pixels.
pub struct PointMarkerOverlay {
    display: *mut Display,
    root: Window,
    pixel: c_ulong,
    windows: Vec<Window>,
    shown: usize,
}

// SAFETY: the raw display pointer is owned exclusively by this struct — only
// `&mut self` methods and `Drop` touch it, so it is never used from two threads
// at once even after the value is moved.
unsafe impl Send for PointMarkerOverlay {}

impl PointMarkerOverlay {
    pub fn open() -> Result<Self, CaptureError> {
        // SAFETY: `open_x_display` null-checks; color alloc and the root window
        // use that live connection. Failure closes the display.
        unsafe {
            let display = open_x_display()?;
            let screen = XDefaultScreen(display);
            let root = XDefaultRootWindow(display);
            let pixel = match alloc_stroke_pixel(display, screen) {
                Ok(p) => p,
                Err(e) => {
                    schedule_x_close(display);
                    return Err(CaptureError::Message(e));
                }
            };
            Ok(Self {
                display,
                root,
                pixel,
                windows: Vec::new(),
                shown: 0,
            })
        }
    }

    /// Place a square on each point. Extra windows from a previous frame are parked.
    pub fn set_points(&mut self, points: &[(i32, i32)]) {
        if self.display.is_null() {
            return;
        }
        let points = sample_marker_points(points);
        if points.is_empty() {
            self.clear();
            return;
        }
        // SAFETY: `display` is the live connection from `open`; windows in
        // `self.windows` were created on it.
        unsafe {
            let before = self.windows.len();
            while self.windows.len() < points.len() {
                match create_marker(self.display, self.root, self.pixel) {
                    Ok(win) => self.windows.push(win),
                    Err(_) => break,
                }
            }
            if self.windows.len() > before {
                let _ = shape_input_empty(self.display, &self.windows[before..]);
            }
            for (win, &(x, y)) in self.windows.iter().zip(points.iter()) {
                let (wx, wy, w, h) = marker_window(x, y);
                configure(self.display, *win, wx, wy, w, h);
                XMapWindow(self.display, *win);
            }
            for win in self.windows.iter().skip(points.len()).take(self.shown) {
                configure(self.display, *win, -20, -20, 1, 1);
            }
            XFlush(self.display);
            drain_x_events(self.display);
        }
        self.shown = points.len().min(self.windows.len());
    }

    pub fn clear(&mut self) {
        if self.shown == 0 || self.display.is_null() {
            return;
        }
        // SAFETY: windows were created on `self.display`.
        unsafe {
            for win in self.windows.iter().take(self.shown) {
                configure(self.display, *win, -20, -20, 1, 1);
            }
            XFlush(self.display);
            drain_x_events(self.display);
        }
        self.shown = 0;
    }
}

impl Drop for PointMarkerOverlay {
    fn drop(&mut self) {
        if self.display.is_null() {
            return;
        }
        // SAFETY: windows were created on this display; the pointer is nulled
        // before the close thread runs.
        unsafe {
            for &win in &self.windows {
                if win != 0 {
                    XDestroyWindow(self.display, win);
                }
            }
            let display = self.display;
            self.display = ptr::null_mut();
            schedule_x_close(display);
        }
    }
}

/// # Safety
/// `display` is a live connection; `root` and `pixel` belong to it.
unsafe fn create_marker(display: *mut Display, root: Window, pixel: c_ulong) -> Result<Window, ()> {
    // SAFETY: caller guarantees a live display; `attrs` sets every field in `mask`
    // and outlives `XCreateWindow`.
    unsafe {
        let screen = XDefaultScreen(display);
        let mut attrs: XSetWindowAttributes = std::mem::zeroed();
        attrs.background_pixel = pixel;
        attrs.border_pixel = pixel;
        attrs.override_redirect = True;
        let mask = CWBackPixel | CWBorderPixel | CWOverrideRedirect;
        let win = XCreateWindow(
            display,
            root,
            -20,
            -20,
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
            Err(())
        } else {
            Ok(win)
        }
    }
}

/// # Safety
/// `display` is live and `win` was created on it. `changes` outlives the call.
unsafe fn configure(display: *mut Display, win: Window, x: i32, y: i32, w: i32, h: i32) {
    // SAFETY: caller guarantees `win` was created on `display`.
    unsafe {
        let mut changes = XWindowChanges {
            x,
            y,
            width: w.max(1),
            height: h.max(1),
            border_width: 0,
            sibling: 0,
            stack_mode: 0,
        };
        let mask = (CWX | CWY | CWWidth | CWHeight) as c_uint;
        XConfigureWindow(display, win, mask, &mut changes);
        // Background is already the stroke pixel; clear paints it after a resize.
        XClearWindow(display, win);
    }
}

/// Empty `ShapeInput` so the cursor is not inside a window we move every sample.
fn shape_input_empty(display: *mut Display, windows: &[Window]) -> bool {
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
        for &win in windows {
            let region = XFixesCreateRegion(display, ptr::null_mut(), 0);
            if region == 0 {
                return false;
            }
            XFixesSetWindowShapeRegion(display, win, SHAPE_INPUT, 0, 0, region);
            XFixesDestroyRegion(display, region);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::PointMarkerOverlay;

    #[test]
    fn open_or_skip() {
        let _ = PointMarkerOverlay::open();
    }
}
