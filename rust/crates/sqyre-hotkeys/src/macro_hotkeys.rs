//! Per-macro chord register / match / fire (Go `macrohotkey` + `hotkeytrigger`).

use crate::normalize_key_name;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

/// How a macro chord fires once the keys are held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyTrigger {
    Press,
    Release,
}

impl HotkeyTrigger {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "release" => Self::Release,
            _ => Self::Press,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Press => "press",
            Self::Release => "release",
        }
    }

    pub fn ui_label(self) -> &'static str {
        match self {
            Self::Press => "On press",
            Self::Release => "On release",
        }
    }

    pub fn from_ui_label(s: &str) -> Self {
        if s.eq_ignore_ascii_case("On release") || s.eq_ignore_ascii_case("release") {
            Self::Release
        } else {
            Self::Press
        }
    }
}

/// One registered macro chord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroHotkeyBinding {
    pub macro_name: String,
    pub chord: Vec<String>,
    pub trigger: HotkeyTrigger,
}

impl MacroHotkeyBinding {
    pub fn new(
        macro_name: impl Into<String>,
        chord: Vec<String>,
        trigger: HotkeyTrigger,
    ) -> Self {
        Self {
            macro_name: macro_name.into(),
            chord: normalize_chord(&chord),
            trigger,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.chord.is_empty()
    }
}

/// Display `"ctrl + shift + a"` (Go `ReverseParseMacroHotkey`).
pub fn format_hotkey(chord: &[String]) -> String {
    chord.join(" + ")
}

/// Parse `"ctrl + shift + a"` → `["ctrl","shift","a"]` (Go `ParseMacroHotkey`).
pub fn parse_hotkey(display: &str) -> Vec<String> {
    let t = display.trim();
    if t.is_empty() || t == "—" || t == "-" {
        return Vec::new();
    }
    normalize_chord(
        &t.split('+')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>(),
    )
}

fn normalize_chord(keys: &[String]) -> Vec<String> {
    let mut out: Vec<String> = keys
        .iter()
        .map(|k| normalize_key_name(k))
        .filter(|k| !k.is_empty())
        .collect();
    // Stable order for display/compare; match does not require order.
    out.sort();
    out.dedup();
    out
}

/// Whether every chord key (or a modifier alias) is currently held.
pub fn chord_all_pressed(pressed: &HashSet<String>, chord: &[String]) -> bool {
    !chord.is_empty()
        && chord
            .iter()
            .all(|k| key_or_alias_pressed(pressed, k))
}

/// Whether no chord key (or alias) remains held.
pub fn chord_fully_released(pressed: &HashSet<String>, chord: &[String]) -> bool {
    chord.iter().all(|k| !key_or_alias_pressed(pressed, k))
}

fn key_or_alias_pressed(pressed: &HashSet<String>, key: &str) -> bool {
    if pressed.contains(key) {
        return true;
    }
    match key {
        "shift" => pressed.contains("rshift"),
        "rshift" => pressed.contains("shift"),
        "alt" => pressed.contains("ralt"),
        "ralt" => pressed.contains("alt"),
        "cmd" => pressed.contains("rcmd") || pressed.contains("super"),
        "rcmd" => pressed.contains("cmd") || pressed.contains("super"),
        "super" => pressed.contains("cmd") || pressed.contains("rcmd"),
        "backspace" => pressed.contains("delete"),
        "delete" => pressed.contains("backspace"),
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReleasePhase {
    Idle,
    WaitingLeave,
    WaitingFullRelease,
}

struct BindingRuntime {
    binding: MacroHotkeyBinding,
    latched: bool,
    release: ReleasePhase,
}

struct Inner {
    bindings: Mutex<Vec<BindingRuntime>>,
    suspend_count: Mutex<u32>,
    pressed: Mutex<HashSet<String>>,
}

/// Shared registry between the hook thread and the UI.
#[derive(Clone)]
pub struct MacroHotkeyBridge {
    inner: Arc<Inner>,
}

impl MacroHotkeyBridge {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                bindings: Mutex::new(Vec::new()),
                suspend_count: Mutex::new(0),
                pressed: Mutex::new(HashSet::new()),
            }),
        }
    }

    /// Replace all bindings (skip empty chords). Resets per-chord fire state.
    pub fn set_bindings(&self, bindings: Vec<MacroHotkeyBinding>) {
        let runtimes = bindings
            .into_iter()
            .filter(|b| !b.is_empty())
            .map(|binding| BindingRuntime {
                binding,
                latched: false,
                release: ReleasePhase::Idle,
            })
            .collect();
        *self.inner.bindings.lock() = runtimes;
    }

    pub fn bindings(&self) -> Vec<MacroHotkeyBinding> {
        self.inner
            .bindings
            .lock()
            .iter()
            .map(|r| r.binding.clone())
            .collect()
    }

    /// Refcounted: while >0, chords do not fire.
    pub fn suspend(&self) {
        *self.inner.suspend_count.lock() += 1;
        self.reset_fire_state();
    }

    pub fn resume(&self) {
        let mut c = self.inner.suspend_count.lock();
        if *c > 0 {
            *c -= 1;
        }
        if *c == 0 {
            self.reset_fire_state();
        }
    }

    pub fn is_suspended(&self) -> bool {
        *self.inner.suspend_count.lock() > 0
    }

    fn reset_fire_state(&self) {
        for r in self.inner.bindings.lock().iter_mut() {
            r.latched = false;
            r.release = ReleasePhase::Idle;
        }
    }

    /// Snapshot of currently pressed key names (for record UI).
    pub fn pressed_keys(&self) -> Vec<String> {
        let mut v: Vec<String> = self.inner.pressed.lock().iter().cloned().collect();
        v.sort();
        v
    }

    /// Hotkey thread: mirror pressed set + evaluate chords.
    pub fn on_pressed_keys(
        &self,
        pressed: &HashSet<String>,
        on_fire: &dyn Fn(String),
    ) {
        *self.inner.pressed.lock() = pressed.clone();
        if *self.inner.suspend_count.lock() > 0 {
            return;
        }

        let mut bindings = self.inner.bindings.lock();
        for runtime in bindings.iter_mut() {
            let all = chord_all_pressed(pressed, &runtime.binding.chord);
            let released = chord_fully_released(pressed, &runtime.binding.chord);
            match runtime.binding.trigger {
                HotkeyTrigger::Press => {
                    if all {
                        if !runtime.latched {
                            runtime.latched = true;
                            on_fire(runtime.binding.macro_name.clone());
                        }
                    } else if runtime.latched {
                        runtime.latched = false;
                    }
                }
                HotkeyTrigger::Release => match runtime.release {
                    ReleasePhase::Idle => {
                        if all {
                            runtime.release = ReleasePhase::WaitingLeave;
                        }
                    }
                    ReleasePhase::WaitingLeave => {
                        if !all {
                            runtime.release = ReleasePhase::WaitingFullRelease;
                        }
                    }
                    ReleasePhase::WaitingFullRelease => {
                        if released {
                            runtime.release = ReleasePhase::Idle;
                            on_fire(runtime.binding.macro_name.clone());
                        } else if all {
                            // Chord re-held before full release — restart wait.
                            runtime.release = ReleasePhase::WaitingLeave;
                        }
                    }
                },
            }
        }
    }

    /// Block until `chord` is fully released (used after recording).
    pub fn wait_until_released(&self, chord: &[String], poll: Duration) {
        let chord = normalize_chord(chord);
        if chord.is_empty() {
            return;
        }
        loop {
            {
                let pressed = self.inner.pressed.lock();
                if chord_fully_released(&pressed, &chord) {
                    return;
                }
            }
            std::thread::sleep(poll);
        }
    }
}

impl Default for MacroHotkeyBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn parse_and_format_roundtrip() {
        let keys = parse_hotkey("ctrl + shift + a");
        assert_eq!(keys, vec!["a".to_string(), "ctrl".into(), "shift".into()]);
        assert_eq!(format_hotkey(&keys), "a + ctrl + shift");
        assert!(parse_hotkey("—").is_empty());
        assert!(parse_hotkey("").is_empty());
    }

    #[test]
    fn trigger_parse_and_labels() {
        assert_eq!(HotkeyTrigger::parse("release"), HotkeyTrigger::Release);
        assert_eq!(HotkeyTrigger::parse(""), HotkeyTrigger::Press);
        assert_eq!(
            HotkeyTrigger::from_ui_label("On release"),
            HotkeyTrigger::Release
        );
        assert_eq!(HotkeyTrigger::Press.ui_label(), "On press");
    }

    #[test]
    fn press_fires_once_until_release() {
        let bridge = MacroHotkeyBridge::new();
        bridge.set_bindings(vec![MacroHotkeyBinding::new(
            "M",
            vec!["ctrl".into(), "a".into()],
            HotkeyTrigger::Press,
        )]);
        let fires = AtomicUsize::new(0);
        let fire = |_: String| {
            fires.fetch_add(1, Ordering::SeqCst);
        };

        let mut pressed = HashSet::new();
        pressed.insert("ctrl".into());
        pressed.insert("a".into());
        bridge.on_pressed_keys(&pressed, &fire);
        bridge.on_pressed_keys(&pressed, &fire); // repeat / latch
        assert_eq!(fires.load(Ordering::SeqCst), 1);

        pressed.remove("a");
        bridge.on_pressed_keys(&pressed, &fire);
        pressed.insert("a".into());
        bridge.on_pressed_keys(&pressed, &fire);
        assert_eq!(fires.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn release_fires_after_full_release() {
        let bridge = MacroHotkeyBridge::new();
        bridge.set_bindings(vec![MacroHotkeyBinding::new(
            "M",
            vec!["ctrl".into(), "b".into()],
            HotkeyTrigger::Release,
        )]);
        let fires = AtomicUsize::new(0);
        let fire = |name: String| {
            assert_eq!(name, "M");
            fires.fetch_add(1, Ordering::SeqCst);
        };

        let mut pressed = HashSet::new();
        pressed.insert("ctrl".into());
        pressed.insert("b".into());
        bridge.on_pressed_keys(&pressed, &fire);
        assert_eq!(fires.load(Ordering::SeqCst), 0);

        pressed.remove("b");
        bridge.on_pressed_keys(&pressed, &fire);
        assert_eq!(fires.load(Ordering::SeqCst), 0);

        pressed.remove("ctrl");
        bridge.on_pressed_keys(&pressed, &fire);
        assert_eq!(fires.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn suspend_blocks_fire() {
        let bridge = MacroHotkeyBridge::new();
        bridge.set_bindings(vec![MacroHotkeyBinding::new(
            "M",
            vec!["f9".into()],
            HotkeyTrigger::Press,
        )]);
        let fires = AtomicUsize::new(0);
        let fire = |_: String| {
            fires.fetch_add(1, Ordering::SeqCst);
        };
        bridge.suspend();
        let mut pressed = HashSet::new();
        pressed.insert("f9".into());
        bridge.on_pressed_keys(&pressed, &fire);
        assert_eq!(fires.load(Ordering::SeqCst), 0);
        bridge.resume();
        pressed.clear();
        bridge.on_pressed_keys(&pressed, &fire);
        pressed.insert("f9".into());
        bridge.on_pressed_keys(&pressed, &fire);
        assert_eq!(fires.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn shift_alias_matches_rshift() {
        let mut pressed = HashSet::new();
        pressed.insert("rshift".into());
        pressed.insert("a".into());
        assert!(chord_all_pressed(
            &pressed,
            &["shift".into(), "a".into()]
        ));
    }
}
