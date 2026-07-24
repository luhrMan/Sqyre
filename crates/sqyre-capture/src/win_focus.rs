//! Windows window list + activate.

use crate::WindowInfo;
use sqyre_executor::WindowFocuser;
use std::collections::HashSet;
use std::path::Path;
use windows::core::{Owned, BOOL, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetForegroundWindow, ShowWindow, GWL_EXSTYLE, GW_OWNER, SW_RESTORE, WS_EX_TOOLWINDOW,
};

/// Focus a top-level window by executable path + window title.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsWindowFocuser;

impl WindowFocuser for OsWindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), String> {
        activate_window(process_path, window_title)
    }
}

/// List open top-level windows with title + executable path.
pub fn list_open_windows() -> Result<Vec<WindowInfo>, String> {
    let hwnds = enum_top_level_windows()?;
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
pub fn get_active_window() -> Result<Option<WindowInfo>, String> {
    // SAFETY: GetForegroundWindow is always safe to call.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return Ok(None);
    }
    Ok(window_info_of(hwnd))
}

fn activate_window(process_path: &str, window_title: &str) -> Result<(), String> {
    let path = process_path.trim();
    let title = window_title.trim();
    if path.is_empty() || title.is_empty() {
        return Err("path and title required".into());
    }

    for hwnd in enum_top_level_windows()? {
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
        return set_foreground(hwnd);
    }
    Err(format!(
        "no window with title {window_title:?} from {process_path:?}"
    ))
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
    Some(WindowInfo {
        title,
        process_name: name,
        process_path: path,
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

fn paths_equal(a: &str, b: &str) -> bool {
    let a = Path::new(a.trim());
    let b = Path::new(b.trim());
    if a.as_os_str().is_empty() || b.as_os_str().is_empty() {
        return false;
    }
    // Path::eq is case-insensitive on Windows.
    a == b
}

fn titles_equal(a: &str, b: &str) -> bool {
    a.trim() == b.trim()
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
