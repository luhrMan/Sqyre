//! Process-wide held key/button tracking and injectable release.
//!
//! Hold state is OS-neutral so abort/failsafe paths can be tested without a
//! display, portal, or `/dev/input`.

use std::collections::HashSet;

/// Keys/buttons this process still has down (after a successful `key_down` /
/// click-down). Names are always [`canonical_key`] / [`canonical_button`].
#[derive(Debug, Default, Clone)]
pub struct HeldInputs {
    keys: HashSet<String>,
    buttons: HashSet<String>,
}

impl HeldInputs {
    pub fn note_key_down(&mut self, key: &str) {
        self.keys.insert(canonical_key(key));
    }

    pub fn note_key_up(&mut self, key: &str) {
        self.keys.remove(&canonical_key(key));
    }

    pub fn note_button_down(&mut self, button: &str) {
        self.buttons.insert(canonical_button(button).to_string());
    }

    pub fn note_button_up(&mut self, button: &str) {
        self.buttons.remove(canonical_button(button));
    }

    pub fn take(&mut self) -> (HashSet<String>, HashSet<String>) {
        (
            std::mem::take(&mut self.keys),
            std::mem::take(&mut self.buttons),
        )
    }

    /// Put keys/buttons back after a failed release so a later attempt can retry.
    pub fn reinsert(&mut self, keys: HashSet<String>, buttons: HashSet<String>) {
        self.keys.extend(keys);
        self.buttons.extend(buttons);
    }
}

/// Sink used by [`release_held_into`] — production wraps portal / rustautogui;
/// tests use a recording fake.
pub trait HeldReleaseSink {
    fn key_up(&mut self, key: &str) -> Result<(), String>;
    fn button_up(&mut self, button: &str) -> Result<(), String>;
}

/// Release via `sink`. When `sink` is `None`, returns the inputs unchanged so
/// the caller can [`HeldInputs::reinsert`] them for a later retry.
pub fn release_held_into(
    keys: HashSet<String>,
    buttons: HashSet<String>,
    sink: Option<&mut dyn HeldReleaseSink>,
) -> (HashSet<String>, HashSet<String>) {
    let Some(sink) = sink else {
        return (keys, buttons);
    };
    for key in &keys {
        let _ = sink.key_up(key);
    }
    for button in &buttons {
        let _ = sink.button_up(button);
    }
    (HashSet::new(), HashSet::new())
}

/// Canonical hold-tracking / rustautogui key name.
///
/// Portal `evdev_for_name` accepts the same aliases, so storing this form once
/// at note_key_down keeps portal and rustautogui release paths aligned.
pub fn canonical_key(key: &str) -> String {
    match key.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => "control".into(),
        "cmd" | "command" | "super" | "win" => "command".into(),
        "esc" | "escape" => "escape".into(),
        "return" | "enter" => "enter".into(),
        "space" | "spacebar" => "space".into(),
        other => other.to_string(),
    }
}

/// Canonical button name used for hold tracking (`left` / `right` / `middle`).
pub fn canonical_button(button: &str) -> &'static str {
    match button {
        "right" => "right",
        "center" | "middle" => "middle",
        _ => "left",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct RecordingSink {
        keys: Mutex<Vec<String>>,
        buttons: Mutex<Vec<String>>,
        fail: bool,
    }

    impl HeldReleaseSink for RecordingSink {
        fn key_up(&mut self, key: &str) -> Result<(), String> {
            if self.fail {
                return Err("unavailable".into());
            }
            self.keys.lock().unwrap().push(key.to_string());
            Ok(())
        }

        fn button_up(&mut self, button: &str) -> Result<(), String> {
            if self.fail {
                return Err("unavailable".into());
            }
            self.buttons.lock().unwrap().push(button.to_string());
            Ok(())
        }
    }

    #[test]
    fn hold_tracking_take_clears() {
        let mut held = HeldInputs::default();
        held.note_key_down("control");
        held.note_key_down("a");
        held.note_button_down("left");
        held.note_key_up("a");
        let (keys, buttons) = held.take();
        assert!(keys.contains("control"));
        assert!(!keys.contains("a"));
        assert!(buttons.contains("left"));
        let (keys2, buttons2) = held.take();
        assert!(keys2.is_empty());
        assert!(buttons2.is_empty());
    }

    #[test]
    fn release_via_sink_clears_keys_and_buttons() {
        let mut held = HeldInputs::default();
        held.note_key_down("ctrl");
        held.note_button_down("center");
        let (keys, buttons) = held.take();
        let mut sink = RecordingSink {
            keys: Mutex::new(Vec::new()),
            buttons: Mutex::new(Vec::new()),
            fail: false,
        };
        let (left_k, left_b) = release_held_into(keys, buttons, Some(&mut sink));
        assert!(left_k.is_empty());
        assert!(left_b.is_empty());
        assert_eq!(
            sink.keys.lock().unwrap().as_slice(),
            &["control".to_string()]
        );
        assert_eq!(
            sink.buttons.lock().unwrap().as_slice(),
            &["middle".to_string()]
        );
    }

    #[test]
    fn release_reinserts_when_sink_unavailable() {
        let mut held = HeldInputs::default();
        held.note_key_down("escape");
        held.note_button_down("right");
        let (keys, buttons) = held.take();
        let (left_k, left_b) = release_held_into(keys, buttons, None);
        held.reinsert(left_k, left_b);
        let (keys, buttons) = held.take();
        assert!(keys.contains("escape"));
        assert!(buttons.contains("right"));
    }

    #[test]
    fn portal_and_gui_use_same_key_canonical_names() {
        assert_eq!(canonical_key("ctrl"), "control");
        assert_eq!(canonical_key("control"), "control");
        assert_eq!(canonical_key("esc"), "escape");
        assert_eq!(canonical_key("escape"), "escape");
        assert_eq!(canonical_key("return"), "enter");
        assert_eq!(canonical_key("CMD"), "command");
        assert_eq!(canonical_key("super"), "command");
        assert_eq!(canonical_key("a"), "a");
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
