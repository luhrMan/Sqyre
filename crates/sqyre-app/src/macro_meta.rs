//! Macro name, delay, and tags toolbar widgets.

use crate::action_tooltip::help;
use eframe::egui::{self, TextureHandle};
use sqyre_domain::Macro;
use std::collections::HashMap;

const DELAY_MIN: i32 = 0;
const DELAY_MAX: i32 = 1000;

#[derive(Debug, Default)]
pub struct MacroMetaUi {
    name_draft: String,
    name_error: Option<String>,
    tag_draft: String,
    delay_open: bool,
    /// Hotkey controls expanded beside the key toggle; kept across macro switches.
    pub(crate) hotkey_open: bool,
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

/// Painted geometry of the name field, tag field, and created-tag row.
pub(crate) struct NameTagLayout {
    pub mutations: MetaMutations,
    /// Read by the header layout test; production only needs [`Self::mutations`].
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "asserted by the header layout test")
    )]
    pub name_rect: egui::Rect,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "asserted by the header layout test")
    )]
    pub tag_rect: egui::Rect,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "asserted by the header layout test")
    )]
    pub chips_rect: Option<egui::Rect>,
}

impl MacroMetaUi {
    pub(crate) fn open_delay_for_docs(&mut self) {
        self.delay_open = true;
    }

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

    /// Name entry row. `edit_limit` caps the field; past that the text scrolls.
    ///
    /// `name_id` stays put when the row wraps, so keyboard focus survives.
    pub fn paint_name_row(
        &mut self,
        ui: &mut egui::Ui,
        m: &mut Macro,
        other_macro_names: &[String],
        enabled: bool,
        edit_limit: f32,
        name_id: egui::Id,
    ) -> (MetaMutations, egui::Rect) {
        let mut out = MetaMutations::default();

        let framed =
            crate::widgets::pair_labeled(ui, egui_phosphor::regular::TEXT_T, "Name", |ui| {
                let name_w = crate::widgets::fields::name_edit_width_limited(
                    ui,
                    &self.name_draft,
                    "Macro name",
                    edit_limit,
                );
                let edit = ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(&mut self.name_draft)
                        .id(name_id)
                        .desired_width(name_w)
                        .hint_text("Macro name"),
                );
                if let Some(err) = &self.name_error {
                    crate::widgets::error_icon(ui, err);
                }
                edit
            });
        let name_rect = framed.response.rect;
        let resp = framed.inner.on_hover_text(help::META_NAME);
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
        (out, name_rect)
    }

    /// Name field and tag field on one row; created tags wrap on the rows below.
    ///
    /// The name hugs its text and pushes the tag field right. When the tag
    /// field reaches the pane edge the name stops growing and the text scrolls
    /// inside the field.
    ///
    /// A focused entry, or a row too narrow to show the name, wraps the two
    /// fields. The focused text then uses the pane width instead of scrolling
    /// inside a sliver.
    pub(crate) fn paint_name_and_tags(
        &mut self,
        ui: &mut egui::Ui,
        m: &mut Macro,
        other_macro_names: &[String],
        all_tags: &[String],
        enabled: bool,
        program_icons: Option<&HashMap<String, TextureHandle>>,
    ) -> NameTagLayout {
        let name_id = ui.make_persistent_id("macro_name_edit");
        let tag_id = ui.make_persistent_id("macro_tag_draft");
        // Focus from the previous frame: a click lands during this paint, so
        // the wrap shows on the following frame.
        let editing = ui
            .ctx()
            .memory(|mem| mem.has_focus(name_id) || mem.has_focus(tag_id));
        ui.vertical(|ui| {
            let gap = ui.spacing().item_spacing.x;
            let avail = ui.available_width();
            let tag_w = tag_input_width(ui);
            let chrome = name_chrome(ui, self.name_error.is_some());
            // One point of slack so rounding cannot push the tag past the clip.
            let budget = (avail - gap - tag_w - 1.0).max(1.0);
            let edit_limit = (budget - chrome).max(1.0);
            let relax = editing || edit_limit < crate::widgets::fields::name_edit_floor(ui);
            let name_limit = if relax {
                (avail - chrome - 1.0).max(1.0)
            } else {
                edit_limit
            };
            let draft_width = relax.then(|| relaxed_tag_width(ui, &self.tag_draft, avail));
            let paint_fields = |ui: &mut egui::Ui| {
                let (meta, name_rect) =
                    self.paint_name_row(ui, m, other_macro_names, enabled, name_limit, name_id);
                let (tag_edit, tag_rect) = crate::widgets::tag_draft_editor(
                    ui,
                    &mut m.tags,
                    &mut self.tag_draft,
                    all_tags,
                    crate::widgets::TagChipOptions {
                        enabled,
                        show_add_button: true,
                        label_name: Some("Tags"),
                        draft_hover: Some(help::META_TAGS),
                        reorderable: false,
                        signed_filters: false,
                        single_row: false,
                        locked: &[],
                        locked_hover: None,
                        program_icons,
                        draft_width,
                        draft_id: Some(tag_id),
                    },
                );
                (meta, name_rect, tag_rect, tag_edit.changed)
            };
            let (mut meta, name_rect, tag_rect, draft_changed) = if relax {
                crate::widgets::wrapped_row(ui, paint_fields).inner
            } else {
                ui.horizontal(paint_fields).inner
            };
            let focused_now = ui
                .ctx()
                .memory(|mem| mem.has_focus(name_id) || mem.has_focus(tag_id));
            if focused_now != editing {
                ui.ctx().request_repaint();
            }
            let (chips_changed, chips_rect) = if m.tags.is_empty() {
                (false, None)
            } else {
                let (changed, rect) = crate::widgets::tag_chip_list(
                    ui,
                    &mut m.tags,
                    enabled,
                    false,
                    false,
                    program_icons,
                );
                (changed, Some(rect))
            };
            if draft_changed || chips_changed {
                meta.persist = true;
            }
            NameTagLayout {
                mutations: meta,
                name_rect,
                tag_rect,
                chips_rect,
            }
        })
        .inner
    }

    /// Icon button toggling the delay popup ([`Self::paint_delay_popup`]).
    pub fn paint_delay_button(&mut self, ui: &mut egui::Ui, m: &Macro, enabled: bool) {
        let tip = format_delay_tooltip(m);
        if crate::ui_toolbar::toolbar_icon(ui, egui_phosphor::regular::TIMER, &tip, enabled)
            .clicked()
        {
            self.delay_open = !self.delay_open;
        }
    }

    /// Key icon toggle; returns whether the hotkey controls should be shown beside it.
    pub fn paint_hotkey_toggle(&mut self, ui: &mut egui::Ui) -> bool {
        crate::widgets::icon_toggle(
            ui,
            &mut self.hotkey_open,
            "Hotkey",
            egui_phosphor::regular::KEY,
            egui_phosphor::fill::KEY,
        );
        self.hotkey_open
    }

    pub fn paint_delay_popup(
        &mut self,
        ui: &mut egui::Ui,
        m: &mut Macro,
        enabled: bool,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
    ) -> MetaMutations {
        let mut out = MetaMutations::default();
        if !self.delay_open {
            return out;
        }

        let mut close = false;
        // Avoid `.auto_sized()`: egui paints the title Frame at title+close width
        // without allocating it, so the header stroke outruns the body.
        crate::widgets::fit_dialog_popup(
            egui::Window::new("Delay between actions")
                .collapsible(false)
                .resizable(false)
                .open(&mut self.delay_open),
            ui.ctx(),
            egui::Id::new("Delay between actions"),
            pending_scale,
        )
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
            });
            if crate::widgets::dismiss_row(ui, true) {
                close = true;
            }
        });
        if close {
            self.delay_open = false;
        }

        out
    }
}

fn delay_row(ui: &mut egui::Ui, label: &str, value: &mut i32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let resp = ui.add(
            crate::widgets::num_entry(value)
                .range(DELAY_MIN..=DELAY_MAX)
                .speed(1),
        );
        changed = resp.changed();
    });
    changed
}

fn format_delay_tooltip(m: &Macro) -> String {
    let mut parts = vec!["Action delays (ms)".to_string()];
    if m.global_delay > 0 {
        parts.push(format!("Global: {} ms", m.global_delay));
    }
    if m.keyboard_delay > 0 {
        parts.push(format!("Keyboard: {} ms", m.keyboard_delay));
    }
    if m.mouse_delay > 0 {
        parts.push(format!("Mouse: {} ms", m.mouse_delay));
    }
    parts.join("\n")
}

fn glyph_width(ui: &egui::Ui, glyph: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(
            glyph.to_owned(),
            egui::TextStyle::Body.resolve(ui.style()),
            egui::Color32::WHITE,
        )
        .size()
        .x
}

/// Width of a label+widget pair aside from the widget: frame margin, stroke, icon, gap.
fn pair_chrome(ui: &egui::Ui, glyph: &str) -> f32 {
    glyph_width(ui, glyph) + 8.0 + 2.0 + crate::widgets::PAIR_SLOT_GAP
}

fn name_chrome(ui: &egui::Ui, show_error: bool) -> f32 {
    let mut chrome = pair_chrome(ui, egui_phosphor::regular::TEXT_T);
    if show_error {
        chrome += glyph_width(ui, egui_phosphor::regular::X_CIRCLE);
    }
    chrome
}

/// Width of the tag field (icon, draft, Add) so the name can stop before it.
fn tag_input_width(ui: &egui::Ui) -> f32 {
    pair_chrome(ui, egui_phosphor::regular::TAG)
        + crate::widgets::tags::TAG_DRAFT_WIDTH
        + crate::widgets::icon_btn_side(ui)
}

/// Draft width while the name and tag wrap: at least the resting field, wide
/// enough for `draft`, and never wider than the pane.
fn relaxed_tag_width(ui: &egui::Ui, draft: &str, pane: f32) -> f32 {
    let mut wanted = crate::widgets::tags::TAG_DRAFT_WIDTH;
    if !draft.trim().is_empty() {
        let text_w = ui
            .painter()
            .layout_no_wrap(
                draft.to_owned(),
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::WHITE,
            )
            .size()
            .x;
        wanted = wanted.max(text_w + 18.0);
    }
    let max = (pane
        - pair_chrome(ui, egui_phosphor::regular::TAG)
        - crate::widgets::icon_btn_side(ui)
        - ui.spacing().item_spacing.x
        - 4.0)
        .max(1.0);
    wanted.min(max)
}

fn validate_rename(
    new_name: &str,
    current: &str,
    other_macro_names: &[String],
) -> Result<(), String> {
    sqyre_validate::validate_entity_name(new_name).map_err(|e| e.to_string())?;
    if new_name == current {
        return Ok(());
    }
    if other_macro_names.iter().any(|n| n == new_name) {
        return Err("macro name already exists".into());
    }
    Ok(())
}

/// Case-insensitive display-name ordering; original-string tie-break for stability.
pub(crate) fn cmp_display_name(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_ascii_lowercase()
        .cmp(&b.to_ascii_lowercase())
        .then_with(|| a.cmp(b))
}

/// Sorted unique tags across macros (for completion).
pub fn collect_all_macro_tags(macros: &[Macro]) -> Vec<String> {
    unique_sorted(macros.iter().flat_map(|m| m.tags.iter().cloned()).collect())
}

/// Macro tags plus each program's name tag, for completion.
///
/// The recording scratch program is omitted; it is not a catalog the user tags against.
pub fn collect_macro_tag_completions(
    macros: &[Macro],
    program_names: impl IntoIterator<Item = impl AsRef<str>>,
) -> Vec<String> {
    let mut tags = collect_all_macro_tags(macros);
    for name in program_names {
        let name = name.as_ref().trim();
        if name.is_empty() || name == sqyre_domain::TEMPORARY_PROGRAM {
            continue;
        }
        if !tags.iter().any(|tag| tag == name) {
            tags.push(name.to_string());
        }
    }
    unique_sorted(tags)
}

pub(crate) fn unique_sorted(mut items: Vec<String>) -> Vec<String> {
    items.sort_by(|a, b| cmp_display_name(a, b));
    items.dedup();
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::tags::{remove_tag, tag_completion_options, try_add_tag};
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
        assert!(validate_rename("bad:name", "a", &others).is_err());
    }

    #[test]
    fn completions_include_program_names_except_the_scratch_program() {
        let tags = collect_macro_tag_completions(
            &[m("fight", &["loot"])],
            ["Game", "temporary", "  ", "Game"],
        );
        assert_eq!(tags, vec!["Game".to_string(), "loot".to_string()]);
    }

    #[test]
    fn tag_add_remove_and_completion() {
        let mut tags = vec!["alpha".to_string()];
        assert!(!try_add_tag(&mut tags, "  "));
        assert!(!try_add_tag(&mut tags, "alpha"));
        assert!(try_add_tag(&mut tags, "beta"));
        assert_eq!(tags, vec!["alpha", "beta"]);
        assert!(remove_tag(&mut tags, "alpha"));
        assert_eq!(tags, vec!["beta"]);

        let all_tags =
            collect_all_macro_tags(&[m("x", &["beta"]), m("y", &["beta", "gamma", "gator"])]);
        let opts = tag_completion_options("ga", &["beta".into()], &all_tags);
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
        assert_eq!(
            format_delay_tooltip(&macro_),
            "Action delays (ms)\nGlobal: 10 ms\nMouse: 5 ms"
        );
    }

    /// Layout plus the widest name galley that contains `name_needle`, if any.
    fn paint_header(
        width: f32,
        name: &str,
        tags: &[&str],
        name_needle: &str,
    ) -> (NameTagLayout, f32, Option<(String, f32)>) {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width + 80.0, 280.0));
        let mut layout = None;
        let mut pane_right = 0.0;
        let all_tags: Vec<String> = tags.iter().map(|s| (*s).to_string()).collect();
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| {
                let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(width, 220.0));
                pane_right = pane.right();
                let mut mid = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(pane)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                mid.set_clip_rect(pane);
                let mut meta = MacroMetaUi::default();
                let mut macro_ = m(name, tags);
                meta.sync_selection(0, &macro_);
                layout = Some(meta.paint_name_and_tags(
                    &mut mid,
                    &mut macro_,
                    &[],
                    &all_tags,
                    true,
                    None,
                ));
            },
        );
        let galley = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.job.text.contains(name_needle) => {
                    Some((text.galley.job.text.clone(), text.galley.size().x))
                }
                _ => None,
            });
        output.drop_without_applying_deltas();
        (layout.expect("header painted"), pane_right, galley)
    }

    #[test]
    fn name_and_tag_fields_share_a_baseline() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 280.0));
        let mut meta = MacroMetaUi::default();
        let mut macro_ = m("new macro copy", &[]);
        meta.sync_selection(0, &macro_);
        for pass in 0..2 {
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    let layout = meta.paint_name_and_tags(ui, &mut macro_, &[], &[], true, None);
                    assert!(
                        (layout.name_rect.top() - layout.tag_rect.top()).abs() < 1.0,
                        "pass {pass} tops: name {:?} tag {:?}",
                        layout.name_rect,
                        layout.tag_rect
                    );
                    assert!(
                        (layout.name_rect.bottom() - layout.tag_rect.bottom()).abs() < 1.0,
                        "pass {pass} bottoms: name {:?} tag {:?}",
                        layout.name_rect,
                        layout.tag_rect
                    );
                },
            );
            output.drop_without_applying_deltas();
        }
    }

    #[test]
    fn name_pushes_tags_until_the_edge_then_scrolls() {
        let pane_w = 480.0;
        let (short, pane_right, _) = paint_header(pane_w, "Alpha", &["demo", "desktop"], "Alpha");
        let long_name = "Inventory restock and backup ".repeat(8);
        let (long, long_right, long_galley) = paint_header(
            pane_w,
            &long_name,
            &["demo", "desktop"],
            "Inventory restock",
        );
        assert!((pane_right - long_right).abs() < 0.5);

        let short_gap = short.tag_rect.left() - short.name_rect.right();
        assert!(
            (-1.0..24.0).contains(&short_gap),
            "tag field should sit against the name, gap {short_gap}, name {:?} tag {:?}",
            short.name_rect,
            short.tag_rect
        );
        assert!(
            short.tag_rect.right() < pane_right - 40.0,
            "short name pushed the tag field to the pane edge: {:?} pane_right {pane_right}",
            short.tag_rect
        );

        assert!(
            long.tag_rect.left() + 1.0 >= long.name_rect.right(),
            "tag overlapped the name: name {:?} tag {:?}",
            long.name_rect,
            long.tag_rect
        );
        assert!(
            long.tag_rect.right() <= pane_right + 1.0,
            "tag field past the pane: {:?} pane_right {pane_right}",
            long.tag_rect
        );
        assert!(
            long.tag_rect.right() > pane_right - 16.0,
            "long name did not push the tag field to the limit: {:?} pane_right {pane_right}",
            long.tag_rect
        );
        assert!(
            long.name_rect.width() > short.name_rect.width() + 8.0,
            "name did not grow before the tag field hit the edge"
        );
        assert!(
            long.tag_rect.left() > short.tag_rect.left() + 8.0,
            "tag field was not pushed right"
        );

        let (galley_text, galley_w) = long_galley.expect("name galley");
        assert!(
            !galley_text.contains('…'),
            "name was elided instead of scrolling: {galley_text}"
        );
        assert!(
            long.name_rect.width() < galley_w,
            "name field grew to the full text ({galley_w}) instead of scrolling: {:?}",
            long.name_rect
        );

        for (label, layout) in [("short", &short), ("long", &long)] {
            let chips = layout
                .chips_rect
                .unwrap_or_else(|| panic!("{label} header painted no tag chips"));
            let row_bottom = layout.name_rect.bottom().max(layout.tag_rect.bottom());
            assert!(
                chips.top() + 1.0 >= row_bottom,
                "{label} chips are not below the name/tag row: chips {chips:?} row_bottom {row_bottom}"
            );
            assert!(
                chips.left() <= layout.name_rect.left() + 2.0,
                "{label} chips did not start under the name row"
            );
        }

        let (bare, _, _) = paint_header(pane_w, "Alpha", &[], "Alpha");
        assert!(bare.chips_rect.is_none(), "empty tags reserved a chip row");
    }

    #[test]
    fn narrow_row_wraps_so_the_name_stays_visible() {
        let (layout, pane_right, galley) = paint_header(240.0, "Alpha", &["demo"], "Alpha");
        let (text, _) = galley.expect("name galley");
        assert!(!text.contains('…'), "name was elided: {text}");
        assert!(
            layout.name_rect.width() > 40.0,
            "name collapsed: {:?}",
            layout.name_rect
        );
        assert!(
            layout.tag_rect.top() + 1.0 >= layout.name_rect.bottom(),
            "tag stayed beside a crushed name: name {:?} tag {:?}",
            layout.name_rect,
            layout.tag_rect
        );
        assert!(
            layout.tag_rect.right() <= pane_right + 1.0,
            "tag past the pane: {:?} pane_right {pane_right}",
            layout.tag_rect
        );
        let chips = layout.chips_rect.expect("chips");
        assert!(chips.top() + 1.0 >= layout.tag_rect.bottom());
    }

    fn run_header(
        ctx: &egui::Context,
        width: f32,
        frame: f64,
        events: Vec<egui::Event>,
        meta: &mut MacroMetaUi,
        macro_: &mut Macro,
    ) -> (NameTagLayout, egui::FullOutput) {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width + 80.0, 400.0));
        let mut layout = None;
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(frame),
                events,
                ..Default::default()
            },
            |ui| {
                let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(width, 320.0));
                let mut mid = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(pane)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                mid.set_clip_rect(pane);
                layout = Some(meta.paint_name_and_tags(&mut mid, macro_, &[], &[], true, None));
            },
        );
        (layout.expect("header painted"), output)
    }

    fn click(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    fn visible_text_width(output: &egui::FullOutput, needle: &str) -> f32 {
        output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text().contains(needle) => {
                    let galley = egui::Rect::from_min_size(text.pos, text.galley.size());
                    Some(galley.intersect(clipped.clip_rect).width())
                }
                _ => None,
            })
            .unwrap_or(0.0)
    }

    #[test]
    fn focused_name_wraps_so_its_text_is_visible() {
        let ctx = egui::Context::default();
        let mut meta = MacroMetaUi::default();
        let long = "Inventory restock and backup ".repeat(6);
        let mut macro_ = m(&long, &["demo"]);
        meta.sync_selection(0, &macro_);
        let width = 480.0;
        let mut before = None;
        let mut before_visible = 0.0;
        for frame in 0..2 {
            let (laid, out) = run_header(
                &ctx,
                width,
                frame as f64 * 0.1,
                vec![],
                &mut meta,
                &mut macro_,
            );
            before_visible = visible_text_width(&out, "Inventory restock");
            before = Some(laid);
            out.drop_without_applying_deltas();
        }
        let before = before.expect("settled");
        assert!(
            (before.name_rect.top() - before.tag_rect.top()).abs() < 1.0,
            "unfocused fields left the shared row: name {:?} tag {:?}",
            before.name_rect,
            before.tag_rect
        );
        let pos = egui::pos2(before.name_rect.right() - 12.0, before.name_rect.center().y);
        let (_, press) = run_header(&ctx, width, 0.3, click(pos, true), &mut meta, &mut macro_);
        press.drop_without_applying_deltas();
        let (_, release) = run_header(&ctx, width, 0.4, click(pos, false), &mut meta, &mut macro_);
        release.drop_without_applying_deltas();
        let (after, out) = run_header(&ctx, width, 0.5, vec![], &mut meta, &mut macro_);
        let after_visible = visible_text_width(&out, "Inventory restock");
        out.drop_without_applying_deltas();
        assert!(
            after.tag_rect.top() + 1.0 >= after.name_rect.bottom(),
            "focused name did not wrap the tag under it: name {:?} tag {:?}",
            after.name_rect,
            after.tag_rect
        );
        assert!(
            after_visible > before_visible + 40.0,
            "focused name still hides its text: before {before_visible} after {after_visible}"
        );
    }

    #[test]
    fn focused_tag_wraps_so_its_draft_is_visible() {
        let ctx = egui::Context::default();
        let mut meta = MacroMetaUi::default();
        let mut macro_ = m("Alpha", &["demo"]);
        meta.sync_selection(0, &macro_);
        meta.tag_draft = "evening-backup-route-for-the-northern-camp".into();
        let width = 480.0;
        let mut before = None;
        let mut before_visible = 0.0;
        for frame in 0..2 {
            let (laid, out) = run_header(
                &ctx,
                width,
                frame as f64 * 0.1,
                vec![],
                &mut meta,
                &mut macro_,
            );
            before_visible = visible_text_width(&out, "evening-backup");
            before = Some(laid);
            out.drop_without_applying_deltas();
        }
        let before = before.expect("settled");
        assert!(
            (before.name_rect.top() - before.tag_rect.top()).abs() < 1.0,
            "short name should keep the tag beside it until focus: name {:?} tag {:?}",
            before.name_rect,
            before.tag_rect
        );
        let pos = egui::pos2(before.tag_rect.left() + 48.0, before.tag_rect.center().y);
        let (_, press) = run_header(&ctx, width, 0.3, click(pos, true), &mut meta, &mut macro_);
        press.drop_without_applying_deltas();
        let (_, release) = run_header(&ctx, width, 0.4, click(pos, false), &mut meta, &mut macro_);
        release.drop_without_applying_deltas();
        let (after, out) = run_header(&ctx, width, 0.5, vec![], &mut meta, &mut macro_);
        let after_visible = visible_text_width(&out, "evening-backup");
        out.drop_without_applying_deltas();
        assert!(
            after.tag_rect.top() + 1.0 >= after.name_rect.bottom()
                || after.tag_rect.width() > before.tag_rect.width() + 40.0,
            "focused tag neither wrapped nor grew: before {:?} after {:?}",
            before.tag_rect,
            after.tag_rect
        );
        assert!(
            after_visible > before_visible + 40.0,
            "focused tag still hides its draft: before {before_visible} after {after_visible}"
        );
    }
}
