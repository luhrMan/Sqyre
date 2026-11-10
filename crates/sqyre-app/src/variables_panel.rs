//! Macro variable declarations panel.

use crate::action_tooltip::help;
use crate::help_text as ht;
use crate::status_banner::StatusBanner;
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
    status_banner: StatusBanner,
    synced_macro: String,
    bottom_tab: BottomTab,
    /// Cached display count for the Built-ins tab (avoids opening X11 every frame).
    cached_monitor_count: Option<usize>,
    declared_filter: String,
    runtime_filter: String,
    /// Painted height of each bottom tab section, measured last frame so the
    /// declared list can reclaim whatever the bottom does not need.
    runtime_section_h: Option<f32>,
    builtins_section_h: Option<f32>,
}

#[derive(Debug, Clone)]
struct EditState {
    /// `None` = adding a new decl; `Some(i)` = editing decls[i].
    index: Option<usize>,
    name: String,
    type_: VariableType,
    initial_value: String,
    description: String,
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
    pub(crate) fn open_for_docs(&mut self, builtins: bool) {
        self.open = true;
        self.bottom_tab = if builtins {
            BottomTab::Builtins
        } else {
            BottomTab::Runtime
        };
    }

    pub fn sync_macro(&mut self, macro_name: &str) {
        if self.synced_macro == macro_name {
            return;
        }
        self.synced_macro = macro_name.to_string();
        self.editing = None;
        self.status_banner.clear();
        self.declared_filter.clear();
        self.runtime_filter.clear();
    }

    /// Cached display count for the Built-ins tab (queries capture once).
    fn resolve_monitor_count(&mut self) -> usize {
        #[cfg(feature = "native-runtime")]
        {
            *self
                .cached_monitor_count
                .get_or_insert_with(sqyre_capture::monitor_count)
        }
        #[cfg(not(feature = "native-runtime"))]
        {
            *self.cached_monitor_count.get_or_insert(1)
        }
    }

    /// Returns true when the caller should persist the macro.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        macro_: &mut Macro,
        enabled: bool,
        runtime_vars: &SharedRuntimeVars,
        running: bool,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
    ) -> bool {
        if !self.open {
            return false;
        }
        self.sync_macro(&macro_.name);
        let num_monitors = self.resolve_monitor_count();
        let mut persist = false;
        let mut open = self.open;
        let mut close = false;
        crate::widgets::fit_dialog_popup(
            egui::Window::new(format!("Variables — {}", macro_.name))
                .open(&mut open)
                .resizable(true)
                .default_width(520.0)
                .default_height(480.0)
                .min_size(crate::widgets::FLOATER_MIN_PANEL),
            ctx,
            egui::Id::new("sqyre_variables_panel"),
            pending_scale,
        )
        .show(ctx, |ui| {
            crate::widgets::fill_resize_body(ui, |ui| {
                // Split remaining height between declared list (top) and Runtime/Built-ins.
                // Two `section_separator` bands (each one `section_gap`) + the tab row.
                let tab_chrome = 2.0 * crate::theme::section_gap(ui)
                    + ui.spacing().interact_size.y
                    + ui.spacing().item_spacing.y;
                /// Compact Runtime strip when there is no snapshot to list
                /// (heading + two-line empty state); the declared list takes
                /// the leftover instead of leaving a dead band at the bottom.
                const RUNTIME_EMPTY_H: f32 = 84.0;
                let runtime_snap = matches!(self.bottom_tab, BottomTab::Runtime)
                    .then(|| runtime_vars.snapshot())
                    .unwrap_or_default();
                let runtime_empty =
                    matches!(self.bottom_tab, BottomTab::Runtime) && runtime_snap.is_empty();
                // The bottom tab hugs its measured height (capped at the share
                // it used to claim); the declared list absorbs the slack, so a
                // short bottom section no longer leaves a dead band above the
                // window edge.
                let measured_bottom = match self.bottom_tab {
                    BottomTab::Runtime => self.runtime_section_h,
                    BottomTab::Builtins => self.builtins_section_h,
                };
                let avail = ui.available_height();
                let min_bottom = 100.0;
                let min_top = 120.0;
                let need = tab_chrome + min_bottom + min_top;
                let (top_h, bottom_h) = if runtime_empty {
                    let cap = RUNTIME_EMPTY_H.min((avail - tab_chrome - min_top).max(1.0));
                    let bottom = measured_bottom.map(|h| h.min(cap)).unwrap_or(cap);
                    ((avail - tab_chrome - bottom).max(1.0), bottom)
                } else if avail >= need {
                    let cap = ((avail - tab_chrome) * 0.38).max(min_bottom);
                    let bottom = measured_bottom.map(|h| h.min(cap)).unwrap_or(cap);
                    ((avail - tab_chrome - bottom).max(min_top), bottom)
                } else {
                    let body = (avail - tab_chrome).max(1.0);
                    let bottom = (body * 0.38).max(1.0);
                    let top = (body - bottom).max(1.0);
                    (top, bottom)
                };

                ui.add_enabled_ui(enabled, |ui| {
                    persist |= self.body(ui, macro_, top_h);
                });
                crate::widgets::section_separator(ui);
                crate::widgets::show_overflow(ui, "var_tabs", 1.0, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(&mut self.bottom_tab, BottomTab::Runtime, "Runtime")
                            .on_hover_text(help::VAR_TAB_RUNTIME);
                        ui.selectable_value(&mut self.bottom_tab, BottomTab::Builtins, "Built-ins")
                            .on_hover_text(help::VAR_TAB_BUILTINS);
                    });
                });
                crate::widgets::section_separator(ui);
                let section_h = match self.bottom_tab {
                    BottomTab::Runtime => self.show_runtime(ui, runtime_snap, running, bottom_h),
                    BottomTab::Builtins => self.show_builtins(ui, num_monitors, bottom_h),
                };
                let slot = match self.bottom_tab {
                    BottomTab::Runtime => &mut self.runtime_section_h,
                    BottomTab::Builtins => &mut self.builtins_section_h,
                };
                // Converge the split on the real painted height (one frame lag,
                // like the edit tip's `fields_height`).
                if slot.map(|h| (h - section_h).abs() > 1.0).unwrap_or(true) {
                    *slot = Some(section_h);
                    ui.ctx().request_repaint();
                }
            });
            // After the body so an inline variable edit cancels first.
            close = crate::widgets::consume_escape(ui);
        });
        self.open = open && !close;
        if running {
            ctx.request_repaint();
        }
        persist
    }

    /// Paints the Runtime section; returns its painted height (chrome plus the
    /// scroll's real content height, not the claimed viewport).
    fn show_runtime(
        &mut self,
        ui: &mut egui::Ui,
        mut snap: Vec<(String, String)>,
        running: bool,
        max_h: f32,
    ) -> f32 {
        let start = ui.min_rect().height();
        snap.sort_by(|(a, _), (b, _)| crate::macro_meta::cmp_display_name(a, b));
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
            let (title, body) = if running {
                ("Waiting for variables…", None)
            } else {
                (
                    "No runtime values yet",
                    Some("Run a macro to capture them."),
                )
            };
            let _ = crate::widgets::empty_state(ui, title, body, None, None);
            return ui.min_rect().height() - start;
        }
        crate::widgets::search_field(
            ui,
            &mut self.runtime_filter,
            crate::widgets::SearchFieldOpts {
                hint: "Filter runtime variables…",
                focus_id: None,
                clearable: true,
                stable_id: Some(egui::Id::new("runtime_vars_filter")),
                trailing: None,
            },
        );
        let q = self.runtime_filter.trim().to_ascii_lowercase();
        let filtered: Vec<_> = snap
            .into_iter()
            .filter(|(name, value)| {
                q.is_empty()
                    || name.to_ascii_lowercase().contains(&q)
                    || value.to_ascii_lowercase().contains(&q)
            })
            .collect();
        // No footer chip under this list — fill the pane (A4).
        let list_h = max_h.max(60.0);
        let list_w = crate::widgets::visible_width(ui);
        let chrome = ui.min_rect().height() - start;
        // Dense name=value rows: Extend so long values H-scroll instead of wrap/clip.
        let out = crate::widgets::show_dialog_scroll(ui, list_w, list_h, |ui| {
            crate::widgets::enable_dense_row_extend(ui);
            if filtered.is_empty() {
                crate::widgets::list_vacancy(ui, &q, 0, "variables");
                return;
            }
            for (name, value) in filtered {
                ui.horizontal(|ui| {
                    ui.monospace(name);
                    ui.label("=");
                    ui.weak(value);
                });
            }
        });
        chrome + out.content_size.y
    }

    /// Paints the Built-ins section; returns its painted height like
    /// [`show_runtime`]. Descriptions are fixed prose, so they wrap — the
    /// Extend+H-scroll treatment is only for user data rows.
    fn show_builtins(&self, ui: &mut egui::Ui, num_monitors: usize, max_h: f32) -> f32 {
        let start = ui.min_rect().height();
        ui.label(
            egui::RichText::new(
                "Set automatically by the runtime or certain actions. Names are fixed.",
            )
            .weak(),
        );
        let catalog = builtin_variable_catalog(num_monitors);
        let list_h = (max_h - 28.0).max(60.0);
        let list_w = crate::widgets::visible_width(ui);
        let chrome = ui.min_rect().height() - start;
        let out = crate::widgets::show_dialog_scroll(ui, list_w, list_h, |ui| {
            for info in &catalog {
                ui.horizontal(|ui| {
                    ui.monospace(&info.name);
                    // Wrap at the row's remaining width: `ui.horizontal` forces
                    // Extend on children, which would otherwise report the full
                    // sentence and trip a useless H-bar for a few px.
                    ui.add(egui::Label::new(egui::RichText::new(info.description).weak()).wrap());
                });
            }
        });
        chrome + out.content_size.y
    }

    fn body(&mut self, ui: &mut egui::Ui, macro_: &mut Macro, max_h: f32) -> bool {
        let mut persist = false;

        let pane = crate::widgets::visible_width(ui);
        let count = crate::widgets::count_text(macro_.variable_decls.len());
        let need = crate::widgets::text_width(ui, "Declared variables", egui::TextStyle::Heading)
            + crate::widgets::text_width(ui, &count, egui::TextStyle::Body)
            + crate::widgets::text_width(ui, "+ Add", egui::TextStyle::Button)
            + ui.spacing().button_padding.x * 2.0
            + ui.spacing().item_spacing.x * 4.0
            + 16.0;
        if need <= pane {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.set_max_width(pane);
                ui.horizontal(|ui| {
                    crate::action_tooltip::help::tip(
                        ui.heading("Declared variables"),
                        crate::action_tooltip::help::VARS_DECLARED,
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::widgets::add_button(ui, "Add variable", true).clicked() {
                            self.editing = Some(EditState {
                                index: None,
                                name: String::new(),
                                type_: VariableType::Auto,
                                initial_value: String::new(),
                                description: String::new(),
                                baseline_name: String::new(),
                                baseline_type: VariableType::Auto,
                                baseline_initial: String::new(),
                                baseline_description: String::new(),
                            });
                            self.status_banner.clear();
                        }
                        crate::widgets::count_suffix(ui, macro_.variable_decls.len());
                    });
                });
            });
        } else {
            crate::widgets::show_overflow(ui, "var_declared_header", 1.0, |ui| {
                ui.horizontal(|ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    crate::action_tooltip::help::tip(
                        ui.heading("Declared variables"),
                        crate::action_tooltip::help::VARS_DECLARED,
                    );
                    crate::widgets::count_suffix(ui, macro_.variable_decls.len());
                    if crate::widgets::add_button(ui, "Add variable", true).clicked() {
                        self.editing = Some(EditState {
                            index: None,
                            name: String::new(),
                            type_: VariableType::Auto,
                            initial_value: String::new(),
                            description: String::new(),
                            baseline_name: String::new(),
                            baseline_type: VariableType::Auto,
                            baseline_initial: String::new(),
                            baseline_description: String::new(),
                        });
                        self.status_banner.clear();
                    }
                });
            });
        }
        crate::widgets::search_field(
            ui,
            &mut self.declared_filter,
            crate::widgets::SearchFieldOpts {
                hint: "Filter declared variables…",
                focus_id: None,
                clearable: true,
                stable_id: Some(egui::Id::new("declared_vars_filter")),
                trailing: None,
            },
        );
        crate::widgets::section_separator(ui);

        let mut remove_idx: Option<usize> = None;
        let mut start_edit: Option<usize> = None;

        // Reserve space for optional edit form / status below the list.
        let edit_reserve = if self.editing.is_some() { 168.0 } else { 0.0 };
        let status_reserve = if self.status_banner.is_set() {
            24.0
        } else {
            0.0
        };
        let list_h = (max_h - 80.0 - edit_reserve - status_reserve).max(80.0);
        let list_w = crate::widgets::visible_width(ui);
        let declared_q = self.declared_filter.trim().to_ascii_lowercase();

        crate::widgets::show_dialog_scroll(ui, list_w, list_h, |ui| {
            crate::widgets::enable_dense_row_extend(ui);
            if macro_.variable_decls.is_empty() {
                crate::widgets::list_vacancy(ui, "", 0, "declared variables");
                return;
            }
            let mut any = false;
            for (i, d) in macro_.variable_decls.iter().enumerate() {
                if !declared_q.is_empty()
                    && !d.name.to_ascii_lowercase().contains(&declared_q)
                    && !d.description.to_ascii_lowercase().contains(&declared_q)
                {
                    continue;
                }
                any = true;
                let row = ui
                    .horizontal(|ui| {
                        ui.monospace(&d.name);
                        ui.label(d.type_.as_str());
                        if !d.initial_value.trim().is_empty() {
                            ui.weak(format!("= {}", d.initial_value));
                        }
                        if !d.description.trim().is_empty() {
                            ui.weak(&d.description);
                        }
                    })
                    .response
                    .on_hover_text(ht::VARS_ROW_MENU);
                let menu_id = egui::Id::new(("declared_var_menu", &d.name));
                crate::widgets::row_context_menu(ui, menu_id, row.rect, Some(&row), |ui| {
                    if crate::widgets::menu_item(ui, "Edit", true) {
                        start_edit = Some(i);
                    }
                    if crate::widgets::menu_item_danger(ui, "Remove", true) {
                        remove_idx = Some(i);
                    }
                });
            }
            if !any {
                crate::widgets::list_vacancy(ui, &declared_q, 0, "variables");
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
                    baseline_name: d.name,
                    baseline_type: d.type_,
                    baseline_initial: d.initial_value,
                    baseline_description: d.description,
                });
                self.status_banner.clear();
            }
        }
        if let Some(i) = remove_idx {
            if i < macro_.variable_decls.len() {
                let name = macro_.variable_decls[i].name.clone();
                macro_.remove_variable_decl(&name);
                self.editing = None;
                self.status_banner.set_ok(format!("Removed {name}"));
                persist = true;
            }
        }

        if let Some(edit) = self.editing.clone() {
            crate::widgets::section_separator(ui);
            ui.heading(if edit.index.is_some() {
                "Edit variable"
            } else {
                "New variable"
            });
            persist |= self.edit_form(ui, macro_, edit);
        }

        self.status_banner.paint(ui);

        persist
    }

    fn edit_form(&mut self, ui: &mut egui::Ui, macro_: &mut Macro, mut edit: EditState) -> bool {
        let mut persist = false;

        crate::widgets::text_field_opts(
            ui,
            egui_phosphor::regular::TEXT_T,
            "Name",
            help::VAR_NAME,
            &mut edit.name,
            crate::widgets::TextFieldOpts { width: 160.0 },
        );
        crate::widgets::pair_labeled(ui, egui_phosphor::regular::LIST_BULLETS, "Type", |ui| {
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
        crate::widgets::num_text_field_opts(
            ui,
            egui_phosphor::regular::FUNCTION,
            "Initial",
            help::VAR_INITIAL,
            &mut edit.initial_value,
            crate::widgets::NumFieldOpts {
                width: 220.0,
                range: i32::MIN..=i32::MAX,
            },
        );
        crate::widgets::text_field_opts(
            ui,
            egui_phosphor::regular::CHAT_TEXT,
            "Description",
            help::VAR_DESC,
            &mut edit.description,
            crate::widgets::TextFieldOpts { width: 280.0 },
        );

        match crate::widgets::save_row(ui, edit.save_enabled()) {
            crate::widgets::SaveCancel::Cancel => {
                self.editing = None;
                self.status_banner.clear();
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
                    self.status_banner
                        .set_err(format!("variable {trimmed:?} already exists"));
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
                    self.status_banner.set_ok(format!("Saved {trimmed}"));
                    persist = true;
                }
            }
            Err(e) => {
                self.status_banner.set_err(e.to_string());
                self.editing = Some(edit);
            }
        }

        persist
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_ports::RuntimeVarSink;

    fn run_panel(
        builtins: bool,
        vars: &[(&str, &str)],
        screen: egui::Vec2,
    ) -> (egui::Context, Vec<egui::epaint::ClippedShape>) {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);
        let mut panel = VariablesPanelUi::default();
        panel.open_for_docs(builtins);
        let mut macro_ = crate::docs_fixture::demo_macro();
        let runtime = SharedRuntimeVars::new();
        runtime.publish(
            &vars
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect::<Vec<_>>(),
        );
        let mut last = None;
        // Extra frames let the measured bottom-section split converge.
        for frame in 0..8 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), screen)),
                time: Some(frame as f64 * 0.1),
                predicted_dt: 0.1,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                panel.show(ui.ctx(), &mut macro_, true, &runtime, false, None);
            });
            last = Some(std::mem::take(&mut out.shapes));
            out.drop_without_applying_deltas();
        }
        (ctx, last.unwrap())
    }

    fn window_rect(ctx: &egui::Context) -> egui::Rect {
        ctx.memory(|m| m.area_rect(egui::Id::new("sqyre_variables_panel")))
            .expect("variables window painted")
    }

    /// The bottom tab hugs its content: a couple of runtime rows must not
    /// leave a dead band between the list viewport and the window edge.
    #[test]
    fn short_runtime_list_reaches_window_bottom() {
        let (ctx, shapes) = run_panel(
            false,
            &[("alpha", "1"), ("beta", "two")],
            egui::vec2(900.0, 700.0),
        );
        let win = window_rect(&ctx);
        // The bottom section's scroll clip starts well below the declared
        // list's; its viewport should end at the frame edge (float slop only).
        let bottom = shapes
            .iter()
            .map(|c| c.clip_rect)
            .filter(|r| r.min.y > 200.0 && r.max.y < win.bottom() && r.width() > 200.0)
            .map(|r| r.max.y)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            bottom.is_finite() && win.bottom() - bottom <= 12.0,
            "bottom section viewport ends {bottom} but window ends {} — dead band below the list",
            win.bottom()
        );
    }

    /// Fixed-prose descriptions wrap instead of overflowing a few px — the
    /// Built-ins list must not paint a horizontal scrollbar.
    #[test]
    fn builtins_list_has_no_horizontal_scrollbar() {
        let (ctx, shapes) = run_panel(true, &[], egui::vec2(900.0, 700.0));
        let win = window_rect(&ctx);
        let hbar = shapes.iter().any(|c| match &c.shape {
            egui::Shape::Rect(r) => {
                r.rect.height() <= 6.0
                    && r.rect.width() > 60.0
                    && r.rect.width() < win.width()
                    && r.rect.min.y > win.top()
            }
            _ => false,
        });
        assert!(!hbar, "built-ins list painted a horizontal scrollbar");
    }
}
