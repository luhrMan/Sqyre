//! Programs tab "Add running": one Program per process with an open window,
//! or one per window title when several windows share a Wine/Proton runtime.

use super::helpers::{is_editor_listed_program, unique_name};
use super::persist::{persist_running_program_icon, play_ui_add_sound};
use super::{DataEditor, DataEditorCtx};
use crate::window_types::{ProcessIcon, WindowInfo};
use sqyre_persist::ProgramCatalog;
use sqyre_validate::FS_FORBIDDEN_FILENAME_CHARS;
use std::collections::HashSet;
use std::sync::mpsc::{self, TryRecvError};

pub(super) type RunningProgramsRx = mpsc::Receiver<Result<Vec<WindowInfo>, String>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NewRunningProgram {
    pub name: String,
    pub process_path: String,
    pub window_title: String,
    pub icon: Option<ProcessIcon>,
}

/// Programs to create from `windows`.
///
/// Ordinary apps: one entry per process (first icon-bearing window), named by
/// process name. Wine/Proton runtimes share one binary across games, so those
/// get one entry per window title (named from a sanitized title).
///
/// Skips names that are invalid, reserved, or already in the catalog, and
/// identities already bound to a catalog program.
pub(super) fn plan_running_programs(
    windows: &[WindowInfo],
    catalog: &ProgramCatalog,
) -> Vec<NewRunningProgram> {
    let mut bound_paths: HashSet<String> = HashSet::new();
    let mut bound_identities: HashSet<(String, String)> = HashSet::new();
    for name in catalog.program_names() {
        let Some(prog) = catalog.get(name) else {
            continue;
        };
        let path = prog.process_path.trim().to_string();
        if path.is_empty() {
            continue;
        }
        let title = prog.window_title.trim().to_string();
        bound_identities.insert((path.clone(), title));
        bound_paths.insert(path);
    }
    let mut taken_names: HashSet<String> = catalog.program_names().cloned().collect();
    let mut seen_identity: HashSet<(String, String)> = HashSet::new();
    let mut out = Vec::new();
    for w in windows {
        let proc_name = w.process_name.trim();
        let key = w.process_key();
        let title = w.title.trim();
        if proc_name.is_empty() || w.icon.is_none() {
            continue;
        }
        let shared = is_shared_runtime(proc_name, key.trim());
        let identity = if shared {
            (key.trim().to_string(), title.to_string())
        } else {
            (key.trim().to_string(), String::new())
        };
        if !seen_identity.insert(identity) {
            continue;
        }
        if shared {
            if bound_identities.contains(&(key.trim().to_string(), title.to_string())) {
                continue;
            }
        } else if bound_paths.contains(key.trim()) {
            continue;
        }
        let Some(name) = (if shared && !title.is_empty() {
            catalog_name_from_title(title, &taken_names)
        } else {
            catalog_name_from_process(proc_name, &taken_names)
        }) else {
            continue;
        };
        taken_names.insert(name.clone());
        out.push(NewRunningProgram {
            name,
            process_path: key,
            window_title: w.title.clone(),
            icon: w.icon.clone(),
        });
    }
    out
}

/// Wine/Proton (and similar) host many games under one executable path.
fn is_shared_runtime(process_name: &str, process_path: &str) -> bool {
    let name = process_name.trim().to_ascii_lowercase();
    let base = std::path::Path::new(process_path.trim())
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    fn runtime(s: &str) -> bool {
        matches!(
            s,
            "wine-preloader"
                | "wine64-preloader"
                | "wine"
                | "wine64"
                | "proton"
                | "pressure-vessel-adverb"
        ) || s.contains("proton")
    }
    runtime(&name) || runtime(&base)
}

fn catalog_name_from_process(proc_name: &str, taken: &HashSet<String>) -> Option<String> {
    if proc_name.is_empty()
        || !is_editor_listed_program(proc_name)
        || sqyre_validate::validate_entity_name(proc_name).is_err()
        || taken.contains(proc_name)
    {
        return None;
    }
    Some(proc_name.to_string())
}

fn catalog_name_from_title(title: &str, taken: &HashSet<String>) -> Option<String> {
    let base = sanitize_program_name(title);
    if base.is_empty() || !is_editor_listed_program(&base) {
        return None;
    }
    if sqyre_validate::validate_entity_name(&base).is_err() {
        return None;
    }
    let name = unique_name(&base, |n| taken.contains(n));
    if !is_editor_listed_program(&name) || sqyre_validate::validate_entity_name(&name).is_err() {
        return None;
    }
    Some(name)
}

/// Turn a window title into a catalog-safe program name.
fn sanitize_program_name(raw: &str) -> String {
    let mapped: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c == '\0'
                || c.is_control()
                || sqyre_domain::PROGRAM_DELIMITER.contains(c)
                || FS_FORBIDDEN_FILENAME_CHARS.contains(&c)
            {
                ' '
            } else {
                c
            }
        })
        .collect();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl DataEditor {
    pub(super) fn add_running_programs_pending(&self) -> bool {
        self.running_programs_pending.is_some()
    }

    pub(super) fn start_add_running_programs(&mut self) {
        if self.running_programs_pending.is_some() {
            return;
        }
        self.clear_status();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            // Receiver dropped means the editor went away; nothing to report.
            let _ = tx.send(crate::pickers::fetch_open_windows());
        });
        self.running_programs_pending = Some(rx);
    }

    pub(super) fn poll_running_programs(&mut self, env: &mut DataEditorCtx<'_>) {
        let Some(rx) = self.running_programs_pending.as_ref() else {
            return;
        };
        let windows = match rx.try_recv() {
            Ok(Ok(list)) => list,
            Ok(Err(e)) => {
                self.running_programs_pending = None;
                self.set_err(format!(
                    "Could not list running programs: {}",
                    crate::status_banner::user_error_detail(&e)
                ));
                return;
            }
            Err(TryRecvError::Empty) => {
                env.ctx.request_repaint();
                return;
            }
            Err(TryRecvError::Disconnected) => {
                self.running_programs_pending = None;
                self.set_err("Could not list running programs.");
                return;
            }
        };
        self.running_programs_pending = None;
        let plan = plan_running_programs(&windows, env.catalog);
        if plan.is_empty() {
            self.set_ok("No new running programs to add.");
            return;
        }
        let mut added = 0usize;
        let mut failed = Vec::new();
        for p in plan {
            let res = env.catalog.create_program(&p.name).and_then(|()| {
                env.catalog.set_process_binding(
                    &p.name,
                    p.process_path.clone(),
                    p.window_title.clone(),
                )
            });
            match res {
                Ok(()) => {
                    persist_running_program_icon(
                        env.catalog,
                        env.icons,
                        &p.name,
                        "",
                        &p.process_path,
                        &p.window_title,
                        p.icon,
                    );
                    added += 1;
                }
                Err(e) => failed.push(format!(
                    "{}: {}",
                    p.name,
                    crate::status_banner::user_error_detail(&e)
                )),
            }
        }
        if added > 0 {
            if let Err(e) = self.persist(env.db, env.macros, env.catalog) {
                self.set_err(e);
                return;
            }
            play_ui_add_sound(env.settings);
        }
        let noun = if added == 1 { "program" } else { "programs" };
        if failed.is_empty() {
            self.set_ok(format!("Added {added} running {noun}."));
        } else {
            self.set_err(format!(
                "Added {added} running {noun}. Failed: {}",
                failed.join("; ")
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon() -> Option<ProcessIcon> {
        Some(ProcessIcon {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        })
    }

    fn win(title: &str, name: &str, path: &str) -> WindowInfo {
        WindowInfo {
            title: title.into(),
            process_name: name.into(),
            process_path: path.into(),
            icon: icon(),
        }
    }

    #[test]
    fn plan_dedupes_processes_and_skips_existing() {
        let mut cat = ProgramCatalog::default();
        cat.create_program("firefox").unwrap();
        cat.create_program("Mail").unwrap();
        cat.set_process_binding("Mail", "/usr/bin/thunderbird", "")
            .unwrap();
        let iconless = WindowInfo {
            icon: None,
            ..win("NoIcon", "noicon", "/usr/bin/noicon")
        };
        let windows = [
            win("Docs", "code", "/usr/bin/code"),
            win("Other", "code", "/usr/bin/code"),
            win("Web", "firefox", "/usr/lib/firefox/firefox"),
            win("Inbox", "thunderbird", "/usr/bin/thunderbird"),
            win("Bad", "a/b", "/opt/ab"),
            win("Nameless", "", "/opt/nameless"),
            win("Term", "kitty", ""),
            iconless,
        ];
        let plan = plan_running_programs(&windows, &cat);
        let got: Vec<(&str, &str, &str)> = plan
            .iter()
            .map(|p| {
                (
                    p.name.as_str(),
                    p.process_path.as_str(),
                    p.window_title.as_str(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                ("code", "/usr/bin/code", "Docs"),
                ("kitty", "kitty", "Term")
            ]
        );
    }

    #[test]
    fn plan_splits_shared_proton_binaries_by_title() {
        let cat = ProgramCatalog::default();
        let proton =
            "/home/chris/.local/share/Steam/steamapps/common/Proton - Experimental/files/bin/wine-preloader";
        let windows = [
            win("Hades II", "wine-preloader", proton),
            win("Stardew Valley", "wine-preloader", proton),
            win("Hades II", "wine-preloader", proton),
        ];
        let plan = plan_running_programs(&windows, &cat);
        let got: Vec<(&str, &str)> = plan
            .iter()
            .map(|p| (p.name.as_str(), p.window_title.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                ("Hades II", "Hades II"),
                ("Stardew Valley", "Stardew Valley")
            ]
        );
    }

    #[test]
    fn plan_sanitizes_titles_with_forbidden_chars() {
        let cat = ProgramCatalog::default();
        let proton = "/Steam/Proton/wine-preloader";
        let windows = [
            win("Game: Subtitle?", "wine-preloader", proton),
            win("Other / Title", "wine-preloader", proton),
        ];
        let plan = plan_running_programs(&windows, &cat);
        let names: Vec<&str> = plan.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Game Subtitle", "Other Title"]);
    }

    #[test]
    fn sanitize_strips_forbidden_filename_chars() {
        assert_eq!(sanitize_program_name("  A:B/C?  "), "A B C");
        assert_eq!(sanitize_program_name("ok"), "ok");
    }
}
