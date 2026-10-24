//! Which overlay buttons are on screen. Shared by the Android host and tests.
//!
//! Linux applies the same rules in [`crate::macro_overlay`] before the X11 host
//! sees a button, then map/unmaps image gates on that host. Android decides here
//! because the activity stops drawing while another app is in front.

use sqyre_capture::{
    window_is_our_process, window_matches_binding, window_matches_program, WindowInfo,
};
use std::collections::HashMap;

/// One button the Android host may show. Positions are physical pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct AndroidButtonSpec {
    pub id: String,
    pub macro_name: String,
    pub program: String,
    /// Catalog process path. Empty → match [`Self::program`] as a process name.
    pub process_path: String,
    pub window_title: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub bg: [u8; 4],
    pub border: [u8; 4],
    pub border_width: f32,
    pub corner_radius: f32,
    pub icon_glyph: char,
    pub icon: [u8; 4],
    pub icon_hover: [u8; 4],
    /// Hide unless the bound program is in front (General buttons are not gated).
    pub focus_gated: bool,
    /// Hide unless the image-search found-map says this id is on screen.
    pub visibility_gated: bool,
    /// Overlay-editor preview: stay up while Sqyre is in front even when
    /// [`shown_buttons`] is told not to draw over this app.
    pub editor_pinned: bool,
}

/// Buttons that should have a window right now.
///
/// `relocate` shows every hosted button so the Overlay editor can drag them.
/// `show_over_self` keeps buttons up while this app is in front; when it is
/// false those buttons hide (a pinned editor preview still shows).
/// A focus-gated button with no active window stays eligible only while its
/// image gate is on, and still hides until `found` says the template is there.
pub fn shown_buttons<'a>(
    buttons: &'a [AndroidButtonSpec],
    focus: Option<&WindowInfo>,
    relocate: bool,
    show_over_self: bool,
    found: &HashMap<String, bool>,
) -> Vec<&'a AndroidButtonSpec> {
    let ours = focus.is_some_and(window_is_our_process);
    buttons
        .iter()
        .filter(|btn| button_is_shown(btn, focus, ours, relocate, show_over_self, found))
        .collect()
}

fn button_is_shown(
    btn: &AndroidButtonSpec,
    focus: Option<&WindowInfo>,
    ours: bool,
    relocate: bool,
    show_over_self: bool,
    found: &HashMap<String, bool>,
) -> bool {
    if !relocate && !btn.editor_pinned && ours && !show_over_self {
        return false;
    }
    if !relocate && btn.focus_gated {
        if ours {
            return false;
        }
        let matched = program_matches(btn, focus);
        let unfocused_gate = focus.is_none() && btn.visibility_gated;
        if !matched && !unfocused_gate {
            return false;
        }
    }
    if !relocate && btn.visibility_gated && !found.get(&btn.id).copied().unwrap_or(false) {
        return false;
    }
    true
}

fn program_matches(btn: &AndroidButtonSpec, focus: Option<&WindowInfo>) -> bool {
    let Some(win) = focus else {
        return false;
    };
    if !btn.process_path.trim().is_empty() {
        return window_matches_binding(win, &btn.process_path, &btn.window_title);
    }
    window_matches_program(win, &btn.program)
}

/// Keep the button's top-left on the display. A zero screen size leaves `x,y` alone.
pub fn clamp_origin(x: i32, y: i32, w: u32, h: u32, screen_w: i32, screen_h: i32) -> (i32, i32) {
    if screen_w <= 0 || screen_h <= 0 {
        return (x, y);
    }
    let max_x = screen_w.saturating_sub(w as i32).max(0);
    let max_y = screen_h.saturating_sub(h as i32).max(0);
    (x.clamp(0, max_x), y.clamp(0, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, gated: bool) -> AndroidButtonSpec {
        AndroidButtonSpec {
            id: id.into(),
            macro_name: "Go".into(),
            program: "Game".into(),
            process_path: "com.example.game".into(),
            window_title: String::new(),
            x: 10,
            y: 20,
            w: 40,
            h: 40,
            bg: [0, 0, 0, 0],
            border: [0, 0, 0, 255],
            border_width: 1.0,
            corner_radius: 4.0,
            icon_glyph: '▶',
            icon: [255, 255, 255, 255],
            icon_hover: [255, 200, 0, 255],
            focus_gated: gated,
            visibility_gated: false,
            editor_pinned: false,
        }
    }

    fn win(path: &str, title: &str) -> WindowInfo {
        WindowInfo {
            title: title.into(),
            process_name: "game".into(),
            process_path: path.into(),
            icon: None,
        }
    }

    fn ids<'a>(shown: &[&'a AndroidButtonSpec]) -> Vec<&'a str> {
        shown.iter().map(|b| b.id.as_str()).collect()
    }

    #[test]
    fn general_button_shows_with_no_focus() {
        let mut btn = spec("g", false);
        btn.program.clear();
        btn.process_path.clear();
        let found = HashMap::new();
        let buttons = [btn];
        let shown = shown_buttons(&buttons, None, false, true, &found);
        assert_eq!(ids(&shown), ["g"]);
    }

    #[test]
    fn program_button_follows_the_foreground_package() {
        let btn = spec("g", true);
        let found = HashMap::new();
        assert!(shown_buttons(
            std::slice::from_ref(&btn),
            Some(&win("com.other", "")),
            false,
            true,
            &found
        )
        .is_empty());
        let buttons = [btn];
        let shown = shown_buttons(
            &buttons,
            Some(&win("com.example.game", "ignored")),
            false,
            true,
            &found,
        );
        assert_eq!(ids(&shown), ["g"]);
    }

    #[test]
    fn title_binding_must_match() {
        let mut btn = spec("g", true);
        btn.window_title = "Lobby".into();
        let found = HashMap::new();
        assert!(shown_buttons(
            std::slice::from_ref(&btn),
            Some(&win("com.example.game", "Menu")),
            false,
            true,
            &found
        )
        .is_empty());
        let buttons = [btn];
        let shown = shown_buttons(
            &buttons,
            Some(&win("com.example.game", "Lobby")),
            false,
            true,
            &found,
        );
        assert_eq!(ids(&shown), ["g"]);
    }

    #[test]
    fn sqyre_in_front_hides_program_buttons() {
        let btn = spec("g", true);
        let found = HashMap::new();
        let ours = win(sqyre_capture::ANDROID_APP_PACKAGE, "Sqyre");
        assert!(shown_buttons(&[btn], Some(&ours), false, true, &found).is_empty());
    }

    #[test]
    fn sqyre_in_front_hides_general_buttons_unless_allowed() {
        let mut btn = spec("g", false);
        btn.program.clear();
        btn.process_path.clear();
        let found = HashMap::new();
        let ours = win(sqyre_capture::ANDROID_APP_PACKAGE, "Sqyre");
        assert!(shown_buttons(
            std::slice::from_ref(&btn),
            Some(&ours),
            false,
            false,
            &found
        )
        .is_empty());
        assert_eq!(
            ids(&shown_buttons(
                std::slice::from_ref(&btn),
                Some(&ours),
                false,
                true,
                &found
            )),
            ["g"]
        );
        btn.editor_pinned = true;
        assert_eq!(
            ids(&shown_buttons(
                std::slice::from_ref(&btn),
                Some(&ours),
                false,
                false,
                &found
            )),
            ["g"]
        );
        btn.editor_pinned = false;
        assert_eq!(
            ids(&shown_buttons(&[btn], Some(&ours), true, false, &found)),
            ["g"]
        );
    }

    #[test]
    fn image_gate_hides_until_found_and_relocate_overrides() {
        let mut btn = spec("g", true);
        btn.visibility_gated = true;
        let mut found = HashMap::new();
        assert!(shown_buttons(std::slice::from_ref(&btn), None, false, true, &found).is_empty());
        found.insert("g".into(), true);
        assert_eq!(
            ids(&shown_buttons(
                std::slice::from_ref(&btn),
                None,
                false,
                true,
                &found
            )),
            ["g"]
        );
        found.insert("g".into(), false);
        let ours = win(sqyre_capture::ANDROID_APP_PACKAGE, "Sqyre");
        assert_eq!(
            ids(&shown_buttons(&[btn], Some(&ours), true, false, &found)),
            ["g"]
        );
    }

    #[test]
    fn clamp_origin_pins_the_button_to_the_display() {
        assert_eq!(clamp_origin(-30, 10, 40, 40, 100, 80), (0, 10));
        assert_eq!(clamp_origin(90, 70, 40, 40, 100, 80), (60, 40));
        assert_eq!(clamp_origin(12, 14, 40, 40, 0, 0), (12, 14));
    }
}
