//! Cross-platform window matching helpers shared by the X11/Windows focus backends.
//!
//! Pure logic only — OS FFI (enumerating windows, reading titles/PIDs/icons) stays in
//! `x11_focus` / `win_focus`.

use crate::ProcessIcon;

/// True when two executable paths refer to the same file, ignoring surrounding whitespace.
/// Compares via [`std::path::Path`] (component-wise, so `/` and `\` match on Windows);
/// Windows paths also compare case-insensitively, since `Path` equality does not.
pub(crate) fn paths_equal(a: &str, b: &str) -> bool {
    let (a, b) = (fold_path_case(a.trim()), fold_path_case(b.trim()));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    std::path::Path::new(a.as_ref()) == std::path::Path::new(b.as_ref())
}

#[cfg(windows)]
fn fold_path_case(path: &str) -> std::borrow::Cow<'_, str> {
    std::borrow::Cow::Owned(path.to_lowercase())
}

#[cfg(not(windows))]
fn fold_path_case(path: &str) -> std::borrow::Cow<'_, str> {
    std::borrow::Cow::Borrowed(path)
}

/// True when two window titles match, ignoring surrounding whitespace.
pub(crate) fn titles_equal(a: &str, b: &str) -> bool {
    a.trim() == b.trim()
}

/// Icon pick loop shared by X11/Windows `process_icon`: scans `candidates` for one whose
/// path matches `process_path`, preferring a candidate whose title also matches
/// `window_title`.
///
/// When `window_title` is non-empty, only a title match is returned — never the first
/// same-path window. Shared executables (Proton/`wine-preloader`) would otherwise steal
/// each other's icons. Path-only fallback applies only for process-only bindings
/// (empty title).
///
/// `info` cheaply resolves a candidate's `(title, path)` and is called for every candidate.
/// `icon` resolves its icon and is only called once a candidate's path has matched, since
/// icon extraction can be expensive (e.g. GDI icon conversion on Windows).
pub(crate) fn pick_matching_icon<T>(
    candidates: impl IntoIterator<Item = T>,
    process_path: &str,
    window_title: &str,
    mut info: impl FnMut(&T) -> Option<(String, String)>,
    mut icon: impl FnMut(&T, &str, &str) -> Option<ProcessIcon>,
) -> Option<ProcessIcon> {
    let path = process_path.trim();
    let title = window_title.trim();
    let mut path_fallback: Option<ProcessIcon> = None;
    for candidate in candidates {
        let Some((wtitle, wpath)) = info(&candidate) else {
            continue;
        };
        if !paths_equal(&wpath, path) {
            continue;
        }
        let Some(found) = icon(&candidate, &wtitle, &wpath) else {
            continue;
        };
        if !title.is_empty() {
            if titles_equal(&wtitle, title) {
                return Some(found);
            }
            continue;
        }
        if path_fallback.is_none() {
            path_fallback = Some(found);
        }
    }
    path_fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_titles() {
        assert!(paths_equal("/usr/bin/foo", "/usr/bin/foo"));
        assert!(!paths_equal("/usr/bin/foo", "/usr/bin/bar"));
        assert!(!paths_equal("", ""));
        assert!(titles_equal(" Hi ", "Hi"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_paths_ignore_case_and_separator_style() {
        assert!(paths_equal(
            r"C:\Program Files\App\app.exe",
            r"c:\program files\app\APP.EXE"
        ));
        assert!(paths_equal(
            r"C:\Program Files\App\app.exe",
            "C:/Program Files/App/app.exe"
        ));
        assert!(!paths_equal(
            r"C:\Program Files\App\app.exe",
            r"C:\Program Files\App\other.exe"
        ));
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_paths_stay_case_sensitive() {
        assert!(!paths_equal("/usr/bin/Foo", "/usr/bin/foo"));
    }

    fn icon(tag: u8) -> ProcessIcon {
        ProcessIcon {
            width: 1,
            height: 1,
            rgba: vec![tag, tag, tag, tag],
        }
    }

    #[test]
    fn pick_matching_icon_prefers_title_match_over_first_path_match() {
        let candidates = vec![
            ("Other".to_string(), "/bin/app".to_string(), icon(1)),
            ("Target".to_string(), "/bin/app".to_string(), icon(2)),
        ];
        let found = pick_matching_icon(
            candidates,
            "/bin/app",
            "Target",
            |(t, p, _)| Some((t.clone(), p.clone())),
            |(_, _, i), _, _| Some(i.clone()),
        );
        assert_eq!(found.unwrap().rgba, icon(2).rgba);
    }

    #[test]
    fn pick_matching_icon_falls_back_to_first_path_match_when_title_empty() {
        let candidates = vec![
            ("Other".to_string(), "/bin/app".to_string(), icon(1)),
            ("Unmatched".to_string(), "/bin/other".to_string(), icon(2)),
        ];
        let found = pick_matching_icon(
            candidates,
            "/bin/app",
            "",
            |(t, p, _)| Some((t.clone(), p.clone())),
            |(_, _, i), _, _| Some(i.clone()),
        );
        assert_eq!(found.unwrap().rgba, icon(1).rgba);
    }

    #[test]
    fn pick_matching_icon_none_when_title_misses_shared_binary() {
        let candidates = vec![
            (
                "Game A".to_string(),
                "/Proton/wine-preloader".to_string(),
                icon(1),
            ),
            (
                "Game B".to_string(),
                "/Proton/wine-preloader".to_string(),
                icon(2),
            ),
        ];
        let found = pick_matching_icon(
            candidates,
            "/Proton/wine-preloader",
            "Missing",
            |(t, p, _)| Some((t.clone(), p.clone())),
            |(_, _, i), _, _| Some(i.clone()),
        );
        assert!(found.is_none());
    }

    #[test]
    fn pick_matching_icon_none_when_no_path_matches() {
        let candidates = vec![("Other".to_string(), "/bin/other".to_string(), icon(1))];
        let found = pick_matching_icon(
            candidates,
            "/bin/app",
            "",
            |(t, p, _)| Some((t.clone(), p.clone())),
            |(_, _, i), _, _| Some(i.clone()),
        );
        assert!(found.is_none());
    }
}
