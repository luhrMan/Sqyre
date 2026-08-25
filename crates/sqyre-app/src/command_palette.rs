//! App-wide command window (Ctrl/Cmd+K): add actions, jump to macros, open editors.

use crate::data_editor::{DataEditorCtx, EditorTab};
use crate::overlay_icons;
use crate::pickers::fuzzy_match_fold;
use crate::SqyreApp;
use eframe::egui::{self, Color32, CornerRadius, Key, Modifiers, Sense};
use sqyre_domain::{action_type_table, Macro};
use sqyre_persist::{OverlayButtonConfig, ProgramCatalog};
use sqyre_ui_model::action_picker_category;

const WINDOW_ID: &str = "sqyre_command_palette";
const ROW_H: f32 = 28.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandKind {
    AddAction {
        type_key: String,
    },
    OpenMacro {
        name: String,
    },
    NewMacro,
    OpenDataEditor,
    OpenSettings,
    OpenVariables,
    ShowMacroList,
    NewCatalogEntity {
        tab: EditorTab,
    },
    OpenProgram {
        name: String,
    },
    OpenCatalogEntity {
        tab: EditorTab,
        program: String,
        entity: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct CommandItem {
    pub title: String,
    pub hint: String,
    pub icon: &'static str,
    pub kind: CommandKind,
    pub keywords: Vec<String>,
}

pub(crate) struct CommandSources<'a> {
    pub macros: &'a [Macro],
    pub catalog: &'a ProgramCatalog,
    pub overlay_buttons: &'a [OverlayButtonConfig],
    pub running: bool,
}

#[derive(Debug, Default)]
pub(crate) struct CommandPaletteUi {
    open: bool,
    query: String,
    selected: usize,
    focus_search: bool,
    scroll_selected: bool,
}

impl CommandPaletteUi {
    pub(crate) fn open_palette(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.focus_search = true;
        self.scroll_selected = true;
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
        self.focus_search = false;
        self.scroll_selected = false;
    }

    pub(crate) fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open_palette();
        }
    }

    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        commands: &[CommandItem],
    ) -> Option<CommandKind> {
        if !self.open {
            return None;
        }

        let mut close = false;
        let mut run = None;
        let mut query_changed = false;

        // Full-screen hit target under the window: click outside dismisses.
        // Drawn first so the palette window stays on top in Foreground order.
        egui::Area::new(egui::Id::new(format!("{WINDOW_ID}_dismiss")))
            .order(egui::Order::Foreground)
            .fixed_pos(ctx.content_rect().min)
            .interactable(true)
            .show(ctx, |ui| {
                let resp = ui.allocate_rect(ctx.content_rect(), Sense::click());
                if resp.clicked() {
                    close = true;
                }
            });

        let mut open = self.open;
        egui::Window::new("Command palette")
            .id(egui::Id::new(WINDOW_ID))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 72.0])
            .default_size([520.0, 380.0])
            .min_size([400.0, 200.0])
            .order(egui::Order::Foreground)
            .open(&mut open)
            .show(ctx, |ui| {
                let (esc, enter, down, up) = ui.input_mut(|i| {
                    (
                        i.consume_key(Modifiers::NONE, Key::Escape),
                        i.consume_key(Modifiers::NONE, Key::Enter),
                        i.consume_key(Modifiers::NONE, Key::ArrowDown),
                        i.consume_key(Modifiers::NONE, Key::ArrowUp),
                    )
                });
                if esc {
                    close = true;
                    return;
                }

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(
                        egui_phosphor::regular::MAGNIFYING_GLASS,
                    ));
                    let edit = egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Add action, open macro, new item…")
                        .desired_width(f32::INFINITY);
                    let resp = ui.add(edit);
                    if self.focus_search {
                        resp.request_focus();
                        self.focus_search = false;
                    }
                    query_changed = resp.changed();
                });
                ui.separator();

                let filtered = filter_ranked(&self.query, commands);
                if query_changed {
                    self.selected = 0;
                    self.scroll_selected = true;
                }
                if filtered.is_empty() {
                    ui.weak("No matching commands.");
                    return;
                }
                if down {
                    self.selected = (self.selected + 1).min(filtered.len() - 1);
                    self.scroll_selected = true;
                } else if up {
                    self.selected = self.selected.saturating_sub(1);
                    self.scroll_selected = true;
                }
                self.selected = self.selected.min(filtered.len() - 1);

                if enter {
                    run = Some(filtered[self.selected].kind.clone());
                    return;
                }

                crate::pickers::scroll_vertical()
                    .auto_shrink([false, false])
                    .max_height(ROW_H * 10.0 + 8.0)
                    .show(ui, |ui| {
                        for (i, item) in filtered.iter().enumerate() {
                            let resp = command_row(ui, item, i == self.selected);
                            if resp.clicked() {
                                run = Some(item.kind.clone());
                            }
                            if i == self.selected && self.scroll_selected {
                                ui.scroll_to_rect(resp.rect, Some(egui::Align::Center));
                                self.scroll_selected = false;
                            }
                        }
                    });
            });
        if close || run.is_some() {
            self.close();
        } else {
            self.open = open;
        }
        run
    }
}

fn command_row(ui: &mut egui::Ui, item: &CommandItem, selected: bool) -> egui::Response {
    let fill = if selected {
        crate::theme::accent_dim()
    } else {
        Color32::TRANSPARENT
    };
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), Sense::click());
    ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
    let font = egui::TextStyle::Body.resolve(ui.style());
    let small = egui::TextStyle::Small.resolve(ui.style());
    let text_color = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let icon_font = overlay_icons::glyph_font_id(font.size);
    let icon_galley = ui
        .painter()
        .layout_no_wrap(item.icon.to_owned(), icon_font, text_color);
    let title_galley = ui
        .painter()
        .layout_no_wrap(item.title.clone(), font, text_color);
    let hint_galley = ui.painter().layout_no_wrap(item.hint.clone(), small, weak);
    let y = rect.center().y;
    let mut x = rect.left() + 8.0;
    ui.painter().galley(
        egui::pos2(x, y - icon_galley.size().y * 0.5),
        icon_galley,
        ui.visuals().text_color(),
    );
    x += 22.0;
    ui.painter().galley(
        egui::pos2(x, y - title_galley.size().y * 0.5),
        title_galley,
        ui.visuals().text_color(),
    );
    let hint_pos = egui::pos2(
        rect.right() - hint_galley.size().x - 10.0,
        y - hint_galley.size().y * 0.5,
    );
    ui.painter()
        .galley(hint_pos, hint_galley, ui.visuals().weak_text_color());
    resp
}

pub(crate) fn collect_commands(src: CommandSources<'_>) -> Vec<CommandItem> {
    let mut out = Vec::new();
    let has_macros = !src.macros.is_empty();
    let mutating = !src.running;

    push_nav(&mut out, has_macros);
    if mutating {
        out.push(item(
            "New Macro",
            "Go to",
            ph("plus"),
            CommandKind::NewMacro,
            &["create", "add", "macro"],
        ));
        push_new_entities(&mut out);
        if has_macros {
            push_actions(&mut out);
        }
    }
    for m in src.macros {
        let mut keys = vec!["macro".into(), "open".into(), "goto".into()];
        keys.extend(m.tags.iter().cloned());
        out.push(CommandItem {
            title: m.name.clone(),
            hint: "Macro".into(),
            icon: ph("play"),
            kind: CommandKind::OpenMacro {
                name: m.name.clone(),
            },
            keywords: keys,
        });
    }
    push_catalog_entities(&mut out, src.catalog, src.overlay_buttons);
    out
}

fn push_nav(out: &mut Vec<CommandItem>, has_macros: bool) {
    out.push(item(
        "Open Data Editor",
        "Go to",
        ph("folder"),
        CommandKind::OpenDataEditor,
        &["data", "editor", "catalog", "goto"],
    ));
    out.push(item(
        "Open Settings",
        "Go to",
        ph("gear"),
        CommandKind::OpenSettings,
        &["preferences", "options", "goto"],
    ));
    if has_macros {
        out.push(item(
            "Open Variables",
            "Go to",
            ph("equals"),
            CommandKind::OpenVariables,
            &["vars", "goto"],
        ));
    }
    out.push(item(
        "Show Macro List",
        "Go to",
        ph("list"),
        CommandKind::ShowMacroList,
        &["sidebar", "goto"],
    ));
}

fn push_new_entities(out: &mut Vec<CommandItem>) {
    for (tab, title, extra) in [
        (EditorTab::Programs, "New Program", "program model"),
        (EditorTab::Items, "New Item", "item model"),
        (EditorTab::Points, "New Point", "coordinate model"),
        (
            EditorTab::SearchAreas,
            "New Search Area",
            "search area model",
        ),
        (EditorTab::Masks, "New Mask", "mask model"),
        (EditorTab::Collections, "New Collection", "collection model"),
        (EditorTab::Atlases, "New Atlas", "atlas model"),
        (
            EditorTab::Overlay,
            "New Overlay Button",
            "overlay button model",
        ),
    ] {
        out.push(item(
            title,
            "Data Editor",
            ph("plus"),
            CommandKind::NewCatalogEntity { tab },
            &["create", "add", "new", extra],
        ));
    }
}

fn push_actions(out: &mut Vec<CommandItem>) {
    for meta in action_type_table() {
        let category = action_picker_category(meta.type_key);
        out.push(CommandItem {
            title: format!("Add {}", meta.label),
            hint: "Action".into(),
            icon: action_phosphor(meta.type_key),
            kind: CommandKind::AddAction {
                type_key: meta.type_key.to_string(),
            },
            keywords: vec![
                meta.type_key.to_string(),
                meta.label.to_string(),
                category.to_string(),
                meta.description.to_string(),
                "action".into(),
                "add".into(),
            ],
        });
    }
}

fn push_catalog_entities(
    out: &mut Vec<CommandItem>,
    catalog: &ProgramCatalog,
    overlay_buttons: &[OverlayButtonConfig],
) {
    for name in catalog.program_names() {
        out.push(CommandItem {
            title: name.clone(),
            hint: "Program".into(),
            icon: ph("folder"),
            kind: CommandKind::OpenProgram { name: name.clone() },
            keywords: vec!["program".into(), "goto".into()],
        });
        let Some(prog) = catalog.get(name) else {
            continue;
        };
        push_named_map(
            out,
            name,
            EditorTab::Items,
            "Item",
            ph("image"),
            prog.items.iter().map(|(k, it)| {
                let display = nonempty_or(it.name.as_str(), k);
                (k.clone(), display, it.tags.clone())
            }),
        );
        let res = catalog.resolution_key();
        if let Some(points) = prog.points.get(res).or_else(|| prog.points.values().next()) {
            push_named_map(
                out,
                name,
                EditorTab::Points,
                "Point",
                ph("map-pin"),
                points
                    .iter()
                    .map(|(k, pt)| (k.clone(), nonempty_or(pt.name.as_str(), k), Vec::new())),
            );
        }
        if let Some(areas) = prog
            .search_areas
            .get(res)
            .or_else(|| prog.search_areas.values().next())
        {
            push_named_map(
                out,
                name,
                EditorTab::SearchAreas,
                "Search Area",
                ph("selection"),
                areas
                    .iter()
                    .map(|(k, sa)| (k.clone(), nonempty_or(sa.name.as_str(), k), Vec::new())),
            );
        }
        push_named_map(
            out,
            name,
            EditorTab::Masks,
            "Mask",
            ph("circle-dashed"),
            prog.masks
                .iter()
                .map(|(k, m)| (k.clone(), nonempty_or(m.name.as_str(), k), Vec::new())),
        );
        push_named_map(
            out,
            name,
            EditorTab::Collections,
            "Collection",
            ph("grid-four"),
            prog.collections
                .iter()
                .map(|(k, c)| (k.clone(), nonempty_or(c.name.as_str(), k), Vec::new())),
        );
        push_named_map(
            out,
            name,
            EditorTab::Atlases,
            "Atlas",
            ph("stack"),
            prog.atlases
                .iter()
                .map(|(k, a)| (k.clone(), nonempty_or(a.name.as_str(), k), Vec::new())),
        );
    }
    for btn in overlay_buttons {
        let title = if btn.label.trim().is_empty() {
            btn.id.clone()
        } else {
            btn.label.clone()
        };
        let program = if btn.program.is_empty() {
            "Overlay".to_string()
        } else {
            btn.program.clone()
        };
        out.push(CommandItem {
            title,
            hint: format!("Overlay · {program}"),
            icon: ph("square"),
            kind: CommandKind::OpenCatalogEntity {
                tab: EditorTab::Overlay,
                program: btn.program.clone(),
                entity: btn.id.clone(),
            },
            keywords: vec![
                "overlay".into(),
                "button".into(),
                btn.macro_name.clone(),
                btn.id.clone(),
            ],
        });
    }
}

fn push_named_map(
    out: &mut Vec<CommandItem>,
    program: &str,
    tab: EditorTab,
    kind_label: &'static str,
    icon: &'static str,
    rows: impl Iterator<Item = (String, String, Vec<String>)>,
) {
    for (key, display, tags) in rows {
        let mut keywords = vec![
            kind_label.to_ascii_lowercase(),
            program.to_string(),
            key.clone(),
            "goto".into(),
        ];
        keywords.extend(tags);
        out.push(CommandItem {
            title: display,
            hint: format!("{kind_label} · {program}"),
            icon,
            kind: CommandKind::OpenCatalogEntity {
                tab,
                program: program.to_string(),
                entity: key,
            },
            keywords,
        });
    }
}

fn nonempty_or<'a>(name: &'a str, key: &'a str) -> String {
    if name.trim().is_empty() {
        key.to_string()
    } else {
        name.to_string()
    }
}

fn item(
    title: &str,
    hint: &str,
    icon: &'static str,
    kind: CommandKind,
    keywords: &[&str],
) -> CommandItem {
    CommandItem {
        title: title.into(),
        hint: hint.into(),
        icon,
        kind,
        keywords: keywords.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn ph(id: &str) -> &'static str {
    overlay_icons::resolve(id).glyph
}

fn action_phosphor(type_key: &str) -> &'static str {
    ph(match type_key {
        "move" => "arrow-right",
        "click" => "mouse",
        "key" => "key",
        "type" => "keyboard",
        "imagesearch" => "magnifying-glass",
        "ocr" => "text-aa",
        "findpixel" => "drop",
        "setvariable" => "equals",
        "savevariable" => "floppy-disk",
        "loop" | "while" => "arrows-clockwise",
        "loopjump" => "stop",
        "foreachrow" => "list-bullets",
        "conditional" => "question",
        "wait" => "timer",
        "pause" => "pause",
        "focuswindow" => "app-window",
        "runmacro" => "play",
        "navigateselect" => "crosshair",
        "navigatekey" => "key",
        _ => "plus",
    })
}

/// Empty query keeps collect-order static commands only (no macros / catalog entities).
pub(crate) fn filter_ranked<'a>(query: &str, items: &'a [CommandItem]) -> Vec<&'a CommandItem> {
    let q = query.trim();
    if q.is_empty() {
        return items
            .iter()
            .filter(|item| is_static_command(&item.kind))
            .collect();
    }
    let mut ranked: Vec<(u32, &CommandItem)> = items
        .iter()
        .filter_map(|item| {
            best_score(q, &item.title, &item.hint, &item.keywords).map(|s| (s, item))
        })
        .collect();
    ranked.sort_by(|(sa, a), (sb, b)| {
        sa.cmp(sb).then_with(|| {
            a.title
                .to_ascii_lowercase()
                .cmp(&b.title.to_ascii_lowercase())
        })
    });
    ranked.into_iter().map(|(_, item)| item).collect()
}

fn is_static_command(kind: &CommandKind) -> bool {
    matches!(
        kind,
        CommandKind::AddAction { .. }
            | CommandKind::NewMacro
            | CommandKind::OpenDataEditor
            | CommandKind::OpenSettings
            | CommandKind::OpenVariables
            | CommandKind::ShowMacroList
            | CommandKind::NewCatalogEntity { .. }
    )
}

fn best_score(query: &str, title: &str, hint: &str, keywords: &[String]) -> Option<u32> {
    let mut best = score_field(query, title);
    if let Some(s) = score_field(query, hint) {
        best = Some(best.map_or(s + 5, |b| b.min(s + 5)));
    }
    for k in keywords {
        if let Some(s) = score_field(query, k) {
            best = Some(best.map_or(s + 15, |b| b.min(s + 15)));
        }
    }
    best
}

fn score_field(query: &str, haystack: &str) -> Option<u32> {
    if !fuzzy_match_fold(query, haystack) {
        return None;
    }
    let h: String = haystack.chars().flat_map(char::to_lowercase).collect();
    let n: String = query.chars().flat_map(char::to_lowercase).collect();
    if h == n {
        return Some(0);
    }
    if h.starts_with(&n) {
        return Some(10);
    }
    if h.split(|c: char| !c.is_alphanumeric())
        .any(|w| !w.is_empty() && w.starts_with(&n))
    {
        return Some(20);
    }
    Some(100 + u32::try_from(h.len().saturating_sub(n.len())).unwrap_or(u32::MAX))
}

impl SqyreApp {
    pub(crate) fn run_palette_command(&mut self, ctx: &egui::Context, kind: CommandKind) {
        match kind {
            CommandKind::AddAction { type_key } => {
                let Some(action) = self.add_action_picker.create_action(&type_key) else {
                    return;
                };
                let anchor = ctx
                    .pointer_interact_pos()
                    .unwrap_or_else(|| ctx.content_rect().center());
                self.insert_blank_action(action, anchor);
            }
            CommandKind::OpenMacro { name } => {
                self.macro_list_open = true;
                self.select_macro_by_name(&name);
            }
            CommandKind::NewMacro => self.create_macro(),
            CommandKind::OpenDataEditor => self.data_editor.request_open(ctx),
            CommandKind::OpenSettings => self.settings_ui.open = true,
            CommandKind::OpenVariables => self.variables_panel.open = true,
            CommandKind::ShowMacroList => self.macro_list_open = true,
            CommandKind::NewCatalogEntity { tab } => {
                self.data_editor.open_new(
                    tab,
                    &mut DataEditorCtx {
                        ctx,
                        db: &mut self.workspace.db,
                        macros: &mut self.workspace.macros,
                        catalog: &mut self.workspace.catalog,
                        icons: &mut self.icon_cache,
                        screen_click: &self.screen_click,
                        settings: self.settings_ui.settings_mut(),
                    },
                );
            }
            CommandKind::OpenProgram { name } => {
                self.data_editor.open_program(
                    ctx,
                    &name,
                    &self.workspace.catalog,
                    self.settings_ui.settings(),
                );
            }
            CommandKind::OpenCatalogEntity {
                tab,
                program,
                entity,
            } => {
                self.data_editor.open_entity(
                    ctx,
                    tab,
                    &program,
                    &entity,
                    &self.workspace.catalog,
                    self.settings_ui.settings(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_persist::{ProgramData, ProgramItem};
    use std::collections::BTreeMap;

    fn sources<'a>(
        macros: &'a [Macro],
        catalog: &'a ProgramCatalog,
        overlay: &'a [OverlayButtonConfig],
        running: bool,
    ) -> CommandSources<'a> {
        CommandSources {
            macros,
            catalog,
            overlay_buttons: overlay,
            running,
        }
    }

    fn titles(query: &str, items: &[CommandItem]) -> Vec<String> {
        filter_ranked(query, items)
            .iter()
            .map(|i| i.title.clone())
            .collect()
    }

    #[test]
    fn empty_query_excludes_macros_and_entities() {
        let macros = vec![Macro::new("Farm gold", 0, vec![])];
        let mut catalog = ProgramCatalog::default();
        catalog.programs_mut().insert(
            "Game".into(),
            ProgramData {
                name: "Game".into(),
                items: BTreeMap::from([(
                    "Flask".into(),
                    ProgramItem {
                        name: "Health Flask".into(),
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        );
        let items = collect_commands(sources(&macros, &catalog, &[], false));
        let shown = titles("", &items);
        assert!(shown.iter().any(|t| t == "Add Click"));
        assert!(shown.iter().any(|t| t == "New Item"));
        assert!(shown.iter().any(|t| t == "Open Data Editor"));
        assert!(!shown.iter().any(|t| t == "Farm gold"));
        assert!(!shown.iter().any(|t| t == "Health Flask"));
        assert!(!shown.iter().any(|t| t == "Game"));
    }

    #[test]
    fn clk_ranks_add_click() {
        let macros = vec![Macro::new("demo", 0, vec![])];
        let catalog = ProgramCatalog::default();
        let items = collect_commands(sources(&macros, &catalog, &[], false));
        let shown = titles("clk", &items);
        assert_eq!(shown.first().map(String::as_str), Some("Add Click"));
    }

    #[test]
    fn new_itm_matches_new_item() {
        let catalog = ProgramCatalog::default();
        let items = collect_commands(sources(&[], &catalog, &[], false));
        let shown = titles("new itm", &items);
        assert!(shown.iter().any(|t| t == "New Item"));
    }

    #[test]
    fn heal_matches_item_entity() {
        let mut catalog = ProgramCatalog::default();
        catalog.programs_mut().insert(
            "Game".into(),
            ProgramData {
                name: "Game".into(),
                items: BTreeMap::from([(
                    "Flask".into(),
                    ProgramItem {
                        name: "Health Flask".into(),
                        tags: vec!["healing".into()],
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        );
        let items = collect_commands(sources(&[], &catalog, &[], false));
        let shown = titles("heal", &items);
        assert!(shown.iter().any(|t| t == "Health Flask"));
    }

    #[test]
    fn prefix_beats_subsequence() {
        let items = vec![
            item("Acyclic", "Other", "?", CommandKind::ShowMacroList, &[]),
            item("Click", "Action", "?", CommandKind::ShowMacroList, &[]),
        ];
        let shown = titles("cli", &items);
        assert_eq!(shown.first().map(String::as_str), Some("Click"));
    }

    #[test]
    fn running_omits_mutating_commands() {
        let macros = vec![Macro::new("demo", 0, vec![])];
        let catalog = ProgramCatalog::default();
        let items = collect_commands(sources(&macros, &catalog, &[], true));
        assert!(!items
            .iter()
            .any(|i| matches!(i.kind, CommandKind::AddAction { .. })));
        assert!(!items
            .iter()
            .any(|i| matches!(i.kind, CommandKind::NewMacro)));
        assert!(!items
            .iter()
            .any(|i| matches!(i.kind, CommandKind::NewCatalogEntity { .. })));
        assert!(items
            .iter()
            .any(|i| matches!(i.kind, CommandKind::OpenDataEditor)));
    }

    #[test]
    fn palette_icon_ids_exist_in_phosphor_catalog() {
        for id in [
            "plus",
            "play",
            "folder",
            "gear",
            "equals",
            "list",
            "image",
            "map-pin",
            "selection",
            "circle-dashed",
            "grid-four",
            "stack",
            "square",
            "arrow-right",
            "mouse",
            "key",
            "keyboard",
            "magnifying-glass",
            "text-aa",
            "drop",
            "floppy-disk",
            "arrows-clockwise",
            "stop",
            "list-bullets",
            "question",
            "timer",
            "pause",
            "app-window",
            "crosshair",
        ] {
            assert_eq!(
                overlay_icons::resolve(id).id,
                id,
                "unknown phosphor id {id}"
            );
        }
    }
}
