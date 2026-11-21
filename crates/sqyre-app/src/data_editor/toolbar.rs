//! Data Editor header: search/program toggles, search controls, section bar.

use super::tabs::{EditorSection, EditorTab, SearchScope};
use super::{helpers, DataEditor, DataEditorCtx};
use crate::icon_cache::IconCache;
use eframe::egui;
use sqyre_persist::{CatalogView, ProgramCatalog, UserSettings};

impl DataEditor {
    /// Tab strip under the standard window title bar: the search and program
    /// toggle icons stay adjacent on the left so expanding a pair never moves
    /// the other icon — its controls open to the right of both toggles. The
    /// section/subtab icons follow in an overflow scroll.
    pub(super) fn ui_tab_bar(&mut self, ui: &mut egui::Ui, env: &mut DataEditorCtx<'_>) {
        ui.horizontal(|ui| {
            let just_opened = self.ui_search_toggle(ui);
            self.program_toggle(ui, CatalogView::new(env.catalog), env.icons, env.settings);
            if self.search_open {
                self.ui_search_controls(ui, env.catalog, env.icons, env.settings, just_opened);
            }
            if self.program_open {
                self.ui_program_combo(ui, CatalogView::new(env.catalog), env.icons, env.settings);
            }
            crate::widgets::section_separator(ui);
            crate::widgets::show_overflow(ui, "de_tab_bar", 1.0, |ui| {
                ui.horizontal(|ui| self.paint_section_bar(ui, env));
            });
        });
    }

    /// Magnifier toggle; returns true on the click that opened the search pair
    /// (so the caller can focus the freshly-shown field).
    fn ui_search_toggle(&mut self, ui: &mut egui::Ui) -> bool {
        let (tip, color) = if self.search_open {
            ("Hide search", Some(crate::theme::accent()))
        } else {
            ("Search this tab's list", None)
        };
        let toggle = ui.add(crate::widgets::IconButton::new(
            egui_phosphor::regular::MAGNIFYING_GLASS,
            tip,
            color,
        ));
        let just_opened = toggle.clicked() && !self.search_open;
        if toggle.clicked() {
            self.search_open = !self.search_open;
            if self.search_open {
                self.program_open = false;
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
        let hint = format!("Search {}…", self.tab.label());
        // Hug the query text but stay clear of the tab strip;
        // `available_width` here is what is left in the header row.
        let width = crate::widgets::name_edit_width(ui, &self.search, &hint)
            .min((ui.available_width() * 0.45).max(80.0));
        let resp = ui.add(
            egui::TextEdit::singleline(&mut self.search)
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
        if resp.changed() && self.selected_program.is_some() {
            // A newly-visible selection can scroll into view after filter edits.
            self.scroll_left_list_to_selection = true;
        }
        // Programs lists programs — scope only applies to entity tabs.
        if !matches!(self.tab, EditorTab::Programs) {
            // Funnel icon opens the scope menu; tinted while a non-default
            // scope is active since the icon can't show the selection.
            let scope_btn = ui.add(crate::widgets::IconButton::new(
                egui_phosphor::regular::FUNNEL,
                "List scope",
                (self.search_scope != SearchScope::SelectedProgram)
                    .then_some(crate::theme::accent()),
            ));
            let mut scope = self.search_scope;
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
            if scope != self.search_scope {
                self.search_scope = scope;
                // Same as a tab switch: open + scroll to the selected program's group.
                self.scroll_left_list_to_selection = self.selected_program.is_some();
            }
        }
        // Items / Match Check sort, next to the scope filter.
        if matches!(self.tab, EditorTab::Items | EditorTab::MatchCheck) {
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
                            &mut self.items_tag_priority_draft,
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

    /// Every section's subtab buttons in one row — the subsection buttons are
    /// self-explanatory, so there are no section-level icons — with separators
    /// between the sections.
    fn paint_section_bar(&mut self, ui: &mut egui::Ui, env: &mut DataEditorCtx<'_>) {
        let mut items = Vec::new();
        let mut targets = Vec::new();
        for sec in [
            EditorSection::Programs,
            EditorSection::Items,
            EditorSection::Coordinates,
            EditorSection::Tools,
        ] {
            if !sec.is_available() {
                continue;
            }
            if !items.is_empty() {
                items.push(crate::widgets::OverflowItem::Separator);
                // Separators never register a click; the slot only keeps
                // `targets` index-aligned with `items`.
                targets.push(sec.default_tab());
            }
            for &tab in sec.subtabs() {
                items.push(crate::widgets::OverflowItem::Choice {
                    label: tab.label(),
                    icon: tab.icon(),
                    selected: self.tab == tab,
                });
                targets.push(tab);
            }
        }
        if let Some(i) = crate::widgets::always_bar(ui, &items) {
            let next = targets[i];
            if next != self.tab {
                self.switch_tab(next, env.catalog, env.settings);
            }
        }
    }
}
