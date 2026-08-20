//! Real `AutomationBackend` using rustautogui (lite) + arboard.
//!
//! Tracks keys/buttons this process has pressed so hard exits (failsafe /
//! `process::exit`) can still release them — executor cleanup never runs then.
//!
//! Windows: `SendInput` / `WH_KEYBOARD_LL` cannot reach a higher-integrity
//! process (UIPI). Sqyre must run at the same elevation as the target app.

use arboard::Clipboard;
use rustautogui::{MouseClick, RustAutoGui};
use sqyre_ports::{AutomationBackend, AutomationError, MoveOptions};
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

/// Keys currently held via [`OsAutomation::key_down`] (rustautogui names).
static HELD_KEYS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
/// Mouse buttons currently held via [`OsAutomation::click`] down.
static HELD_BUTTONS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn note_key_down(key: &str) {
    if let Ok(mut g) = HELD_KEYS.lock() {
        g.insert(key.to_string());
    }
}

fn note_key_up(key: &str) {
    if let Ok(mut g) = HELD_KEYS.lock() {
        g.remove(key);
    }
}

fn note_button_down(button: &str) {
    if let Ok(mut g) = HELD_BUTTONS.lock() {
        g.insert(button.to_string());
    }
}

fn note_button_up(button: &str) {
    if let Ok(mut g) = HELD_BUTTONS.lock() {
        g.remove(button);
    }
}

fn take_held() -> (HashSet<String>, HashSet<String>) {
    let keys = HELD_KEYS
        .lock()
        .map(|mut g| std::mem::take(&mut *g))
        .unwrap_or_default();
    let buttons = HELD_BUTTONS
        .lock()
        .map(|mut g| std::mem::take(&mut *g))
        .unwrap_or_default();
    (keys, buttons)
}

/// Canonical button name used for hold tracking (`left` / `right` / `middle`).
fn canonical_button(button: &str) -> &'static str {
    match button {
        "right" => "right",
        "center" | "middle" => "middle",
        _ => "left",
    }
}

/// Best-effort release of every key/button this process still has held.
///
/// Safe to call from any thread (including failsafe / `process::exit` paths).
/// No-ops when nothing is held or when the OS input backend cannot start.
pub fn release_held_inputs() {
    let (keys, buttons) = take_held();
    if keys.is_empty() && buttons.is_empty() {
        return;
    }
    #[cfg(all(target_os = "linux", feature = "portal-capture"))]
    if sqyre_capture::portal_input_ready() {
        for key in &keys {
            if let Some(evdev) = evdev_for_name(key) {
                let _ = sqyre_capture::portal_input_key(evdev, false);
            }
        }
        for button in &buttons {
            let _ = sqyre_capture::portal_input_click(button, false);
        }
        return;
    }
    let Ok(gui) = RustAutoGui::new(false) else {
        return;
    };
    for key in keys {
        let _ = gui.key_up(&key);
    }
    for button in buttons {
        let _ = gui.click_up(OsAutomation::map_button(&button));
    }
}

/// Clear OS mouse-capture / stuck button state before a macro run.
///
/// Call from the **UI thread** (e.g. inside the Start click handler). winit's
/// `SetCapture` / `ReleaseCapture` are thread-affine — calling this on the run
/// worker never clears capture taken by the Start button. No-op on other platforms.
pub fn prepare_for_automation() {
    #[cfg(target_os = "windows")]
    prepare_windows_automation();
}

#[cfg(target_os = "windows")]
fn prepare_windows_automation() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, ReleaseCapture, SendInput, INPUT, INPUT_0, INPUT_MOUSE,
        MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, VK_LBUTTON,
        VK_MBUTTON, VK_RBUTTON,
    };

    // Warm the unlock HWND on the UI thread so Move workers never pay CreateWindow.
    let _ = win_cursor::ensure_unlock_hwnd();

    // SAFETY: ReleaseCapture / GetAsyncKeyState / SendInput use process-global input
    // state; INPUT values are stack locals.
    unsafe {
        let _ = ReleaseCapture();

        // MSDN: SendInput does not clear already-pressed buttons — correct them first.
        for flag in [
            MOUSEEVENTF_LEFTUP,
            MOUSEEVENTF_RIGHTUP,
            MOUSEEVENTF_MIDDLEUP,
        ] {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: flag,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }

        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            let down = ((GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16) & 0x8000) != 0
                || ((GetAsyncKeyState(VK_RBUTTON.0 as i32) as u16) & 0x8000) != 0
                || ((GetAsyncKeyState(VK_MBUTTON.0 as i32) as u16) & 0x8000) != 0;
            if !down {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Let the Start-button mouse-up / capture release fully settle.
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Windows cursor unlock + absolute move helpers.
///
/// Relative-mouse games reject `SetCursorPos` until another window takes
/// activation. A process-lifetime 1×1 tool HWND (created on the UI thread in
/// [`prepare_for_automation`]) is shown briefly to break that lock.
#[cfg(target_os = "windows")]
mod win_cursor {
    use std::sync::{Mutex, OnceLock};
    use windows::core::w;
    use windows::Win32::Foundation::{
        GetLastError, COLORREF, ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, VIRTUAL_KEY,
        VK_MENU,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, ClipCursor, CreateWindowExW, DefWindowProcW, GetForegroundWindow,
        GetSystemMetrics, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindow, RegisterClassW,
        SetCursorPos, SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW,
        SetWindowPos, ShowCursor, ShowWindow, ASFW_ANY, CS_HREDRAW, CS_VREDRAW, GWL_EXSTYLE,
        HWND_TOPMOST, LWA_ALPHA, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNA, WM_DESTROY, WNDCLASSW,
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };

    use super::virtual_desk_normalized;

    const UNLOCK_CLASS: windows::core::PCWSTR = w!("SqyreCursorUnlock");

    unsafe extern "system" fn unlock_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_DESTROY {
            return LRESULT(0);
        }
        // SAFETY: standard DefWindowProc forwarding for unused messages.
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    /// Process-lifetime unlock HWND (prefer create on UI thread via prepare).
    pub(super) fn ensure_unlock_hwnd() -> Option<HWND> {
        static HWND_CELL: OnceLock<Mutex<isize>> = OnceLock::new();
        let cell = HWND_CELL.get_or_init(|| Mutex::new(0));
        let mut guard = cell.lock().ok()?;
        if *guard != 0 {
            let hwnd = HWND(*guard as *mut _);
            // SAFETY: IsWindow on a cached HWND.
            if unsafe { IsWindow(Some(hwnd)).as_bool() } {
                apply_click_through(hwnd);
                return Some(hwnd);
            }
            *guard = 0;
        }
        // SAFETY: register class once; CreateWindowEx for a process-owned tool popup.
        unsafe {
            let module = GetModuleHandleW(None).ok()?;
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(unlock_wnd_proc),
                hInstance: module.into(),
                lpszClassName: UNLOCK_CLASS,
                ..Default::default()
            };
            let atom = RegisterClassW(&class);
            if atom == 0 {
                let err = GetLastError();
                if err != ERROR_CLASS_ALREADY_EXISTS {
                    return None;
                }
            }
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT,
                UNLOCK_CLASS,
                w!("Sqyre cursor unlock"),
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
            .ok()?;
            if hwnd.is_invalid() {
                return None;
            }
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 1, LWA_ALPHA);
            apply_click_through(hwnd);
            *guard = hwnd.0 as isize;
            Some(hwnd)
        }
    }

    /// Clicks must hit-test the game, not our 1×1 unlock popup.
    fn apply_click_through(hwnd: HWND) {
        // SAFETY: process-owned HWND; GWL_EXSTYLE read/write is the usual layered-window pattern.
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let add = WS_EX_TRANSPARENT.0 as isize;
            if ex & add == 0 {
                let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | add);
            }
        }
    }

    fn key_input(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: windows::Win32::UI::Input::KeyboardAndMouse::KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    /// Steal activation long enough for `SetCursorPos` to succeed.
    ///
    /// Returns the previous foreground HWND so the caller can restore it after
    /// moving. Click needs the game as foreground; a hidden Sqyre popup as FG
    /// also leaves focused Esc (egui) inactive.
    pub(super) fn unlock_cursor() -> Option<HWND> {
        let hwnd = ensure_unlock_hwnd()?;
        // SAFETY: focus/capture APIs; AttachThreadInput pairs always detached below.
        unsafe {
            let previous = GetForegroundWindow();
            let _ = ShowWindow(hwnd, SW_SHOWNA);
            let _ = ClipCursor(None);
            // Bound the ShowCursor loop — games can drive the display count very negative.
            for _ in 0..32 {
                if ShowCursor(true) >= 0 {
                    break;
                }
            }

            let fg_tid = if previous.is_invalid() {
                0
            } else {
                GetWindowThreadProcessId(previous, None)
            };
            let cur = GetCurrentThreadId();
            let attached =
                fg_tid != 0 && fg_tid != cur && AttachThreadInput(cur, fg_tid, true).as_bool();

            // Momentary Alt lets a background thread call SetForegroundWindow repeatedly.
            let alt = [
                key_input(VK_MENU, KEYBD_EVENT_FLAGS(0)),
                key_input(VK_MENU, KEYEVENTF_KEYUP),
            ];
            let _ = SendInput(&alt, std::mem::size_of::<INPUT>() as i32);
            let _ = SetForegroundWindow(hwnd);
            // Do not SetCapture — capture is thread-affine and routes every later
            // SendInput click to this 1×1 window (clicks look dead).

            if attached {
                let _ = AttachThreadInput(cur, fg_tid, false);
            }

            if previous.is_invalid() || previous == hwnd {
                None
            } else {
                Some(previous)
            }
        }
    }

    fn restore_foreground(hwnd: HWND) {
        // SAFETY: same AttachThreadInput pattern as unlock / win_focus.
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground == hwnd {
                return;
            }
            let fg_tid = if foreground.is_invalid() {
                0
            } else {
                GetWindowThreadProcessId(foreground, None)
            };
            let target_tid = GetWindowThreadProcessId(hwnd, None);
            let cur = GetCurrentThreadId();
            let mut attached_fg = false;
            let mut attached_target = false;
            if fg_tid != 0 && fg_tid != cur {
                attached_fg = AttachThreadInput(cur, fg_tid, true).as_bool();
            }
            if target_tid != 0 && target_tid != cur && target_tid != fg_tid {
                attached_target = AttachThreadInput(cur, target_tid, true).as_bool();
            }
            // We briefly own foreground via the unlock HWND — allow restoring any PID.
            let _ = AllowSetForegroundWindow(ASFW_ANY);
            let alt = [
                key_input(VK_MENU, KEYBD_EVENT_FLAGS(0)),
                key_input(VK_MENU, KEYEVENTF_KEYUP),
            ];
            let _ = SendInput(&alt, std::mem::size_of::<INPUT>() as i32);
            let _ = SetForegroundWindow(hwnd);
            if attached_target {
                let _ = AttachThreadInput(cur, target_tid, false);
            }
            if attached_fg {
                let _ = AttachThreadInput(cur, fg_tid, false);
            }
        }
    }

    pub(super) fn hide_unlock(restore: Option<HWND>) {
        let Some(hwnd) = ensure_unlock_hwnd() else {
            return;
        };
        // SAFETY: hide without activating; restore FG while we can still AllowSetForeground.
        unsafe {
            if let Some(prev) = restore {
                restore_foreground(prev);
            }
            let _ = ShowWindow(hwnd, SW_HIDE);
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 1, 1, SWP_NOACTIVATE);
            // If restore failed, we may still be FG on a hidden tool window — try once more.
            if let Some(prev) = restore {
                let fg = GetForegroundWindow();
                if fg == hwnd || fg.is_invalid() {
                    restore_foreground(prev);
                }
            }
        }
    }

    /// Returns whether `SetCursorPos` accepted the request (cursor is movable).
    pub(super) fn set_pos(px: i32, py: i32) -> bool {
        // SAFETY: SetCursorPos is process-global.
        unsafe { SetCursorPos(px, py).is_ok() }
    }

    /// Absolute SendInput backup (used after unlock when SetCursorPos alone is flaky).
    pub(super) fn set_pos_absolute_inject(px: i32, py: i32) {
        // SAFETY: GetSystemMetrics desktop extents are process-global.
        let (vx, vy, vw, vh) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        let Some((nx, ny)) = virtual_desk_normalized(px, py, vx, vy, vw, vh) else {
            let _ = set_pos(px, py);
            return;
        };
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: nx,
                    dy: ny,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        // SAFETY: stack-local INPUT.
        unsafe {
            let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            let _ = SetCursorPos(px, py);
        }
    }
}

pub struct OsAutomation {
    gui: Option<RustAutoGui>,
    clipboard: Option<Clipboard>,
    #[cfg(all(target_os = "linux", feature = "portal-capture"))]
    portal: bool,
}

impl OsAutomation {
    pub fn new() -> Result<Self, AutomationError> {
        let clipboard = Clipboard::new().ok();
        #[cfg(all(target_os = "linux", feature = "portal-capture"))]
        {
            if linux_session_is_wayland() {
                return Ok(Self {
                    gui: RustAutoGui::new(false).ok(),
                    clipboard,
                    portal: true,
                });
            }
        }
        let gui = RustAutoGui::new(false)
            .map_err(|e| AutomationError::Backend(format!("rustautogui: {e}")))?;
        Ok(Self {
            gui: Some(gui),
            clipboard,
            #[cfg(all(target_os = "linux", feature = "portal-capture"))]
            portal: false,
        })
    }

    fn map_button(button: &str) -> MouseClick {
        match button {
            "right" => MouseClick::RIGHT,
            "center" | "middle" => MouseClick::MIDDLE,
            _ => MouseClick::LEFT,
        }
    }

    fn map_key(key: &str) -> String {
        // Legacy key names → rustautogui US keyboard names.
        match key.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => "control".into(),
            "cmd" | "command" | "super" | "win" => "command".into(),
            "esc" | "escape" => "escape".into(),
            "return" | "enter" => "enter".into(),
            "space" | "spacebar" => "space".into(),
            other => other.to_string(),
        }
    }

    fn gui(&mut self) -> Result<&mut RustAutoGui, AutomationError> {
        self.gui
            .as_mut()
            .ok_or(AutomationError::Backend("no X11 input backend".into()))
    }
}

#[cfg(all(target_os = "linux", feature = "portal-capture"))]
fn linux_session_is_wayland() -> bool {
    sqyre_capture::LinuxSessionInfo::detect().session_kind
        == sqyre_capture::LinuxSessionKind::Wayland
}

#[cfg(all(target_os = "linux", feature = "portal-capture"))]
fn ensure_portal_input() -> Result<(), AutomationError> {
    if sqyre_capture::portal_input_ready() {
        return Ok(());
    }
    let _ = sqyre_capture::shared_capturer();
    if sqyre_capture::portal_input_ready() {
        Ok(())
    } else if sqyre_capture::portal_remote_desktop_granted() {
        Err(AutomationError::Backend(
            "Remote Desktop granted but EIS is not ready".into(),
        ))
    } else {
        Err(AutomationError::Backend(
            "desktop control not granted (enable Allow Remote Interaction, then Share)".into(),
        ))
    }
}

#[cfg(all(target_os = "linux", feature = "portal-capture"))]
fn evdev_for_name(key: &str) -> Option<u32> {
    Some(match key.trim().to_ascii_lowercase().as_str() {
        "escape" | "esc" => 1,
        "1" => 2,
        "2" => 3,
        "3" => 4,
        "4" => 5,
        "5" => 6,
        "6" => 7,
        "7" => 8,
        "8" => 9,
        "9" => 10,
        "0" => 11,
        "minus" | "-" => 12,
        "equal" | "=" => 13,
        "backspace" => 14,
        "tab" => 15,
        "q" => 16,
        "w" => 17,
        "e" => 18,
        "r" => 19,
        "t" => 20,
        "y" => 21,
        "u" => 22,
        "i" => 23,
        "o" => 24,
        "p" => 25,
        "return" | "enter" => 28,
        "ctrl" | "control" | "ctrlleft" | "lctrl" => 29,
        "a" => 30,
        "s" => 31,
        "d" => 32,
        "f" => 33,
        "g" => 34,
        "h" => 35,
        "j" => 36,
        "k" => 37,
        "l" => 38,
        "shift" | "shiftleft" | "lshift" => 42,
        "z" => 44,
        "x" => 45,
        "c" => 46,
        "v" => 47,
        "b" => 48,
        "n" => 49,
        "m" => 50,
        "alt" | "altleft" | "lalt" => 56,
        "space" | "spacebar" => 57,
        "f1" => 59,
        "f2" => 60,
        "f3" => 61,
        "f4" => 62,
        "f5" => 63,
        "f6" => 64,
        "f7" => 65,
        "f8" => 66,
        "f9" => 67,
        "f10" => 68,
        "f11" => 87,
        "f12" => 88,
        "home" => 102,
        "up" => 103,
        "pageup" | "page_up" => 104,
        "left" => 105,
        "right" => 106,
        "end" => 107,
        "down" => 108,
        "pagedown" | "page_down" => 109,
        "insert" => 110,
        "delete" | "del" => 111,
        "super" | "win" | "cmd" | "command" | "meta" => 125,
        other if other.len() == 1 => {
            let c = other.chars().next()?;
            if c.is_ascii_alphabetic() {
                return evdev_for_name(&c.to_ascii_lowercase().to_string());
            }
            return None;
        }
        _ => return None,
    })
}

/// Windows/macOS smooth duration from [`MoveOptions`] (legacy formula).
#[cfg(any(test, not(target_os = "linux")))]
fn smooth_moving_time_secs(opts: MoveOptions) -> f32 {
    if !opts.smooth {
        return 0.0;
    }
    let base = if opts.delay_ms > 0 {
        opts.delay_ms as f32 * 0.05
    } else {
        0.2
    };
    base.clamp(0.05, 2.0)
}

/// Distance-aware smooth-move duration and step interval for Linux.
///
/// rustautogui smooth moves step once per pixel (`step_by` over the path), which
/// is extremely slow on GNOME Wayland where each `XWarpPointer` round-trips through
/// XWayland. Sqyre caps warp count and uses [`MoveOptions::delay_ms`] as the step
/// interval instead.
#[cfg(any(test, target_os = "linux"))]
fn linux_smooth_move_plan(opts: MoveOptions, dx: i32, dy: i32) -> (f32, u64) {
    let step_ms = opts.delay_ms.max(1) as u64;
    let distance = ((dx * dx + dy * dy) as f64).sqrt();
    // ~8 px per warp, at most 100 warps (Wayland/XWayland is costly per warp).
    let ideal_steps = ((distance / 8.0).ceil() as u64).clamp(2, 100);
    let mut duration_secs = ideal_steps as f32 * step_ms as f32 / 1000.0;
    let min_t = opts.low as f32;
    let max_t = opts.high as f32;
    duration_secs = if max_t >= min_t {
        duration_secs.clamp(min_t, max_t)
    } else {
        duration_secs.max(min_t)
    };
    (duration_secs, step_ms)
}

#[cfg(target_os = "linux")]
fn move_mouse_instant(gui: &mut RustAutoGui, px: i32, py: i32) {
    let xu = u32::try_from(px).unwrap_or(0);
    let yu = u32::try_from(py).unwrap_or(0);
    let _ = gui.move_mouse_to_pos(xu, yu, 0.0);
}

/// Timed interpolation between `start` and `end`, calling `warp` each step.
#[cfg(target_os = "linux")]
fn run_linux_smooth_move(
    start: (i32, i32),
    end: (i32, i32),
    opts: MoveOptions,
    mut warp: impl FnMut(i32, i32),
) {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    if dx == 0 && dy == 0 {
        return;
    }
    let (moving_time, step_ms) = linux_smooth_move_plan(opts, dx, dy);
    if moving_time <= 0.0 {
        warp(end.0, end.1);
        return;
    }
    let start_t = std::time::Instant::now();
    let step = Duration::from_millis(step_ms);
    loop {
        let t = start_t.elapsed().as_secs_f32() / moving_time;
        if t >= 1.0 {
            warp(end.0, end.1);
            break;
        }
        let nx = start.0 as f32 + t * dx as f32;
        let ny = start.1 as f32 + t * dy as f32;
        warp(nx as i32, ny as i32);
        std::thread::sleep(step);
    }
}

/// Linux smooth move: timed interpolation with instant warps (moving_time=0).
///
/// Avoids rustautogui's per-pixel smooth path (XTest + sleep per pixel).
#[cfg(target_os = "linux")]
fn move_mouse_linux(gui: &mut RustAutoGui, x: i32, y: i32, opts: MoveOptions) {
    let start = match gui.get_mouse_position() {
        Ok(pos) => pos,
        Err(_) => {
            move_mouse_instant(gui, x, y);
            return;
        }
    };
    run_linux_smooth_move(start, (x, y), opts, |px, py| {
        move_mouse_instant(gui, px, py)
    });
}

/// Portal EIS smooth move. Start pos is last EIS warp, else X11 query via rustautogui.
#[cfg(all(target_os = "linux", feature = "portal-capture"))]
fn move_mouse_portal(start: Option<(i32, i32)>, x: i32, y: i32, opts: MoveOptions) {
    let Some(start) = start else {
        sqyre_capture::note("input: portal smooth has no start pos, instant warp");
        if let Err(e) = sqyre_capture::portal_input_move(x, y) {
            sqyre_capture::note(&format!("input move: {e}"));
        }
        return;
    };
    run_linux_smooth_move(start, (x, y), opts, |px, py| {
        if let Err(e) = sqyre_capture::portal_input_move(px, py) {
            sqyre_capture::note(&format!("input move: {e}"));
        }
    });
}

/// Map a virtual-desktop pixel into SendInput's 0..65535 absolute range.
///
/// `None` when the virtual desktop metrics are unusable (caller should fall back).
#[cfg(any(test, target_os = "windows"))]
fn virtual_desk_normalized(
    x: i32,
    y: i32,
    virt_x: i32,
    virt_y: i32,
    virt_w: i32,
    virt_h: i32,
) -> Option<(i32, i32)> {
    if virt_w <= 1 || virt_h <= 1 {
        return None;
    }
    let nx = ((x - virt_x) as i64 * 65535) / (virt_w as i64 - 1);
    let ny = ((y - virt_y) as i64 * 65535) / (virt_h as i64 - 1);
    Some((nx.clamp(0, 65535) as i32, ny.clamp(0, 65535) as i32))
}

/// Absolute move with signed virtual-desktop coords (Windows origin may be negative).
///
/// Fast path is a single `SetCursorPos`. Only if that fails (exclusive/relative mouse
/// lock) do we briefly activate the unlock popup — that path is ~tens of ms, so it
/// must not run when the cursor is already free.
#[cfg(target_os = "windows")]
fn move_mouse_windows(x: i32, y: i32, moving_time: f32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    fn move_instant(px: i32, py: i32) {
        // Trust SetCursorPos's BOOL — do not also require GetCursorPos match.
        // Games often warp the cursor after a successful set; re-checking would
        // falsely trigger the expensive unlock path every Move (~60–70ms).
        if win_cursor::set_pos(px, py) {
            return;
        }
        let previous = win_cursor::unlock_cursor();
        win_cursor::set_pos_absolute_inject(px, py);
        win_cursor::hide_unlock(previous);
        // Restoring the game's FG often recenters relative-mouse locks — put the
        // cursor back so the following Click lands on the Image Search target.
        if !win_cursor::set_pos(px, py) {
            win_cursor::set_pos_absolute_inject(px, py);
        }
    }

    if moving_time <= 0.0 {
        move_instant(x, y);
        return;
    }

    let mut start = POINT::default();
    // SAFETY: GetCursorPos writes into stack-local POINT.
    let start = unsafe {
        if GetCursorPos(&mut start).is_err() {
            move_instant(x, y);
            return;
        }
        start
    };
    let start_t = std::time::Instant::now();
    let dx = x - start.x;
    let dy = y - start.y;
    let step = Duration::from_millis(10);
    loop {
        let t = start_t.elapsed().as_secs_f32() / moving_time;
        if t >= 1.0 {
            move_instant(x, y);
            break;
        }
        let nx = start.x as f32 + t * dx as f32;
        let ny = start.y as f32 + t * dy as f32;
        let _ = win_cursor::set_pos(nx as i32, ny as i32);
        std::thread::sleep(step);
    }
}

impl AutomationBackend for OsAutomation {
    fn milli_sleep(&mut self, ms: i32) {
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms as u64));
        }
    }

    fn move_to(&mut self, x: i32, y: i32, opts: MoveOptions) {
        #[cfg(target_os = "windows")]
        {
            let moving_time = smooth_moving_time_secs(opts);
            move_mouse_windows(x, y, moving_time);
        }
        #[cfg(target_os = "linux")]
        {
            #[cfg(feature = "portal-capture")]
            if self.portal {
                if let Err(e) = ensure_portal_input() {
                    sqyre_capture::note(&format!("input move skipped: {e}"));
                    return;
                }
                if opts.smooth {
                    let start = sqyre_capture::portal_input_last_pos()
                        .or_else(|| self.gui.as_mut().and_then(|g| g.get_mouse_position().ok()));
                    move_mouse_portal(start, x, y, opts);
                } else if let Err(e) = sqyre_capture::portal_input_move(x, y) {
                    sqyre_capture::note(&format!("input move: {e}"));
                }
                return;
            }
            let Some(gui) = self.gui.as_mut() else {
                return;
            };
            if opts.smooth {
                move_mouse_linux(gui, x, y, opts);
            } else {
                move_mouse_instant(gui, x, y);
            }
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
        {
            let moving_time = smooth_moving_time_secs(opts);
            // rustautogui's public API is u32; X11 virtual desktop starts at (0,0).
            let xu = u32::try_from(x).unwrap_or(0);
            let yu = u32::try_from(y).unwrap_or(0);
            if let Ok(gui) = self.gui() {
                if let Err(e) = gui.move_mouse_to_pos(xu, yu, moving_time) {
                    let _ = gui.move_mouse_to_pos(xu, yu, 0.0);
                    let _ = e;
                }
            }
        }
    }

    fn click(&mut self, button: &str, down: bool) -> Result<(), AutomationError> {
        let canonical = canonical_button(button);
        #[cfg(all(target_os = "linux", feature = "portal-capture"))]
        if self.portal {
            ensure_portal_input()?;
            sqyre_capture::portal_input_click(canonical, down)?;
            if down {
                note_button_down(canonical);
            } else {
                note_button_up(canonical);
            }
            return Ok(());
        }
        let btn = Self::map_button(canonical);
        if down {
            self.gui()?
                .click_down(btn)
                .map_err(|e| AutomationError::Backend(format!("click down: {e}")))?;
            note_button_down(canonical);
            Ok(())
        } else {
            self.gui()?
                .click_up(btn)
                .map_err(|e| AutomationError::Backend(format!("click up: {e}")))?;
            note_button_up(canonical);
            Ok(())
        }
    }

    fn scroll(&mut self, up: bool) -> Result<(), AutomationError> {
        #[cfg(all(target_os = "linux", feature = "portal-capture"))]
        if self.portal {
            ensure_portal_input()?;
            return sqyre_capture::portal_input_scroll(up);
        }
        // Scroll intensity ~3 notches.
        if up {
            self.gui()?
                .scroll_up(3)
                .map_err(|e| AutomationError::Backend(format!("scroll up: {e}")))
        } else {
            self.gui()?
                .scroll_down(3)
                .map_err(|e| AutomationError::Backend(format!("scroll down: {e}")))
        }
    }

    fn key_down(&mut self, key: &str) -> Result<(), AutomationError> {
        #[cfg(all(target_os = "linux", feature = "portal-capture"))]
        if self.portal {
            ensure_portal_input()?;
            let evdev = evdev_for_name(key)
                .ok_or_else(|| AutomationError::InvalidArg(format!("unknown key: {key}")))?;
            sqyre_capture::portal_input_key(evdev, true)?;
            note_key_down(key);
            return Ok(());
        }
        let k = Self::map_key(key);
        self.gui()?
            .key_down(&k)
            .map_err(|e| AutomationError::Backend(format!("key down {k}: {e}")))?;
        note_key_down(&k);
        Ok(())
    }

    fn key_up(&mut self, key: &str) -> Result<(), AutomationError> {
        #[cfg(all(target_os = "linux", feature = "portal-capture"))]
        if self.portal {
            let evdev = evdev_for_name(key)
                .ok_or_else(|| AutomationError::InvalidArg(format!("unknown key: {key}")))?;
            let _ = sqyre_capture::portal_input_key(evdev, false);
            note_key_up(key);
            return Ok(());
        }
        let k = Self::map_key(key);
        self.gui()?
            .key_up(&k)
            .map_err(|e| AutomationError::Backend(format!("key up {k}: {e}")))?;
        note_key_up(&k);
        Ok(())
    }

    fn type_char(&mut self, ch: char) {
        #[cfg(all(target_os = "linux", feature = "portal-capture"))]
        if self.portal && ensure_portal_input().is_ok() {
            if let Some(evdev) = evdev_for_name(&ch.to_ascii_lowercase().to_string()) {
                let _ = sqyre_capture::portal_input_key(evdev, true);
                let _ = sqyre_capture::portal_input_key(evdev, false);
                return;
            }
        }
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        if let Ok(gui) = self.gui() {
            let _ = gui.keyboard_input(s);
        }
    }

    fn write_clipboard(&mut self, s: &str) -> Result<(), AutomationError> {
        let clip = self
            .clipboard
            .as_mut()
            .ok_or(AutomationError::Unsupported("clipboard"))?;
        clip.set_text(s.to_string())
            .map_err(|e| AutomationError::Backend(format!("clipboard: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_buttons_and_keys() {
        assert!(matches!(OsAutomation::map_button("left"), MouseClick::LEFT));
        assert!(matches!(
            OsAutomation::map_button("right"),
            MouseClick::RIGHT
        ));
        assert!(matches!(
            OsAutomation::map_button("middle"),
            MouseClick::MIDDLE
        ));
        assert!(matches!(
            OsAutomation::map_button("center"),
            MouseClick::MIDDLE
        ));
        assert_eq!(OsAutomation::map_key("ctrl"), "control");
        assert_eq!(OsAutomation::map_key("control"), "control");
        assert_eq!(OsAutomation::map_key("esc"), "escape");
        assert_eq!(OsAutomation::map_key("escape"), "escape");
        assert_eq!(OsAutomation::map_key("return"), "enter");
        assert_eq!(OsAutomation::map_key("enter"), "enter");
        assert_eq!(OsAutomation::map_key("spacebar"), "space");
        assert_eq!(OsAutomation::map_key("cmd"), "command");
        assert_eq!(OsAutomation::map_key("super"), "command");
        assert_eq!(OsAutomation::map_key("a"), "a");
    }

    #[test]
    fn smooth_move_time_clamped() {
        let opts = MoveOptions {
            smooth: true,
            low: 0.05,
            high: 0.2,
            delay_ms: 100,
        };
        assert!((smooth_moving_time_secs(opts) - 2.0).abs() < f32::EPSILON);
        assert_eq!(
            smooth_moving_time_secs(MoveOptions {
                smooth: false,
                ..opts
            }),
            0.0
        );
    }

    #[test]
    fn linux_smooth_move_plan_caps_warps_and_duration() {
        let opts = MoveOptions {
            smooth: true,
            low: 0.05,
            high: 0.2,
            delay_ms: 1,
        };
        // Short move: distance-based, clamped up to smooth_low.
        let (dur, step) = linux_smooth_move_plan(opts, 40, 0);
        assert_eq!(step, 1);
        assert!((dur - 0.05).abs() < f32::EPSILON);

        // Long move: warp count capped at 100 → 100ms at 1ms steps.
        let (dur_long, _) = linux_smooth_move_plan(opts, 3000, 0);
        assert!((dur_long - 0.1).abs() < f32::EPSILON);

        // Duration capped at smooth_high when steps would exceed it.
        let slow = MoveOptions {
            smooth: true,
            low: 0.05,
            high: 0.2,
            delay_ms: 5,
        };
        let (dur_capped, _) = linux_smooth_move_plan(slow, 3000, 0);
        assert!((dur_capped - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn virtual_desk_normalized_primary_and_negative_origin() {
        // Primary corners → absolute range endpoints.
        assert_eq!(
            virtual_desk_normalized(0, 0, 0, 0, 1920, 1080),
            Some((0, 0))
        );
        assert_eq!(
            virtual_desk_normalized(1919, 1079, 0, 0, 1920, 1080),
            Some((65535, 65535))
        );
        // Secondary monitor left of primary (virt origin negative).
        let (nx, ny) = virtual_desk_normalized(-100, 10, -1920, 0, 3840, 1080).unwrap();
        assert!((1..65535).contains(&nx));
        assert!((0..65535).contains(&ny));
        assert!(virtual_desk_normalized(0, 0, 0, 0, 0, 0).is_none());
    }

    #[test]
    fn hold_tracking_take_clears() {
        let _ = take_held(); // isolate from other tests
        note_key_down("control");
        note_key_down("a");
        note_button_down("left");
        note_key_up("a");
        let (keys, buttons) = take_held();
        assert!(keys.contains("control"));
        assert!(!keys.contains("a"));
        assert!(buttons.contains("left"));
        let (keys2, buttons2) = take_held();
        assert!(keys2.is_empty());
        assert!(buttons2.is_empty());
    }

    #[test]
    fn canonical_button_aliases() {
        assert_eq!(canonical_button("left"), "left");
        assert_eq!(canonical_button("right"), "right");
        assert_eq!(canonical_button("middle"), "middle");
        assert_eq!(canonical_button("center"), "middle");
        assert_eq!(canonical_button("other"), "left");
    }
}
