//! Linux X11 absolute virtual-desktop capture.

use crate::error::CaptureError;
use crate::pixel_convert::{zpixmap_to_rgb, zpixmap_to_rgba};
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

/// Shared X11 display connection (mutex serializes access).
pub struct X11Capturer {
    inner: Mutex<X11State>,
}

struct X11State {
    display: *mut _XDisplay,
    root: u64,
    width: i32,
    height: i32,
}

// SAFETY: the raw display pointer is only ever touched while `X11Capturer::inner`
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
        // `X11Capturer::open`; all out-params are stack-local and correctly sized.
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
                (data, w, h, bpp, stride)
            }
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
    // SAFETY: `st.display` is the live display; `screens` is null/`count`-checked
    // before the slice is built, and `XFree` releases the Xinerama-allocated array.
    unsafe {
        if XineramaIsActive(st.display) == 0 {
            return vec![fallback];
        }
        let mut count = 0;
        let screens = XineramaQueryScreens(st.display, &mut count);
        if screens.is_null() || count <= 0 {
            return vec![fallback];
        }
        let slice =
            std::slice::from_raw_parts(screens as *const XineramaScreenInfo, count as usize);
        let rects: Vec<DesktopRect> = slice
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
