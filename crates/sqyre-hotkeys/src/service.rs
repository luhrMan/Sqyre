//! Hotkey service trait, callbacks, stop flag, and null stub.
//!
//! Leaf module so platform hooks import types here instead of through `lib.rs`.

use crate::error::HotkeyError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Callbacks invoked from the hook thread (keep short).
#[derive(Clone)]
pub struct HotkeyCallbacks {
    pub on_escape_stop: Arc<dyn Fn() + Send + Sync>,
    pub on_failsafe: Arc<dyn Fn() + Send + Sync>,
    /// Fired with the macro name when a registered chord matches.
    pub on_macro_hotkey: Arc<dyn Fn(String) + Send + Sync>,
    /// Backend fallback / permission notes (evdev skip, portal→evdev, listen fail).
    /// The app logs these through its diag path; hotkeys never `eprintln!`.
    pub on_warn: Arc<dyn Fn(String) + Send + Sync>,
    /// Single key that stops a running macro (rdev/vk name like `esc`, `f12`;
    /// empty disables panic stop). Shared so Settings edits apply live without
    /// restarting the hook.
    pub panic_key: Arc<parking_lot::RwLock<String>>,
}

impl Default for HotkeyCallbacks {
    fn default() -> Self {
        Self {
            on_escape_stop: Arc::new(|| {}),
            on_failsafe: Arc::new(|| {}),
            on_macro_hotkey: Arc::new(|_| {}),
            on_warn: Arc::new(|_| {}),
            panic_key: Arc::new(parking_lot::RwLock::new("esc".to_string())),
        }
    }
}

/// Shared stop flag used by app + executor.
#[derive(Clone, Default)]
pub struct StopFlag(Arc<AtomicBool>);

impl StopFlag {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn request_stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn clear(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    pub fn raw(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.0)
    }
}

pub trait HotkeyService: Send {
    fn start(&mut self, callbacks: HotkeyCallbacks) -> Result<(), HotkeyError>;
    fn stop(&mut self);
}

/// Always-available stub (CI / tests).
#[derive(Debug, Default)]
pub struct NullHotkeys {
    running: bool,
}

impl HotkeyService for NullHotkeys {
    fn start(&mut self, _callbacks: HotkeyCallbacks) -> Result<(), HotkeyError> {
        self.running = true;
        Ok(())
    }
    fn stop(&mut self) {
        self.running = false;
    }
}
