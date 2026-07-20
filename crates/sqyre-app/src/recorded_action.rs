use sqyre_domain::ActionKind;

pub(crate) fn apply_recorded_key(kind: &mut ActionKind, recorded: String) {
    if let ActionKind::Key { key, .. } = kind {
        *key = recorded;
    }
}

pub(crate) fn apply_recorded_chord(kind: &mut ActionKind, recorded: Vec<String>) -> bool {
    if let ActionKind::Pause { continue_key, .. } = kind {
        *continue_key = recorded;
        true
    } else {
        false
    }
}

pub(crate) fn apply_recorded_color(kind: &mut ActionKind, recorded: String) {
    if let ActionKind::FindPixel { target_color, .. } = kind {
        *target_color = sqyre_domain::normalize_hex_rgb(&recorded);
    }
}
