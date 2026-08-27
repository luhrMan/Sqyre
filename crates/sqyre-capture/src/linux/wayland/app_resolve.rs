//! Resolve a Wayland `app_id` to a process name + executable path.

use std::fs;
use std::path::{Path, PathBuf};

/// `(process_name, process_path)` best-effort from a compositor `app_id`.
pub(crate) fn resolve_app_id(app_id: &str) -> (String, String) {
    let app_id = app_id.trim();
    if app_id.is_empty() {
        return (String::new(), String::new());
    }
    let desktop = load_desktop(app_id);
    let exec_bin = desktop
        .as_ref()
        .and_then(|d| parse_exec_binary(&d.exec))
        .or_else(|| Some(app_id.to_string()));
    let path = exec_bin
        .as_deref()
        .and_then(running_exe_matching)
        .or_else(|| exec_bin.as_ref().and_then(|b| which(b)))
        .unwrap_or_default();
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

struct DesktopFile {
    name: String,
    exec: String,
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
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }
    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for dir in data_dirs.split(':').filter(|s| !s.is_empty()) {
        dirs.push(PathBuf::from(dir).join("applications"));
    }
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
        let exe = fs::read_link(ent.path().join("exe")).ok()?;
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
    let exe = running_exe_matching(path)?;
    let proc = fs::read_dir("/proc").ok()?;
    for ent in proc.flatten() {
        let pid = ent.file_name();
        let Some(pid) = pid.to_str() else {
            continue;
        };
        if !pid.as_bytes().iter().all(u8::is_ascii_digit) {
            continue;
        }
        let got = fs::read_link(ent.path().join("exe")).ok()?;
        if got.to_string_lossy() == exe {
            let comm = fs::read_to_string(ent.path().join("comm")).ok()?;
            let name = comm.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

pub(crate) fn process_from_pid(pid: u32) -> (String, String) {
    if pid == 0 {
        return (String::new(), String::new());
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
    desktop_file_for_pid(pid, process_path).and_then(|d| d.app_id)
}

fn desktop_file_for_pid(pid: u32, process_path: &str) -> Option<DesktopFile> {
    if let Some(path) = environ_value(pid, "GIO_LAUNCHED_DESKTOP_FILE") {
        if let Some(parsed) = parse_desktop_file(Path::new(&path)) {
            return Some(parsed);
        }
    }
    let bin = Path::new(process_path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())?;
    load_desktop(&bin)
}

fn environ_value(pid: u32, key: &str) -> Option<String> {
    let raw = fs::read(format!("/proc/{pid}/environ")).ok()?;
    let prefix = format!("{key}=");
    raw.split(|b| *b == 0)
        .filter_map(|kv| std::str::from_utf8(kv).ok())
        .find_map(|kv| kv.strip_prefix(&prefix).map(str::to_string))
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
    fn parse_desktop_entry_reads_name_and_exec() {
        let d = parse_desktop_entry(
            "[Desktop Entry]\nName=Files\nExec=nautilus --new-window %U\nType=Application\n",
        )
        .expect("desktop");
        assert_eq!(d.name, "Files");
        assert_eq!(parse_exec_binary(&d.exec).as_deref(), Some("nautilus"));
    }

    #[test]
    fn desktop_file_names_include_id_and_last_segment() {
        let names = desktop_file_names("org.gnome.Nautilus");
        assert!(names.iter().any(|n| n == "org.gnome.Nautilus.desktop"));
        assert!(names.iter().any(|n| n == "Nautilus.desktop"));
    }
}
