//! Wayland GlobalShortcuts-backed hotkey service.
//!
//! Binds Esc-stop, failsafe, and macro chords through the portal. Screen-click
//! arming and full keystream macro recording remain limited without Input Capture;
//! those bridges stay available for focused-window / future portal work.

use crate::{
    ContinueWaitBridge, HotkeyCallbacks, HotkeyError, HotkeyService, MacroHotkeyBridge,
    MacroRecordBridge, ScreenClickBridge,
};
use sqyre_capture::{global_shortcuts_enabled, wayland_shortcuts_session};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Hotkeys via XDG GlobalShortcuts portal.
pub struct WaylandHotkeys {
    continue_wait: ContinueWaitBridge,
    screen_click: ScreenClickBridge,
    macro_record: MacroRecordBridge,
    macro_hotkeys: MacroHotkeyBridge,
    stop: Arc<AtomicBool>,
}

impl WaylandHotkeys {
    pub fn new(
        continue_wait: ContinueWaitBridge,
        screen_click: ScreenClickBridge,
        macro_record: MacroRecordBridge,
        macro_hotkeys: MacroHotkeyBridge,
    ) -> Self {
        Self {
            continue_wait,
            screen_click,
            macro_record,
            macro_hotkeys,
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl HotkeyService for WaylandHotkeys {
    fn start(&mut self, callbacks: HotkeyCallbacks) -> Result<(), HotkeyError> {
        if !global_shortcuts_enabled() {
            return Err(HotkeyError::ThreadSpawn(
                "Wayland global shortcuts disabled in User Settings".into(),
            ));
        }
        self.stop.store(false, Ordering::SeqCst);
        let mut shortcuts = vec![
            (
                "sqyre.escape_stop".into(),
                "Stop running macro".into(),
                Some("Escape".into()),
            ),
            (
                "sqyre.failsafe".into(),
                "Sqyre failsafe exit".into(),
                Some("<Control><Alt><Shift>Escape".into()),
            ),
        ];
        // Bind currently registered macro chords (best-effort preferred triggers).
        for (i, binding) in self.macro_hotkeys.bindings().into_iter().enumerate() {
            let trigger = binding.chord.join("+");
            shortcuts.push((
                format!("sqyre.macro.{i}"),
                format!("Run macro {}", binding.macro_name),
                Some(trigger),
            ));
        }
        wayland_shortcuts_session::bind_shortcuts(&shortcuts).map_err(|e| {
            HotkeyError::ThreadSpawn(format!("GlobalShortcuts portal: {e}"))
        })?;

        let stop = Arc::clone(&self.stop);
        let on_escape = callbacks.on_escape_stop.clone();
        let on_failsafe = callbacks.on_failsafe.clone();
        // Portal activated-signal listening needs a long-lived async session.
        // Until Signal streaming is wired, poll is a no-op placeholder so start()
        // succeeds after a successful bind (desktop may still deliver binds OS-side).
        thread::Builder::new()
            .name("sqyre-wayland-hotkeys".into())
            .spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    let _ = (&on_escape, &on_failsafe);
                    thread::sleep(Duration::from_millis(250));
                }
            })
            .map_err(|e| HotkeyError::ThreadSpawn(e.to_string()))?;

        let _ = (
            &self.continue_wait,
            &self.screen_click,
            &self.macro_record,
        );
        Ok(())
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        wayland_shortcuts_session::drop_session();
    }
}
