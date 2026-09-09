//! While-focused hotkey tag selection from Program macro tags.

use sqyre_capture::{
    get_active_window, window_is_our_process, window_is_transient_shell_focus,
    window_matches_binding, window_matches_program, WindowInfo,
};
use sqyre_persist::{ProgramCatalog, GENERAL_PROGRAM};
use std::time::{Duration, Instant};

/// Hold last tagged Program across brief Sqyre / shell focus blips.
const HOLD_GRACE: Duration = Duration::from_millis(1500);
/// Throttle OS focus queries.
const POLL_EVERY: Duration = Duration::from_millis(250);

#[derive(Debug, Default)]
pub(crate) struct HotkeyFocusTagPoller {
    last_poll: Option<Instant>,
    /// Last emitted filter set (for change detection).
    last_emitted: Option<Vec<String>>,
    our_since: Option<Instant>,
    none_since: Option<Instant>,
}

enum FocusResolve<'a> {
    Window(&'a WindowInfo),
    Hold,
    Clear,
}

impl HotkeyFocusTagPoller {
    pub fn new() -> Self {
        Self::default()
    }

    /// When `enabled`, return the filters that should be active (`None` = no change this tick).
    /// Empty vec = hotkeys off.
    pub fn poll(&mut self, enabled: bool, catalog: &ProgramCatalog) -> Option<Vec<String>> {
        if !enabled {
            self.last_emitted = None;
            self.our_since = None;
            self.none_since = None;
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
        let desired = match self.resolve(focus.as_ref()) {
            FocusResolve::Hold => return None,
            FocusResolve::Clear => Vec::new(),
            FocusResolve::Window(win) => match find_focused_program(catalog, win) {
                Some((_, tags)) if !tags.is_empty() => tags,
                _ => Vec::new(),
            },
        };

        if self.last_emitted.as_ref() == Some(&desired) {
            return None;
        }
        self.last_emitted = Some(desired.clone());
        Some(desired)
    }
}

impl HotkeyFocusTagPoller {
    fn resolve<'a>(&mut self, focus: Option<&'a WindowInfo>) -> FocusResolve<'a> {
        let Some(active) = focus else {
            let started = *self.none_since.get_or_insert_with(Instant::now);
            self.our_since = None;
            if started.elapsed() >= HOLD_GRACE {
                return FocusResolve::Clear;
            }
            return FocusResolve::Hold;
        };

        if window_is_transient_shell_focus(active) {
            self.none_since = None;
            self.our_since = None;
            return FocusResolve::Hold;
        }

        if window_is_our_process(active) {
            self.none_since = None;
            let started = *self.our_since.get_or_insert_with(Instant::now);
            if started.elapsed() >= HOLD_GRACE {
                return FocusResolve::Clear;
            }
            return FocusResolve::Hold;
        }

        self.none_since = None;
        self.our_since = None;
        FocusResolve::Window(active)
    }
}

/// Prefer process-bound Programs; skip General.
fn find_focused_program(
    catalog: &ProgramCatalog,
    win: &WindowInfo,
) -> Option<(String, Vec<String>)> {
    let mut bound: Option<(String, Vec<String>)> = None;
    let mut fuzzy: Option<(String, Vec<String>)> = None;

    for name in catalog.program_names() {
        if name == GENERAL_PROGRAM {
            continue;
        }
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
        let entry = (name.clone(), data.tags.clone());
        if !path.is_empty() {
            bound = Some(entry);
            break;
        }
        if fuzzy.is_none() {
            fuzzy = Some(entry);
        }
    }

    bound.or(fuzzy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_persist::ProgramCatalog;

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
        assert_eq!(tags, vec!["combat".to_string()]);
    }
}
