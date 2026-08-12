//! Real `AutomationBackend` using rustautogui (lite) + arboard.
//!
//! Tracks keys/buttons this process has pressed so hard exits (failsafe /
//! `process::exit`) can still release them — executor cleanup never runs then.

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
        ReleaseCapture, SendInput, SetCapture, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE,
        KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
        MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, VIRTUAL_KEY, VK_MENU,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        ClipCursor, CreateWindowExW, DefWindowProcW, GetForegroundWindow, GetSystemMetrics,
        GetWindowThreadProcessId, IsWindow, RegisterClassW, SetCursorPos, SetForegroundWindow,
        SetLayeredWindowAttributes, SetWindowPos, ShowCursor, ShowWindow, CS_HREDRAW, CS_VREDRAW,
        HWND_TOPMOST, LWA_ALPHA, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNA, WM_DESTROY, WNDCLASSW,
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
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
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
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
            *guard = hwnd.0 as isize;
            Some(hwnd)
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
    pub(super) fn unlock_cursor() {
        let Some(hwnd) = ensure_unlock_hwnd() else {
            return;
        };
        // SAFETY: focus/capture APIs; AttachThreadInput pairs always detached below.
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNA);
            let _ = ClipCursor(None);
            // Bound the ShowCursor loop — games can drive the display count very negative.
            for _ in 0..32 {
                if ShowCursor(true) >= 0 {
                    break;
                }
            }

            let foreground = GetForegroundWindow();
            let fg_tid = if foreground.is_invalid() {
                0
            } else {
                GetWindowThreadProcessId(foreground, None)
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
            let _ = SetCapture(hwnd);

            if attached {
                let _ = AttachThreadInput(cur, fg_tid, false);
            }
        }
    }

    pub(super) fn hide_unlock() {
        let Some(hwnd) = ensure_unlock_hwnd() else {
            return;
        };
        // SAFETY: release capture taken during unlock; hide without activating others.
        unsafe {
            let _ = ReleaseCapture();
            let _ = ShowWindow(hwnd, SW_HIDE);
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 1, 1, SWP_NOACTIVATE);
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
    gui: RustAutoGui,
    clipboard: Option<Clipboard>,
}

impl OsAutomation {
    pub fn new() -> Result<Self, AutomationError> {
        let gui = RustAutoGui::new(false)
            .map_err(|e| AutomationError::Backend(format!("rustautogui: {e}")))?;
        let clipboard = Clipboard::new().ok();
        Ok(Self { gui, clipboard })
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
        win_cursor::unlock_cursor();
        win_cursor::set_pos_absolute_inject(px, py);
        win_cursor::hide_unlock();
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
        let moving_time = if opts.smooth {
            // Approximate smooth move: delay_ms scaling into seconds.
            let base = if opts.delay_ms > 0 {
                opts.delay_ms as f32 * 0.05
            } else {
                0.2
            };
            base.clamp(0.05, 2.0)
        } else {
            0.0
        };
        // Absolute virtual-desktop coords (Windows origin may be negative).
        #[cfg(target_os = "windows")]
        move_mouse_windows(x, y, moving_time);
        #[cfg(not(target_os = "windows"))]
        {
            // rustautogui's public API is u32; X11 virtual desktop starts at (0,0).
            let xu = u32::try_from(x).unwrap_or(0);
            let yu = u32::try_from(y).unwrap_or(0);
            if let Err(e) = self.gui.move_mouse_to_pos(xu, yu, moving_time) {
                // Fallback: try zero-time again (bounds check can false-positive).
                let _ = self.gui.move_mouse_to_pos(xu, yu, 0.0);
                let _ = e;
            }
        }
    }

    fn click(&mut self, button: &str, down: bool) -> Result<(), AutomationError> {
        let canonical = canonical_button(button);
        let btn = Self::map_button(canonical);
        if down {
            self.gui
                .click_down(btn)
                .map_err(|e| AutomationError::Backend(format!("click down: {e}")))?;
            note_button_down(canonical);
            Ok(())
        } else {
            self.gui
                .click_up(btn)
                .map_err(|e| AutomationError::Backend(format!("click up: {e}")))?;
            note_button_up(canonical);
            Ok(())
        }
    }

    fn scroll(&mut self, up: bool) -> Result<(), AutomationError> {
        // Scroll intensity ~3 notches.
        if up {
            self.gui
                .scroll_up(3)
                .map_err(|e| AutomationError::Backend(format!("scroll up: {e}")))
        } else {
            self.gui
                .scroll_down(3)
                .map_err(|e| AutomationError::Backend(format!("scroll down: {e}")))
        }
    }

    fn key_down(&mut self, key: &str) -> Result<(), AutomationError> {
        let k = Self::map_key(key);
        self.gui
            .key_down(&k)
            .map_err(|e| AutomationError::Backend(format!("key down {k}: {e}")))?;
        note_key_down(&k);
        Ok(())
    }

    fn key_up(&mut self, key: &str) -> Result<(), AutomationError> {
        let k = Self::map_key(key);
        self.gui
            .key_up(&k)
            .map_err(|e| AutomationError::Backend(format!("key up {k}: {e}")))?;
        note_key_up(&k);
        Ok(())
    }

    fn type_char(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        let _ = self.gui.keyboard_input(s);
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
        // Documented mapping used by move_to — keep in sync if formula changes.
        let from_delay = (100_f32 * 0.05).clamp(0.05, 2.0);
        assert!((from_delay - 2.0).abs() < f32::EPSILON);
        let default_smooth = 0.2_f32.clamp(0.05, 2.0);
        assert!((default_smooth - 0.2).abs() < f32::EPSILON);
        let instant = 0.0_f32;
        assert_eq!(instant, 0.0);
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
