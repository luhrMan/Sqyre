//! Macro variable declarations panel.

use crate::action_tooltip::help;
use eframe::egui;
use sqyre_domain::{builtin_variable_catalog, Macro, VariableDecl, VariableType};
use sqyre_ports::SharedRuntimeVars;
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
    /// Snapshot at open for dirty Save enablement.
    baseline_name: String,
    baseline_type: VariableType,
    baseline_initial: String,
    baseline_description: String,
}

impl EditState {
    fn save_enabled(&self) -> bool {
        self.name != self.baseline_name
            || self.type_ != self.baseline_type
            || self.initial_value != self.baseline_initial
            || self.description != self.baseline_description
    }
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
        *self.cached_monitor_count.get_or_insert_with(|| {
            #[cfg(feature = "native-runtime")]
            {
                sqyre_capture::monitor_count()
            }
            #[cfg(not(feature = "native-runtime"))]
            {
                1
            }
        })
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
        crate::widgets::fit_dialog_popup(
            egui::Window::new(format!("Variables — {}", macro_.name))
                .open(&mut open)
                .resizable(true)
                .default_width(520.0)
                .default_height(480.0)
                .min_size([360.0, 320.0]),
            ctx,
        )
        .show(ctx, |ui| {
            // Split remaining height between declared list (top) and Runtime/Built-ins.
            const TAB_CHROME: f32 = 48.0;
            let avail = ui.available_height();
            let bottom_h = ((avail - TAB_CHROME) * 0.38).max(100.0);
            let top_h = (avail - TAB_CHROME - bottom_h).max(120.0);

            ui.add_enabled_ui(enabled, |ui| {
                persist |= self.body(ui, macro_, top_h);
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.bottom_tab, BottomTab::Runtime, "Runtime")
                    .on_hover_text(help::VAR_TAB_RUNTIME);
                ui.selectable_value(&mut self.bottom_tab, BottomTab::Builtins, "Built-ins")
                    .on_hover_text(help::VAR_TAB_BUILTINS);
            });
            ui.separator();
            match self.bottom_tab {
                BottomTab::Runtime => self.show_runtime(ui, runtime_vars, running, bottom_h),
                BottomTab::Builtins => self.show_builtins(ui, num_monitors, bottom_h),
            }
        });
        self.open = open;
        if running {
            ctx.request_repaint();
        }
        persist
    }

    fn show_runtime(
        &self,
        ui: &mut egui::Ui,
        runtime_vars: &SharedRuntimeVars,
        running: bool,
        max_h: f32,
    ) {
        let snap = runtime_vars.snapshot();
        crate::widgets::heading_with_count(
            ui,
            if running {
                "Live runtime"
            } else {
                "Last runtime"
            },
            snap.len(),
        );
        if snap.is_empty() {
            ui.weak(if running {
                "Waiting for variables…"
            } else {
                "No runtime snapshot yet — run a macro."
            });
            return;
        }
        let list_h = (max_h - 28.0).max(60.0);
        crate::pickers::scroll_vertical()
            .auto_shrink([false, false])
            .max_height(list_h)
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

    fn show_builtins(&self, ui: &mut egui::Ui, num_monitors: usize, max_h: f32) {
        ui.label(
            egui::RichText::new(
                "Set automatically by the runtime or certain actions. Names are fixed.",
            )
            .weak(),
        );
        ui.add_space(4.0);
        let catalog = builtin_variable_catalog(num_monitors);
        let list_h = (max_h - 28.0).max(60.0);
        crate::pickers::scroll_vertical()
            .auto_shrink([false, false])
            .max_height(list_h)
            .show(ui, |ui| {
                for info in &catalog {
                    ui.horizontal(|ui| {
                        ui.monospace(&info.name);
                        ui.weak(info.description);
                    });
                }
            });
    }

    fn body(&mut self, ui: &mut egui::Ui, macro_: &mut Macro, max_h: f32) -> bool {
        let mut persist = false;

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            ui.set_max_width(crate::widgets::visible_width(ui));
            ui.horizontal(|ui| {
                ui.heading("Declared variables");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("+ Add").color(crate::theme::MACRO_START))
                        .clicked()
                    {
                        self.editing = Some(EditState {
                            index: None,
                            name: String::new(),
                            type_: VariableType::Auto,
                            initial_value: String::new(),
                            description: String::new(),
                            error: None,
                            baseline_name: String::new(),
                            baseline_type: VariableType::Auto,
                            baseline_initial: String::new(),
                            baseline_description: String::new(),
                        });
                        self.status = None;
                    }
                    ui.label(
                        egui::RichText::new(format!("({})", macro_.variable_decls.len())).weak(),
                    );
                });
            });
        });
        ui.label(
            "Initial values seed the runtime store at macro start. Action outputs appear in Live runtime while running.",
        );
        ui.separator();

        let mut remove_idx: Option<usize> = None;
        let mut start_edit: Option<usize> = None;

        // Reserve space for optional edit form / status below the list.
        let edit_reserve = if self.editing.is_some() { 168.0 } else { 0.0 };
        let status_reserve = if self.status.is_some() { 24.0 } else { 0.0 };
        let list_h = (max_h - 56.0 - edit_reserve - status_reserve).max(80.0);

        crate::pickers::scroll_vertical()
            .auto_shrink([false, false])
            .max_height(list_h)
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
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Remove")
                                            .color(crate::theme::MACRO_STOP),
                                    )
                                    .small(),
                                )
                                .clicked()
                            {
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
                    name: d.name.clone(),
                    type_: d.type_,
                    initial_value: d.initial_value.clone(),
                    description: d.description.clone(),
                    error: None,
                    baseline_name: d.name,
                    baseline_type: d.type_,
                    baseline_initial: d.initial_value,
                    baseline_description: d.description,
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
            let color = if self.status_error {
                crate::theme::error_fg()
            } else {
                crate::theme::ok_fg()
            };
            ui.colored_label(color, msg);
        }

        persist
    }

    fn edit_form(&mut self, ui: &mut egui::Ui, macro_: &mut Macro, mut edit: EditState) -> bool {
        let mut persist = false;

        crate::widgets::text_field_width(ui, "Name", help::VAR_NAME, &mut edit.name, 160.0);
        ui.horizontal(|ui| {
            help::label(ui, "Type", help::VAR_TYPE);
            for (label, ty) in [
                ("auto", VariableType::Auto),
                ("text", VariableType::Text),
                ("number", VariableType::Number),
            ] {
                if ui
                    .selectable_label(edit.type_ == ty, label)
                    .on_hover_text(help::VAR_TYPE)
                    .clicked()
                {
                    edit.type_ = ty;
                }
            }
        });
        crate::widgets::text_field_width(
            ui,
            "Initial",
            help::VAR_INITIAL,
            &mut edit.initial_value,
            220.0,
        );
        crate::widgets::text_field_width(
            ui,
            "Description",
            help::VAR_DESC,
            &mut edit.description,
            280.0,
        );

        if let Some(err) = &edit.error {
            ui.colored_label(crate::theme::error_fg(), err);
        }

        match crate::widgets::save_cancel_row_ltr(ui, edit.save_enabled()) {
            crate::widgets::SaveCancel::Cancel => {
                self.editing = None;
                return false;
            }
            crate::widgets::SaveCancel::None => {
                self.editing = Some(edit);
                return false;
            }
            crate::widgets::SaveCancel::Save => {}
        }

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

        persist
    }
}
