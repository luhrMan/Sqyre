//! Overlay button press policy shared by X11 and Windows hosts.

/// What a left press on a button should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPressKind {
    /// A macro is running and press-to-stop is on.
    Stop,
    /// This button is the busy one and press-to-stop is off.
    IgnoreBusy,
    /// Press already consumed; wait for release.
    IgnoreArmed,
    /// Enqueue the button's macro.
    Fire,
}

/// Classify a button press from host click-policy flags.
pub fn overlay_press_kind(
    press_stops: bool,
    running: bool,
    busy: bool,
    armed: bool,
) -> OverlayPressKind {
    if press_stops && running {
        OverlayPressKind::Stop
    } else if busy {
        OverlayPressKind::IgnoreBusy
    } else if armed {
        OverlayPressKind::IgnoreArmed
    } else {
        OverlayPressKind::Fire
    }
}

/// True when `macro_name` is the quick-access hub face token.
pub fn is_hub_face_token(name: &str) -> bool {
    name.trim() == sqyre_persist::QUICK_ACCESS_HUB_ID
}

/// Hub expand/collapse / dismiss should keep at most one queued press.
pub fn should_coalesce_hub_click(pending: &[String], macro_name: &str) -> bool {
    let hub_token = is_hub_face_token(macro_name)
        || macro_name.trim() == sqyre_persist::QUICK_ACCESS_HUB_DISMISS;
    hub_token && pending.iter().any(|n| n == macro_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_press_kind_matches_click_policy() {
        assert_eq!(
            overlay_press_kind(true, true, true, false),
            OverlayPressKind::Stop,
            "press-to-stop wins over busy"
        );
        assert_eq!(
            overlay_press_kind(false, true, true, false),
            OverlayPressKind::IgnoreBusy
        );
        assert_eq!(
            overlay_press_kind(false, false, false, true),
            OverlayPressKind::IgnoreArmed
        );
        assert_eq!(
            overlay_press_kind(false, false, false, false),
            OverlayPressKind::Fire
        );
        assert_eq!(
            overlay_press_kind(true, false, false, false),
            OverlayPressKind::Fire,
            "press-to-stop does nothing while idle"
        );
    }

    #[test]
    fn hub_click_coalesce_keeps_one() {
        let hub = sqyre_persist::QUICK_ACCESS_HUB_ID.to_string();
        assert!(should_coalesce_hub_click(std::slice::from_ref(&hub), &hub));
        assert!(!should_coalesce_hub_click(&[], &hub));
        assert!(!should_coalesce_hub_click(
            std::slice::from_ref(&hub),
            "other"
        ));
        let dismiss = sqyre_persist::QUICK_ACCESS_HUB_DISMISS.to_string();
        assert!(should_coalesce_hub_click(
            std::slice::from_ref(&dismiss),
            &dismiss
        ));
    }
}
