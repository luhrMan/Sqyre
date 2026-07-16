//! Macro variable declarations panel.

use eframe::egui;
use sqyre_domain::{builtin_variable_catalog, Macro, VariableDecl, VariableType};
use sqyre_executor::SharedRuntimeVars;
use sqyre_validate::validate_variable_assignment_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum BottomTab {
    #[default]
    Runtime,
    Builtins,
}

#[derive(Debug, Default)]
pub struct VariablesPanelUi {
    pub open: bool,
    /// Index into `macro_.variable_decls` being edited, or `None` for add-new.
    editing: Option<EditState>,
    status: Option<String>,
    status_error: bool,
    synced_macro: String,
    bottom_tab: BottomTab,
    /// Cached display count for the Built-ins tab (avoids opening X11 every frame).
    cached_monitor_count: Option<usize>,
}

#[derive(Debug, Clone)]
struct EditState {
    /// `None` = adding a new decl; `Some(i)` = editing decls[i].
    index: Option<usize>,
    name: String,
    type_: VariableType,
    initial_value: String,
    description: String,
    error: Option<String>,
}

impl VariablesPanelUi {
    pub fn sync_macro(&mut self, macro_name: &str) {
        if self.synced_macro == macro_name {
            return;
        }
        self.synced_macro = macro_name.to_string();
        self.editing = None;
        self.status = None;
        self.status_error = false;
    }

    /// Cached display count for the Built-ins tab (queries capture once).
    fn resolve_monitor_count(&mut self) -> usize {
        *self
            .cached_monitor_count
            .get_or_insert_with(sqyre_capture::monitor_count)
    }

    /// Returns true when the caller should persist the macro.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        macro_: &mut Macro,
        enabled: bool,
        runtime_vars: &SharedRuntimeVars,
        running: bool,
    ) -> bool {
        if !self.open {
            return false;
        }
        self.sync_macro(&macro_.name);
        let num_monitors = self.resolve_monitor_count();
        let mut persist = false;
        let mut open = self.open;
        egui::Window::new(format!("Variables — {}", macro_.name))
            .open(&mut open)
            .default_width(520.0)
            .default_height(480.0)
            .show(ctx, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    persist |= self.body(ui, macro_);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.bottom_tab, BottomTab::Runtime, "Runtime");
                    ui.selectable_value(&mut self.bottom_tab, BottomTab::Builtins, "Built-ins");
                });
                ui.separator();
                match self.bottom_tab {
                    BottomTab::Runtime => self.show_runtime(ui, runtime_vars, running),
                    BottomTab::Builtins => self.show_builtins(ui, num_monitors),
                }
            });
        self.open = open;
        if running {
            ctx.request_repaint();
        }
        persist
    }

    fn show_runtime(&self, ui: &mut egui::Ui, runtime_vars: &SharedRuntimeVars, running: bool) {
        ui.heading(if running {
            "Live runtime"
        } else {
            "Last runtime"
        });
        let snap = runtime_vars.snapshot();
        if snap.is_empty() {
            ui.weak(if running {
                "Waiting for variables…"
            } else {
                "No runtime snapshot yet — run a macro."
            });
            return;
        }
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show(ui, |ui| {
                for (name, value) in snap {
                    ui.horizontal(|ui| {
                        ui.monospace(name);
                        ui.label("=");
                        ui.weak(value);
                    });
                }
            });
    }

    fn show_builtins(&self, ui: &mut egui::Ui, num_monitors: usize) {
        ui.label(
            egui::RichText::new(
                "Set automatically by the runtime or certain actions. Names are fixed.",
            )
            .weak(),
        );
        ui.add_space(4.0);
        let catalog = builtin_variable_catalog(num_monitors);
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                for info in &catalog {
                    ui.horizontal(|ui| {
                        ui.monospace(&info.name);
                        ui.weak(info.description);
                    });
                }
            });
    }

    fn body(&mut self, ui: &mut egui::Ui, macro_: &mut Macro) -> bool {
        let mut persist = false;

        ui.horizontal(|ui| {
            ui.heading("Declared variables");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ Add").clicked() {
                    self.editing = Some(EditState {
                        index: None,
                        name: String::new(),
                        type_: VariableType::Auto,
                        initial_value: String::new(),
                        description: String::new(),
                        error: None,
                    });
                    self.status = None;
                }
            });
        });
        ui.label(
            "Initial values seed the runtime store at macro start. Action outputs appear in Live runtime while running.",
        );
        ui.separator();

        let mut remove_idx: Option<usize> = None;
        let mut start_edit: Option<usize> = None;

        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                if macro_.variable_decls.is_empty() {
                    ui.weak("No declared variables yet.");
                    return;
                }
                for (i, d) in macro_.variable_decls.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.monospace(&d.name);
                        ui.label(d.type_.as_str());
                        if !d.initial_value.trim().is_empty() {
                            ui.weak(format!("= {}", d.initial_value));
                        }
                        if !d.description.trim().is_empty() {
                            ui.weak(&d.description);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Remove").clicked() {
                                remove_idx = Some(i);
                            }
                            if ui.small_button("Edit").clicked() {
                                start_edit = Some(i);
                            }
                        });
                    });
                }
            });

        if let Some(i) = start_edit {
            if let Some(d) = macro_.variable_decls.get(i).cloned() {
                self.editing = Some(EditState {
                    index: Some(i),
                    name: d.name,
                    type_: d.type_,
                    initial_value: d.initial_value,
                    description: d.description,
                    error: None,
                });
                self.status = None;
            }
        }
        if let Some(i) = remove_idx {
            if i < macro_.variable_decls.len() {
                let name = macro_.variable_decls[i].name.clone();
                macro_.remove_variable_decl(&name);
                self.editing = None;
                self.status = Some(format!("Removed {name}"));
                self.status_error = false;
                persist = true;
            }
        }

        if let Some(edit) = self.editing.clone() {
            ui.separator();
            ui.heading(if edit.index.is_some() {
                "Edit variable"
            } else {
                "New variable"
            });
            persist |= self.edit_form(ui, macro_, edit);
        }

        if let Some(msg) = &self.status {
            if self.status_error {
                ui.colored_label(ui.visuals().error_fg_color, msg);
            } else {
                ui.weak(msg);
            }
        }

        persist
    }

    fn edit_form(&mut self, ui: &mut egui::Ui, macro_: &mut Macro, mut edit: EditState) -> bool {
        let mut persist = false;
        let mut cancel = false;
        let mut save = false;

        ui.horizontal(|ui| {
            ui.label("Name");
            ui.add(
                egui::TextEdit::singleline(&mut edit.name)
                    .desired_width(160.0)
                    .hint_text("myVar"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Type");
            for (label, ty) in [
                ("auto", VariableType::Auto),
                ("text", VariableType::Text),
                ("number", VariableType::Number),
            ] {
                if ui.selectable_label(edit.type_ == ty, label).clicked() {
                    edit.type_ = ty;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Initial");
            ui.add(
                egui::TextEdit::singleline(&mut edit.initial_value)
                    .desired_width(220.0)
                    .hint_text("optional"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Description");
            ui.add(
                egui::TextEdit::singleline(&mut edit.description)
                    .desired_width(280.0)
                    .hint_text("optional"),
            );
        });

        if let Some(err) = &edit.error {
            ui.colored_label(ui.visuals().error_fg_color, err);
        }

        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                save = true;
            }
            if ui.button("Cancel").clicked() {
                cancel = true;
            }
        });

        if cancel {
            self.editing = None;
            return false;
        }

        if save {
            let trimmed = edit.name.trim().to_string();
            match validate_variable_assignment_name(&trimmed) {
                Ok(()) => {
                    let collision = macro_.variable_decls.iter().enumerate().any(|(i, d)| {
                        d.name.eq_ignore_ascii_case(&trimmed)
                            && edit.index.map(|ei| ei != i).unwrap_or(true)
                    });
                    if collision {
                        edit.error = Some(format!("variable {trimmed:?} already exists"));
                        self.editing = Some(edit);
                    } else {
                        if let Some(i) = edit.index {
                            if let Some(old) = macro_.variable_decls.get(i) {
                                if !old.name.eq_ignore_ascii_case(&trimmed) {
                                    macro_.remove_variable_decl(&old.name.clone());
                                }
                            }
                        }
                        macro_.upsert_variable(VariableDecl {
                            name: trimmed.clone(),
                            type_: edit.type_,
                            initial_value: edit.initial_value.clone(),
                            description: edit.description.clone(),
                        });
                        self.editing = None;
                        self.status = Some(format!("Saved {trimmed}"));
                        self.status_error = false;
                        persist = true;
                    }
                }
                Err(e) => {
                    edit.error = Some(e.to_string());
                    self.editing = Some(edit);
                }
            }
        } else {
            self.editing = Some(edit);
        }

        persist
    }
}
