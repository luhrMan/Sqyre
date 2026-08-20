//! Linux X11 absolute virtual-desktop capture.

use crate::error::CaptureError;
use crate::pixel_convert::{zpixmap_to_rgb, zpixmap_to_rgba};
use crate::x11_errors::with_capture_error_handler;
use image::RgbaImage;
use parking_lot::Mutex;
use sqyre_ports::{DesktopRect, RgbCapture};
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;
use x11::xinerama::{XineramaIsActive, XineramaQueryScreens, XineramaScreenInfo};
use x11::xlib::{
    XCloseDisplay, XDefaultRootWindow, XDestroyImage, XDisplayHeight, XDisplayWidth, XFree,
    XGetImage, XOpenDisplay, XQueryPointer, XResourceManagerString, ZPixmap, _XDisplay,
};

const ALLPLANES: u64 = !0;

/// Shared X11 display connection (internal; public entry is [`crate::linux::capturer::OsCapturer`]).
pub struct X11Capturer {
    inner: Mutex<X11State>,
}

struct X11State {
    display: *mut _XDisplay,
    root: u64,
    width: i32,
    height: i32,
}

// SAFETY: the raw display pointer is only ever touched while `OsCapturer::inner`
// (a `Mutex`) is held, so concurrent access from another thread never overlaps.
unsafe impl Send for X11State {}

impl X11Capturer {
    pub fn open() -> Result<Self, CaptureError> {
        // SAFETY: `XOpenDisplay(null)` connects to the default display; the
        // returned pointer is checked for null before any other Xlib call uses it.
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(CaptureError::OpenDisplay);
            }
            crate::x11_secondary::register(display);
            let screen = x11::xlib::XDefaultScreen(display);
            let root = XDefaultRootWindow(display);
            let width = XDisplayWidth(display, screen);
            let height = XDisplayHeight(display, screen);
            Ok(Self {
                inner: Mutex::new(X11State {
                    display,
                    root,
                    width,
                    height,
                }),
            })
        }
    }

    /// Absolute pointer position on the virtual desktop (root coords).
    pub fn pointer_position(&self) -> Result<(i32, i32), CaptureError> {
        let st = self.inner.lock();
        // SAFETY: `st.display`/`st.root` are the live display/root opened by
        // `OsCapturer::open`; all out-params are stack-local and correctly sized.
        unsafe {
            let mut root_ret = 0u64;
            let mut child_ret = 0u64;
            let mut root_x = 0i32;
            let mut root_y = 0i32;
            let mut win_x = 0i32;
            let mut win_y = 0i32;
            let mut mask = 0u32;
            let ok = XQueryPointer(
                st.display,
                st.root,
                &mut root_ret,
                &mut child_ret,
                &mut root_x,
                &mut root_y,
                &mut win_x,
                &mut win_y,
                &mut mask,
            );
            if ok == 0 {
                return Err(CaptureError::QueryPointer);
            }
            Ok((root_x, root_y))
        }
    }

    /// Capture a desktop rect (`&self` — safe to call via [`Arc`] from worker threads).
    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        self.with_zpixmap(rect, |data, w, h, bpp, stride| {
            zpixmap_to_rgba(data, w, h, bpp, stride).map_err(CaptureError::Message)
        })
    }

    /// Capture RGB directly (no alpha channel / no second conversion pass).
    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        self.with_zpixmap(rect, |data, w, h, bpp, stride| {
            let data = zpixmap_to_rgb(data, w, h, bpp, stride).map_err(CaptureError::Message)?;
            Ok(RgbCapture {
                width: w,
                height: h,
                data,
            })
        })
    }

    fn with_zpixmap<T>(
        &self,
        rect: DesktopRect,
        convert: impl FnOnce(&[u8], u32, u32, usize, usize) -> Result<T, CaptureError>,
    ) -> Result<T, CaptureError> {
        if rect.is_empty() {
            return Err(CaptureError::EmptyRect);
        }
        // Fetch and own the raw pixels while the lock is held, then release the
        // lock before running the (Rayon-parallel) RGB/RGBA swizzle below, so
        // other threads aren't blocked on the X11 connection during conversion.
        let (data, w, h, bpp, stride) = {
            let st = self.inner.lock();
            with_capture_error_handler(st.display, || {
                // SAFETY: `st.display`/`st.root` are the live display/root; `ximage` is
                // null-checked before dereference, and `XDestroyImage` runs on every
                // return path (including the `bpp < 3` error) so the image is never leaked.
                unsafe {
                    let ximage = XGetImage(
                        st.display,
                        st.root,
                        rect.x,
                        rect.y,
                        rect.w as u32,
                        rect.h as u32,
                        ALLPLANES,
                        ZPixmap,
                    );
                    if ximage.is_null() {
                        return Err(CaptureError::GetImage {
                            x: rect.x,
                            y: rect.y,
                            w: rect.w,
                            h: rect.h,
                        });
                    }
                    let img = &*ximage;
                    let w = img.width as u32;
                    let h = img.height as u32;
                    let bpp = (img.bits_per_pixel / 8) as usize;
                    if bpp < 3 {
                        let bits = img.bits_per_pixel;
                        XDestroyImage(ximage);
                        return Err(CaptureError::BitsPerPixel(bits));
                    }
                    let stride = img.bytes_per_line as usize;
                    let data_len = stride.saturating_mul(h as usize);
                    let data = std::slice::from_raw_parts(img.data as *const u8, data_len).to_vec();
                    XDestroyImage(ximage);
                    Ok((data, w, h, bpp, stride))
                }
            })?
        };
        convert(&data, w, h, bpp, stride)
    }

    /// Virtual desktop bounds (`&self`).
    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, CaptureError> {
        let st = self.inner.lock();
        Ok(DesktopRect {
            x: 0,
            y: 0,
            w: st.width,
            h: st.height,
        })
    }

    /// Per-monitor bounds in virtual-desktop coordinates (`&self`).
    pub fn monitor_rects_ref(&self) -> Result<Vec<DesktopRect>, CaptureError> {
        let st = self.inner.lock();
        Ok(xinerama_monitor_rects(&st))
    }

    /// Monitor sizes (`&self`).
    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, CaptureError> {
        Ok(self
            .monitor_rects_ref()?
            .into_iter()
            .map(|r| (r.w, r.h))
            .collect())
    }
}

fn xinerama_monitor_rects(st: &X11State) -> Vec<DesktopRect> {
    let fallback = DesktopRect {
        x: 0,
        y: 0,
        w: st.width,
        h: st.height,
    };
    xinerama_monitor_rects_on(st.display, fallback)
}

/// Virtual-desktop monitor rects from Xinerama (same space as the X11 outline).
/// Used by portal capture to place PipeWire streams on XWayland layouts,
/// and by General seeding when ScreenCast is not ready yet.
pub(crate) fn query_x11_monitor_rects() -> Vec<DesktopRect> {
    // SAFETY: connection is null-checked; closed before return; not registered as a
    // long-lived secondary display.
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return Vec::new();
        }
        let screen = x11::xlib::XDefaultScreen(display);
        let fallback = DesktopRect {
            x: 0,
            y: 0,
            w: XDisplayWidth(display, screen),
            h: XDisplayHeight(display, screen),
        };
        let rects = xinerama_monitor_rects_on(display, fallback);
        XCloseDisplay(display);
        rects
    }
}

#[cfg(feature = "portal-capture")]
pub(crate) use compositor_kick::CompositorKick;

/// 0-alpha overlay that forces GNOME to damage `rect` so ScreenCast copies the
/// current stage (PipeWire framerate is 0/1, emit-on-damage).
#[cfg(feature = "portal-capture")]
mod compositor_kick {
    use super::*;
    use std::os::raw::c_int;
    use x11::xlib::{
        AllocNone, CWBackPixel, CWBorderPixel, CWColormap, CWOverrideRedirect, Display,
        InputOutput, True, TrueColor, VisualClassMask, VisualDepthMask, VisualScreenMask,
        XCreateColormap, XCreateWindow, XDestroyWindow, XFlush, XFreeColormap, XGetVisualInfo,
        XMapWindow, XSetWindowAttributes, XSync, XUnmapWindow, XVisualInfo,
    };

    pub(crate) struct CompositorKick {
        display: *mut _XDisplay,
        window: u64,
        colormap: u64,
    }

    impl CompositorKick {
        pub(crate) fn map(rect: DesktopRect) -> Option<Self> {
            if rect.w <= 1 || rect.h <= 1 {
                return None;
            }
            // SAFETY: `XOpenDisplay` is null-checked; failures close the connection
            // before return; `Drop` unmaps/destroys/closes exactly once.
            unsafe { map_compositor_kick(rect) }
        }
    }

    impl Drop for CompositorKick {
        fn drop(&mut self) {
            unsafe {
                if self.display.is_null() {
                    return;
                }
                if self.window != 0 {
                    XUnmapWindow(self.display, self.window);
                    XDestroyWindow(self.display, self.window);
                }
                if self.colormap != 0 {
                    XFreeColormap(self.display, self.colormap);
                }
                XFlush(self.display);
                XCloseDisplay(self.display);
                self.display = ptr::null_mut();
                self.window = 0;
                self.colormap = 0;
            }
        }
    }

    unsafe fn map_compositor_kick(rect: DesktopRect) -> Option<CompositorKick> {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return None;
        }
        let screen = x11::xlib::XDefaultScreen(display);
        let root = XDefaultRootWindow(display);
        let Some((visual, depth)) = find_argb_visual(display, screen) else {
            XCloseDisplay(display);
            return None;
        };
        let colormap = XCreateColormap(display, root, visual, AllocNone);
        let mut attrs: XSetWindowAttributes = std::mem::zeroed();
        attrs.colormap = colormap;
        attrs.background_pixel = 0;
        attrs.border_pixel = 0;
        attrs.override_redirect = True;
        let mask = CWColormap | CWBackPixel | CWBorderPixel | CWOverrideRedirect;
        let window = XCreateWindow(
            display,
            root,
            rect.x,
            rect.y,
            rect.w.max(1) as u32,
            rect.h.max(1) as u32,
            0,
            depth,
            InputOutput as u32,
            visual,
            mask,
            &mut attrs,
        );
        if window == 0 {
            XFreeColormap(display, colormap);
            XCloseDisplay(display);
            return None;
        }
        XMapWindow(display, window);
        XFlush(display);
        XSync(display, 0);
        Some(CompositorKick {
            display,
            window,
            colormap,
        })
    }

    unsafe fn find_argb_visual(
        display: *mut Display,
        screen: c_int,
    ) -> Option<(*mut x11::xlib::Visual, c_int)> {
        let mut tmpl = std::mem::zeroed::<XVisualInfo>();
        tmpl.screen = screen;
        tmpl.depth = 32;
        tmpl.class = TrueColor;
        let mut n = 0;
        let infos = XGetVisualInfo(
            display,
            VisualScreenMask | VisualDepthMask | VisualClassMask,
            &mut tmpl,
            &mut n,
        );
        if infos.is_null() || n <= 0 {
            return None;
        }
        let slice = std::slice::from_raw_parts(infos, n as usize);
        let found = slice.iter().find(|info| {
            let rgb = info.red_mask | info.green_mask | info.blue_mask;
            rgb != 0 && rgb != 0xFFFF_FFFF
        });
        let result = found.map(|info| (info.visual, info.depth));
        XFree(infos as *mut c_void);
        result
    }
}

pub(crate) fn xinerama_monitor_rects_on(
    display: *mut _XDisplay,
    fallback: DesktopRect,
) -> Vec<DesktopRect> {
    // SAFETY: `display` is a live connection; `screens` is null/`count`-checked
    // before the slice is built, and `XFree` releases the Xinerama-allocated array.
    unsafe {
        if XineramaIsActive(display) == 0 {
            return vec![fallback];
        }
        let mut count = 0;
        let screens = XineramaQueryScreens(display, &mut count);
        if screens.is_null() || count <= 0 {
            return vec![fallback];
        }
        let slice =
            std::slice::from_raw_parts(screens as *const XineramaScreenInfo, count as usize);
        let mut rects: Vec<DesktopRect> = slice
            .iter()
            .map(|s| DesktopRect {
                x: s.x_org as i32,
                y: s.y_org as i32,
                w: s.width as i32,
                h: s.height as i32,
            })
            .filter(|r| r.w > 0 && r.h > 0)
            .collect();
        XFree(screens as *mut c_void);
        if rects.is_empty() {
            vec![fallback]
        } else {
            rects.sort_by_key(|r| (r.x, r.y, r.w, r.h));
            rects
        }
    }
}

impl Drop for X11State {
    fn drop(&mut self) {
        // SAFETY: `self.display` is null-checked; closing it here is sound since
        // no other reference to this `X11State` (and thus this display) can exist.
        unsafe {
            if !self.display.is_null() {
                XCloseDisplay(self.display);
                self.display = ptr::null_mut();
            }
        }
    }
}

/// Primary monitor DPI scale from `Xft.dpi` (`dpi / 96`), else `1.0`.
/// Returns `None` when the display cannot be opened.
pub(crate) fn primary_monitor_scale() -> Option<f32> {
    if let Ok(cap) = X11Capturer::open() {
        let st = cap.inner.lock();
        return Some(xft_dpi_scale(st.display));
    }
    // SAFETY: `display` is null-checked before use, and `XCloseDisplay` runs
    // exactly once after `xft_dpi_scale` returns, before this pointer is dropped.
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return None;
        }
        let scale = xft_dpi_scale(display);
        XCloseDisplay(display);
        Some(scale)
    }
}

fn xft_dpi_scale(display: *mut _XDisplay) -> f32 {
    // SAFETY: `display` is a live connection owned by the caller; `res` is
    // null-checked before `CStr::from_ptr`, and the string is owned by Xlib
    // (not freed here), matching Xlib's resource-manager string contract.
    unsafe {
        let res = XResourceManagerString(display);
        if res.is_null() {
            return 1.0;
        }
        let Ok(s) = CStr::from_ptr(res).to_str() else {
            return 1.0;
        };
        for line in s.split('\n') {
            let line = line.trim();
            let Some(rest) = line
                .strip_prefix("Xft.dpi:")
                .or_else(|| line.strip_prefix("Xft.dpi:\t"))
            else {
                continue;
            };
            if let Ok(dpi) = rest.trim().parse::<f32>() {
                if dpi > 0.0 {
                    return dpi / 96.0;
                }
            }
        }
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_or_skip() {
        // CI / headless: open may fail — that's ok.
        let _ = X11Capturer::open();
    }
}
