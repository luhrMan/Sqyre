//! Hub fan outside-dismiss arming and chrome hit tests (OS-neutral).

use crate::host_common::{spec_contains_rounded, OverlayButtonSpec};
use crate::hub_fan;

/// Sync dismiss-arm flags when the fan opens or closes.
///
/// Returns `true` when `dismiss` changed. Callers log the platform-specific note.
/// When arming, `last_btn1` is set true so the open click cannot count as dismiss.
pub fn sync_hub_fan_dismiss_fields(
    dismiss: &mut bool,
    last_btn1: &mut bool,
    open_or_traveling: bool,
) -> bool {
    if *dismiss == open_or_traveling {
        return false;
    }
    *dismiss = open_or_traveling;
    *last_btn1 = open_or_traveling;
    true
}

/// Rising edge of a held button (press that was not held last sample).
pub fn rising_edge(now: bool, was: &mut bool) -> bool {
    let pressed = now && !*was;
    *was = now;
    pressed
}

/// True when `(x, y)` hits any mapped hub chrome button (face or satellites).
pub fn hub_chrome_contains_specs<'a, I>(buttons: I, x: i32, y: i32) -> bool
where
    I: IntoIterator<Item = (bool, &'a OverlayButtonSpec)>,
{
    buttons.into_iter().any(|(mapped, spec)| {
        mapped && hub_fan::is_hub_chrome_id(&spec.id) && spec_contains_rounded(spec, x, y)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ButtonFace;

    fn face(id: &str, x: i32, y: i32) -> OverlayButtonSpec {
        OverlayButtonSpec {
            id: id.into(),
            macro_name: String::new(),
            face: ButtonFace {
                x,
                y,
                w: 40,
                h: 40,
                bg: [0; 4],
                border: [0; 4],
                border_width: 0.0,
                corner_radius: 0.0,
                icon_glyph: ' ',
                icon: [0; 4],
                icon_hover: [0; 4],
                brand: false,
            },
            tip: String::new(),
            ui_scale: 1.0,
            busy: false,
            visibility_gated: false,
            input_enabled: true,
        }
    }

    #[test]
    fn sync_arms_and_requires_repress() {
        let mut dismiss = false;
        let mut last = false;
        assert!(sync_hub_fan_dismiss_fields(&mut dismiss, &mut last, true));
        assert!(dismiss && last);
        assert!(!sync_hub_fan_dismiss_fields(&mut dismiss, &mut last, true));
        assert!(sync_hub_fan_dismiss_fields(&mut dismiss, &mut last, false));
        assert!(!dismiss && !last);
    }

    #[test]
    fn rising_edge_fires_once_per_press() {
        let mut was = false;
        assert!(rising_edge(true, &mut was));
        assert!(!rising_edge(true, &mut was));
        assert!(!rising_edge(false, &mut was));
        assert!(rising_edge(true, &mut was));
    }

    #[test]
    fn hub_chrome_hit_uses_mapped_hub_ids() {
        let hub = face(sqyre_persist::QUICK_ACCESS_HUB_ID, 10, 10);
        let other = face("macro-btn", 10, 10);
        assert!(hub_chrome_contains_specs([(true, &hub)], 20, 20));
        assert!(!hub_chrome_contains_specs([(false, &hub)], 20, 20));
        assert!(!hub_chrome_contains_specs([(true, &other)], 20, 20));
    }
}
