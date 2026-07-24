//! Runtime adapters shared by macro execution (OCR, continue-wait, stop-watch).

use parking_lot::Mutex;
use sqyre_hotkeys::{ContinueWaitBridge, MacroHotkeyBridge, StopFlag};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) struct BridgeContinueWait {
    pub(crate) continue_wait: ContinueWaitBridge,
    pub(crate) macro_hotkeys: MacroHotkeyBridge,
}

impl sqyre_executor::ContinueKeyWaiter for BridgeContinueWait {
    fn wait_for_continue(
        &self,
        keys: &[String],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<(), String> {
        self.macro_hotkeys.suspend();
        let result = self
            .continue_wait
            .wait_for_continue(keys, pass_through, stop);
        self.macro_hotkeys.resume();
        result
    }

    fn wait_for_any_chord(
        &self,
        chords: &[Vec<String>],
        hold_repeat: &[bool],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<usize, String> {
        self.macro_hotkeys.suspend();
        let result = self
            .continue_wait
            .wait_for_any_chord(chords, hold_repeat, pass_through, stop);
        self.macro_hotkeys.resume();
        result
    }
}

pub(crate) struct RunState {
    pub(crate) stop: StopFlag,
    pub(crate) running: Arc<AtomicBool>,
    pub(crate) status: Arc<Mutex<String>>,
}

impl Default for RunState {
    fn default() -> Self {
        Self {
            stop: StopFlag::new(),
            running: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(String::new())),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::StopFlag;
    use sqyre_executor::{OcrEngine, OcrResult};
    use sqyre_input::OsAutomation;
    use sqyre_match::ImageBuf;
    use sqyre_vision::LeptessOcr;

    pub(crate) struct AppOcr(pub(crate) LeptessOcr);

    impl OcrEngine for AppOcr {
        fn recognize(&self, image: &ImageBuf) -> Result<OcrResult, String> {
            let r = self.0.recognize(image)?;
            Ok(OcrResult {
                text: r.text,
                words: r.words,
            })
        }
    }

    /// Forwards automation but surfaces stop via milli_sleep / between calls.
    pub(crate) struct StopWatchAutomation<'a> {
        pub(crate) inner: &'a mut OsAutomation,
        pub(crate) stop: &'a StopFlag,
    }

    impl sqyre_executor::AutomationBackend for StopWatchAutomation<'_> {
        fn milli_sleep(&mut self, ms: i32) {
            let mut left = ms.max(0);
            while left > 0 {
                if self.stop.is_stopped() {
                    return;
                }
                let chunk = left.min(50);
                self.inner.milli_sleep(chunk);
                left -= chunk;
            }
        }
        fn move_to(&mut self, x: i32, y: i32, opts: sqyre_executor::MoveOptions) {
            if !self.stop.is_stopped() {
                self.inner.move_to(x, y, opts);
            }
        }
        fn click(&mut self, button: &str, down: bool) -> Result<(), String> {
            // Always forward releases so end-of-macro cleanup can unstick buttons.
            if down && self.stop.is_stopped() {
                return Ok(());
            }
            self.inner.click(button, down)
        }
        fn scroll(&mut self, up: bool) -> Result<(), String> {
            if self.stop.is_stopped() {
                return Ok(());
            }
            self.inner.scroll(up)
        }
        fn key_down(&mut self, key: &str) -> Result<(), String> {
            if self.stop.is_stopped() {
                return Ok(());
            }
            self.inner.key_down(key)
        }
        fn key_up(&mut self, key: &str) -> Result<(), String> {
            // Always forward releases so end-of-macro cleanup can unstick keys.
            self.inner.key_up(key)
        }
        fn type_char(&mut self, ch: char) {
            if !self.stop.is_stopped() {
                self.inner.type_char(ch);
            }
        }
        fn write_clipboard(&mut self, s: &str) -> Result<(), String> {
            if self.stop.is_stopped() {
                return Ok(());
            }
            self.inner.write_clipboard(s)
        }
    }

    /// Ask glibc to return freed pages after large image/OCR allocations.
    pub(crate) fn trim_process_heap() {
        #[cfg(target_os = "linux")]
        {
            unsafe {
                extern "C" {
                    fn malloc_trim(pad: usize) -> i32;
                }
                let _ = malloc_trim(0);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::{trim_process_heap, AppOcr, StopWatchAutomation};
