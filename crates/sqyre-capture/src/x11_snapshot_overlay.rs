//! Fullscreen mouse-owning selection cover (X11 override-redirect).
//!
//! Used on GNOME/Wayland while Point / Color / SearchArea recording is armed so
//! pointer events land on our window instead of an XWayland game.
//!
//! - [`FrozenSelectionOverlay::capture_and_open`]: freeze pixmap + gold stroke.
//! - [`FrozenSelectionOverlay::open_input_cover`]: invisible `InputOnly` hit-test
//!   cover; the gold rubber-band is drawn by [`crate::SelectionOutline`] edges.

use crate::outline_geometry::{
    edge_placements, outline_should_clear, STROKE_B, STROKE_G, STROKE_R,
};
use crate::outline_rect::OutlineRect;
use crate::selection_grab::GrabPoll;
use crate::CaptureError;
use image::RgbaImage;
use sqyre_ports::DesktopRect;
use std::os::raw::{c_char, c_int, c_uint, c_ulong};
use std::ptr;
use x11::xlib::{
    ButtonPress, ButtonPressMask, ButtonRelease, ButtonReleaseMask, CWBackingStore, CWBorderPixel,
    CWEventMask, CWOverrideRedirect, CurrentTime, Display, Expose, ExposureMask, InputOnly,
    InputOutput, KeyPress, KeyPressMask, LSBFirst, MotionNotify, PointerMotionMask, RevertToParent,
    True, WhenMapped, Window, XAllocColor, XClearWindow, XCloseDisplay, XColor, XCopyArea,
    XCreateFontCursor, XCreateGC, XCreateImage, XCreatePixmap, XCreateWindow, XDefaultColormap,
    XDefaultDepth, XDefaultRootWindow, XDefaultScreen, XDefaultVisual, XDefineCursor, XDestroyImage,
    XDestroyWindow, XEvent, XFillRectangle, XFlush, XFreeCursor, XFreeGC, XFreePixmap,
    XKeycodeToKeysym, XMapRaised, XNextEvent, XOpenDisplay, XPending, XPutImage, XSelectInput,
    XSetForeground, XSetInputFocus, XSetWindowAttributes, XSetWindowBackgroundPixmap,
    XUnmapWindow, ZPixmap, _XDisplay,
};

/// X11 cursorfont crosshair (`X11/cursorfont.h`).
const XC_CROSSHAIR: c_uint = 34;
/// Keysym for Escape.
const XK_ESCAPE: u64 = 0xFF1B;

/// Frozen pixels kept after the cover unmaps (Find Pixel samples this, not live capture).
pub struct FrozenFrame {
    bounds: DesktopRect,
    image: RgbaImage,
}

impl FrozenFrame {
    /// Sample `rrggbb` at absolute desktop `(x, y)`.
    pub fn sample_hex(&self, x: i32, y: i32) -> Option<String> {
        sample_hex(&self.image, self.bounds, x, y)
    }
}

/// Fullscreen selection cover that owns pointer/Esc while recording is armed.
///
/// Either a freeze pixmap (`capture_and_open`) or a nearly-transparent input
/// cover (`open_input_cover`) with `pixmap == 0`.
pub struct FrozenSelectionOverlay {
    display: *mut _XDisplay,
    window: Window,
    pixmap: c_ulong,
    gc: x11::xlib::GC,
    cursor: u64,
    bounds: DesktopRect,
    image: RgbaImage,
    last_pos: (i32, i32),
    gold_pixel: c_ulong,
    last_rect: Option<OutlineRect>,
    needs_paint: bool,
    cached_rects: Vec<DesktopRect>,
}

// SAFETY: the raw Display/window/pixmap/GC are owned exclusively by this struct
// and only touched from `&mut self` / `Drop` on the UI thread.
unsafe impl Send for FrozenSelectionOverlay {}

impl FrozenSelectionOverlay {
    /// Map an invisible `InputOnly` cover over the virtual desktop (no freeze).
    ///
    /// Owns pointer/Esc so clicks do not reach windows underneath. Draw the gold
    /// rubber-band with [`crate::SelectionOutline`] — this cover has no pixels.
    pub fn open_input_cover() -> Result<Self, CaptureError> {
        crate::mark_site("snapshot:input_cover");
        let bounds = input_cover_bounds()?;
        // SAFETY: `XOpenDisplay(null)` is null-checked; failure paths close it.
        unsafe { open_input_overlay(bounds) }
    }

    /// Capture the current desktop and map a cover window over it.
    pub fn capture_and_open() -> Result<Self, CaptureError> {
        crate::mark_site("snapshot:capture_start");
        let cap = crate::shared_capturer_nonblocking()?;
        let bounds = cap.virtual_bounds_ref()?;
        crate::mark_site("snapshot:capture_rect");
        let image = cap.capture_rect_ref(bounds)?;
        crate::mark_site("snapshot:map");
        let overlay = Self::open(image, bounds)?;
        crate::mark_site("snapshot:ready");
        Ok(overlay)
    }

    /// Map `image` as an override-redirect window at `bounds`.
    pub fn open(image: RgbaImage, bounds: DesktopRect) -> Result<Self, CaptureError> {
        let width = image.width();
        let height = image.height();
        if width == 0 || height == 0 {
            return Err(CaptureError::EmptyRect);
        }
        let placed = DesktopRect {
            x: bounds.x,
            y: bounds.y,
            w: width as i32,
            h: height as i32,
        };
        // SAFETY: `XOpenDisplay(null)` is null-checked; every failure path
        // destroys created resources and closes the connection.
        unsafe { open_overlay(image, placed) }
    }

    pub fn bounds(&self) -> DesktopRect {
        self.bounds
    }

    /// Xinerama outputs cached when the cover opened.
    pub fn virtual_rects(&self) -> Vec<DesktopRect> {
        if self.cached_rects.is_empty() {
            vec![self.bounds]
        } else {
            self.cached_rects.clone()
        }
    }

    /// Sample `rrggbb` from a freeze pixmap at absolute desktop `(x, y)`.
    ///
    /// Input covers have no freeze pixels — returns `None` so callers fall back
    /// to live capture.
    pub fn sample_hex(&self, x: i32, y: i32) -> Option<String> {
        if self.pixmap == 0 {
            return None;
        }
        sample_hex(&self.image, self.bounds, x, y)
    }

    /// True when this cover holds a real freeze pixmap (not an input-only cover).
    pub fn has_freeze_pixels(&self) -> bool {
        self.pixmap != 0
    }

    /// Paint or clear the gold rubber-band in window-local coordinates.
    pub fn set_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        let rect = OutlineRect::normalize(left, top, right, bottom);
        if outline_should_clear(rect) {
            self.clear_rect();
            return;
        }
        if self.last_rect == Some(rect) && !self.needs_paint {
            return;
        }
        self.paint(Some(rect));
        self.last_rect = Some(rect);
        self.needs_paint = false;
    }

    /// Restore the freeze with no selection stroke.
    pub fn clear_rect(&mut self) {
        if self.last_rect.is_none() && !self.needs_paint {
            return;
        }
        self.paint(None);
        self.last_rect = None;
        self.needs_paint = false;
    }

    /// Keep pixels after the X11 cover is destroyed.
    pub fn into_frame(mut self) -> FrozenFrame {
        FrozenFrame {
            bounds: self.bounds,
            image: std::mem::take(&mut self.image),
        }
    }

    /// True when capture is not ready yet (retry next UI frame; do not stall).
    pub fn capture_retryable(err: &CaptureError) -> bool {
        snapshot_capture_retryable(err)
    }

    /// Drain motion / click / Esc from this cover (UI thread).
    pub fn poll(&mut self) -> GrabPoll {
        let mut out = GrabPoll {
            x: self.last_pos.0,
            y: self.last_pos.1,
            ..GrabPoll::default()
        };
        // SAFETY: live display; events are stack-local.
        unsafe {
            while XPending(self.display) != 0 {
                let mut event: XEvent = std::mem::zeroed();
                XNextEvent(self.display, &mut event);
                apply_x_event(
                    self.display,
                    &event,
                    &mut out,
                    &mut self.last_pos,
                    &mut self.needs_paint,
                );
            }
        }
        // Expose: refill from the background pixmap (cheap) then redraw gold.
        // Never full-desktop XCopyArea here — that stalls GNOME/XWayland.
        if self.needs_paint {
            self.repaint_from_background();
            self.needs_paint = false;
        }
        out.x = self.last_pos.0;
        out.y = self.last_pos.1;
        out
    }

    /// Redraw via `XClearWindow` (freeze pixmap or translucent black) + gold edges.
    fn repaint_from_background(&mut self) {
        if self.display.is_null() || self.window == 0 || self.gc.is_null() {
            return;
        }
        // SAFETY: live display/window/GC.
        unsafe {
            XClearWindow(self.display, self.window);
            if let Some(rect) = self.last_rect {
                XSetForeground(self.display, self.gc, self.gold_pixel);
                for (x, y, ew, eh) in window_local_edges(self.bounds, rect) {
                    XFillRectangle(self.display, self.window, self.gc, x, y, ew, eh);
                }
            }
            XFlush(self.display);
        }
    }

    /// Draw `rect` gold edges. Freeze mode restores prior stroke from the pixmap.
    /// Input covers (`pixmap == 0`) have no drawable pixels — outline edges paint instead.
    fn paint(&mut self, rect: Option<OutlineRect>) {
        if self.display.is_null() || self.window == 0 || self.gc.is_null() || self.pixmap == 0 {
            return;
        }
        let t0 = std::time::Instant::now();
        // SAFETY: display/window/GC/pixmap are live for freeze covers.
        unsafe {
            if let Some(prev) = self.last_rect {
                blit_edges_from_pixmap(
                    self.display,
                    self.pixmap,
                    self.window,
                    self.gc,
                    self.bounds,
                    prev,
                );
            }
            if let Some(rect) = rect {
                XSetForeground(self.display, self.gc, self.gold_pixel);
                for (x, y, ew, eh) in window_local_edges(self.bounds, rect) {
                    XFillRectangle(self.display, self.window, self.gc, x, y, ew, eh);
                }
            }
            XFlush(self.display);
        }
        let ms = t0.elapsed().as_millis();
        if ms >= 32 {
            crate::note(&format!(
                "snapshot paint slow ms={ms} edges={}",
                rect.is_some()
            ));
        }
    }
}

impl Drop for FrozenSelectionOverlay {
    fn drop(&mut self) {
        crate::mark_site("snapshot:drop:start");
        let t0 = std::time::Instant::now();
        // SAFETY: destroy only resources we created on `self.display`.
        // Must flush/close: skipping XFlush left the override-redirect cover
        // mapped on GNOME/XWayland until process exit (screen looked "frozen").
        unsafe {
            if !self.display.is_null() {
                if self.window != 0 {
                    XUnmapWindow(self.display, self.window);
                    XDestroyWindow(self.display, self.window);
                    self.window = 0;
                }
                if !self.gc.is_null() {
                    XFreeGC(self.display, self.gc);
                    self.gc = ptr::null_mut();
                }
                if self.pixmap != 0 {
                    XFreePixmap(self.display, self.pixmap);
                    self.pixmap = 0;
                }
                if self.cursor != 0 {
                    XFreeCursor(self.display, self.cursor);
                    self.cursor = 0;
                }
                // Flush destroy before closing so the compositor drops the cover.
                XFlush(self.display);
                crate::x11_secondary::unregister(self.display);
                XCloseDisplay(self.display);
                self.display = ptr::null_mut();
            }
        }
        crate::cap_log(
            "SNAPSHOT",
            "drop",
            &format!("ms={}", t0.elapsed().as_millis()),
        );
        crate::mark_site("snapshot:drop:done");
    }
}

unsafe fn open_overlay(
    image: RgbaImage,
    bounds: DesktopRect,
) -> Result<FrozenSelectionOverlay, CaptureError> {
    let display = XOpenDisplay(ptr::null());
    if display.is_null() {
        return Err(CaptureError::Message(
            "XOpenDisplay failed for snapshot overlay (need X11)".into(),
        ));
    }
    crate::x11_secondary::register(display);
    match map_overlay(display, image, bounds) {
        Ok(overlay) => Ok(overlay),
        Err(e) => {
            crate::x11_secondary::unregister(display);
            XCloseDisplay(display);
            Err(e)
        }
    }
}

fn input_cover_bounds() -> Result<DesktopRect, CaptureError> {
    if let Ok(cap) = crate::shared_capturer_nonblocking() {
        if let Ok(b) = cap.virtual_bounds_ref() {
            if b.w > 0 && b.h > 0 {
                return Ok(b);
            }
        }
    }
    let rects = crate::preferred_monitor_rects();
    let mut iter = rects.into_iter();
    let Some(first) = iter.next() else {
        return Err(CaptureError::Message(
            "no monitor rects for selection cover".into(),
        ));
    };
    Ok(iter.fold(first, |acc, r| DesktopRect {
        x: acc.x.min(r.x),
        y: acc.y.min(r.y),
        w: (acc.x + acc.w).max(r.x + r.w) - acc.x.min(r.x),
        h: (acc.y + acc.h).max(r.y + r.h) - acc.y.min(r.y),
    }))
}

/// Invisible but still hit-tested (`InputOnly`, same idea as [`crate::SelectionGrab`]).
unsafe fn open_input_overlay(bounds: DesktopRect) -> Result<FrozenSelectionOverlay, CaptureError> {
    let display = XOpenDisplay(ptr::null());
    if display.is_null() {
        return Err(CaptureError::Message(
            "XOpenDisplay failed for selection cover (need X11)".into(),
        ));
    }
    crate::x11_secondary::register(display);
    match map_input_overlay(display, bounds) {
        Ok(overlay) => Ok(overlay),
        Err(e) => {
            crate::x11_secondary::unregister(display);
            XCloseDisplay(display);
            Err(e)
        }
    }
}

unsafe fn map_input_overlay(
    display: *mut Display,
    bounds: DesktopRect,
) -> Result<FrozenSelectionOverlay, CaptureError> {
    let screen = XDefaultScreen(display);
    let root = XDefaultRootWindow(display);
    let width = bounds.w.max(1) as c_uint;
    let height = bounds.h.max(1) as c_uint;

    let mut attrs: XSetWindowAttributes = std::mem::zeroed();
    attrs.override_redirect = True;
    let window = XCreateWindow(
        display,
        root,
        bounds.x,
        bounds.y,
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
        return Err(CaptureError::Message(
            "XCreateWindow failed for selection cover".into(),
        ));
    }

    XSelectInput(
        display,
        window,
        ButtonPressMask | ButtonReleaseMask | PointerMotionMask | KeyPressMask,
    );
    let screen_w = x11::xlib::XDisplayWidth(display, screen);
    let screen_h = x11::xlib::XDisplayHeight(display, screen);
    let cached_rects = crate::x11_capture::xinerama_monitor_rects_on(
        display,
        DesktopRect {
            x: 0,
            y: 0,
            w: screen_w,
            h: screen_h,
        },
    );
    let cursor = XCreateFontCursor(display, XC_CROSSHAIR);
    XDefineCursor(display, window, cursor);
    XMapRaised(display, window);
    let _ = XSetInputFocus(display, window, RevertToParent, CurrentTime);
    XFlush(display);

    crate::event_log(
        "SQYRE_SNAPSHOT",
        &[
            ("op", "input_cover"),
            (
                "size",
                &format!("{}x{}+{}+{}", width, height, bounds.x, bounds.y),
            ),
        ],
    );
    crate::mark_site("snapshot:ready");

    Ok(FrozenSelectionOverlay {
        display,
        window,
        pixmap: 0,
        gc: ptr::null_mut(),
        cursor,
        bounds,
        image: RgbaImage::new(1, 1),
        last_pos: (bounds.x, bounds.y),
        gold_pixel: 0,
        last_rect: None,
        needs_paint: false,
        cached_rects,
    })
}

unsafe fn map_overlay(
    display: *mut Display,
    image: RgbaImage,
    bounds: DesktopRect,
) -> Result<FrozenSelectionOverlay, CaptureError> {
    let screen = XDefaultScreen(display);
    let root = XDefaultRootWindow(display);
    let visual = XDefaultVisual(display, screen);
    let depth = XDefaultDepth(display, screen);
    let width = image.width();
    let height = image.height();

    let mut attrs: XSetWindowAttributes = std::mem::zeroed();
    attrs.override_redirect = True;
    attrs.border_pixel = 0;
    attrs.backing_store = WhenMapped;
    attrs.event_mask =
        ButtonPressMask | ButtonReleaseMask | PointerMotionMask | KeyPressMask | ExposureMask;
    let window = XCreateWindow(
        display,
        root,
        bounds.x,
        bounds.y,
        width,
        height,
        0,
        depth,
        InputOutput as c_uint,
        visual,
        CWOverrideRedirect | CWBorderPixel | CWBackingStore | CWEventMask,
        &mut attrs,
    );
    if window == 0 {
        return Err(CaptureError::Message(
            "XCreateWindow failed for snapshot overlay".into(),
        ));
    }

    let pixmap = XCreatePixmap(display, window, width, height, depth as c_uint);
    if pixmap == 0 {
        XDestroyWindow(display, window);
        return Err(CaptureError::Message(
            "XCreatePixmap failed for snapshot overlay".into(),
        ));
    }
    let gc = XCreateGC(display, pixmap, 0, ptr::null_mut());
    if gc.is_null() {
        XFreePixmap(display, pixmap);
        XDestroyWindow(display, window);
        return Err(CaptureError::Message(
            "XCreateGC failed for snapshot overlay".into(),
        ));
    }

    let (red_mask, green_mask, blue_mask) = if visual.is_null() {
        (0x00FF_0000, 0x0000_FF00, 0x0000_00FF)
    } else {
        (
            (*visual).red_mask,
            (*visual).green_mask,
            (*visual).blue_mask,
        )
    };
    let mut packed = pack_truecolor(&image, red_mask, green_mask, blue_mask);
    let ximage = XCreateImage(
        display,
        visual,
        depth as c_uint,
        ZPixmap,
        0,
        packed.as_mut_ptr().cast::<c_char>(),
        width,
        height,
        32,
        (width * 4) as c_int,
    );
    if ximage.is_null() {
        XFreeGC(display, gc);
        XFreePixmap(display, pixmap);
        XDestroyWindow(display, window);
        return Err(CaptureError::Message(
            "XCreateImage failed for snapshot overlay".into(),
        ));
    }
    (*ximage).byte_order = LSBFirst;
    (*ximage).bits_per_pixel = 32;
    XPutImage(display, pixmap, gc, ximage, 0, 0, 0, 0, width, height);
    // XDestroyImage would XFree our Vec buffer; detach it first.
    (*ximage).data = ptr::null_mut();
    XDestroyImage(ximage);

    XSetWindowBackgroundPixmap(display, window, pixmap);
    XSelectInput(
        display,
        window,
        ButtonPressMask | ButtonReleaseMask | PointerMotionMask | KeyPressMask | ExposureMask,
    );
    let gold_pixel = alloc_stroke_pixel(display, screen, visual);
    let screen_w = x11::xlib::XDisplayWidth(display, screen);
    let screen_h = x11::xlib::XDisplayHeight(display, screen);
    let cached_rects = crate::x11_capture::xinerama_monitor_rects_on(
        display,
        DesktopRect {
            x: 0,
            y: 0,
            w: screen_w,
            h: screen_h,
        },
    );
    let cursor = XCreateFontCursor(display, XC_CROSSHAIR);
    XDefineCursor(display, window, cursor);
    XMapRaised(display, window);
    // Show the freeze background pixmap without a full XCopyArea/XSync (both
    // stall hard on large GNOME/XWayland virtual desktops).
    XClearWindow(display, window);
    let _ = XSetInputFocus(display, window, RevertToParent, CurrentTime);
    XFlush(display);

    crate::event_log(
        "SQYRE_SNAPSHOT",
        &[
            ("op", "open"),
            (
                "size",
                &format!("{}x{}+{}+{}", width, height, bounds.x, bounds.y),
            ),
        ],
    );

    Ok(FrozenSelectionOverlay {
        display,
        window,
        pixmap,
        gc,
        cursor,
        bounds,
        image,
        last_pos: (bounds.x, bounds.y),
        gold_pixel,
        last_rect: None,
        needs_paint: false,
        cached_rects,
    })
}

unsafe fn apply_x_event(
    display: *mut Display,
    event: &XEvent,
    out: &mut GrabPoll,
    last_pos: &mut (i32, i32),
    needs_paint: &mut bool,
) {
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
    } else if ty == ButtonRelease {
        let button = &*(event as *const XEvent as *const x11::xlib::XButtonEvent);
        *last_pos = (button.x_root, button.y_root);
        out.moved = true;
        if button.button == 1 {
            out.left_releases = out.left_releases.saturating_add(1);
        }
    } else if ty == KeyPress {
        let key = &*(event as *const XEvent as *const x11::xlib::XKeyEvent);
        let keysym = XKeycodeToKeysym(display, key.keycode as u8, 0);
        if keysym == XK_ESCAPE {
            out.escape = true;
        }
    } else if ty == Expose {
        *needs_paint = true;
    }
}

unsafe fn alloc_stroke_pixel(
    display: *mut Display,
    screen: c_int,
    visual: *mut x11::xlib::Visual,
) -> c_ulong {
    let mut color = XColor {
        pixel: 0,
        red: u16::from(STROKE_R) << 8,
        green: u16::from(STROKE_G) << 8,
        blue: u16::from(STROKE_B) << 8,
        flags: 0,
        pad: 0,
    };
    let cmap = XDefaultColormap(display, screen);
    if XAllocColor(display, cmap, &mut color) != 0 {
        return color.pixel;
    }
    let (rm, gm, bm) = if visual.is_null() {
        (0x00FF_0000, 0x0000_FF00, 0x0000_00FF)
    } else {
        (
            (*visual).red_mask,
            (*visual).green_mask,
            (*visual).blue_mask,
        )
    };
    u64::from(pack_pixel(STROKE_R, STROKE_G, STROKE_B, rm, gm, bm))
}

fn window_local_edges(bounds: DesktopRect, rect: OutlineRect) -> Vec<(i32, i32, c_uint, c_uint)> {
    edge_placements(rect)
        .into_iter()
        .filter_map(|(x, y, w, h)| {
            clip_to_window(x - bounds.x, y - bounds.y, w, h, bounds.w, bounds.h)
        })
        .collect()
}

unsafe fn blit_edges_from_pixmap(
    display: *mut Display,
    pixmap: c_ulong,
    window: Window,
    gc: x11::xlib::GC,
    bounds: DesktopRect,
    rect: OutlineRect,
) {
    for (x, y, ew, eh) in window_local_edges(bounds, rect) {
        XCopyArea(
            display,
            pixmap,
            window,
            gc,
            x,
            y,
            ew,
            eh,
            x,
            y,
        );
    }
}

fn clip_to_window(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    max_w: i32,
    max_h: i32,
) -> Option<(i32, i32, c_uint, c_uint)> {
    let right = x.saturating_add(w).min(max_w);
    let bottom = y.saturating_add(h).min(max_h);
    let left = x.max(0);
    let top = y.max(0);
    if right <= left || bottom <= top {
        return None;
    }
    Some((
        left,
        top,
        (right - left) as c_uint,
        (bottom - top) as c_uint,
    ))
}

fn pack_truecolor(image: &RgbaImage, red_mask: u64, green_mask: u64, blue_mask: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(image.len());
    for px in image.pixels() {
        let packed = pack_pixel(px[0], px[1], px[2], red_mask, green_mask, blue_mask);
        out.extend_from_slice(&packed.to_le_bytes());
    }
    out
}

fn pack_pixel(r: u8, g: u8, b: u8, red_mask: u64, green_mask: u64, blue_mask: u64) -> u32 {
    place_channel(r, red_mask) | place_channel(g, green_mask) | place_channel(b, blue_mask)
}

fn place_channel(value: u8, mask: u64) -> u32 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    (u32::from(value) << shift) & mask as u32
}

pub(crate) fn sample_hex(image: &RgbaImage, bounds: DesktopRect, x: i32, y: i32) -> Option<String> {
    let px = u32::try_from(x.checked_sub(bounds.x)?).ok()?;
    let py = u32::try_from(y.checked_sub(bounds.y)?).ok()?;
    if px >= image.width() || py >= image.height() {
        return None;
    }
    let p = image.get_pixel(px, py);
    Some(format!("{:02x}{:02x}{:02x}", p[0], p[1], p[2]))
}

/// True when capture is not ready yet (retry next UI frame; do not stall).
pub(crate) fn snapshot_capture_retryable(err: &CaptureError) -> bool {
    match err {
        CaptureError::Message(m) => {
            m.contains("no frame yet")
                || m.contains("waiting for portal")
                || m.contains("waiting for the first frame")
                || m.contains("still waiting")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn pack_pixel_truecolor_888() {
        let p = pack_pixel(0xab, 0xcd, 0xef, 0x00FF_0000, 0x0000_FF00, 0x0000_00FF);
        assert_eq!(p, 0x00AB_CDEF);
        assert_eq!(p.to_le_bytes(), [0xEF, 0xCD, 0xAB, 0x00]);
    }

    #[test]
    fn sample_hex_from_freeze() {
        let mut img = RgbaImage::new(4, 2);
        img.put_pixel(2, 1, Rgba([0x12, 0x34, 0x56, 0xff]));
        let bounds = DesktopRect {
            x: 100,
            y: 50,
            w: 4,
            h: 2,
        };
        assert_eq!(sample_hex(&img, bounds, 102, 51).as_deref(), Some("123456"));
        assert!(sample_hex(&img, bounds, 99, 51).is_none());
        assert!(sample_hex(&img, bounds, 104, 51).is_none());
    }

    #[test]
    fn retryable_portal_not_ready() {
        assert!(snapshot_capture_retryable(&CaptureError::Message(
            "portal capture: no frame yet from PipeWire".into(),
        )));
        assert!(!snapshot_capture_retryable(&CaptureError::EmptyRect));
    }

    #[test]
    fn clip_to_window_keeps_visible_gold_bars() {
        let clipped = clip_to_window(-2, 10, 8, 2, 100, 50);
        assert_eq!(clipped, Some((0, 10, 6, 2)));
        assert!(clip_to_window(120, 0, 10, 2, 100, 50).is_none());
    }

    #[test]
    fn window_local_edges_offset_by_bounds() {
        let bounds = DesktopRect {
            x: 1920,
            y: 0,
            w: 2560,
            h: 1440,
        };
        let rect = OutlineRect::normalize(2000, 100, 2200, 300);
        let edges = window_local_edges(bounds, rect);
        assert_eq!(edges[0], (80, 100, 200, 2));
        assert_eq!(edges[2], (80, 102, 2, 196));
    }
}
