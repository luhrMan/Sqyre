//! Macro name, delay, and tags toolbar widgets.

use eframe::egui;
use sqyre_domain::Macro;

const DELAY_MIN: i32 = 0;
const DELAY_MAX: i32 = 1000;

#[derive(Debug, Default)]
pub struct MacroMetaUi {
    name_draft: String,
    name_error: Option<String>,
    tag_draft: String,
    delay_open: bool,
    /// Selection identity used to refresh drafts when the user switches macros.
    synced_name: String,
    synced_idx: Option<usize>,
}

/// Mutations requested by the meta toolbar (applied by the app shell).
#[derive(Debug, Default)]
pub struct MetaMutations {
    pub rename_to: Option<String>,
    pub persist: bool,
}

impl MacroMetaUi {
    /// Refresh draft fields when the selected macro changes.
    pub fn sync_selection(&mut self, idx: usize, m: &Macro) {
        let changed = self.synced_idx != Some(idx) || self.synced_name != m.name;
        if !changed {
            return;
        }
        self.synced_idx = Some(idx);
        self.synced_name = m.name.clone();
        self.name_draft = m.name.clone();
        self.name_error = None;
        self.tag_draft.clear();
        self.delay_open = false;
    }

    /// Name row + delay/tags controls. Hotkey stays in `main` beside this block.
    ///
    /// `other_macro_names` / `all_tags` are pre-collected so we can mutably borrow
    /// the selected macro without aliasing the macros slice.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        m: &mut Macro,
        other_macro_names: &[String],
        all_tags: &[String],
        enabled: bool,
    ) -> MetaMutations {
        let mut out = MetaMutations::default();

        ui.horizontal(|ui| {
            ui.label("Name:");
            let te = egui::TextEdit::singleline(&mut self.name_draft)
                .desired_width(220.0)
                .hint_text("Macro name");
            let resp = ui.add_enabled(enabled, te);
            // Commit on Enter, or when focus leaves with a different value.
            let enter = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let lost_dirty = resp.lost_focus() && self.name_draft.trim() != m.name;
            if enabled && (enter || lost_dirty) {
                let trimmed = self.name_draft.trim().to_string();
                match validate_rename(&trimmed, &m.name, other_macro_names) {
                    Ok(()) => {
                        self.name_error = None;
                        if trimmed != m.name {
                            out.rename_to = Some(trimmed);
                        } else {
                            self.name_draft = m.name.clone();
                        }
                    }
                    Err(e) => {
                        self.name_error = Some(e);
                        self.name_draft = m.name.clone();
                    }
                }
            }
            if let Some(err) = &self.name_error {
                ui.colored_label(ui.visuals().error_fg_color, err);
            }
        });

        ui.horizontal(|ui| {
            let delay_tip = format_delay_tooltip(m);
            if ui
                .add_enabled(enabled, egui::Button::new("⏱ Delays"))
                .on_hover_text(delay_tip)
                .clicked()
            {
                self.delay_open = !self.delay_open;
            }

            ui.label("Tags:");
            let tag_te = egui::TextEdit::singleline(&mut self.tag_draft)
                .desired_width(140.0)
                .hint_text("Add tag…");
            let tag_resp = ui.add_enabled(enabled, tag_te);
            let add_enter = tag_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enabled && add_enter {
                if try_add_tag(m, &self.tag_draft) {
                    out.persist = true;
                }
                self.tag_draft.clear();
            }

            // Existing tags as removable chips to the right of the entry.
            let mut remove: Option<String> = None;
            for tag in &m.tags {
                if ui
                    .add_enabled(enabled, egui::Button::new(format!("{tag} ×")))
                    .on_hover_text("Remove tag")
                    .clicked()
                {
                    remove = Some(tag.clone());
                }
            }
            if let Some(tag) = remove {
                if remove_tag(m, &tag) {
                    out.persist = true;
                }
            }

            // Inline completion suggestions from the union of all macro tags.
            if enabled && !self.tag_draft.trim().is_empty() {
                let suggestions = tag_completion_options(&self.tag_draft, &m.tags, all_tags, 8);
                if !suggestions.is_empty() {
                    ui.separator();
                    for sug in suggestions {
                        if ui.small_button(&sug).clicked() {
                            if try_add_tag(m, &sug) {
                                out.persist = true;
                            }
                            self.tag_draft.clear();
                        }
                    }
                }
            }
        });

        if self.delay_open {
            let mut close = false;
            egui::Window::new("Delay between actions")
                .collapsible(false)
                .resizable(false)
                .auto_sized()
                .open(&mut self.delay_open)
                .show(ui.ctx(), |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        if delay_row(ui, "Global (ms)", &mut m.global_delay) {
                            out.persist = true;
                        }
                        if delay_row(ui, "Keyboard (ms)", &mut m.keyboard_delay) {
                            out.persist = true;
                        }
                        if delay_row(ui, "Mouse (ms)", &mut m.mouse_delay) {
                            out.persist = true;
                        }
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
            if close {
                self.delay_open = false;
            }
        }

        out
    }
}

fn delay_row(ui: &mut egui::Ui, label: &str, value: &mut i32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let resp = ui.add(
            egui::DragValue::new(value)
                .range(DELAY_MIN..=DELAY_MAX)
                .speed(1),
        );
        changed = resp.changed();
    });
    changed
}

fn format_delay_tooltip(m: &Macro) -> String {
    let mut parts = Vec::new();
    if m.global_delay > 0 {
        parts.push(format!("Global: {} ms", m.global_delay));
    }
    if m.keyboard_delay > 0 {
        parts.push(format!("Keyboard: {} ms", m.keyboard_delay));
    }
    if m.mouse_delay > 0 {
        parts.push(format!("Mouse: {} ms", m.mouse_delay));
    }
    if parts.is_empty() {
        "Action delays (ms)".into()
    } else {
        parts.join("\n")
    }
}

fn validate_rename(
    new_name: &str,
    current: &str,
    other_macro_names: &[String],
) -> Result<(), String> {
    if new_name.is_empty() {
        return Err("macro name cannot be empty".into());
    }
    if new_name == current {
        return Ok(());
    }
    if other_macro_names.iter().any(|n| n == new_name) {
        return Err("macro name already exists".into());
    }
    Ok(())
}

/// Sorted unique tags across macros (for completion).
pub fn collect_all_macro_tags(macros: &[Macro]) -> Vec<String> {
    let mut tags: Vec<String> = macros.iter().flat_map(|m| m.tags.iter().cloned()).collect();
    tags.sort();
    tags.dedup();
    tags
}

fn try_add_tag(m: &mut Macro, raw: &str) -> bool {
    let tag = raw.trim();
    if tag.is_empty() || m.tags.iter().any(|t| t == tag) {
        return false;
    }
    m.tags.push(tag.to_string());
    true
}

fn remove_tag(m: &mut Macro, tag: &str) -> bool {
    let before = m.tags.len();
    m.tags.retain(|t| t != tag);
    m.tags.len() != before
}

fn tag_completion_options(
    search: &str,
    on_macro: &[String],
    all_tags: &[String],
    limit: usize,
) -> Vec<String> {
    let search_l = search.trim().to_lowercase();
    if search_l.is_empty() {
        return Vec::new();
    }
    all_tags
        .iter()
        .filter(|t| !on_macro.iter().any(|c| c == *t))
        .filter(|t| t.to_lowercase().contains(&search_l))
        .take(limit)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::Macro;

    fn m(name: &str, tags: &[&str]) -> Macro {
        let mut macro_ = Macro::new(name, 0, Vec::new());
        macro_.tags = tags.iter().map(|s| (*s).to_string()).collect();
        macro_
    }

    #[test]
    fn rename_rejects_empty_and_duplicate() {
        let others = vec!["b".to_string()];
        assert!(validate_rename("", "a", &others).is_err());
        assert!(validate_rename("b", "a", &others).is_err());
        assert!(validate_rename("c", "a", &others).is_ok());
        assert!(validate_rename("a", "a", &others).is_ok());
    }

    #[test]
    fn tag_add_remove_and_completion() {
        let mut macro_ = m("x", &["alpha"]);
        assert!(!try_add_tag(&mut macro_, "  "));
        assert!(!try_add_tag(&mut macro_, "alpha"));
        assert!(try_add_tag(&mut macro_, "beta"));
        assert_eq!(macro_.tags, vec!["alpha", "beta"]);
        assert!(remove_tag(&mut macro_, "alpha"));
        assert_eq!(macro_.tags, vec!["beta"]);

        let all_tags =
            collect_all_macro_tags(&[m("x", &["beta"]), m("y", &["beta", "gamma", "gator"])]);
        let opts = tag_completion_options("ga", &["beta".into()], &all_tags, 10);
        assert_eq!(opts, vec!["gamma", "gator"]);
    }

    #[test]
    fn delay_tooltip_lists_nonzero() {
        let mut macro_ = m("x", &[]);
        macro_.global_delay = 0;
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        assert_eq!(format_delay_tooltip(&macro_), "Action delays (ms)");
        macro_.global_delay = 10;
        macro_.mouse_delay = 5;
        assert_eq!(format_delay_tooltip(&macro_), "Global: 10 ms\nMouse: 5 ms");
    }
}
