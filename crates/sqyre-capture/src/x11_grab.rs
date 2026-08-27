//! Fullscreen X11 pointer grab for screen-click recording.
//!
//! Creates an override-redirect `InputOnly` window covering the virtual desktop and
//! grabs the pointer (and keyboard for Esc) so games that confine/relative-capture
//! the mouse cannot block Point / Color / SearchArea selection.

use std::os::raw::{c_int, c_uint};
use std::ptr;
use x11::xlib::{
    Above, ButtonPress, ButtonPressMask, ButtonReleaseMask, CWHeight, CWOverrideRedirect,
    CWStackMode, CWWidth, CurrentTime, Display, GrabModeAsync, InputOnly, KeyPress, KeyPressMask,
    MotionNotify, PointerMotionMask, RevertToParent, Success, True, Window, XConfigureWindow,
    XCreateFontCursor, XCreateWindow, XDefaultRootWindow, XDefaultScreen, XDestroyWindow,
    XDisplayHeight, XDisplayWidth, XEvent, XFlush, XFreeCursor, XGrabKeyboard, XGrabPointer,
    XKeycodeToKeysym, XMapRaised, XNextEvent, XOpenDisplay, XPending, XSelectInput, XSetInputFocus,
    XSetWindowAttributes, XUngrabKeyboard, XUngrabPointer, XUnmapWindow, XWindowChanges, _XDisplay,
    CWX, CWY,
};

use crate::selection_grab::GrabPoll;
use crate::CaptureError;

/// X11 cursorfont crosshair (from `X11/cursorfont.h`).
const XC_CROSSHAIR: c_uint = 34;
/// Keysym for Escape.
const XK_ESCAPE: u64 = 0xFF1B;

/// Fullscreen invisible layer that owns the pointer while screen-click recording is armed.
pub struct SelectionGrab {
    display: *mut _XDisplay,
    window: Window,
    cursor: u64,
    armed: bool,
    last_pos: (i32, i32),
}

// SAFETY: the raw display pointer is owned exclusively by this struct — only
// `&mut self` methods and `Drop` touch it.
unsafe impl Send for SelectionGrab {}

impl SelectionGrab {
    pub fn open() -> Result<Self, CaptureError> {
        // SAFETY: `XOpenDisplay(null)` connects to the default display; null-checked
        // before any further Xlib use. Early failures close the connection.
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(CaptureError::Message(
                    "XOpenDisplay failed for selection grab (need X11)".into(),
                ));
            }
            crate::x11_secondary::register(display);
            let screen = XDefaultScreen(display);
            let root = XDefaultRootWindow(display);
            let width = XDisplayWidth(display, screen).max(1) as c_uint;
            let height = XDisplayHeight(display, screen).max(1) as c_uint;
            let mut attrs: XSetWindowAttributes = std::mem::zeroed();
            attrs.override_redirect = True;
            let window = XCreateWindow(
                display,
                root,
                0,
                0,
                width,
                height,
                0,
                0,
                InputOnly as c_uint,
                ptr::null_mut(),
                CWOverrideRedirect,
                &mut attrs,
            );
            if window == 0 {
                crate::x11_secondary::unregister(display);
                x11::xlib::XCloseDisplay(display);
                return Err(CaptureError::Message(
                    "XCreateWindow failed for selection grab".into(),
                ));
            }
            XSelectInput(
                display,
                window,
                ButtonPressMask | ButtonReleaseMask | PointerMotionMask | KeyPressMask,
            );
            let cursor = XCreateFontCursor(display, XC_CROSSHAIR);
            Ok(Self {
                display,
                window,
                cursor,
                armed: false,
                last_pos: (0, 0),
            })
        }
    }

    /// Map the grab window, take pointer + keyboard, and free any prior confinement.
    pub fn arm(&mut self) -> Result<(), CaptureError> {
        // SAFETY: `self.display` / `self.window` are live since `open`.
        unsafe {
            let screen = XDefaultScreen(self.display);
            let width = XDisplayWidth(self.display, screen).max(1);
            let height = XDisplayHeight(self.display, screen).max(1);
            let mut changes = XWindowChanges {
                x: 0,
                y: 0,
                width,
                height,
                border_width: 0,
                sibling: 0,
                stack_mode: Above,
            };
            let mask = (CWX | CWY | CWWidth | CWHeight | CWStackMode) as c_uint;
            XConfigureWindow(self.display, self.window, mask, &mut changes);
            XMapRaised(self.display, self.window);
            // Prefer keyboard focus on the grab so Esc is delivered here even when
            // the game previously owned the focus.
            let _ = XSetInputFocus(self.display, self.window, RevertToParent, CurrentTime);
            let grab = XGrabPointer(
                self.display,
                self.window,
                0, // owner_events = False — do not deliver to the game underneath
                (ButtonPressMask | ButtonReleaseMask | PointerMotionMask) as c_uint,
                GrabModeAsync,
                GrabModeAsync,
                0, // confine_to = None (free across the desktop)
                self.cursor,
                CurrentTime,
            );
            if grab != Success as c_int {
                XUnmapWindow(self.display, self.window);
                XFlush(self.display);
                return Err(CaptureError::Message(format!(
                    "XGrabPointer failed (status {grab})"
                )));
            }
            let key_grab = XGrabKeyboard(
                self.display,
                self.window,
                True,
                GrabModeAsync,
                GrabModeAsync,
                CurrentTime,
            );
            if key_grab != Success as c_int {
                XUngrabPointer(self.display, CurrentTime);
                XUnmapWindow(self.display, self.window);
                XFlush(self.display);
                return Err(CaptureError::Message(format!(
                    "XGrabKeyboard failed (status {key_grab})"
                )));
            }
            XFlush(self.display);
        }
        self.armed = true;
        Ok(())
    }

    pub fn disarm(&mut self) {
        if !self.armed {
            return;
        }
        // SAFETY: display/window still live while armed.
        unsafe {
            XUngrabPointer(self.display, CurrentTime);
            XUngrabKeyboard(self.display, CurrentTime);
            XUnmapWindow(self.display, self.window);
            XFlush(self.display);
        }
        self.armed = false;
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Drain X events into a [`GrabPoll`]. Call from the UI thread.
    pub fn poll(&mut self) -> GrabPoll {
        let mut out = GrabPoll {
            x: self.last_pos.0,
            y: self.last_pos.1,
            ..GrabPoll::default()
        };
        if !self.armed {
            return out;
        }
        // SAFETY: live display; events are stack-local.
        unsafe {
            while XPending(self.display) != 0 {
                let mut event: XEvent = std::mem::zeroed();
                XNextEvent(self.display, &mut event);
                apply_x_event(self.display, &event, &mut out, &mut self.last_pos);
            }
        }
        out.x = self.last_pos.0;
        out.y = self.last_pos.1;
        out
    }
}

impl Drop for SelectionGrab {
    fn drop(&mut self) {
        self.disarm();
        // SAFETY: destroy the window we created, free the cursor, close the display.
        unsafe {
            if self.window != 0 {
                XDestroyWindow(self.display, self.window);
                self.window = 0;
            }
            if self.cursor != 0 {
                XFreeCursor(self.display, self.cursor);
                self.cursor = 0;
            }
            if !self.display.is_null() {
                crate::x11_secondary::unregister(self.display);
                x11::xlib::XCloseDisplay(self.display);
                self.display = ptr::null_mut();
            }
        }
    }
}

// SAFETY: `event` is a fully initialized XEvent from XNextEvent on `display`.
unsafe fn apply_x_event(
    display: *mut Display,
    event: &XEvent,
    out: &mut GrabPoll,
    last_pos: &mut (i32, i32),
) {
    // Compare against Xlib event-type constants with `==` (not `match`) so Clippy's
    // `non_upper_case_globals` lint does not fire on the X11 names.
    let ty = event.get_type();
    if ty == MotionNotify {
        let motion = &*(event as *const XEvent as *const x11::xlib::XMotionEvent);
        *last_pos = (motion.x_root, motion.y_root);
        out.moved = true;
    } else if ty == ButtonPress {
        let button = &*(event as *const XEvent as *const x11::xlib::XButtonEvent);
        *last_pos = (button.x_root, button.y_root);
        out.moved = true;
        if button.button == 1 {
            out.left_clicks = out.left_clicks.saturating_add(1);
        }
    } else if ty == KeyPress {
        let key = &*(event as *const XEvent as *const x11::xlib::XKeyEvent);
        let keysym = XKeycodeToKeysym(display, key.keycode as u8, 0);
        if keysym == XK_ESCAPE {
            out.escape = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SelectionGrab;

    #[test]
    fn open_or_skip() {
        let _ = SelectionGrab::open();
    }
}
