//! Programs tab "Add running": one Program per process with an open window.

use super::helpers::is_editor_listed_program;
use super::persist::{persist_running_program_icon, play_ui_add_sound};
use super::{DataEditor, DataEditorCtx};
use crate::window_types::{ProcessIcon, WindowInfo};
use sqyre_persist::ProgramCatalog;
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

/// Programs to create from `windows`: named by process name, first window per process.
///
/// Skips names that are invalid, reserved, or already in the catalog, and processes
/// already bound to a catalog program.
pub(super) fn plan_running_programs(
    windows: &[WindowInfo],
    catalog: &ProgramCatalog,
) -> Vec<NewRunningProgram> {
    let mut bound: HashSet<String> = catalog
        .program_names()
        .filter_map(|n| catalog.get(n))
        .map(|p| p.process_path.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let mut names = HashSet::new();
    let mut out = Vec::new();
    for w in windows {
        let name = w.process_name.trim();
        let key = w.process_key();
        if name.is_empty()
            || !is_editor_listed_program(name)
            || sqyre_validate::validate_entity_name(name).is_err()
            || catalog.get(name).is_some()
            || bound.contains(key.trim())
            || !names.insert(name.to_string())
        {
            continue;
        }
        bound.insert(key.trim().to_string());
        out.push(NewRunningProgram {
            name: name.to_string(),
            process_path: key,
            window_title: w.title.clone(),
            icon: w.icon.clone(),
        });
    }
    out
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
                self.set_err(format!("Could not list running programs: {e}"));
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
                Err(e) => failed.push(format!("{}: {e}", p.name)),
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

    fn win(title: &str, name: &str, path: &str) -> WindowInfo {
        WindowInfo {
            title: title.into(),
            process_name: name.into(),
            process_path: path.into(),
            icon: None,
        }
    }

    #[test]
    fn plan_dedupes_processes_and_skips_existing() {
        let mut cat = ProgramCatalog::default();
        cat.create_program("firefox").unwrap();
        cat.create_program("Mail").unwrap();
        cat.set_process_binding("Mail", "/usr/bin/thunderbird", "")
            .unwrap();
        let windows = [
            win("Docs", "code", "/usr/bin/code"),
            win("Other", "code", "/usr/bin/code"),
            win("Web", "firefox", "/usr/lib/firefox/firefox"),
            win("Inbox", "thunderbird", "/usr/bin/thunderbird"),
            win("Bad", "a/b", "/opt/ab"),
            win("Nameless", "", "/opt/nameless"),
            win("Term", "kitty", ""),
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
}
