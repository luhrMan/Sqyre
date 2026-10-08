//! Freedesktop `.desktop` entry + icon for unsandboxed Linux runs.
//!
//! GNOME resolves the portal dialog name ("Sqyre wants to remotely control…") and
//! Wayland dock icons from `<APP_ID>.desktop` in the XDG data dirs. Without it,
//! the Remote Desktop / ScreenCast prompts show "Unknown Application".

use crate::assets::{APP_ICON_SVG, APP_ID};
use std::path::{Path, PathBuf};

/// Keep in sync with `scripts/linux/packaging/appimage/com.sqyre.app.desktop`
/// (Flatpak builds only see `crates/`, so it cannot be `include_str!`'d).
const DESKTOP_TEMPLATE: &str = "\
[Desktop Entry]
Type=Application
Name=Sqyre
Comment=Desktop macro builder — screen-aware automation
Categories=Development;Utility;
Keywords=sqyre;macro;automation;ocr;image-search;macro-recorder;auto-clicker;hotkey;
Icon=com.sqyre.app
Exec=sqyre
Terminal=false
StartupWMClass=sqyre
";

/// Write or refresh the user-local desktop entry and icon (no-op inside Flatpak).
pub fn install() {
    if std::env::var_os("FLATPAK_ID").is_some() {
        return;
    }
    let Some(data_home) = data_home() else {
        return;
    };
    let Some(exec) = launcher_path() else {
        return;
    };
    let desktop = data_home
        .join("applications")
        .join(format!("{APP_ID}.desktop"));
    let icon = data_home
        .join("icons/hicolor/scalable/apps")
        .join(format!("{APP_ID}.svg"));
    for (path, contents) in [
        (desktop, desktop_entry(&exec).into_bytes()),
        (icon, APP_ICON_SVG.to_vec()),
    ] {
        if let Err(e) = write_if_changed(&path, &contents) {
            crate::log::warn(format!("desktop entry: {}: {e}", path.display()));
        }
    }
}

fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
}

/// AppImage runs from a transient mount; `$APPIMAGE` is the stable file to launch.
fn launcher_path() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
}

fn desktop_entry(exec: &Path) -> String {
    let exec_line = format!("Exec={}", quote_exec_arg(&exec.to_string_lossy()));
    DESKTOP_TEMPLATE
        .lines()
        .map(|line| {
            if line.starts_with("Exec=") {
                exec_line.as_str()
            } else {
                line
            }
        })
        .fold(String::new(), |mut out, line| {
            out.push_str(line);
            out.push('\n');
            out
        })
}

/// Quote per the Desktop Entry spec `Exec` rules (reserved chars, `%` field codes).
fn quote_exec_arg(arg: &str) -> String {
    let needs_quotes = arg.chars().any(|c| {
        c.is_whitespace()
            || matches!(
                c,
                '"' | '\''
                    | '\\'
                    | '>'
                    | '<'
                    | '~'
                    | '|'
                    | '&'
                    | ';'
                    | '$'
                    | '*'
                    | '?'
                    | '#'
                    | '('
                    | ')'
                    | '`'
            )
    });
    let escaped = arg.replace('%', "%%");
    if !needs_quotes {
        return escaped;
    }
    let mut out = String::with_capacity(escaped.len() + 2);
    out.push('"');
    for c in escaped.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

fn write_if_changed(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|cur| cur == contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_points_exec_at_binary() {
        let entry = desktop_entry(Path::new("/opt/sqyre-dev/sqyre"));
        assert!(entry.contains("\nExec=/opt/sqyre-dev/sqyre\n"));
        assert!(entry.contains("\nName=Sqyre\n"));
        assert!(entry.contains(&format!("\nIcon={APP_ID}\n")));
        assert_eq!(entry.matches("Exec=").count(), 1);
    }

    #[test]
    fn exec_quotes_reserved_chars() {
        assert_eq!(quote_exec_arg("/a/b"), "/a/b");
        assert_eq!(quote_exec_arg("/my dir/sq$"), "\"/my dir/sq\\$\"");
        assert_eq!(quote_exec_arg("/100%/sqyre"), "/100%%/sqyre");
    }
}
