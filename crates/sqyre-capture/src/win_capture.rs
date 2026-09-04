//! Windows GDI absolute virtual-desktop capture.

use crate::error::CaptureError;
use crate::pixel_convert::zpixmap_to_rgb;
use image::RgbaImage;
use parking_lot::Mutex;
use sqyre_ports::{DesktopRect, RgbCapture};
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    EnumDisplayMonitors, GetDC, GetDIBits, GetMonitorInfoW, MonitorFromPoint, ReleaseDC,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HDC, HGDIOBJ, HMONITOR,
    MONITORINFO, MONITOR_DEFAULTTOPRIMARY, SRCCOPY,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForMonitor, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

/// Shared GDI desktop capture (serialized via mutex).
pub struct OsCapturer {
    inner: Mutex<()>,
}

crate::define_shared_run_capturer!();

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
    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        let _guard = self.inner.lock();
        capture_rect_gdi(rect)
    }

    /// GDI capture is always synchronous; identical to [`Self::capture_rect_ref`].
    pub fn capture_rect_fresh_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        self.capture_rect_ref(rect)
    }

    /// Capture RGB directly (no alpha channel / no second conversion pass).
    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        let _guard = self.inner.lock();
        capture_rect_rgb_gdi(rect)
    }

    /// GDI capture is always synchronous; identical to [`Self::capture_rect_rgb_ref`].
    pub fn capture_rect_rgb_fresh_ref(
        &self,
        rect: DesktopRect,
    ) -> Result<RgbCapture, CaptureError> {
        self.capture_rect_rgb_ref(rect)
    }

    /// Virtual desktop bounds (`&self`).
    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, CaptureError> {
        let _guard = self.inner.lock();
        virtual_screen_metrics()
    }

    /// Per-monitor bounds in virtual-desktop coordinates (`&self`).
    pub fn monitor_rects_ref(&self) -> Result<Vec<DesktopRect>, CaptureError> {
        let _guard = self.inner.lock();
        enum_monitor_rects()
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

fn virtual_screen_metrics() -> Result<DesktopRect, CaptureError> {
    // SAFETY: GetSystemMetrics with SM_*VIRTUALSCREEN needs no live handles; values are process-global.
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

fn capture_rect_gdi(rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
    let (mut bgra, w, h) = capture_rect_bgra(rect)?;

    // Sequential BGRA→RGBA: avoid rayon/pulp here. Preview captures can run on the
    // UI thread (glow), and off-thread SIMD pools have hard-crashed some Windows
    // setups without a Rust panic hook.
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }

    RgbaImage::from_raw(w, h, bgra)
        .ok_or_else(|| CaptureError::Message("invalid RGBA buffer".into()))
}

/// Capture RGB directly from the GDI BGRA buffer (no RGBA intermediate / no second
/// allocation pass — mirrors the X11 ZPixmap→RGB path for search/OCR hot paths).
fn capture_rect_rgb_gdi(rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
    let (bgra, w, h) = capture_rect_bgra(rect)?;
    let data = zpixmap_to_rgb(&bgra, w, h, 4, 0).map_err(CaptureError::Message)?;
    Ok(RgbCapture {
        width: w,
        height: h,
        data,
    })
}

/// BitBlt the desktop rect into a compatible bitmap and read it back as tightly
/// packed top-down BGRA via `GetDIBits`. Shared by the RGBA and RGB-direct paths.
fn capture_rect_bgra(rect: DesktopRect) -> Result<(Vec<u8>, u32, u32), CaptureError> {
    if rect.is_empty() {
        return Err(CaptureError::EmptyRect);
    }
    let w = rect.w as u32;
    let h = rect.h as u32;

    // SAFETY: every GDI object created here is released on all return paths; GetDIBits writes
    // into `bgra`, which is sized for tightly packed w×h×4 BGRA.
    unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.is_invalid() {
            return Err(CaptureError::Gdi("GetDC failed".into()));
        }

        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.is_invalid() {
            ReleaseDC(None, screen_dc);
            return Err(CaptureError::Gdi("CreateCompatibleDC failed".into()));
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, rect.w, rect.h);
        if bitmap.is_invalid() {
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            return Err(CaptureError::Gdi("CreateCompatibleBitmap failed".into()));
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
            });
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
            });
        }

        Ok((bgra, w, h))
    }
}

fn enum_monitor_rects() -> Result<Vec<DesktopRect>, CaptureError> {
    let mut rects: Vec<DesktopRect> = Vec::new();
    // SAFETY: callback only mutates `rects` via lparam for the duration of EnumDisplayMonitors.
    unsafe {
        let ok = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum_proc),
            LPARAM(&mut rects as *mut Vec<DesktopRect> as isize),
        );
        if !ok.as_bool() || rects.is_empty() {
            let vb = virtual_screen_metrics()?;
            return Ok(vec![vb]);
        }
    }
    rects.sort_by_key(|r| (r.x, r.y, r.w, r.h));
    // Match Linux: Windows Settings primary display is Sqyre Monitor 1.
    Ok(order_primary_first(rects))
}

/// Virtual-desktop rect of the Windows primary monitor (`MONITOR_DEFAULTTOPRIMARY`).
pub(crate) fn query_windows_primary_rect() -> Option<DesktopRect> {
    // SAFETY: MonitorFromPoint / GetMonitorInfoW use stack out-params; no GDI handle retained.
    unsafe {
        let mon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        if mon.is_invalid() {
            return None;
        }
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(mon, &mut mi).as_bool() {
            return None;
        }
        let r = mi.rcMonitor;
        let w = r.right - r.left;
        let h = r.bottom - r.top;
        if w > 1 && h > 1 {
            Some(DesktopRect {
                x: r.left,
                y: r.top,
                w,
                h,
            })
        } else {
            None
        }
    }
}

/// Put the Windows primary display first, then keep remaining L→R.
pub(crate) fn order_primary_first(rects: Vec<DesktopRect>) -> Vec<DesktopRect> {
    crate::with_primary_monitor_first(rects, query_windows_primary_rect())
}

/// Primary monitor DPI scale (`dpi / 96`).
pub(crate) fn primary_monitor_scale() -> Option<f32> {
    // SAFETY: MonitorFromPoint / GetDpiForMonitor use stack out-params; no GDI handle is retained.
    unsafe {
        let mon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        if mon.is_invalid() {
            return None;
        }
        let mut dpix = 0u32;
        let mut dpiy = 0u32;
        GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dpix, &mut dpiy).ok()?;
        let _ = dpiy;
        if dpix == 0 {
            return None;
        }
        Some(dpix as f32 / 96.0)
    }
}

/// Per-Monitor DPI awareness V2 so GDI / metrics / input use physical pixels.
pub(crate) fn enable_per_monitor_dpi_v2() {
    // Ignore failure (already set, or older Windows) — best-effort.
    // SAFETY: the awareness constant is a documented process-wide value; no handles involved.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

unsafe extern "system" fn monitor_enum_proc(
    _monitor: HMONITOR,
    _hdc: HDC,
    lprc: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let rects = &mut *(lparam.0 as *mut Vec<DesktopRect>);
    if !lprc.is_null() {
        let r = *lprc;
        let w = r.right - r.left;
        let h = r.bottom - r.top;
        if w > 0 && h > 0 {
            rects.push(DesktopRect {
                x: r.left,
                y: r.top,
                w,
                h,
            });
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
