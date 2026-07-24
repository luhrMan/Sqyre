//! Linux X11 absolute virtual-desktop capture.

use crate::error::CaptureError;
use crate::pixel_convert::{zpixmap_to_rgb, zpixmap_to_rgba};
use image::RgbaImage;
use parking_lot::Mutex;
use sqyre_executor::{DesktopRect, RgbCapture};
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;
use x11::xinerama::{XineramaIsActive, XineramaQueryScreens, XineramaScreenInfo};
use x11::xlib::{
    XCloseDisplay, XDefaultRootWindow, XDestroyImage, XDisplayHeight, XDisplayWidth, XFree,
    XGetImage, XOpenDisplay, XQueryPointer, XResourceManagerString, ZPixmap, _XDisplay,
};

const ALLPLANES: u64 = !0;

/// Shared X11 display connection (public type [`OsCapturer`]; mutex serializes access).
pub struct OsCapturer {
    inner: Mutex<X11State>,
}

struct X11State {
    display: *mut _XDisplay,
    root: u64,
    width: i32,
    height: i32,
}

// X11 display pointer: we serialize all access via Mutex.
unsafe impl Send for X11State {}

crate::define_shared_run_capturer!();

impl OsCapturer {
    pub fn open() -> Result<Self, CaptureError> {
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
    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, String> {
        self.with_zpixmap(rect, |data, w, h, bpp, stride| {
            zpixmap_to_rgba(data, w, h, bpp, stride)
        })
    }

    /// Capture RGB directly (no alpha channel / no second conversion pass).
    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, String> {
        self.with_zpixmap(rect, |data, w, h, bpp, stride| {
            let data = zpixmap_to_rgb(data, w, h, bpp, stride)?;
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
        convert: impl FnOnce(&[u8], u32, u32, usize, usize) -> Result<T, String>,
    ) -> Result<T, String> {
        if rect.is_empty() {
            return Err(CaptureError::EmptyRect.into());
        }
        let st = self.inner.lock();
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
                }
                .into());
            }
            let img = &*ximage;
            let w = img.width as u32;
            let h = img.height as u32;
            let bpp = (img.bits_per_pixel / 8) as usize;
            if bpp < 3 {
                let bits = img.bits_per_pixel;
                XDestroyImage(ximage);
                return Err(CaptureError::BitsPerPixel(bits).into());
            }
            let stride = img.bytes_per_line as usize;
            let data_len = stride.saturating_mul(h as usize);
            let data = std::slice::from_raw_parts(img.data as *const u8, data_len);
            let out = convert(data, w, h, bpp, stride).inspect_err(|_e| {
                XDestroyImage(ximage);
            })?;
            XDestroyImage(ximage);
            Ok(out)
        }
    }

    /// Virtual desktop bounds (`&self`).
    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, String> {
        let st = self.inner.lock();
        Ok(DesktopRect {
            x: 0,
            y: 0,
            w: st.width,
            h: st.height,
        })
    }

    /// Monitor sizes (`&self`).
    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, String> {
        let st = self.inner.lock();
        unsafe {
            if XineramaIsActive(st.display) == 0 {
                return Ok(vec![(st.width, st.height)]);
            }
            let mut count = 0;
            let screens = XineramaQueryScreens(st.display, &mut count);
            if screens.is_null() || count <= 0 {
                return Ok(vec![(st.width, st.height)]);
            }
            let slice =
                std::slice::from_raw_parts(screens as *const XineramaScreenInfo, count as usize);
            let sizes: Vec<(i32, i32)> = slice
                .iter()
                .map(|s| (s.width as i32, s.height as i32))
                .filter(|(w, h)| *w > 0 && *h > 0)
                .collect();
            XFree(screens as *mut c_void);
            if sizes.is_empty() {
                Ok(vec![(st.width, st.height)])
            } else {
                Ok(sizes)
            }
        }
    }
}

impl Drop for X11State {
    fn drop(&mut self) {
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
    if let Ok(cap) = shared_capturer() {
        let st = cap.inner.lock();
        return Some(xft_dpi_scale(st.display));
    }
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
        let _ = OsCapturer::open();
    }
}
