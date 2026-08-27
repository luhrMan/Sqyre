//! Live search-area outline via Win32 popup edge windows (no desktop snapshot).
//!
//! Four thin topmost, click-through windows form a hollow gold rectangle so the
//! desktop stays visible underneath — mirrors the Linux X11 outline.

use std::sync::OnceLock;

use windows::core::w;
use windows::Win32::Foundation::{
    GetLastError, COLORREF, ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, InvalidateRect, UpdateWindow, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SetWindowPos, CS_HREDRAW,
    CS_VREDRAW, HTTRANSPARENT, HWND_TOPMOST, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_SHOWWINDOW, WM_NCHITTEST, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::outline_geometry::{
    edge_placements, outline_should_clear, STROKE_B, STROKE_G, STROKE_R,
};
pub use crate::outline_rect::OutlineRect;
use crate::CaptureError;

const CLASS_NAME: windows::core::PCWSTR = w!("SqyreSelectionOutline");

/// Four edge windows forming a hollow rectangle on the virtual desktop.
pub struct SelectionOutline {
    edges: [HWND; 4],
    mapped: bool,
    last: Option<OutlineRect>,
}

// HWND handles: all use stays on the owning poller thread.
unsafe impl Send for SelectionOutline {}

impl SelectionOutline {
    pub fn open() -> Result<Self, CaptureError> {
        ensure_class()?;
        let mut edges = [HWND::default(); 4];
        for i in 0..edges.len() {
            match create_edge() {
                Ok(hwnd) => edges[i] = hwnd,
                Err(e) => {
                    destroy_edges(&edges);
                    return Err(e);
                }
            }
        }
        Ok(Self {
            edges,
            mapped: false,
            last: None,
        })
    }

    /// Show/update the outline for absolute desktop corners.
    pub fn set_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        let rect = OutlineRect::normalize(left, top, right, bottom);
        if outline_should_clear(rect) {
            self.clear();
            return;
        }
        if self.last == Some(rect) && self.mapped {
            return;
        }
        // SAFETY: edge HWNDs were created in `open` and remain valid until Drop.
        unsafe {
            place_edges(&self.edges, rect);
        }
        self.mapped = true;
        self.last = Some(rect);
    }

    pub fn clear(&mut self) {
        if !self.mapped && self.last.is_none() {
            return;
        }
        // SAFETY: edge HWNDs were created in `open` and remain valid until Drop.
        unsafe {
            for &hwnd in &self.edges {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_HIDEWINDOW | SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOMOVE,
                );
            }
        }
        self.mapped = false;
        self.last = None;
    }

    pub fn is_active(&self) -> bool {
        self.last.is_some()
    }
}

impl Drop for SelectionOutline {
    fn drop(&mut self) {
        destroy_edges(&self.edges);
    }
}

fn ensure_class() -> Result<(), CaptureError> {
    static CLASS: OnceLock<Result<(), CaptureError>> = OnceLock::new();
    CLASS
        .get_or_init(|| {
            // SAFETY: RegisterClassW with a process-local class; brush lives for process life.
            unsafe {
                let module = GetModuleHandleW(None)
                    .map_err(|e| CaptureError::Message(format!("GetModuleHandleW failed: {e}")))?;
                let brush = CreateSolidBrush(COLORREF(
                    u32::from(STROKE_R) | (u32::from(STROKE_G) << 8) | (u32::from(STROKE_B) << 16),
                ));
                if brush.is_invalid() {
                    return Err(CaptureError::Message(
                        "CreateSolidBrush failed for selection outline".into(),
                    ));
                }
                let wc = WNDCLASSW {
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(outline_wnd_proc),
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
                    return Err(CaptureError::Message(format!(
                        "RegisterClassW failed: {err:?}"
                    )));
                }
                // Leak the brush: it remains the class background for the process lifetime.
            }
            Ok(())
        })
        .clone()
}

fn create_edge() -> Result<HWND, CaptureError> {
    // SAFETY: class registered; creates an unowned popup HWND for this outline.
    unsafe {
        let module = GetModuleHandleW(None)
            .map_err(|e| CaptureError::Message(format!("GetModuleHandleW failed: {e}")))?;
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            CLASS_NAME,
            w!(""),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(module.into()),
            None,
        )
        .map_err(|e| CaptureError::Message(format!("CreateWindowExW failed: {e}")))?;
        if hwnd.is_invalid() {
            return Err(CaptureError::Message(
                "CreateWindowExW returned null HWND".into(),
            ));
        }
        Ok(hwnd)
    }
}

fn destroy_edges(edges: &[HWND; 4]) {
    // SAFETY: destroys HWNDs we created; null/default HWNDs are skipped.
    unsafe {
        for &hwnd in edges {
            if !hwnd.is_invalid() && hwnd != HWND::default() {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}

unsafe fn place_edges(edges: &[HWND; 4], r: OutlineRect) {
    for (&hwnd, &(x, y, w, h)) in edges.iter().zip(edge_placements(r).iter()) {
        configure(hwnd, x, y, w, h);
    }
}

unsafe fn configure(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    let _ = SetWindowPos(
        hwnd,
        Some(HWND_TOPMOST),
        x,
        y,
        w.max(1),
        h.max(1),
        SWP_SHOWWINDOW | SWP_NOACTIVATE,
    );
    let _ = InvalidateRect(Some(hwnd), None, true);
    let _ = UpdateWindow(hwnd);
}

unsafe extern "system" fn outline_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCHITTEST {
        // Click-through so screen-click recording still receives the second corner.
        return LRESULT(HTTRANSPARENT as isize);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::SelectionOutline;

    #[test]
    fn open_or_skip() {
        let _ = SelectionOutline::open();
    }
}
