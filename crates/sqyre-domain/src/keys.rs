//! Pure hotkey-name vocabulary: normalization, the failsafe chord, and chord validation.
//!
//! No OS hooks live here — `sqyre-hotkeys` owns listening and waiting, this module
//! owns the key-name strings that end up in `db.yaml` and the rules about them.

use std::collections::HashSet;
use thiserror::Error;

/// Failure validating a Pause continue-key or wait chord against the failsafe.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KeyError {
    #[error("pause: continue key not set")]
    ContinueKeyNotSet,
    #[error("pause: continue key cannot match the failsafe hotkey ({label})", label = FAILSAFE_LABEL)]
    ContinueKeyIsFailsafe,
    #[error("key wait: chord cannot match the failsafe hotkey ({label})", label = FAILSAFE_LABEL)]
    WaitChordIsFailsafe,
}

/// Canonical Sqyre key name for `key` (lowercase, left/right variants folded where
/// the chord vocabulary does not distinguish them).
pub fn normalize_key_name(key: &str) -> String {
    match key.trim().to_ascii_lowercase().as_str() {
        "escape" => "esc".into(),
        "control" | "rcontrol" | "controlleft" | "controlright" => "ctrl".into(),
        "return" => "enter".into(),
        "super" | "meta" | "win" | "windows" | "meta_left" | "metaleft" => "cmd".into(),
        "meta_right" | "metaright" => "rcmd".into(),
        "backspace" | "back_space" => "delete".into(),
        "shiftleft" | "shift_left" => "shift".into(),
        "shiftright" | "shift_right" => "rshift".into(),
        "altleft" | "alt_left" => "alt".into(),
        "altright" | "alt_right" | "altgr" => "ralt".into(),
        other => other.to_string(),
    }
}

/// Normalize a chord, dropping entries that normalize to an empty name.
pub fn normalize_keys(keys: &[String]) -> Vec<String> {
    keys.iter()
        .map(|k| normalize_key_name(k))
        .filter(|k| !k.is_empty())
        .collect()
}

/// Emergency-stop chord keys (order-independent): Ctrl + Alt + Shift + Esc.
///
/// Includes Alt so the chord does not collide with Windows Task Manager
/// (`Ctrl+Shift+Esc`), which the OS often delivers outside user-mode hooks.
pub const FAILSAFE_KEYS: &[&str] = &["alt", "ctrl", "esc", "shift"];

/// Human-readable failsafe chord for UI / logs.
pub const FAILSAFE_LABEL: &str = "Esc+Ctrl+Alt+Shift";

/// Collapse left/right modifier variants for failsafe matching.
fn canonicalize_failsafe_key(key: &str) -> &str {
    match key {
        "rshift" => "shift",
        "ralt" => "alt",
        other => other,
    }
}

/// Whether `keys` (already normalized) match the failsafe chord.
///
/// Left/right Shift and Alt are treated as equivalent.
pub fn is_failsafe_chord(keys: &[String]) -> bool {
    let mut sorted: Vec<&str> = keys
        .iter()
        .map(|k| canonicalize_failsafe_key(k.as_str()))
        .collect();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != FAILSAFE_KEYS.len() {
        return false;
    }
    let mut failsafe = FAILSAFE_KEYS.to_vec();
    failsafe.sort_unstable();
    sorted == failsafe
}

/// Whether Ctrl, Alt, and Shift are all held (left/right variants accepted).
pub fn failsafe_modifiers_held(pressed: &HashSet<&'static str>) -> bool {
    let ctrl = pressed.contains("ctrl");
    let alt = pressed.contains("alt") || pressed.contains("ralt");
    let shift = pressed.contains("shift") || pressed.contains("rshift");
    ctrl && alt && shift
}

/// Normalize and validate a Pause continue-key chord.
///
/// Returns normalized key names, or an error if empty / equals the failsafe chord.
pub fn validate_continue_key(keys: &[String]) -> Result<Vec<String>, KeyError> {
    let normalized = normalize_keys(keys);
    if normalized.is_empty() {
        return Err(KeyError::ContinueKeyNotSet);
    }
    if is_failsafe_chord(&normalized) {
        return Err(KeyError::ContinueKeyIsFailsafe);
    }
    Ok(normalized)
}

/// Reject a wait chord that collides with the failsafe hotkey.
pub fn validate_not_failsafe(keys: &[String]) -> Result<(), KeyError> {
    if is_failsafe_chord(keys) {
        return Err(KeyError::WaitChordIsFailsafe);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_aliases_and_case() {
        assert_eq!(normalize_key_name(" Escape "), "esc");
        assert_eq!(normalize_key_name("ControlRight"), "ctrl");
        assert_eq!(normalize_key_name("AltGr"), "ralt");
        assert_eq!(normalize_key_name("F9"), "f9");
    }

    #[test]
    fn normalize_keys_drops_blanks() {
        let keys = vec!["Ctrl".to_string(), "  ".to_string(), "A".to_string()];
        assert_eq!(normalize_keys(&keys), vec!["ctrl", "a"]);
    }

    #[test]
    fn failsafe_chord_requires_alt() {
        assert!(!is_failsafe_chord(&[
            "ctrl".into(),
            "esc".into(),
            "shift".into(),
        ]));
        assert!(is_failsafe_chord(&[
            "ctrl".into(),
            "alt".into(),
            "esc".into(),
            "shift".into(),
        ]));
        assert!(is_failsafe_chord(&[
            "ctrl".into(),
            "ralt".into(),
            "esc".into(),
            "rshift".into(),
        ]));
    }

    #[test]
    fn failsafe_modifiers_accept_left_right() {
        let mut pressed = HashSet::new();
        pressed.insert("ctrl");
        pressed.insert("ralt");
        pressed.insert("rshift");
        assert!(failsafe_modifiers_held(&pressed));
        pressed.remove("ralt");
        assert!(!failsafe_modifiers_held(&pressed));
    }

    #[test]
    fn continue_key_rejects_empty_and_failsafe() {
        assert_eq!(
            validate_continue_key(&[]).unwrap_err(),
            KeyError::ContinueKeyNotSet
        );
        let failsafe = vec![
            "Escape".to_string(),
            "Control".to_string(),
            "alt".to_string(),
            "shift".to_string(),
        ];
        let err = validate_continue_key(&failsafe).unwrap_err();
        assert_eq!(err, KeyError::ContinueKeyIsFailsafe);
        assert!(err.to_string().contains("failsafe"));
        assert_eq!(
            validate_continue_key(&["F9".to_string()]).unwrap(),
            vec!["f9"]
        );
    }
}
