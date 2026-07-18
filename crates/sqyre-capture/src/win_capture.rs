//! Windows GDI absolute virtual-desktop capture.

use crate::error::CaptureError;
use image::RgbaImage;
use parking_lot::Mutex;
use sqyre_executor::{DesktopRect, RgbCapture, ScreenCapturer};
use std::sync::{Arc, OnceLock};
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    EnumDisplayMonitors, GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
    DIB_RGB_COLORS, HDC, HGDIOBJ, HMONITOR, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

/// Shared GDI desktop capture (serialized via mutex).
pub struct OsCapturer {
    inner: Mutex<()>,
}

/// Process-wide capturer for UI offload (cloned via [`Arc`]; access serialized by inner Mutex).
static SHARED_UI_CAPTURER: OnceLock<Result<Arc<OsCapturer>, String>> = OnceLock::new();

/// Shared capturer for UI-thread offload (preview tooltips, AutoPic, etc.).
pub fn shared_capturer() -> Result<Arc<OsCapturer>, String> {
    match SHARED_UI_CAPTURER
        .get_or_init(|| OsCapturer::open().map(Arc::new).map_err(|e| e.to_string()))
    {
        Ok(c) => Ok(Arc::clone(c)),
        Err(e) => Err(e.clone()),
    }
}

impl OsCapturer {
    pub fn open() -> Result<Self, CaptureError> {
        let vb = virtual_screen_metrics()?;
        if vb.w <= 0 || vb.h <= 0 {
            return Err(CaptureError::OpenDisplay);
        }
        Ok(Self {
            inner: Mutex::new(()),
        })
    }

    /// Capture a desktop rect (`&self` — safe to call via [`Arc`] from worker threads).
    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, String> {
        let _guard = self.inner.lock();
        capture_rect_gdi(rect)
    }

    /// Capture RGB directly (no alpha channel / no second conversion pass).
    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, String> {
        let _guard = self.inner.lock();
        let rgba = capture_rect_gdi(rect)?;
        Ok(RgbCapture::from_rgba(&rgba))
    }

    /// Virtual desktop bounds (`&self`).
    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, String> {
        let _guard = self.inner.lock();
        virtual_screen_metrics().map_err(Into::into)
    }

    /// Monitor sizes (`&self`).
    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, String> {
        let _guard = self.inner.lock();
        enum_monitor_sizes().map_err(Into::into)
    }
}

impl ScreenCapturer for OsCapturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, String> {
        if display_index != 0 {
            return Err(CaptureError::UnsupportedDisplay(display_index).into());
        }
        let vb = self.virtual_bounds_ref()?;
        self.capture_rect_ref(vb)
    }

    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, String> {
        self.capture_rect_ref(rect)
    }

    fn capture_rect_rgb(&mut self, rect: DesktopRect) -> Result<RgbCapture, String> {
        self.capture_rect_rgb_ref(rect)
    }

    fn virtual_bounds(&mut self) -> Result<DesktopRect, String> {
        self.virtual_bounds_ref()
    }

    fn monitor_sizes(&mut self) -> Result<Vec<(i32, i32)>, String> {
        self.monitor_sizes_ref()
    }
}

/// [`ScreenCapturer`] over a shared [`Arc`] capturer (macro run thread).
pub struct SharedRunCapturer(pub Arc<OsCapturer>);

impl ScreenCapturer for SharedRunCapturer {
    fn capture_monitor(&mut self, display_index: i32) -> Result<RgbaImage, String> {
        if display_index != 0 {
            return Err(CaptureError::UnsupportedDisplay(display_index).into());
        }
        let vb = self.0.virtual_bounds_ref()?;
        self.0.capture_rect_ref(vb)
    }

    fn capture_rect(&mut self, rect: DesktopRect) -> Result<RgbaImage, String> {
        self.0.capture_rect_ref(rect)
    }

    fn capture_rect_rgb(&mut self, rect: DesktopRect) -> Result<RgbCapture, String> {
        self.0.capture_rect_rgb_ref(rect)
    }

    fn virtual_bounds(&mut self) -> Result<DesktopRect, String> {
        self.0.virtual_bounds_ref()
    }

    fn monitor_sizes(&mut self) -> Result<Vec<(i32, i32)>, String> {
        self.0.monitor_sizes_ref()
    }
}

fn virtual_screen_metrics() -> Result<DesktopRect, CaptureError> {
    unsafe {
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        if w <= 0 || h <= 0 {
            return Err(CaptureError::OpenDisplay);
        }
        Ok(DesktopRect { x, y, w, h })
    }
}

fn capture_rect_gdi(rect: DesktopRect) -> Result<RgbaImage, String> {
    if rect.is_empty() {
        return Err(CaptureError::EmptyRect.into());
    }
    let w = rect.w as u32;
    let h = rect.h as u32;

    unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.is_invalid() {
            return Err(CaptureError::Gdi("GetDC failed".into()).into());
        }

        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.is_invalid() {
            ReleaseDC(None, screen_dc);
            return Err(CaptureError::Gdi("CreateCompatibleDC failed".into()).into());
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, rect.w, rect.h);
        if bitmap.is_invalid() {
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            return Err(CaptureError::Gdi("CreateCompatibleBitmap failed".into()).into());
        }

        let old = SelectObject(mem_dc, HGDIOBJ::from(bitmap));
        let blit_ok = BitBlt(
            mem_dc,
            0,
            0,
            rect.w,
            rect.h,
            Some(screen_dc),
            rect.x,
            rect.y,
            SRCCOPY,
        );
        if blit_ok.is_err() {
            SelectObject(mem_dc, old);
            let _ = DeleteObject(HGDIOBJ::from(bitmap));
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            return Err(CaptureError::GetImage {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
            }
            .into());
        }

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: rect.w,
                biHeight: -rect.h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0, // BI_RGB
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };

        let mut bgra = vec![0u8; (w as usize).saturating_mul(h as usize).saturating_mul(4)];
        let lines = GetDIBits(
            mem_dc,
            bitmap,
            0,
            h,
            Some(bgra.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(mem_dc, old);
        let _ = DeleteObject(HGDIOBJ::from(bitmap));
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);

        if lines == 0 {
            return Err(CaptureError::GetImage {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
            }
            .into());
        }

        // BGRA → RGBA
        for pixel in bgra.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }

        RgbaImage::from_raw(w, h, bgra)
            .ok_or_else(|| CaptureError::Message("invalid RGBA buffer".into()).into())
    }
}

fn enum_monitor_sizes() -> Result<Vec<(i32, i32)>, CaptureError> {
    let mut sizes: Vec<(i32, i32)> = Vec::new();
    unsafe {
        let ok = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum_proc),
            LPARAM(&mut sizes as *mut Vec<(i32, i32)> as isize),
        );
        if !ok.as_bool() || sizes.is_empty() {
            let vb = virtual_screen_metrics()?;
            return Ok(vec![(vb.w, vb.h)]);
        }
    }
    Ok(sizes)
}

unsafe extern "system" fn monitor_enum_proc(
    _monitor: HMONITOR,
    _hdc: HDC,
    lprc: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let sizes = &mut *(lparam.0 as *mut Vec<(i32, i32)>);
    if !lprc.is_null() {
        let r = *lprc;
        let w = r.right - r.left;
        let h = r.bottom - r.top;
        if w > 0 && h > 0 {
            sizes.push((w, h));
        }
    }
    BOOL(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_or_skip() {
        let _ = OsCapturer::open();
    }
}
