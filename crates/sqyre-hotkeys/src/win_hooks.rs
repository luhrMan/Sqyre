//! Win32 low-level keyboard/mouse hooks for global hotkeys.
//!
//! Replaces `rdev` on Windows: keeps the LL hook path fast (VK map only — no
//! `AttachThreadInput` / `ToUnicodeEx`) so Windows does not silently remove the hook.

use crate::continue_wait::{vk_key_name, ContinueWaitBridge};
use crate::macro_hotkeys::MacroHotkeyBridge;
use crate::screen_click::ScreenClickBridge;
use crate::{HotkeyCallbacks, HotkeyService};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MSG,
    MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_MOUSEMOVE, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

struct HookCtx {
    stop: Arc<AtomicBool>,
    callbacks: HotkeyCallbacks,
    continue_wait: ContinueWaitBridge,
    screen_click: ScreenClickBridge,
    macro_hotkeys: MacroHotkeyBridge,
    pressed: HashSet<String>,
}

static CTX: Mutex<Option<HookCtx>> = Mutex::new(None);
static KEY_HOOK: AtomicIsize = AtomicIsize::new(0);
static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

fn hhook_from_atomic(v: &AtomicIsize) -> HHOOK {
    HHOOK(v.load(Ordering::SeqCst) as *mut _)
}

fn store_hhook(slot: &AtomicIsize, hook: HHOOK) {
    slot.store(hook.0 as isize, Ordering::SeqCst);
}

fn take_hhook(slot: &AtomicIsize) -> HHOOK {
    HHOOK(slot.swap(0, Ordering::SeqCst) as *mut _)
}

pub struct WinHotkeys {
    stop: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
    continue_wait: ContinueWaitBridge,
    screen_click: ScreenClickBridge,
    macro_hotkeys: MacroHotkeyBridge,
}

impl WinHotkeys {
    pub fn new(
        continue_wait: ContinueWaitBridge,
        screen_click: ScreenClickBridge,
        macro_hotkeys: MacroHotkeyBridge,
    ) -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            join: Mutex::new(None),
            continue_wait,
            screen_click,
            macro_hotkeys,
        }
    }
}

impl HotkeyService for WinHotkeys {
    fn start(&mut self, callbacks: HotkeyCallbacks) -> Result<(), String> {
        self.stop();
        let stop = Arc::clone(&self.stop);
        stop.store(false, Ordering::SeqCst);

        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let continue_wait = self.continue_wait.clone();
        let screen_click = self.screen_click.clone();
        let macro_hotkeys = self.macro_hotkeys.clone();

        let handle = thread::Builder::new()
            .name("sqyre-hotkeys".into())
            .spawn(move || {
                *CTX.lock() = Some(HookCtx {
                    stop,
                    callbacks,
                    continue_wait,
                    screen_click,
                    macro_hotkeys,
                    pressed: HashSet::new(),
                });

                let install = (|| -> Result<(), String> {
                    // SAFETY: LL hooks; callback is valid for the lifetime of this thread.
                    let key =
                        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) }
                            .map_err(|e| format!("WH_KEYBOARD_LL: {e}"))?;
                    // SAFETY: same as above for mouse.
                    let mouse =
                        unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0) }
                            .map_err(|e| {
                                let _ = unsafe { UnhookWindowsHookEx(key) };
                                format!("WH_MOUSE_LL: {e}")
                            })?;
                    store_hhook(&KEY_HOOK, key);
                    store_hhook(&MOUSE_HOOK, mouse);
                    HOOK_THREAD_ID.store(unsafe { GetCurrentThreadId() }, Ordering::SeqCst);
                    Ok(())
                })();

                if let Err(e) = install {
                    *CTX.lock() = None;
                    let _ = ready_tx.send(Err(e));
                    return;
                }
                let _ = ready_tx.send(Ok(()));

                let mut msg = MSG::default();
                loop {
                    // SAFETY: standard Win32 message pump for the hook thread.
                    let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                    if ret.0 == 0 || ret.0 == -1 {
                        break;
                    }
                    unsafe {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                unhook_all();
                *CTX.lock() = None;
                HOOK_THREAD_ID.store(0, Ordering::SeqCst);
            })
            .map_err(|e| format!("hotkey thread: {e}"))?;

        match ready_rx.recv() {
            Ok(Ok(())) => {
                *self.join.lock() = Some(handle);
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = handle.join();
                Err(e)
            }
            Err(_) => {
                let _ = handle.join();
                Err("hotkey thread exited before ready".into())
            }
        }
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let tid = HOOK_THREAD_ID.load(Ordering::SeqCst);
        if tid != 0 {
            // SAFETY: wakes GetMessageW on the hook thread.
            let _ = unsafe { PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0)) };
        }
        if let Some(handle) = self.join.lock().take() {
            let _ = handle.join();
        }
        unhook_all();
        *CTX.lock() = None;
        HOOK_THREAD_ID.store(0, Ordering::SeqCst);
    }
}

fn unhook_all() {
    let key = take_hhook(&KEY_HOOK);
    let mouse = take_hhook(&MOUSE_HOOK);
    if !key.0.is_null() {
        // SAFETY: hook installed by this process.
        let _ = unsafe { UnhookWindowsHookEx(key) };
    }
    if !mouse.0.is_null() {
        let _ = unsafe { UnhookWindowsHookEx(mouse) };
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let _ = std::panic::catch_unwind(|| handle_keyboard(wparam, lparam));
    }
    let hook = hhook_from_atomic(&KEY_HOOK);
    // SAFETY: forwarding to next hook in the chain.
    unsafe { CallNextHookEx(Some(hook), code, wparam, lparam) }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let _ = std::panic::catch_unwind(|| handle_mouse(wparam, lparam));
    }
    let hook = hhook_from_atomic(&MOUSE_HOOK);
    // SAFETY: forwarding to next hook in the chain.
    unsafe { CallNextHookEx(Some(hook), code, wparam, lparam) }
}

fn handle_keyboard(wparam: WPARAM, lparam: LPARAM) {
    let msg = wparam.0 as u32;
    // SAFETY: lparam points at KBDLLHOOKSTRUCT for the duration of the hook call.
    let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let extended = kb.flags.contains(LLKHF_EXTENDED);
    let Some(name) = vk_key_name(kb.vkCode, extended) else {
        return;
    };
    let is_press = matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN);
    let is_release = matches!(msg, WM_KEYUP | WM_SYSKEYUP);
    if !is_press && !is_release {
        return;
    }

    let mut guard = CTX.lock();
    let Some(ctx) = guard.as_mut() else {
        return;
    };
    if ctx.stop.load(Ordering::SeqCst) {
        return;
    }

    if is_press {
        ctx.pressed.insert(name.clone());
    } else {
        ctx.pressed.remove(&name);
    }

    ctx.continue_wait.on_pressed_keys(&ctx.pressed);
    let on_fire = Arc::clone(&ctx.callbacks.on_macro_hotkey);
    ctx.macro_hotkeys.on_pressed_keys(&ctx.pressed, &*on_fire);

    if is_press && name == "esc" {
        let ctrl = ctx.pressed.contains("ctrl");
        let shift = ctx.pressed.contains("shift") || ctx.pressed.contains("rshift");
        if ctx.screen_click.on_escape() {
            // Recording takes Esc; don't also stop macros.
        } else if ctrl && shift {
            let on_failsafe = Arc::clone(&ctx.callbacks.on_failsafe);
            drop(guard);
            on_failsafe();
        } else if !ctrl && !shift && !ctx.continue_wait.continue_is_escape() {
            let on_escape = Arc::clone(&ctx.callbacks.on_escape_stop);
            drop(guard);
            on_escape();
        }
    }
}

fn handle_mouse(wparam: WPARAM, lparam: LPARAM) {
    let msg = wparam.0 as u32;
    let mut guard = CTX.lock();
    let Some(ctx) = guard.as_mut() else {
        return;
    };
    if ctx.stop.load(Ordering::SeqCst) {
        return;
    }

    if msg == WM_MOUSEMOVE {
        // SAFETY: lparam points at MSLLHOOKSTRUCT for the duration of the hook call.
        let mouse = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        ctx.screen_click.on_mouse_move(mouse.pt.x, mouse.pt.y);
    } else if msg == WM_LBUTTONDOWN && ctx.screen_click.is_armed() {
        ctx.screen_click.on_left_click();
    }
}

#[cfg(test)]
mod tests {
    use crate::continue_wait::vk_key_name;

    #[test]
    fn vk_map_matches_sqyre_names() {
        assert_eq!(vk_key_name(0x1B, false).as_deref(), Some("esc"));
        assert_eq!(vk_key_name(0xA2, false).as_deref(), Some("ctrl"));
        assert_eq!(vk_key_name(0xA0, false).as_deref(), Some("shift"));
        assert_eq!(vk_key_name(0x41, false).as_deref(), Some("a"));
        assert_eq!(vk_key_name(0x0D, false).as_deref(), Some("enter"));
        assert_eq!(vk_key_name(0x0D, true).as_deref(), Some("num_enter"));
    }
}
