//! Windows window list + activate + process icons.

use crate::window_match::{paths_equal, pick_matching_icon, titles_equal};
use crate::{CaptureError, ProcessIcon, WindowInfo, PROCESS_ICON_TARGET_PX};
use sqyre_ports::{AutomationError, WindowFocuser};
use std::collections::HashSet;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::{Owned, BOOL, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateRectRgn, DeleteDC, DeleteObject, GetDC, GetObjectW,
    ReleaseDC, SelectObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId, OpenProcess,
    QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::SHDefExtractIconW;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, DestroyIcon, DrawIconEx, EnumWindows, GetClassLongPtrW, GetForegroundWindow,
    GetIconInfo, GetWindow, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SendMessageW, SetForegroundWindow,
    ShowWindow, DI_NORMAL, GCLP_HICON, GCLP_HICONSM, GWL_EXSTYLE, GW_OWNER, HICON, ICONINFO,
    ICON_BIG, ICON_SMALL, SW_RESTORE, WM_GETICON, WS_EX_TOOLWINDOW,
};

/// Focus a top-level window by executable path + window title.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsWindowFocuser;

impl WindowFocuser for OsWindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), AutomationError> {
        activate_window(process_path, window_title)
    }
}

/// List open top-level windows with title + executable path.
pub fn list_open_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    let hwnds = enum_top_level_windows().map_err(CaptureError::Message)?;
    let mut out = Vec::with_capacity(hwnds.len());
    let mut seen = HashSet::new();
    for hwnd in hwnds {
        let Some(info) = window_info_of(hwnd) else {
            continue;
        };
        let key = format!("{}:{}:{}", info.process_path, info.process_name, info.title);
        if !seen.insert(key) {
            continue;
        }
        out.push(info);
    }
    Ok(out)
}

/// Currently focused top-level window, if any.
pub fn get_active_window() -> Result<Option<WindowInfo>, CaptureError> {
    // SAFETY: GetForegroundWindow is always safe to call.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return Ok(None);
    }
    Ok(window_info_of(hwnd))
}

/// Icon for a bound process: live window icon, else executable resource.
pub fn process_icon(process_path: &str, window_title: &str) -> Option<ProcessIcon> {
    let path = process_path.trim();
    if path.is_empty() {
        return None;
    }
    if let Ok(hwnds) = enum_top_level_windows() {
        let found = pick_matching_icon(
            hwnds,
            path,
            window_title,
            |&hwnd| Some((window_title_of(hwnd)?, window_exe_path(hwnd)?)),
            |&hwnd, _wtitle, wpath| window_icon(hwnd).or_else(|| icon_from_exe(wpath)),
        );
        if found.is_some() {
            return found;
        }
    }
    icon_from_exe(path)
}

fn activate_window(process_path: &str, window_title: &str) -> Result<(), AutomationError> {
    let path = process_path.trim();
    let title = window_title.trim();
    if path.is_empty() || title.is_empty() {
        return Err(AutomationError::InvalidArg(
            "focus window: path and title required".into(),
        ));
    }

    let hwnds = enum_top_level_windows().map_err(AutomationError::Backend)?;
    for hwnd in hwnds {
        let Some(wtitle) = window_title_of(hwnd) else {
            continue;
        };
        if !titles_equal(&wtitle, title) {
            continue;
        }
        let Some(exe) = window_exe_path(hwnd) else {
            continue;
        };
        if !paths_equal(&exe, path) {
            continue;
        }
        return set_foreground(hwnd).map_err(AutomationError::Backend);
    }
    Err(AutomationError::WindowNotFound {
        process_path: path.to_string(),
        title: title.to_string(),
    })
}

fn enum_top_level_windows() -> Result<Vec<HWND>, String> {
    let mut hwnds: Vec<HWND> = Vec::new();
    // SAFETY: callback only touches the Vec via lparam for the duration of EnumWindows.
    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut hwnds as *mut Vec<HWND> as isize),
        )
        .map_err(|e| format!("EnumWindows failed: {e}"))?;
    }
    Ok(hwnds)
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `Vec<HWND>` pointer passed from `enum_top_level_windows`.
    let list = &mut *(lparam.0 as *mut Vec<HWND>);
    if is_listable_window(hwnd) {
        list.push(hwnd);
    }
    BOOL(1)
}

/// Visible top-level app windows (no owner, not tool windows) with a title.
unsafe fn is_listable_window(hwnd: HWND) -> bool {
    if !IsWindowVisible(hwnd).as_bool() {
        return false;
    }
    // Owned windows (e.g. dialogs) — Err means no owner.
    if GetWindow(hwnd, GW_OWNER).is_ok() {
        return false;
    }
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    if ex & WS_EX_TOOLWINDOW.0 != 0 {
        return false;
    }
    matches!(window_title_of(hwnd), Some(t) if !t.trim().is_empty())
}

fn window_info_of(hwnd: HWND) -> Option<WindowInfo> {
    let title = window_title_of(hwnd)?;
    if title.trim().is_empty() {
        return None;
    }
    let path = window_exe_path(hwnd).unwrap_or_default();
    let name = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let icon = window_icon(hwnd).or_else(|| {
        if path.is_empty() {
            None
        } else {
            icon_from_exe(&path)
        }
    });
    Some(WindowInfo {
        title,
        process_name: name,
        process_path: path,
        icon,
    })
}

fn window_title_of(hwnd: HWND) -> Option<String> {
    // SAFETY: hwnd is a live window from EnumWindows / GetForegroundWindow.
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return None;
        }
        let mut buf = vec![0u16; (len as usize) + 1];
        let n = GetWindowTextW(hwnd, &mut buf);
        if n <= 0 {
            return None;
        }
        let s = String::from_utf16_lossy(&buf[..n as usize]);
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

fn window_exe_path(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    // SAFETY: hwnd is valid; pid out-param is stack-local.
    let _tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return None;
    }
    process_exe_path(pid)
}

fn process_exe_path(pid: u32) -> Option<String> {
    // SAFETY: OpenProcess with limited query rights; Owned closes the handle.
    let handle = unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        Owned::new(h)
    };
    let mut buf = vec![0u16; 1024];
    let mut size = buf.len() as u32;
    // SAFETY: buffer length matches `size`; Owned handle remains valid.
    unsafe {
        QueryFullProcessImageNameW(
            *handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .ok()?;
    }
    if size == 0 || (size as usize) >= buf.len() {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..size as usize]))
}

fn window_icon(hwnd: HWND) -> Option<ProcessIcon> {
    // Prefer live window icons; do not DestroyIcon shared handles.
    // SAFETY: hwnd is a live top-level window.
    unsafe {
        for wparam in [ICON_BIG, ICON_SMALL, 2u32] {
            let result = SendMessageW(hwnd, WM_GETICON, Some(WPARAM(wparam as usize)), None);
            let hicon = HICON(result.0 as *mut _);
            if !hicon.is_invalid() {
                if let Some(icon) = hicon_to_rgba(hicon, false) {
                    return Some(icon);
                }
            }
        }
        for index in [GCLP_HICON, GCLP_HICONSM] {
            let ptr = GetClassLongPtrW(hwnd, index);
            if ptr != 0 {
                let hicon = HICON(ptr as *mut _);
                if let Some(icon) = hicon_to_rgba(hicon, false) {
                    return Some(icon);
                }
            }
        }
    }
    None
}

fn icon_from_exe(path: &str) -> Option<ProcessIcon> {
    let wide: Vec<u16> = Path::new(path)
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut large = HICON::default();
    // SAFETY: wide is NUL-terminated; large receives an owned icon we must destroy.
    let hr = unsafe {
        SHDefExtractIconW(
            PCWSTR(wide.as_ptr()),
            0,
            0,
            Some(&mut large),
            None,
            PROCESS_ICON_TARGET_PX,
        )
    };
    if hr.is_err() || large.is_invalid() {
        return None;
    }
    // SAFETY: `large` was created by SHDefExtractIconW; destroy after conversion.
    unsafe { hicon_to_rgba(large, true) }
}

/// Convert `HICON` to RGBA. When `destroy` is true, destroys the icon afterward.
unsafe fn hicon_to_rgba(hicon: HICON, destroy: bool) -> Option<ProcessIcon> {
    let mut info = ICONINFO::default();
    if GetIconInfo(hicon, &mut info).is_err() {
        if destroy {
            let _ = DestroyIcon(hicon);
        }
        return None;
    }

    let color = info.hbmColor;
    let mask = info.hbmMask;
    let mut bm = BITMAP::default();
    let hbmp = if !color.is_invalid() { color } else { mask };
    if hbmp.is_invalid()
        || GetObjectW(
            HGDIOBJ::from(hbmp),
            size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut BITMAP as *mut _),
        ) == 0
    {
        if !color.is_invalid() {
            let _ = DeleteObject(HGDIOBJ::from(color));
        }
        if !mask.is_invalid() {
            let _ = DeleteObject(HGDIOBJ::from(mask));
        }
        if destroy {
            let _ = DestroyIcon(hicon);
        }
        return None;
    }

    let width = bm.bmWidth.max(1);
    let height = bm.bmHeight.unsigned_abs().max(1);

    if !color.is_invalid() {
        let _ = DeleteObject(HGDIOBJ::from(color));
    }
    if !mask.is_invalid() {
        let _ = DeleteObject(HGDIOBJ::from(mask));
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -(height as i32), // top-down
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

    let screen = GetDC(None);
    if screen.is_invalid() {
        if destroy {
            let _ = DestroyIcon(hicon);
        }
        return None;
    }
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let dib = match CreateDIBSection(Some(screen), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
        Ok(h) if !h.is_invalid() && !bits.is_null() => h,
        _ => {
            ReleaseDC(None, screen);
            if destroy {
                let _ = DestroyIcon(hicon);
            }
            return None;
        }
    };
    let mem = CreateCompatibleDC(Some(screen));
    if mem.is_invalid() {
        let _ = DeleteObject(HGDIOBJ::from(dib));
        ReleaseDC(None, screen);
        if destroy {
            let _ = DestroyIcon(hicon);
        }
        return None;
    }
    let old = SelectObject(mem, HGDIOBJ::from(dib));
    let draw_ok = DrawIconEx(mem, 0, 0, hicon, width, height as i32, 0, None, DI_NORMAL).is_ok();
    SelectObject(mem, old);
    let _ = DeleteDC(mem);
    ReleaseDC(None, screen);

    let icon = if draw_ok {
        let px = (width as usize).checked_mul(height as usize)?;
        let src = std::slice::from_raw_parts(bits as *const u8, px * 4);
        let mut rgba = Vec::with_capacity(px * 4);
        // DIB is BGRA.
        for chunk in src.chunks_exact(4) {
            rgba.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
        }
        Some(ProcessIcon {
            width: width as u32,
            height,
            rgba,
        })
    } else {
        None
    };

    let _ = DeleteObject(HGDIOBJ::from(dib));
    if destroy {
        let _ = DestroyIcon(hicon);
    }
    icon
}

fn set_foreground(hwnd: HWND) -> Result<(), String> {
    // SAFETY: Win32 focus APIs; AttachThreadInput pairs are always detached below.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }

        let foreground = GetForegroundWindow();
        if foreground == hwnd {
            return Ok(());
        }

        let target_tid = GetWindowThreadProcessId(hwnd, None);
        let foreground_tid = if foreground.is_invalid() {
            0
        } else {
            GetWindowThreadProcessId(foreground, None)
        };
        let current_tid = GetCurrentThreadId();

        let mut attached_fg = false;
        let mut attached_target = false;
        if foreground_tid != 0 && foreground_tid != current_tid {
            attached_fg = AttachThreadInput(current_tid, foreground_tid, true).as_bool();
        }
        if target_tid != 0 && target_tid != current_tid && target_tid != foreground_tid {
            attached_target = AttachThreadInput(current_tid, target_tid, true).as_bool();
        }

        let _ = BringWindowToTop(hwnd);
        let ok = SetForegroundWindow(hwnd).as_bool();

        if attached_target {
            let _ = AttachThreadInput(current_tid, target_tid, false);
        }
        if attached_fg {
            let _ = AttachThreadInput(current_tid, foreground_tid, false);
        }

        if !ok {
            return Err("SetForegroundWindow failed".into());
        }
        Ok(())
    }
}

/// Enable per-pixel alpha on deferred egui overlay viewports.
///
/// eframe's glow path strips `transparent` when creating secondary windows on Windows
/// (`glutin_winit::finalize_window` + false `supports_transparency`), so winit never
/// calls [`DwmEnableBlurBehindWindow`]. Re-apply the same empty-region blur-behind
/// winit uses for the root window so clear alpha and button α settings composite.
pub fn enable_overlay_window_transparency() -> Result<(), CaptureError> {
    let our_pid = unsafe { GetCurrentProcessId() };
    let mut hwnds: Vec<HWND> = Vec::new();
    // SAFETY: callback only touches the Vec via lparam for the duration of EnumWindows.
    unsafe {
        EnumWindows(
            Some(enum_overlay_windows_proc),
            LPARAM(&mut hwnds as *mut Vec<HWND> as isize),
        )
        .map_err(|e| CaptureError::Message(format!("EnumWindows failed: {e}")))?;
    }

    let mut hinted = 0usize;
    for hwnd in hwnds {
        let mut pid = 0u32;
        // SAFETY: hwnd came from EnumWindows; GetWindowThreadProcessId is always safe.
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        if pid != our_pid {
            continue;
        }
        let Some(title) = window_title_of(hwnd) else {
            continue;
        };
        if title.trim() != crate::OVERLAY_WM_TITLE {
            continue;
        }
        enable_dwm_per_pixel_alpha(hwnd).map_err(CaptureError::Message)?;
        hinted += 1;
    }
    if hinted > 0 {
        crate::diag::mark_site(&format!("win:overlay_transparency:hinted={hinted}"));
    }
    Ok(())
}

unsafe extern "system" fn enum_overlay_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `Vec<HWND>` pointer from `enable_overlay_window_transparency`.
    let list = &mut *(lparam.0 as *mut Vec<HWND>);
    // Include tool windows (`with_taskbar(false)` → WS_EX_TOOLWINDOW); overlays are not
    // "listable" app windows.
    if IsWindowVisible(hwnd).as_bool() {
        list.push(hwnd);
    }
    BOOL(1)
}

fn enable_dwm_per_pixel_alpha(hwnd: HWND) -> Result<(), String> {
    // Mirror winit's transparent-window setup: empty blur region → use framebuffer alpha.
    // SAFETY: CreateRectRgn / DwmEnableBlurBehindWindow / DeleteObject with our HWND + region.
    unsafe {
        let region = CreateRectRgn(0, 0, -1, -1);
        if region.is_invalid() {
            return Err("CreateRectRgn failed for overlay transparency".into());
        }
        let bb = DWM_BLURBEHIND {
            dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
            fEnable: true.into(),
            hRgnBlur: region,
            fTransitionOnMaximized: false.into(),
        };
        let result = DwmEnableBlurBehindWindow(hwnd, &bb)
            .map_err(|e| format!("DwmEnableBlurBehindWindow failed: {e}"));
        let _ = DeleteObject(HGDIOBJ::from(region));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_titles() {
        assert!(paths_equal(
            r"C:\Program Files\App\app.exe",
            r"c:\program files\app\app.exe"
        ));
        assert!(!paths_equal(
            r"C:\Program Files\App\app.exe",
            r"C:\Program Files\App\other.exe"
        ));
        assert!(titles_equal(" Hi ", "Hi"));
    }
}
