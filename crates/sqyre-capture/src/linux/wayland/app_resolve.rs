//! Resolve a Wayland `app_id` / PID to a process name + executable path.

use crate::ProcessIcon;
use std::fs;
use std::path::{Path, PathBuf};

/// `(process_name, process_path)` best-effort from a compositor `app_id`.
pub(crate) fn resolve_app_id(app_id: &str) -> (String, String) {
    let app_id = app_id.trim();
    if app_id.is_empty() {
        return (String::new(), String::new());
    }
    let desktop = load_desktop(app_id);

    // Flatpak exports use `Exec=flatpak run … org.foo.Bar`. Prefer the app id so
    // every Flatpak app does not collapse to `/usr/bin/flatpak`.
    if let Some(fp_id) = desktop
        .as_ref()
        .and_then(|d| flatpak_app_id_from_exec(&d.exec))
    {
        let name = desktop
            .as_ref()
            .map(|d| d.name.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fp_id.clone());
        return (name, fp_id);
    }

    let exec_bin = desktop
        .as_ref()
        .and_then(|d| parse_exec_binary(&d.exec))
        .filter(|b| !is_flatpak_launcher(b))
        .or_else(|| Some(app_id.to_string()));
    let path = exec_bin
        .as_deref()
        .and_then(running_exe_matching)
        .or_else(|| {
            exec_bin
                .as_ref()
                .filter(|b| !is_flatpak_launcher(b))
                .and_then(|b| which(b))
        })
        .unwrap_or_else(|| app_id.to_string());
    let name = comm_for_path(&path)
        .or_else(|| {
            desktop
                .as_ref()
                .map(|d| d.name.clone())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| {
            Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| app_id.to_string())
        });
    (name, path)
}

/// Best-effort desktop theme icon for a compositor `app_id`.
pub(crate) fn desktop_icon_for_app_id(app_id: &str) -> Option<ProcessIcon> {
    let app_id = app_id.trim();
    if app_id.is_empty() {
        return None;
    }
    let icon_name = load_desktop(app_id)?.icon;
    if icon_name.is_empty() {
        return None;
    }
    load_named_icon(&icon_name)
}

struct DesktopFile {
    name: String,
    exec: String,
    icon: String,
    app_id: Option<String>,
}

fn load_desktop(app_id: &str) -> Option<DesktopFile> {
    let candidates = desktop_file_names(app_id);
    for dir in desktop_dirs() {
        for name in &candidates {
            let path = dir.join(name);
            if let Some(parsed) = parse_desktop_file(&path) {
                return Some(parsed);
            }
        }
    }
    None
}

fn desktop_file_names(app_id: &str) -> Vec<String> {
    let mut names = vec![format!("{app_id}.desktop")];
    if !app_id.ends_with(".desktop") {
        names.push(app_id.to_string());
    }
    if let Some((_, rest)) = app_id.rsplit_once('.') {
        if !rest.is_empty() {
            names.push(format!("{rest}.desktop"));
        }
    }
    names
}

fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join(".local/share/applications"));
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }
    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for dir in data_dirs.split(':').filter(|s| !s.is_empty()) {
        dirs.push(PathBuf::from(dir).join("applications"));
    }
    // Host Flatpak exports visible inside the Sqyre Flatpak sandbox.
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs
}

fn parse_desktop_file(path: &Path) -> Option<DesktopFile> {
    let raw = fs::read_to_string(path).ok()?;
    let mut parsed = parse_desktop_entry(&raw)?;
    parsed.app_id = path.file_stem().map(|s| s.to_string_lossy().into_owned());
    Some(parsed)
}

fn parse_desktop_entry(raw: &str) -> Option<DesktopFile> {
    let mut in_entry = false;
    let mut name = String::new();
    let mut exec = String::new();
    let mut try_exec = String::new();
    let mut icon = String::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_entry = line.eq_ignore_ascii_case("[Desktop Entry]");
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "Name" if name.is_empty() => name = value.trim().to_string(),
            "Exec" if exec.is_empty() => exec = value.trim().to_string(),
            "TryExec" if try_exec.is_empty() => try_exec = value.trim().to_string(),
            "Icon" if icon.is_empty() => icon = value.trim().to_string(),
            _ => {}
        }
    }
    if exec.is_empty() {
        exec = try_exec;
    }
    if exec.is_empty() && name.is_empty() {
        return None;
    }
    Some(DesktopFile {
        name,
        exec,
        icon,
        app_id: None,
    })
}

/// First executable token from a freedesktop `Exec=` value.
pub(crate) fn parse_exec_binary(exec: &str) -> Option<String> {
    let mut tokens = tokenize_exec(exec);
    if tokens.first().map(String::as_str) == Some("env") {
        tokens.remove(0);
        while tokens
            .first()
            .is_some_and(|t| t.contains('=') && !t.starts_with('/'))
        {
            tokens.remove(0);
        }
    }
    let bin = tokens.into_iter().find(|t| !t.starts_with('%'))?;
    if bin.is_empty() {
        None
    } else {
        Some(bin)
    }
}

/// App id from `flatpak run [options] org.foo.Bar …`, if `exec` is a Flatpak launcher.
pub(crate) fn flatpak_app_id_from_exec(exec: &str) -> Option<String> {
    let tokens = tokenize_exec(exec);
    let mut i = 0usize;
    if tokens.first().map(String::as_str) == Some("env") {
        i = 1;
        while i < tokens.len() && tokens[i].contains('=') && !tokens[i].starts_with('/') {
            i += 1;
        }
    }
    let bin = tokens.get(i)?;
    if !is_flatpak_launcher(bin) {
        return None;
    }
    i += 1;
    if tokens.get(i).map(String::as_str) != Some("run") {
        return None;
    }
    i += 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if t == "--" {
            i += 1;
            break;
        }
        if let Some(rest) = t.strip_prefix("--") {
            if rest.contains('=') {
                i += 1;
                continue;
            }
            i += 1;
            // Skip option values that are not reverse-DNS app ids.
            if i < tokens.len() && !tokens[i].starts_with('-') && !tokens[i].contains('.') {
                i += 1;
            }
            continue;
        }
        if t.starts_with('-') {
            i += 1;
            continue;
        }
        if t.starts_with('%') {
            i += 1;
            continue;
        }
        return Some(t.clone());
    }
    tokens.get(i).cloned().filter(|t| !t.starts_with('%'))
}

fn is_flatpak_launcher(bin: &str) -> bool {
    Path::new(bin)
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n == "flatpak")
}

fn tokenize_exec(exec: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in exec.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => cur.push(c),
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            None => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn running_exe_matching(bin: &str) -> Option<String> {
    let want = Path::new(bin).file_name()?;
    let proc = fs::read_dir("/proc").ok()?;
    for ent in proc.flatten() {
        let pid = ent.file_name();
        if pid
            .to_str()
            .is_none_or(|s| !s.as_bytes().iter().all(u8::is_ascii_digit))
        {
            continue;
        }
        // Skip unreadable exes (common for pid 1 / other-uid / Flatpak filters).
        let Ok(exe) = fs::read_link(ent.path().join("exe")) else {
            continue;
        };
        if exe.file_name() == Some(want) {
            return Some(exe.to_string_lossy().into_owned());
        }
    }
    None
}

fn which(bin: &str) -> Option<String> {
    let path = Path::new(bin);
    if path.is_absolute() && path.is_file() {
        return Some(bin.to_string());
    }
    let search = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
    for dir in search.split(':').filter(|s| !s.is_empty()) {
        let candidate = Path::new(dir).join(bin);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn comm_for_path(path: &str) -> Option<String> {
    let want = Path::new(path).file_name()?;
    let proc = fs::read_dir("/proc").ok()?;
    for ent in proc.flatten() {
        let pid = ent.file_name();
        let Some(pid) = pid.to_str() else {
            continue;
        };
        if !pid.as_bytes().iter().all(u8::is_ascii_digit) {
            continue;
        }
        let Ok(got) = fs::read_link(ent.path().join("exe")) else {
            continue;
        };
        if got.file_name() != Some(want) && got.to_string_lossy() != path {
            continue;
        }
        let Ok(comm) = fs::read_to_string(ent.path().join("comm")) else {
            continue;
        };
        let name = comm.trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

/// `(process_name, process_path)` for a live PID.
///
/// Prefers `FLATPAK_ID` when set so Flatpak apps keep a stable focus key even when
/// `/proc/pid/exe` is unreadable or points at a sandbox-local `/app/…` path.
pub(crate) fn process_from_pid(pid: u32) -> (String, String) {
    if pid == 0 {
        return (String::new(), String::new());
    }
    if let Some(id) = environ_value(pid, "FLATPAK_ID").filter(|s| !s.is_empty()) {
        let name = fs::read_to_string(format!("/proc/{pid}/comm"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| load_desktop(&id).map(|d| d.name).filter(|s| !s.is_empty()))
            .unwrap_or_else(|| id.clone());
        return (name, id);
    }
    let path = fs::read_link(format!("/proc/{pid}/exe"))
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    if path.is_empty() {
        if let Some(id) = desktop_app_id_for_pid(pid, "").filter(|s| !s.is_empty()) {
            let label = load_desktop(&id)
                .map(|d| d.name)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| name.clone());
            return (if label.is_empty() { id.clone() } else { label }, id);
        }
        if !name.is_empty() {
            // Basename-only key so Focus can still match via process_name.
            return (name.clone(), name);
        }
    }
    (name, path)
}

/// Desktop `Name=` for a running exe, from `GIO_LAUNCHED_DESKTOP_FILE` or a matching `.desktop`.
pub(crate) fn desktop_label_for_pid(pid: u32, process_path: &str) -> Option<String> {
    desktop_file_for_pid(pid, process_path).and_then(|d| {
        if d.name.is_empty() {
            None
        } else {
            Some(d.name)
        }
    })
}

/// Freedesktop app id (`org.gnome.Nautilus`) if we can resolve a desktop file.
pub(crate) fn desktop_app_id_for_pid(pid: u32, process_path: &str) -> Option<String> {
    if let Some(id) = environ_value(pid, "FLATPAK_ID").filter(|s| !s.is_empty()) {
        return Some(id);
    }
    desktop_file_for_pid(pid, process_path).and_then(|d| d.app_id)
}

/// Desktop theme icon for a live PID, when `_NET_WM_ICON` is missing.
pub(crate) fn desktop_icon_for_pid(pid: u32, process_path: &str) -> Option<ProcessIcon> {
    let id = desktop_app_id_for_pid(pid, process_path)?;
    desktop_icon_for_app_id(&id)
}

fn desktop_file_for_pid(pid: u32, process_path: &str) -> Option<DesktopFile> {
    if let Some(path) = environ_value(pid, "GIO_LAUNCHED_DESKTOP_FILE") {
        if let Some(parsed) = parse_desktop_file(Path::new(&path)) {
            return Some(parsed);
        }
    }
    let bin = Path::new(process_path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())?;
    load_desktop(&bin)
}

fn environ_value(pid: u32, key: &str) -> Option<String> {
    let raw = fs::read(format!("/proc/{pid}/environ")).ok()?;
    let prefix = format!("{key}=");
    raw.split(|b| *b == 0)
        .filter_map(|kv| std::str::from_utf8(kv).ok())
        .find_map(|kv| kv.strip_prefix(&prefix).map(str::to_string))
}

fn load_named_icon(name: &str) -> Option<ProcessIcon> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    if Path::new(name).is_absolute() {
        return load_icon_file(Path::new(name));
    }
    let png_name = if name.ends_with(".png") {
        name.to_string()
    } else {
        format!("{name}.png")
    };
    for root in icon_search_roots() {
        for size in ["48x48", "64x64", "32x32", "128x128", "256x256"] {
            for app_dir in ["apps", "applications"] {
                let candidate = root.join(size).join(app_dir).join(&png_name);
                if let Some(icon) = load_icon_file(&candidate) {
                    return Some(icon);
                }
            }
        }
    }
    None
}

fn icon_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut push_themes = |base: PathBuf| {
        for theme in ["hicolor", "Adwaita", "Yaru", "breeze"] {
            roots.push(base.join(theme));
        }
    };
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        push_themes(home.join(".local/share/icons"));
        push_themes(home.join(".local/share/flatpak/exports/share/icons"));
    }
    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for dir in data_dirs.split(':').filter(|s| !s.is_empty()) {
        push_themes(PathBuf::from(dir).join("icons"));
    }
    push_themes(PathBuf::from("/var/lib/flatpak/exports/share/icons"));
    roots
}

fn load_icon_file(path: &Path) -> Option<ProcessIcon> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    // Raster only — avoid pulling resvg into sqyre-capture for theme SVGs.
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "") {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.into_rgba8();
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }
    Some(ProcessIcon {
        width: w,
        height: h,
        rgba: img.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_exec_strips_field_codes_and_env() {
        assert_eq!(
            parse_exec_binary("env BAMF=1 /usr/lib/firefox/firefox %u").as_deref(),
            Some("/usr/lib/firefox/firefox")
        );
        assert_eq!(
            parse_exec_binary("nautilus --new-window %U").as_deref(),
            Some("nautilus")
        );
        assert_eq!(parse_exec_binary("").as_deref(), None);
    }

    #[test]
    fn flatpak_exec_yields_app_id_not_launcher() {
        let exec = "/usr/bin/flatpak run --branch=stable --arch=x86_64 --command=firefox --file-forwarding org.mozilla.firefox @@u %U @@";
        assert_eq!(
            flatpak_app_id_from_exec(exec).as_deref(),
            Some("org.mozilla.firefox")
        );
        assert_eq!(
            flatpak_app_id_from_exec("flatpak run --command=foo org.foo.Bar").as_deref(),
            Some("org.foo.Bar")
        );
        assert_eq!(flatpak_app_id_from_exec("nautilus %U").as_deref(), None);
    }

    #[test]
    fn parse_desktop_entry_reads_name_exec_icon() {
        let d = parse_desktop_entry(
            "[Desktop Entry]\nName=Files\nExec=nautilus --new-window %U\nIcon=org.gnome.Nautilus\nType=Application\n",
        )
        .expect("desktop");
        assert_eq!(d.name, "Files");
        assert_eq!(d.icon, "org.gnome.Nautilus");
        assert_eq!(parse_exec_binary(&d.exec).as_deref(), Some("nautilus"));
    }

    #[test]
    fn desktop_file_names_include_id_and_last_segment() {
        let names = desktop_file_names("org.gnome.Nautilus");
        assert!(names.iter().any(|n| n == "org.gnome.Nautilus.desktop"));
        assert!(names.iter().any(|n| n == "Nautilus.desktop"));
    }
}
