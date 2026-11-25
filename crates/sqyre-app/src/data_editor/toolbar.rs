//! Data Editor header: search/program toggles, search controls, tab combo.

use super::tabs::{EditorSection, EditorTab, SearchScope};
use super::{helpers, DataEditor};
use crate::icon_cache::IconCache;
use eframe::egui;
use sqyre_persist::{CatalogView, ProgramCatalog, UserSettings};

impl DataEditor {
    /// Header under the standard window title bar: the search and program
    /// toggle icons stay adjacent on the left so expanding a pair never moves
    /// the other icon — its controls open to the right of both toggles. The
    /// active-tab combo follows.
    pub(super) fn ui_tab_bar(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        settings: &mut UserSettings,
    ) {
        ui.horizontal(|ui| {
            let just_opened = self.ui_search_toggle(ui);
            self.program_toggle(ui, CatalogView::new(catalog), icons, settings);
            if self.nav.search_open {
                self.ui_search_controls(ui, catalog, icons, settings, just_opened);
            }
            if self.nav.program_open {
                self.ui_program_combo(ui, CatalogView::new(catalog), icons, settings);
            }
            crate::widgets::section_separator(ui);
            self.paint_tab_combo(ui, CatalogView::new(catalog), settings);
        });
    }

    /// Magnifier toggle; returns true on the click that opened the search pair
    /// (so the caller can focus the freshly-shown field).
    fn ui_search_toggle(&mut self, ui: &mut egui::Ui) -> bool {
        let (tip, color) = if self.nav.search_open {
            ("Hide search", Some(crate::theme::accent()))
        } else {
            ("Search this tab's list", None)
        };
        let toggle = ui.add(crate::widgets::IconButton::new(
            egui_phosphor::regular::MAGNIFYING_GLASS,
            tip,
            color,
        ));
        let just_opened = toggle.clicked() && !self.nav.search_open;
        if toggle.clicked() {
            self.nav.search_open = !self.nav.search_open;
            if self.nav.search_open {
                self.nav.program_open = false;
            }
        }
        just_opened
    }

    /// The expanded search pair: query field → scope → item sort → expand/collapse.
    /// Expand/collapse-all trails the row when the list is grouped by program
    /// (All programs), matching item/coord pickers. The query is kept (and
    /// keeps filtering only while visible) when the field hides.
    fn ui_search_controls(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        settings: &mut UserSettings,
        just_opened: bool,
    ) {
        let hint = format!("Search {}…", self.nav.tab.label());
        // Hug the query text but stay clear of the tab strip;
        // `available_width` here is what is left in the header row.
        let width = crate::widgets::name_edit_width(ui, &self.nav.search, &hint)
            .min((ui.available_width() * 0.45).max(80.0));
        let resp = ui.add(
            egui::TextEdit::singleline(&mut self.nav.search)
                // Stable id: the toggling field would otherwise drop keyboard focus.
                .id(egui::Id::new("de_header_search"))
                .hint_text(&hint)
                .desired_width(width),
        );
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, hint.clone())
        });
        if just_opened {
            resp.request_focus();
        }
        if resp.changed() && self.nav.selected_program.is_some() {
            // A newly-visible selection can scroll into view after filter edits.
            self.nav.scroll_left_list_to_selection = true;
        }
        // Programs lists programs — scope only applies to entity tabs.
        if !matches!(self.nav.tab, EditorTab::Programs) {
            // Funnel icon opens the scope menu; tinted while a non-default
            // scope is active since the icon can't show the selection.
            let scope_btn = ui.add(crate::widgets::IconButton::new(
                egui_phosphor::regular::FUNNEL,
                "List scope",
                (self.nav.search_scope != SearchScope::SelectedProgram)
                    .then_some(crate::theme::accent()),
            ));
            let mut scope = self.nav.search_scope;
            crate::widgets::context_menu_popup(
                ui,
                scope_btn.id.with("scope_menu"),
                scope_btn.clicked(),
                Some(scope_btn.rect),
                |ui| {
                    for &s in SearchScope::ALL {
                        ui.radio_value(&mut scope, s, s.label());
                    }
                },
            );
            if scope != self.nav.search_scope {
                self.nav.search_scope = scope;
                // Same as a tab switch: open + scroll to the selected program's group.
                self.nav.scroll_left_list_to_selection = self.nav.selected_program.is_some();
            }
        }
        // Items / Match Check sort, next to the scope filter.
        if matches!(self.nav.tab, EditorTab::Items | EditorTab::MatchCheck) {
            use sqyre_domain::CatalogItemSort as Sort;
            let sort_btn = ui.add(crate::widgets::IconButton::new(
                egui_phosphor::regular::SORT_ASCENDING,
                "Sort items",
                (settings.ui.data_editor_items_sort != Sort::NameAsc)
                    .then_some(crate::theme::accent()),
            ));
            // `Popup` (not `Menu`) kind + `parent_popup`: picking a radio,
            // editing the tag chips, clicking a tag suggestion, or using a
            // chip's right-click menu must not dismiss it — those nested
            // popups live on their own egui layers, which `CloseOnClickOutside`
            // counts as outside. It still closes on a true outside click or
            // Escape. Temp-data bool, not `open_memory`: the chip context menus
            // hold the single `Memory::popups` slot while open.
            let sort_menu_id = sort_btn.id.with("sort_menu");
            let mut open = ui
                .ctx()
                .data(|d| d.get_temp::<bool>(sort_menu_id.with("open")))
                .unwrap_or(false);
            if sort_btn.clicked() {
                open = !open;
            }
            let mut sort_changed = false;
            crate::widgets::parent_popup(
                egui::Popup::new(
                    sort_menu_id,
                    ui.ctx().clone(),
                    egui::PopupAnchor::ParentRect(sort_btn.rect),
                    ui.layer_id(),
                )
                .kind(egui::PopupKind::Popup)
                .layout(egui::Layout::top_down_justified(egui::Align::Min))
                .style(egui::containers::menu::menu_style),
                &mut open,
                |ui| {
                    for &s in Sort::ALL {
                        if ui
                            .radio_value(&mut settings.ui.data_editor_items_sort, s, s.label())
                            .changed()
                        {
                            sort_changed = true;
                        }
                    }
                    if settings.ui.data_editor_items_sort == Sort::Tags {
                        ui.separator();
                        ui.label(egui::RichText::new("Tag priority").strong().small());
                        let suggestions =
                            helpers::collect_all_item_tags(CatalogView::from(catalog));
                        let program_icons =
                            icons.program_name_icons(ui.ctx(), CatalogView::from(catalog));
                        let edit = crate::widgets::tag_chip_editor(
                            ui,
                            &mut settings.ui.data_editor_items_tag_priority,
                            &mut self.nav.items_tag_priority_draft,
                            &suggestions,
                            crate::widgets::TagChipOptions {
                                enabled: true,
                                show_add_button: true,
                                label_name: None,
                                draft_hover: Some(
                                    "Drag chips to set priority. Unmatched items sort by name after.",
                                ),
                                draft_hint: Some("Priority tag…"),
                                reorderable: true,
                                signed_filters: false,
                                single_row: false,
                                locked: &[],
                                locked_hover: None,
                                program_icons: Some(&program_icons),
                                draft_width: None,
                                draft_id: None,
                            },
                        );
                        if edit.changed {
                            sort_changed = true;
                        }
                    }
                },
            );
            ui.ctx()
                .data_mut(|d| d.insert_temp(sort_menu_id.with("open"), open));
            if sort_changed {
                if let Err(e) = settings.save_default() {
                    self.set_err(format!(
                        "Failed to save item sort: {}",
                        crate::status_banner::user_error_detail(&e)
                    ));
                }
            }
        }
        // Right end of the search row — same slot pickers use for expand/collapse.
        self.paint_search_expand_collapse(ui, CatalogView::from(catalog));
    }

    /// Closed set of Data Editor tabs as a combo: each choice is the tab's
    /// phosphor glyph plus its human title, grouped by section.
    fn paint_tab_combo(
        &mut self,
        ui: &mut egui::Ui,
        catalog: CatalogView<'_>,
        settings: &UserSettings,
    ) {
        let mut tab = self.nav.tab;
        egui::ComboBox::from_id_salt("de_tab")
            .selected_text(tab.choice_label())
            .truncate()
            .show_ui(ui, |ui| {
                let mut first_section = true;
                for sec in [
                    EditorSection::Programs,
                    EditorSection::Items,
                    EditorSection::Coordinates,
                    EditorSection::Tools,
                ] {
                    if !sec.is_available() {
                        continue;
                    }
                    if !first_section {
                        ui.separator();
                    }
                    first_section = false;
                    for &t in sec.subtabs() {
                        ui.selectable_value(&mut tab, t, t.choice_label());
                    }
                }
            });
        if tab != self.nav.tab {
            self.switch_tab(tab, catalog, settings);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn tab_combo_lists_icon_and_name_and_switches() {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(480.0, 320.0))
            .build_ui_state(
                |ui, tab: &mut EditorTab| {
                    let selected = tab.choice_label();
                    egui::ComboBox::from_id_salt("de_tab_test")
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for sec in [
                                EditorSection::Programs,
                                EditorSection::Items,
                                EditorSection::Coordinates,
                                EditorSection::Tools,
                            ] {
                                if !sec.is_available() {
                                    continue;
                                }
                                for &t in sec.subtabs() {
                                    ui.selectable_value(tab, t, t.choice_label());
                                }
                            }
                        });
                },
                EditorTab::Programs,
            );
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        let points = EditorTab::Points.choice_label();
        harness.get_by_label(points.as_str()).click();
        harness.run();
        assert_eq!(*harness.state(), EditorTab::Points);
        assert!(
            points.contains(EditorTab::Points.icon()) && points.contains("Points"),
            "choice face keeps glyph + title: {points}"
        );
    }
}
