//! Linux X11 absolute virtual-desktop capture (mirrors robotgo CaptureImg).

use image::{Rgba, RgbaImage};
use sqyre_executor::{DesktopRect, ScreenCapturer};
use std::ptr;
use std::sync::Mutex;
use x11::xlib::{
    XCloseDisplay, XDefaultRootWindow, XDestroyImage, XDisplayHeight, XDisplayWidth, XGetImage,
    XOpenDisplay, XQueryPointer, ZPixmap, _XDisplay,
};

const ALLPLANES: u64 = !0;

/// Shared X11 display connection (not Send across threads freely — mutex serializes).
pub struct X11Capturer {
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

impl X11Capturer {
    pub fn open() -> Result<Self, String> {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err("XOpenDisplay failed (need X11, not Wayland-only)".into());
            }
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
    pub fn pointer_position(&self) -> Result<(i32, i32), String> {
        let st = self.inner.lock().map_err(|e| e.to_string())?;
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
                return Err("XQueryPointer failed".into());
            }
            Ok((root_x, root_y))
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

impl ScreenCapturer for X11Capturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, String> {
        if display_index != 0 {
            return Err(format!(
                "X11Capturer: only display 0 supported for now (got {display_index})"
            ));
        }
        let vb = self.virtual_bounds()?;
        self.capture_rect(vb)
    }

    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, String> {
        if rect.is_empty() {
            return Err("empty capture rect".into());
        }
        let st = self.inner.lock().map_err(|e| e.to_string())?;
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
                return Err(format!(
                    "XGetImage failed for {},{},{},{}",
                    rect.x, rect.y, rect.w, rect.h
                ));
            }
            let img = &*ximage;
            let w = img.width as u32;
            let h = img.height as u32;
            let bpp = (img.bits_per_pixel / 8) as usize;
            if bpp < 3 {
                XDestroyImage(ximage);
                return Err(format!("unexpected bits_per_pixel {}", img.bits_per_pixel));
            }
            let data = std::slice::from_raw_parts(img.data as *const u8, (w * h) as usize * bpp);
            let mut out = RgbaImage::new(w, h);
            // X11 ZPixmap is typically BGRA on little-endian.
            for y in 0..h {
                for x in 0..w {
                    let i = ((y * w + x) as usize) * bpp;
                    let b = data[i];
                    let g = data[i + 1];
                    let r = data[i + 2];
                    let a = if bpp >= 4 { data[i + 3] } else { 255 };
                    out.put_pixel(x, y, Rgba([r, g, b, a]));
                }
            }
            XDestroyImage(ximage);
            Ok(out)
        }
    }

    fn virtual_bounds(&mut self) -> Result<DesktopRect, String> {
        let st = self.inner.lock().map_err(|e| e.to_string())?;
        Ok(DesktopRect {
            x: 0,
            y: 0,
            w: st.width,
            h: st.height,
        })
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
