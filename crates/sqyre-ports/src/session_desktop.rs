//! COSMIC session detection shared by the EIS adapter and the input backend —
//! both gate the same warp workarounds on the session's desktop name.

/// A `XDG_*_DESKTOP`-style value names a COSMIC session.
pub fn desktop_name_is_cosmic(value: &str) -> bool {
    value.to_ascii_lowercase().contains("cosmic")
}

/// Any session desktop env var names COSMIC.
pub fn cosmic_desktop_session() -> bool {
    [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
    ]
    .into_iter()
    .any(|key| {
        std::env::var(key)
            .ok()
            .is_some_and(|value| desktop_name_is_cosmic(&value))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_name_matches_cosmic_only() {
        assert!(desktop_name_is_cosmic("COSMIC"));
        assert!(desktop_name_is_cosmic("pop:COSMIC"));
        assert!(!desktop_name_is_cosmic("GNOME"));
    }
}
