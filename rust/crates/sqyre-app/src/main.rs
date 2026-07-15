//! egui shell: load macros from `~/.sqyre`, Run/Stop with live backends.

mod catalog;

use catalog::{CatalogIcons, CatalogResolver};
use eframe::egui;
use egui_ltreeview::{Action as TreeAction, TreeView, TreeViewBuilder};
use sqyre_capture::X11Capturer;
use sqyre_domain::{Action, ActionId, Macro};
use sqyre_executor::{execute_macro_with, ExecDeps, MatchFacade};
use sqyre_hotkeys::{default_hotkeys, HotkeyCallbacks, HotkeyService, StopFlag};
use sqyre_input::OsAutomation;
use sqyre_persist::{Database, ProgramCatalog};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_title("Sqyre (Rust)"),
        ..Default::default()
    };
    eframe::run_native(
        "Sqyre",
        options,
        Box::new(|_cc| Ok(Box::new(SqyreApp::load()))),
    )
}

struct RunState {
    stop: StopFlag,
    running: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
}

impl Default for RunState {
    fn default() -> Self {
        Self {
            stop: StopFlag::new(),
            running: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(String::new())),
        }
    }
}

struct SqyreApp {
    macros: Vec<Macro>,
    catalog: ProgramCatalog,
    load_error: Option<String>,
    selected_macro: usize,
    selected_node: Option<u64>,
    node_actions: HashMap<u64, ActionId>,
    run: RunState,
    hotkeys: Box<dyn HotkeyService>,
}

impl SqyreApp {
    fn load() -> Self {
        let mut hotkeys = default_hotkeys();
        let run = RunState::default();
        let stop = run.stop.clone();
        let _ = hotkeys.start(HotkeyCallbacks {
            on_escape_stop: Arc::new(move || stop.request_stop()),
            on_failsafe: Arc::new(|| {
                eprintln!("failsafe Esc+Ctrl+Shift — exiting");
                std::process::exit(0);
            }),
        });

        match Database::load_default() {
            Ok(db) => {
                let catalog = db.program_catalog().unwrap_or_default();
                let mut macros: Vec<_> = db.macros.into_values().collect();
                macros.sort_by(|a, b| a.name.cmp(&b.name));
                Self {
                    macros,
                    catalog,
                    load_error: None,
                    selected_macro: 0,
                    selected_node: None,
                    node_actions: HashMap::new(),
                    run,
                    hotkeys,
                }
            }
            Err(e) => Self {
                macros: Vec::new(),
                catalog: ProgramCatalog::default(),
                load_error: Some(e.to_string()),
                selected_macro: 0,
                selected_node: None,
                node_actions: HashMap::new(),
                run,
                hotkeys,
            },
        }
    }

    fn start_macro(&mut self) {
        if self.macros.is_empty() || self.run.running.load(Ordering::SeqCst) {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let mut macro_ = self.macros[idx].clone();
        let catalog = self.catalog.clone();
        let stop_flag = self.run.stop.clone();
        stop_flag.clear();
        let running = Arc::clone(&self.run.running);
        let status = Arc::clone(&self.run.status);
        running.store(true, Ordering::SeqCst);
        *status.lock().unwrap() = format!("Running {}…", macro_.name);

        thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let mut automation =
                    OsAutomation::new().map_err(|e| format!("automation: {e}"))?;
                let mut capturer = X11Capturer::open().map_err(|e| format!("capture: {e}"))?;
                let matcher = MatchFacade::new();
                let resolver = CatalogResolver(&catalog);
                let icons = CatalogIcons(&catalog);
                // Poll stop into executor via sleep wrapper? Phase 2: check between actions
                // by wrapping automation — for now rely on Esc setting flag and checking
                // in a thin wrapper.
                let stop_raw = stop_flag.raw();
                let mut watched = StopWatchAutomation {
                    inner: &mut automation,
                    stop: &stop_flag,
                };
                execute_macro_with(
                    &mut macro_,
                    ExecDeps {
                        automation: &mut watched,
                        capturer: Some(&mut capturer),
                        matcher: Some(&matcher),
                        resolver: Some(&resolver),
                        icons: Some(&icons),
                        stop_flag: Some(stop_raw.as_ref()),
                    },
                )
                .map_err(|e| e.to_string())
            })();

            let msg = match result {
                Ok(()) if stop_flag.is_stopped() => "Stopped.".into(),
                Ok(()) => "Finished.".into(),
                Err(e) => format!("Error: {e}"),
            };
            *status.lock().unwrap() = msg;
            running.store(false, Ordering::SeqCst);
        });
    }

    fn request_stop(&mut self) {
        self.run.stop.request_stop();
        *self.run.status.lock().unwrap() = "Stop requested…".into();
    }
}

/// Forwards automation but surfaces stop via milli_sleep / between calls by setting
/// a flag the executor polls — here we map stop into a short sleep so UI stays responsive.
struct StopWatchAutomation<'a> {
    inner: &'a mut OsAutomation,
    stop: &'a StopFlag,
}

impl sqyre_executor::AutomationBackend for StopWatchAutomation<'_> {
    fn milli_sleep(&mut self, ms: i32) {
        let mut left = ms.max(0);
        while left > 0 {
            if self.stop.is_stopped() {
                return;
            }
            let chunk = left.min(50);
            self.inner.milli_sleep(chunk);
            left -= chunk;
        }
    }
    fn move_to(&mut self, x: i32, y: i32, opts: sqyre_executor::MoveOptions) {
        if !self.stop.is_stopped() {
            self.inner.move_to(x, y, opts);
        }
    }
    fn click(&mut self, button: &str, down: bool) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.click(button, down)
    }
    fn scroll(&mut self, up: bool) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.scroll(up)
    }
    fn key_down(&mut self, key: &str) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.key_down(key)
    }
    fn key_up(&mut self, key: &str) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.key_up(key)
    }
    fn type_char(&mut self, s: &str) {
        if !self.stop.is_stopped() {
            self.inner.type_char(s);
        }
    }
    fn write_clipboard(&mut self, s: &str) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.write_clipboard(s)
    }
}

impl eframe::App for SqyreApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Propagate Esc stop into executor between actions via stop_requested isn't
        // threaded yet — StopWatchAutomation aborts I/O; show status.
        egui::Panel::left("macro_list")
            .default_size(220.0)
            .show_inside(ui, |ui| {
                ui.heading("Macros");
                if let Some(err) = &self.load_error {
                    ui.colored_label(egui::Color32::RED, format!("Load error: {err}"));
                } else {
                    ui.small(format!(
                        "{} from {}",
                        self.macros.len(),
                        sqyre_persist::db_path().display()
                    ));
                }
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, m) in self.macros.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_macro == i, &m.name)
                            .clicked()
                        {
                            self.selected_macro = i;
                            self.selected_node = None;
                        }
                    }
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Sqyre");
            ui.horizontal(|ui| {
                let running = self.run.running.load(Ordering::SeqCst);
                if ui
                    .add_enabled(!running && !self.macros.is_empty(), egui::Button::new("Run"))
                    .clicked()
                {
                    self.start_macro();
                }
                if ui
                    .add_enabled(running, egui::Button::new("Stop"))
                    .clicked()
                {
                    self.request_stop();
                }
                let status = self.run.status.lock().unwrap().clone();
                if !status.is_empty() {
                    ui.label(status);
                }
            });
            ui.small("Esc stops the running macro; Esc+Ctrl+Shift exits (failsafe).");
            ui.separator();

            if self.macros.is_empty() {
                ui.label("No macros loaded. Place a db.yaml under ~/.sqyre.");
                return;
            }
            let idx = self.selected_macro.min(self.macros.len() - 1);
            self.selected_macro = idx;

            let summary = {
                let m = &self.macros[idx];
                format!(
                    "{} — delay {}ms — {} tags",
                    m.name,
                    m.global_delay,
                    m.tags.len()
                )
            };
            ui.label(summary);
            ui.separator();

            let mut next_id = 0u64;
            let mut node_actions = HashMap::new();
            let id = ui.make_persistent_id(("macro_tree", idx));
            let (_, actions) = TreeView::new(id).show(ui, |builder: &mut TreeViewBuilder<'_, u64>| {
                build_tree(
                    builder,
                    &self.macros[idx].root,
                    &mut next_id,
                    &mut node_actions,
                    true,
                );
            });
            self.node_actions = node_actions;

            for action in actions {
                if let TreeAction::SetSelected(sel) = action {
                    self.selected_node = sel.into_iter().next();
                }
            }

            if let Some(nid) = self.selected_node {
                if let Some(aid) = self.node_actions.get(&nid).copied() {
                    let root = &self.macros[idx].root;
                    let action = if aid.is_root() {
                        Some(root)
                    } else {
                        root.find_by_id(aid)
                    };
                    if let Some(action) = action {
                        ui.separator();
                        ui.label(format!(
                            "Selected: {} ({})",
                            action.display_name(),
                            action.type_key()
                        ));
                    }
                }
            }
        });
    }
}

impl Drop for SqyreApp {
    fn drop(&mut self) {
        self.hotkeys.stop();
    }
}

fn build_tree(
    builder: &mut TreeViewBuilder<'_, u64>,
    action: &Action,
    next_id: &mut u64,
    map: &mut HashMap<u64, ActionId>,
    is_root: bool,
) {
    let id = *next_id;
    *next_id += 1;
    map.insert(id, action.id);

    let label = if is_root {
        format!("Loop (root) — {} actions", action.children().len())
    } else {
        action.display_name()
    };

    if action.is_branch() {
        builder.dir(id, label);
        for child in action.children() {
            build_tree(builder, child, next_id, map, false);
        }
        builder.close_dir();
    } else {
        builder.leaf(id, label);
    }
}
