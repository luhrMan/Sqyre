//! Shared labels and action DTO for the overlay-button context menu.
//!
//! Desktop hosts paint a chooser at the cursor; Android uses a long-press
//! `PopupMenu`. Both enqueue [`OverlayButtonContextAction`] for the app to drain.

/// Menu row: open this button in the Data Editor Overlay tab.
pub const CTX_OPEN_SETTINGS: &str = "Open settings";
/// Menu row: set `enabled = false` and persist (button disappears on next sync).
pub const CTX_DISABLE: &str = "Disable";

/// Action queued by a native overlay host after a context-menu pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayButtonContextAction {
    OpenSettings { button_id: String },
    Disable { button_id: String },
}

/// Fixed row labels for [`crate::raster::rasterize_chooser`] (Cancel is separate).
pub fn context_menu_row_names() -> Vec<String> {
    vec![CTX_OPEN_SETTINGS.into(), CTX_DISABLE.into()]
}

/// Map a chooser / PopupMenu pick to an action. Empty / Cancel → `None`.
pub fn action_from_context_pick(
    button_id: &str,
    row_name: &str,
) -> Option<OverlayButtonContextAction> {
    match row_name {
        CTX_OPEN_SETTINGS => Some(OverlayButtonContextAction::OpenSettings {
            button_id: button_id.to_string(),
        }),
        CTX_DISABLE => Some(OverlayButtonContextAction::Disable {
            button_id: button_id.to_string(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_maps_known_rows() {
        assert_eq!(
            action_from_context_pick("b1", CTX_OPEN_SETTINGS),
            Some(OverlayButtonContextAction::OpenSettings {
                button_id: "b1".into()
            })
        );
        assert_eq!(
            action_from_context_pick("b1", CTX_DISABLE),
            Some(OverlayButtonContextAction::Disable {
                button_id: "b1".into()
            })
        );
        assert_eq!(action_from_context_pick("b1", ""), None);
        assert_eq!(action_from_context_pick("b1", "Cancel"), None);
    }

    #[test]
    fn rows_are_open_then_disable() {
        assert_eq!(
            context_menu_row_names(),
            vec![CTX_OPEN_SETTINGS.to_string(), CTX_DISABLE.to_string()]
        );
    }
}
