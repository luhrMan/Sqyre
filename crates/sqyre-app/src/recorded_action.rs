use sqyre_domain::{ActionKind, NavChords};

/// Where a finished [`crate::hotkey_record::HotkeyRecordUi`] chord should land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ChordRecordTarget {
    /// Pause continue-key or Navigate Key chord when those drafts are open;
    /// otherwise the selected macro's global hotkey.
    #[default]
    Inferred,
    /// A Navigate Select direction / select / back chord on the open draft.
    NavSelect(NavChordSlot),
}

/// Which [`NavChords`] field a Navigate Select recording targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavChordSlot {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
}

impl NavChordSlot {
    pub(crate) fn get_mut(self, chords: &mut NavChords) -> &mut Vec<String> {
        match self {
            Self::Up => &mut chords.up,
            Self::Down => &mut chords.down,
            Self::Left => &mut chords.left,
            Self::Right => &mut chords.right,
            Self::Select => &mut chords.select,
            Self::Back => &mut chords.back,
        }
    }
}

pub(crate) fn apply_recorded_key(kind: &mut ActionKind, recorded: String) {
    if let ActionKind::Key { key, .. } = kind {
        *key = recorded;
    }
}

/// Apply a recorded chord. Returns true when `kind` consumed it (so callers
/// should not fall through to the macro hotkey).
pub(crate) fn apply_recorded_chord(
    kind: &mut ActionKind,
    recorded: Vec<String>,
    target: ChordRecordTarget,
) -> bool {
    match target {
        ChordRecordTarget::Inferred => match kind {
            ActionKind::Pause { continue_key, .. } => {
                *continue_key = recorded;
                true
            }
            ActionKind::NavigateKey { chord, .. } => {
                *chord = recorded;
                true
            }
            _ => false,
        },
        ChordRecordTarget::NavSelect(slot) => {
            if let ActionKind::NavigateSelect(data) = kind {
                *slot.get_mut(&mut data.chords) = recorded;
                true
            } else {
                false
            }
        }
    }
}

#[cfg(feature = "native-runtime")]
pub(crate) fn apply_recorded_color(kind: &mut ActionKind, recorded: String) {
    if let ActionKind::FindPixel(d) = kind {
        d.target_color = sqyre_domain::normalize_hex_rgb(&recorded);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::PressState;

    #[test]
    fn inferred_applies_to_pause_and_nav_key() {
        let mut pause = ActionKind::Pause {
            message: String::new(),
            continue_key: Vec::new(),
            pass_through: false,
        };
        assert!(apply_recorded_chord(
            &mut pause,
            vec!["enter".into()],
            ChordRecordTarget::Inferred,
        ));
        match &pause {
            ActionKind::Pause { continue_key, .. } => {
                assert_eq!(continue_key, &["enter".to_string()]);
            }
            other => panic!("unexpected {other:?}"),
        }

        let mut nav = ActionKind::NavigateKey {
            name: "branch".into(),
            chord: Vec::new(),
            exit: false,
            subactions: Vec::new(),
        };
        assert!(apply_recorded_chord(
            &mut nav,
            vec!["f1".into()],
            ChordRecordTarget::Inferred,
        ));
        match &nav {
            ActionKind::NavigateKey { chord, .. } => assert_eq!(chord, &["f1".to_string()]),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn nav_select_slot_targets_correct_chord() {
        let mut kind = ActionKind::NavigateSelect(Box::default());
        assert!(apply_recorded_chord(
            &mut kind,
            vec!["up".into()],
            ChordRecordTarget::NavSelect(NavChordSlot::Up),
        ));
        assert!(apply_recorded_chord(
            &mut kind,
            vec!["left".into(), "alt".into()],
            ChordRecordTarget::NavSelect(NavChordSlot::Left),
        ));
        match &kind {
            ActionKind::NavigateSelect(data) => {
                assert_eq!(data.chords.up, ["up".to_string()]);
                assert_eq!(data.chords.left, ["left".to_string(), "alt".to_string()]);
                // Default blank chords leave other slots at their starter values.
                assert_eq!(data.chords.down, ["down".to_string()]);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn inferred_skips_unrelated_kinds() {
        let mut key = ActionKind::Key {
            key: String::new(),
            state: PressState::Tap,
        };
        assert!(!apply_recorded_chord(
            &mut key,
            vec!["a".into()],
            ChordRecordTarget::Inferred,
        ));
    }
}
