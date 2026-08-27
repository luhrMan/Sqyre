//! Fullscreen Win32 mouse-owning layer for screen-click recording.
//!
//! A nearly-invisible topmost layered popup covers the virtual desktop, takes
//! foreground + capture, and clears `ClipCursor` so games that confine/relative-
//! capture the mouse cannot block Point / Color / SearchArea selection.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::OnceLock;

use windows::core::w;
use windows::Win32::Foundation::{
    GetLastError, COLORREF, ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::{
    ClipCursor, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetSystemMetrics, LoadCursorW, PeekMessageW, RegisterClassW, SetCursor, SetForegroundWindow,
    SetLayeredWindowAttributes, SetWindowPos, ShowCursor, ShowWindow, TranslateMessage, CS_HREDRAW,
    CS_VREDRAW, HWND_TOPMOST, IDC_CROSS, LWA_ALPHA, MSG, PM_REMOVE, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW,
    WM_DESTROY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_SETCURSOR, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::selection_grab::GrabPoll;
use crate::CaptureError;

const CLASS_NAME: windows::core::PCWSTR = w!("SqyreSelectionGrab");
/// Nearly invisible but still hit-tested (1/255 opacity).
const GRAB_ALPHA: u8 = 1;

static POS_X: AtomicI32 = AtomicI32::new(0);
static POS_Y: AtomicI32 = AtomicI32::new(0);
static MOVED: AtomicBool = AtomicBool::new(false);
static LEFT_CLICKS: AtomicU32 = AtomicU32::new(0);
static ESCAPE: AtomicBool = AtomicBool::new(false);

/// Fullscreen layered popup that owns the mouse while screen-click recording is armed.
pub struct SelectionGrab {
    hwnd: HWND,
    armed: bool,
}

// HWND handle: all use stays on the owning UI thread.
unsafe impl Send for SelectionGrab {}

impl SelectionGrab {
    pub fn open() -> Result<Self, CaptureError> {
        ensure_class()?;
        let bounds = virtual_screen()?;
        // SAFETY: class registered; creates an unowned popup HWND for this grab.
        let hwnd = unsafe {
            let module =
                GetModuleHandleW(None).map_err(|e| CaptureError::Message(e.to_string()))?;
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                CLASS_NAME,
                w!("Sqyre selection"),
                WS_POPUP,
                bounds.0,
                bounds.1,
                bounds.2.max(1),
                bounds.3.max(1),
                None,
                None,
                Some(module.into()),
                None,
            )
            .map_err(|e| CaptureError::Message(format!("CreateWindowExW failed: {e}")))?
        };
        if hwnd.is_invalid() {
            return Err(CaptureError::Message(
                "CreateWindowExW returned null HWND for selection grab".into(),
            ));
        }
        // SAFETY: hwnd just created; layered attribute makes it nearly invisible.
        unsafe {
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), GRAB_ALPHA, LWA_ALPHA);
        }
        Ok(Self { hwnd, armed: false })
    }

    /// Show the grab window, take foreground/capture, and clear cursor confinement.
    pub fn arm(&mut self) -> Result<(), CaptureError> {
        let bounds = virtual_screen()?;
        clear_pending();
        // SAFETY: hwnd owned by this struct.
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                bounds.0,
                bounds.1,
                bounds.2.max(1),
                bounds.3.max(1),
                SWP_SHOWWINDOW,
            );
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = UpdateWindow(self.hwnd);
            // Clear any game ClipCursor confinement across the virtual desktop.
            let _ = ClipCursor(None);
            // Force the cursor visible (game often hide it in relative mode).
            while ShowCursor(true) < 0 {}
            let _ = SetCursor(LoadCursorW(None, IDC_CROSS).ok());
            // Steal activation so relative-mouse games release the pointer.
            let _ = SetForegroundWindow(self.hwnd);
            let _ = SetCapture(self.hwnd);
            seed_pos_from_cursor();
        }
        self.armed = true;
        Ok(())
    }

    pub fn disarm(&mut self) {
        if !self.armed {
            return;
        }
        // SAFETY: hwnd still valid.
        unsafe {
            let _ = ReleaseCapture();
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        clear_pending();
        self.armed = false;
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Pump this window's messages and drain input into a [`GrabPoll`].
    pub fn poll(&mut self) -> GrabPoll {
        let mut out = GrabPoll {
            x: POS_X.load(Ordering::Relaxed),
            y: POS_Y.load(Ordering::Relaxed),
            ..GrabPoll::default()
        };
        if !self.armed {
            return out;
        }
        // SAFETY: standard PeekMessage pump scoped to our grab HWND.
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, Some(self.hwnd), 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            // Refresh absolute position even if no WM_MOUSEMOVE arrived (warps).
            seed_pos_from_cursor();
            // Keep capture + unclipped cursor while armed.
            let _ = ClipCursor(None);
            let _ = SetCapture(self.hwnd);
        }
        out.x = POS_X.load(Ordering::Relaxed);
        out.y = POS_Y.load(Ordering::Relaxed);
        out.moved = MOVED.swap(false, Ordering::Relaxed);
        out.left_clicks = LEFT_CLICKS.swap(0, Ordering::Relaxed);
        out.escape = ESCAPE.swap(false, Ordering::Relaxed);
        // Cursor sampling counts as a position update for the HUD.
        out.moved = true;
        out
    }
}

impl Drop for SelectionGrab {
    fn drop(&mut self) {
        self.disarm();
        // SAFETY: destroy HWND we created.
        unsafe {
            if !self.hwnd.is_invalid() && self.hwnd != HWND::default() {
                let _ = DestroyWindow(self.hwnd);
                self.hwnd = HWND::default();
            }
        }
    }
}

fn ensure_class() -> Result<(), CaptureError> {
    static CLASS: OnceLock<Result<(), CaptureError>> = OnceLock::new();
    CLASS
        .get_or_init(|| {
            // SAFETY: RegisterClassW with a process-local class.
            unsafe {
                let module = GetModuleHandleW(None)
                    .map_err(|e| CaptureError::Message(format!("GetModuleHandleW failed: {e}")))?;
                let wc = WNDCLASSW {
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(grab_wnd_proc),
                    hInstance: module.into(),
                    lpszClassName: CLASS_NAME,
                    hCursor: LoadCursorW(None, IDC_CROSS).unwrap_or_default(),
                    ..Default::default()
                };
                let atom = RegisterClassW(&wc);
                if atom == 0 {
                    let err = GetLastError();
                    if err == ERROR_CLASS_ALREADY_EXISTS {
                        return Ok(());
                    }
                    return Err(CaptureError::Message(format!(
                        "RegisterClassW failed: {err:?}"
                    )));
                }
            }
            Ok(())
        })
        .clone()
}

fn virtual_screen() -> Result<(i32, i32, i32, i32), CaptureError> {
    // SAFETY: GetSystemMetrics for virtual-screen bounds.
    unsafe {
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        if w <= 0 || h <= 0 {
            return Err(CaptureError::Message(
                "virtual screen metrics unavailable".into(),
            ));
        }
        Ok((x, y, w, h))
    }
}

fn clear_pending() {
    MOVED.store(false, Ordering::Relaxed);
    LEFT_CLICKS.store(0, Ordering::Relaxed);
    ESCAPE.store(false, Ordering::Relaxed);
}

unsafe fn seed_pos_from_cursor() {
    let mut pt = POINT::default();
    if GetCursorPos(&mut pt).is_ok() {
        let prev_x = POS_X.swap(pt.x, Ordering::Relaxed);
        let prev_y = POS_Y.swap(pt.y, Ordering::Relaxed);
        if prev_x != pt.x || prev_y != pt.y {
            MOVED.store(true, Ordering::Relaxed);
        }
    }
}

unsafe extern "system" fn grab_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_MOUSEMOVE | WM_LBUTTONDOWN => {
            seed_pos_from_cursor();
            MOVED.store(true, Ordering::Relaxed);
            if msg == WM_LBUTTONDOWN {
                LEFT_CLICKS.fetch_add(1, Ordering::Relaxed);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if wparam.0 as u16 == VK_ESCAPE.0 {
                ESCAPE.store(true, Ordering::Relaxed);
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
            let _ = SetCursor(LoadCursorW(None, IDC_CROSS).ok());
            LRESULT(1)
        }
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
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
