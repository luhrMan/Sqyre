//! While-focused hotkey tag selection from Program macro tags.

use sqyre_capture::{
    get_active_window, note, window_is_our_process, window_is_transient_shell_focus,
    window_matches_binding, window_matches_program, WindowInfo,
};
use sqyre_domain::GENERAL_PROGRAM;
use sqyre_persist::ProgramCatalog;
use std::time::{Duration, Instant};

/// Throttle OS focus queries.
const POLL_EVERY: Duration = Duration::from_millis(250);

#[derive(Debug, Default)]
pub(crate) struct HotkeyFocusTagPoller {
    last_poll: Option<Instant>,
    /// Last emitted filter set (for change detection).
    last_emitted: Option<Vec<String>>,
}

enum FocusResolve<'a> {
    Window(&'a WindowInfo),
    Hold,
}

impl HotkeyFocusTagPoller {
    pub fn new() -> Self {
        Self::default()
    }

    /// When `enabled`, return the filters that should be active (`None` = no change this tick).
    /// Empty vec = hotkeys off (unmatched foreign window, or program with no tags).
    pub fn poll(&mut self, enabled: bool, catalog: &ProgramCatalog) -> Option<Vec<String>> {
        if !enabled {
            self.last_emitted = None;
            self.last_poll = None;
            return None;
        }

        let now = Instant::now();
        if self
            .last_poll
            .is_some_and(|t| now.saturating_duration_since(t) < POLL_EVERY)
        {
            return None;
        }
        self.last_poll = Some(now);

        let focus = get_active_window().ok().flatten();
        let desired = match resolve(focus.as_ref()) {
            FocusResolve::Hold => return None,
            FocusResolve::Window(win) => match find_focused_program(catalog, win) {
                Some((_, tags)) if !tags.is_empty() => tags,
                _ => Vec::new(),
            },
        };

        if self.last_emitted.as_ref() == Some(&desired) {
            return None;
        }
        let prev = self.last_emitted.replace(desired.clone());
        let focus_label = focus
            .as_ref()
            .map(|w| format!("{} ({})", w.process_name.trim(), w.process_path.trim()))
            .unwrap_or_else(|| sqyre_domain::EMPTY_NONE.into());
        note(&format!(
            "hotkey-focus: tags {:?} -> {:?} focus={focus_label}",
            prev.unwrap_or_default(),
            desired
        ));
        Some(desired)
    }
}

/// Hold last tags across no-focus / Sqyre / shell blips (mirrors overlay `last_foreign`).
/// Fullscreen XWayland often reports `None` for the whole session — clearing tags
/// there used to disable all hotkeys while overlay gates kept polling.
fn resolve(focus: Option<&WindowInfo>) -> FocusResolve<'_> {
    let Some(active) = focus else {
        return FocusResolve::Hold;
    };
    if window_is_transient_shell_focus(active) || window_is_our_process(active) {
        return FocusResolve::Hold;
    }
    FocusResolve::Window(active)
}

/// Catalog program that owns `win`, if any (skips General). Prefer process bindings.
pub(crate) fn find_focused_program(
    catalog: &ProgramCatalog,
    win: &WindowInfo,
) -> Option<(String, Vec<String>)> {
    let mut bound: Option<(String, Vec<String>)> = None;
    let mut fuzzy: Option<(String, Vec<String>)> = None;

    for name in catalog.program_names() {
        if name == GENERAL_PROGRAM {
            continue;
        }
        let bound_path = {
            let Some(data) = catalog.get(name) else {
                continue;
            };
            let path = data.process_path.trim();
            let owns = if !path.is_empty() {
                window_matches_binding(win, path, &data.window_title)
            } else {
                window_matches_program(win, name)
            };
            if !owns {
                continue;
            }
            !path.is_empty()
        };
        let entry = (name.clone(), catalog.program_macro_tags(name));
        if bound_path {
            bound = Some(entry);
            break;
        }
        if fuzzy.is_none() {
            fuzzy = Some(entry);
        }
    }

    bound.or(fuzzy)
}

/// Editor program for a focus hint / active window, when the catalog knows it.
///
/// Prefer `hint` (e.g. overlay `last_foreign`) so Sqyre-owned focus after a hub
/// click still targets the game. Falls back to the OS active window when it is
/// a foreign client.
pub(crate) fn focused_editor_program(
    catalog: &ProgramCatalog,
    hint: Option<&WindowInfo>,
) -> Option<String> {
    let win = hint
        .filter(|w| !window_is_our_process(w) && !window_is_transient_shell_focus(w))
        .cloned()
        .or_else(|| {
            get_active_window()
                .ok()
                .flatten()
                .filter(|w| !window_is_our_process(w) && !window_is_transient_shell_focus(w))
        })?;
    find_focused_program(catalog, &win).map(|(name, _)| name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_persist::ProgramCatalog;

    #[test]
    fn focused_editor_program_skips_our_process() {
        let mut cat = ProgramCatalog::default();
        cat.create_program("Game").unwrap();
        cat.set_process_binding("Game", "/games/game", "").unwrap();
        let ours = WindowInfo {
            title: "Sqyre".into(),
            process_name: "sqyre".into(),
            process_path: std::env::current_exe()
                .ok()
                .and_then(|p| p.to_str().map(str::to_string))
                .unwrap_or_else(|| "/bin/sqyre".into()),
            icon: None,
        };
        assert!(focused_editor_program(&cat, Some(&ours)).is_none());
        let game = WindowInfo {
            title: "Game".into(),
            process_name: "game".into(),
            process_path: "/games/game".into(),
            icon: None,
        };
        assert_eq!(
            focused_editor_program(&cat, Some(&game)).as_deref(),
            Some("Game")
        );
    }

    #[test]
    fn find_prefers_process_binding() {
        let mut cat = ProgramCatalog::default();
        cat.create_program("Fuzzy").unwrap();
        cat.create_program("Bound").unwrap();
        cat.set_process_binding("Bound", "/opt/game/Game", "Game Title")
            .unwrap();
        cat.set_program_tags("Bound", vec!["combat".into()])
            .unwrap();
        cat.set_program_tags("Fuzzy", vec!["loot".into()]).unwrap();

        let win = WindowInfo {
            title: "Game Title".into(),
            process_name: "Game".into(),
            process_path: "/opt/game/Game".into(),
            icon: None,
        };
        let (name, tags) = find_focused_program(&cat, &win).expect("match");
        assert_eq!(name, "Bound");
        assert_eq!(tags, vec!["Bound".to_string(), "combat".to_string()]);
    }

    #[test]
    fn name_tag_is_present_without_extra_tags() {
        let mut cat = ProgramCatalog::default();
        cat.create_program("Solo").unwrap();
        cat.set_process_binding("Solo", "/opt/solo/Solo", "Solo")
            .unwrap();
        let win = WindowInfo {
            title: "Solo".into(),
            process_name: "Solo".into(),
            process_path: "/opt/solo/Solo".into(),
            icon: None,
        };
        let (name, tags) = find_focused_program(&cat, &win).expect("match");
        assert_eq!(name, "Solo");
        assert_eq!(tags, vec!["Solo".to_string()]);
    }

    #[test]
    fn none_and_overlay_focus_hold() {
        assert!(matches!(resolve(None), FocusResolve::Hold));
        let overlay = WindowInfo {
            title: sqyre_capture::OVERLAY_WM_TITLE.into(),
            process_name: sqyre_ports::APP_ID.into(),
            process_path: "/opt/sqyre".into(),
            icon: None,
        };
        assert!(matches!(resolve(Some(&overlay)), FocusResolve::Hold));
    }

    #[test]
    fn foreign_window_resolves() {
        let win = WindowInfo {
            title: "Other".into(),
            process_name: "other".into(),
            process_path: "/usr/bin/other".into(),
            icon: None,
        };
        assert!(matches!(resolve(Some(&win)), FocusResolve::Window(_)));
    }
}
