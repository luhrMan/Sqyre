//! Click-through squares on recorded macro points (Win32).
//!
//! Same job as the X11 markers: a topmost, click-through popup per point so the
//! marker sits on the recorded desktop pixel instead of an unpositioned viewport.

use std::sync::OnceLock;

use windows::core::w;
use windows::Win32::Foundation::{
    GetLastError, COLORREF, ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{CreateSolidBrush, DeleteObject, HGDIOBJ};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SetWindowPos, CS_HREDRAW,
    CS_VREDRAW, HTTRANSPARENT, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, WM_NCHITTEST,
    WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::outline_geometry::{marker_window, sample_marker_points, STROKE_B, STROKE_G, STROKE_R};
use crate::CaptureError;

const CLASS_NAME: windows::core::PCWSTR = w!("SqyrePointMarker");

/// One gold square per recorded point, positioned in virtual-desktop pixels.
pub struct PointMarkerOverlay {
    windows: Vec<HWND>,
    shown: usize,
}

// SAFETY: HWNDs are opaque handles. Win32 calls on them from a non-owning thread
// fail rather than cause UB, and the wndproc holds no state. Callers keep this
// on the UI thread.
unsafe impl Send for PointMarkerOverlay {}

impl PointMarkerOverlay {
    pub fn open() -> Result<Self, CaptureError> {
        ensure_class()?;
        Ok(Self {
            windows: Vec::new(),
            shown: 0,
        })
    }

    pub fn set_points(&mut self, points: &[(i32, i32)]) {
        let points = sample_marker_points(points);
        if points.is_empty() {
            self.clear();
            return;
        }
        while self.windows.len() < points.len() {
            match create_marker() {
                Ok(hwnd) => self.windows.push(hwnd),
                Err(_) => break,
            }
        }
        // SAFETY: HWNDs in `self.windows` were created by `create_marker`.
        unsafe {
            for (hwnd, &(x, y)) in self.windows.iter().zip(points.iter()) {
                let (wx, wy, w, h) = marker_window(x, y);
                place(*hwnd, wx, wy, w, h);
            }
            for hwnd in self.windows.iter().skip(points.len()).take(self.shown) {
                place(*hwnd, -20, -20, 1, 1);
            }
        }
        self.shown = points.len().min(self.windows.len());
    }

    pub fn clear(&mut self) {
        if self.shown == 0 {
            return;
        }
        // SAFETY: HWNDs were created by `create_marker`.
        unsafe {
            for hwnd in self.windows.iter().take(self.shown) {
                place(*hwnd, -20, -20, 1, 1);
            }
        }
        self.shown = 0;
    }
}

impl Drop for PointMarkerOverlay {
    fn drop(&mut self) {
        // SAFETY: destroys HWNDs this overlay created.
        unsafe {
            for &hwnd in &self.windows {
                if !hwnd.is_invalid() && hwnd != HWND::default() {
                    let _ = DestroyWindow(hwnd);
                }
            }
        }
    }
}

fn ensure_class() -> Result<(), CaptureError> {
    static CLASS: OnceLock<Result<(), CaptureError>> = OnceLock::new();
    CLASS
        .get_or_init(|| {
            // SAFETY: RegisterClassW with a process-local class; the brush lives
            // for the process lifetime as the class background.
            unsafe {
                let module = GetModuleHandleW(None).map_err(|e| CaptureError::Win32 {
                    api: "GetModuleHandleW",
                    detail: e.to_string(),
                })?;
                let brush = CreateSolidBrush(COLORREF(
                    u32::from(STROKE_R) | (u32::from(STROKE_G) << 8) | (u32::from(STROKE_B) << 16),
                ));
                if brush.is_invalid() {
                    return Err(CaptureError::Message(
                        "CreateSolidBrush failed for point markers".into(),
                    ));
                }
                let wc = WNDCLASSW {
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(marker_wnd_proc),
                    hInstance: module.into(),
                    hbrBackground: brush,
                    lpszClassName: CLASS_NAME,
                    ..Default::default()
                };
                let atom = RegisterClassW(&wc);
                if atom == 0 {
                    let err = GetLastError();
                    let _ = DeleteObject(HGDIOBJ::from(brush));
                    if err == ERROR_CLASS_ALREADY_EXISTS {
                        return Ok(());
                    }
                    return Err(CaptureError::Win32 {
                        api: "RegisterClassW",
                        detail: format!("{err:?}"),
                    });
                }
            }
            Ok(())
        })
        .clone()
}

fn create_marker() -> Result<HWND, CaptureError> {
    // SAFETY: class is registered; creates an unowned popup HWND.
    unsafe {
        let module = GetModuleHandleW(None).map_err(|e| CaptureError::Win32 {
            api: "GetModuleHandleW",
            detail: e.to_string(),
        })?;
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            CLASS_NAME,
            w!(""),
            WS_POPUP,
            -20,
            -20,
            1,
            1,
            None,
            None,
            Some(module.into()),
            None,
        )
        .map_err(|e| CaptureError::Win32 {
            api: "CreateWindowExW",
            detail: e.to_string(),
        })?;
        if hwnd.is_invalid() {
            return Err(CaptureError::Message(
                "CreateWindowExW returned null HWND".into(),
            ));
        }
        Ok(hwnd)
    }
}

/// # Safety
/// `hwnd` is a live marker window created by `create_marker`.
unsafe fn place(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    // SAFETY: caller guarantees `hwnd` is live.
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            w.max(1),
            h.max(1),
            SWP_SHOWWINDOW | SWP_NOACTIVATE,
        );
    }
}

/// # Safety
/// Called by the system on a window of class `SqyrePointMarker`.
unsafe extern "system" fn marker_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: the system calls this with a valid HWND of our class.
    unsafe {
        if msg == WM_NCHITTEST {
            return LRESULT(HTTRANSPARENT as isize);
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
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
