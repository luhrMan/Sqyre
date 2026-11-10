//! Labeled form field helpers shared by action edit, variables, and data editor.

use crate::action_tooltip::help;
use crate::help_text as ht;
use crate::pickers::fuzzy_match_fold;
use crate::widgets::elide::{combo_text_elided, text_hover_if};
use crate::widgets::num_entry::{num_entry, NumEntry};
use eframe::egui;
use std::hash::Hash;
use std::ops::RangeInclusive;

/// Standard single-line text edit width.
pub const W_TEXT: f32 = 220.0;
/// Standard variable / scalar ref edit width.
pub const W_VAR: f32 = 160.0;

/// Focus the search field once when a picker/dialog opens (`id` should be stable per window).
pub fn focus_search_once(ui: &mut egui::Ui, id: egui::Id, resp: &egui::Response) {
    let key = id.with("focus_search_once");
    let needs = ui.ctx().data_mut(|d| *d.get_temp_mut_or(key, true));
    if needs {
        resp.request_focus();
        ui.ctx().data_mut(|d| d.insert_temp(key, false));
    }
}

/// Clear one-shot focus so the next open focuses again.
pub fn reset_focus_search(ctx: &egui::Context, id: egui::Id) {
    ctx.data_mut(|d| d.insert_temp(id.with("focus_search_once"), true));
}

fn selected_text_hover(
    ui: &egui::Ui,
    response: egui::Response,
    selected: &str,
    help_text: &str,
) -> egui::Response {
    if combo_text_elided(ui, &response, selected) {
        return if help_text.is_empty() {
            text_hover_if(response, selected, true)
        } else {
            response.on_hover_text(format!("{selected}\n\n{help_text}"))
        };
    }
    help::tip(response, help_text)
}

pub fn text_field(ui: &mut egui::Ui, icon: &str, label: &str, help_text: &str, value: &mut String) {
    text_field_opts(ui, icon, label, help_text, value, TextFieldOpts::default());
}

/// Options for [`text_field_opts`].
#[derive(Clone, Copy)]
pub struct TextFieldOpts {
    /// `desired_width` for the edit; default is [`W_TEXT`].
    pub width: f32,
}

impl Default for TextFieldOpts {
    fn default() -> Self {
        Self { width: W_TEXT }
    }
}

/// [`text_field`] with explicit options.
pub fn text_field_opts(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut String,
    opts: TextFieldOpts,
) {
    super::pair_labeled(ui, icon, label, |ui| {
        let width = if opts.width.is_finite() {
            opts.width
        } else {
            finite_room(ui).max(24.0)
        };
        help::tip(
            ui.add(egui::TextEdit::singleline(value).desired_width(width)),
            help_text,
        );
    });
}

/// Width for a content-sized Name edit: hugs the text (or `hint` while empty),
/// grows as the text fills, and stops at the row's visible width — the
/// singleline scrolls inside the box past that.
pub fn name_edit_width(ui: &egui::Ui, text: &str, hint: &str) -> f32 {
    name_edit_width_limited(ui, text, hint, f32::INFINITY)
}

/// Narrowest name entry that still shows a few characters (`interact_size.y * 4`).
pub(crate) fn name_edit_floor(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y * 4.0
}

/// [`name_edit_width`] that also stops at `limit`.
///
/// Below the usual floor the field still shrinks so a neighbor can stay on
/// the row; past the cap the singleline scrolls inside the box.
pub(crate) fn name_edit_width_limited(ui: &egui::Ui, text: &str, hint: &str, limit: f32) -> f32 {
    let shown = if text.trim().is_empty() { hint } else { text };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let text_w = ui
        .painter()
        .layout_no_wrap(shown.to_owned(), font, egui::Color32::WHITE)
        .size()
        .x;
    // TextEdit margin (8) + frame stroke (~2) + caret/selection slop.
    let w = text_w + 18.0;
    let min = name_edit_floor(ui);
    let visible = super::visible_width(ui);
    let room = if limit.is_finite() {
        visible.min(limit).max(1.0)
    } else {
        visible.max(min)
    };
    let floor = if limit.is_finite() {
        min.min(room)
    } else {
        min
    };
    w.clamp(floor, room)
}

/// Options for [`num_text_field_opts`].
#[derive(Clone)]
pub struct NumFieldOpts {
    /// `desired_width` for the edit; default is [`W_TEXT`].
    pub width: f32,
    /// Accepted integer range (the steppers clamp to it).
    pub range: RangeInclusive<i32>,
}

/// Labeled integer-capable text field (see [`num_text_edit`]).
pub fn num_text_field_opts(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut String,
    opts: NumFieldOpts,
) {
    super::pair_labeled(ui, icon, label, |ui| {
        help::tip(
            crate::widgets::num_text_edit(ui, value, opts.width, opts.range.clone()),
            help_text,
        );
    });
}

/// [`egui::TextEdit`] with the S-stroke stepper column reserved on its right
/// edge — the same steppers [`num_entry`] fields get. Steppers appear only
/// while `value` parses as a pure integer; steps clamp to `range`.
pub fn num_text_edit(
    ui: &mut egui::Ui,
    value: &mut String,
    width: f32,
    range: RangeInclusive<i32>,
) -> egui::Response {
    let resp = ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(width)
            .margin(crate::widgets::num_entry::TEXT_STEPPER_MARGIN),
    );
    crate::widgets::num_entry::string_steppers(ui, resp.id, resp.rect, value, range);
    resp
}

/// One row whose `fill` content (e.g. a `desired_width(f32::INFINITY)` field)
/// takes only the width left after `trailing`.
///
/// A fill-width widget followed by siblings in a plain `horizontal` pushes them
/// past the pane edge; egui then widens the parent region, so right-aligned
/// footers and later full-width content stay off-screen at any window size.
/// `trailing` is laid out right-to-left: the first widget added is rightmost.
pub fn fill_row<R>(
    ui: &mut egui::Ui,
    trailing: impl FnOnce(&mut egui::Ui),
    fill: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.horizontal(|ui| {
        // A reserved floating-scrollbar lane shrinks the scroll clip but not
        // max_rect — bound the row to the clip so `trailing` cannot land in it.
        super::clamp_row_right_to_clip(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            trailing(ui);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), fill)
                .inner
        })
        .inner
    })
    .inner
}

/// Options for [`search_field`].
pub struct SearchFieldOpts<'a> {
    /// Placeholder inside the field; empty for none.
    pub hint: &'a str,
    /// Focus the field once when this surface opens
    /// ([`focus_search_once`] keyed off this window/picker id).
    pub focus_id: Option<egui::Id>,
    /// Trailing red eraser that empties the field while it has text.
    pub clearable: bool,
    /// Pinned field id — needed when a sibling drawn before the field (e.g.
    /// the toggling clear eraser) would shift the auto id and drop keyboard focus.
    pub stable_id: Option<egui::Id>,
    /// Icon button inside the pair, after the clear eraser.
    pub trailing: Option<SearchTrailing<'a>>,
}

/// Trailing icon drawn inside a [`search_field`] pair.
pub struct SearchTrailing<'a> {
    pub glyph: &'a str,
    pub tip: &'a str,
    pub color: Option<egui::Color32>,
}

/// Outcome of [`search_field`].
pub struct SearchFieldResp {
    /// The field's response — the magnifier's when the field drops for width.
    /// `has_focus` / `request_focus` / `surrender_focus` apply to it.
    pub response: egui::Response,
    /// Query text changed this frame (edit or eraser clear — `value` is already updated).
    pub changed: bool,
    /// The [`SearchFieldOpts::trailing`] icon was clicked this frame.
    pub trailing_clicked: bool,
}

/// Magnifier + fill-width filter field (+ optional trailing clear eraser) — the
/// one search row for lists, pickers, dialogs, and panels.
///
/// The magnifier, the edit, the eraser, and any [`SearchTrailing`] share one
/// [`super::pair_icon`] frame. Wrap the row in [`super::show_overflow`] or a
/// [`fill_row`] slot when siblings share the row.
pub fn search_field(
    ui: &mut egui::Ui,
    value: &mut String,
    opts: SearchFieldOpts<'_>,
) -> SearchFieldResp {
    let SearchFieldOpts {
        hint,
        focus_id,
        clearable,
        stable_id,
        trailing,
    } = opts;
    // `body` holds the `&mut String`, so snapshot emptiness before it is built.
    let has_text = !value.is_empty();
    let trailing = trailing.map(|action| (action.glyph, action.tip, action.color));
    // egui never allocates a TextEdit narrower than 24px (`MIN_WIDTH`), so
    // below that the field's rect underlaps what follows it. A sliver-wide
    // field is unusable anyway — drop it and keep the icon.
    if ui.available_width() < 24.0 {
        let response = search_icon(ui);
        let trailing_clicked = trailing.is_some_and(|(glyph, tip, color)| {
            ui.add(super::IconButton::new(glyph, tip, color)).clicked()
        });
        return SearchFieldResp {
            changed: false,
            response,
            trailing_clicked,
        };
    }
    let show_clear = clearable && has_text;
    let has_trailing = trailing.is_some();
    let slots = u32::from(show_clear) + u32::from(has_trailing);
    let mut cleared = false;
    let mut trailing_clicked = false;
    let response = if slots == 0 {
        super::pair_leading(
            ui,
            |ui| {
                search_icon(ui);
            },
            |ui| search_edit(ui, value, hint, stable_id, focus_id, 0),
        )
        .inner
    } else {
        let mut clear_slot = |ui: &mut egui::Ui| {
            cleared = crate::widgets::clear_icon_button(ui, "Clear search").clicked();
        };
        let mut extra_slot = |ui: &mut egui::Ui| {
            if let Some((glyph, tip, color)) = trailing {
                trailing_clicked = ui.add(super::IconButton::new(glyph, tip, color)).clicked();
            }
        };
        let mut slot_refs: Vec<&mut dyn FnMut(&mut egui::Ui)> = Vec::with_capacity(2);
        if show_clear {
            slot_refs.push(&mut clear_slot);
        }
        if has_trailing {
            slot_refs.push(&mut extra_slot);
        }
        super::pair_slots(
            ui,
            |ui| {
                search_icon(ui);
            },
            |ui| search_edit(ui, value, hint, stable_id, focus_id, slots),
            &mut slot_refs,
        )
        .inner
    };
    if cleared {
        value.clear();
    }
    SearchFieldResp {
        changed: cleared || response.changed(),
        response,
        trailing_clicked,
    }
}

/// Remaining width that is actually finite: layout budget, else the clip.
pub(crate) fn finite_room(ui: &egui::Ui) -> f32 {
    let avail = ui.available_width();
    let clip = (ui.clip_rect().right() - ui.cursor().left()).max(0.0);
    if avail.is_finite() && clip.is_finite() {
        avail.min(clip)
    } else if avail.is_finite() {
        avail
    } else if clip.is_finite() && clip > 1.0 {
        clip
    } else {
        240.0
    }
}

fn search_icon(ui: &mut egui::Ui) -> egui::Response {
    let icon = ui
        .label(egui::RichText::new(
            egui_phosphor::regular::MAGNIFYING_GLASS,
        ))
        .on_hover_text(ht::FIELD_SEARCH);
    icon.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Search".to_owned())
    });
    icon
}

fn search_edit(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    stable_id: Option<egui::Id>,
    focus_id: Option<egui::Id>,
    trailing_slots: u32,
) -> egui::Response {
    // A finite width, never `INFINITY`. Inside an overflow scroller
    // `available_width()` is infinite, and an infinite edit makes the row
    // wider than the pane so the scroller turns on and off every frame.
    // `stable_id` keeps focus when the trailing eraser appears and disappears.
    // Each trailing slot (eraser, extra icon) keeps its own gap so it stays
    // inside the pair instead of past the pane edge.
    let room = finite_room(ui);
    let slot = crate::widgets::icon_btn_side(ui) + super::PAIR_SLOT_GAP;
    let width = (room - trailing_slots as f32 * slot).max(24.0);
    let mut edit = egui::TextEdit::singleline(value).desired_width(width);
    if let Some(id) = stable_id {
        edit = edit.id(id);
    }
    if !hint.is_empty() {
        edit = edit.hint_text(hint);
    }
    let resp = ui.add(edit);
    if let Some(fid) = focus_id {
        focus_search_once(ui, fid, &resp);
    }
    resp
}

/// Labeled numeric entry (no `.prefix`); configure speed/range/etc via `configure`.
pub fn drag_field<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut Num,
    configure: impl Fn(NumEntry<'_, Num>) -> NumEntry<'_, Num>,
) {
    drag_field_enabled(ui, icon, label, help_text, value, true, configure);
}

pub fn drag_field_enabled<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut Num,
    enabled: bool,
    configure: impl Fn(NumEntry<'_, Num>) -> NumEntry<'_, Num>,
) {
    super::pair_labeled(ui, icon, label, |ui| {
        help::tip(
            ui.add_enabled(enabled, configure(num_entry(value))),
            help_text,
        );
    });
}

pub fn combo_str(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut String,
    options: &[&str],
) {
    super::pair_labeled(ui, icon, label, |ui| {
        let display = if value.is_empty() {
            sqyre_domain::EMPTY_UNSET.to_string()
        } else {
            value.clone()
        };
        let mut custom = None;
        if !options.contains(&value.as_str()) && !value.is_empty() {
            custom = Some(value.clone());
        }
        let response = egui::ComboBox::from_id_salt(label)
            .selected_text(display.clone())
            .truncate()
            .show_ui(ui, |ui| {
                for opt in options {
                    let text = if opt.is_empty() {
                        sqyre_domain::EMPTY_UNSET
                    } else {
                        opt
                    };
                    ui.selectable_value(value, (*opt).to_string(), text);
                }
                if let Some(c) = custom {
                    ui.selectable_value(value, c.clone(), c);
                }
            })
            .response;
        selected_text_hover(ui, response, &display, help_text);
    });
}

pub fn combo_condition_operator(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut sqyre_domain::ConditionOperator,
) {
    use sqyre_domain::ConditionOperator;

    combo_enum(
        ui,
        icon,
        label,
        help_text,
        value,
        ConditionOperator::ALL,
        ConditionOperator::as_str,
    );
}

/// ComboBox over a closed set of copyable enum values.
pub fn combo_enum<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut T,
    options: &[T],
    display: impl Fn(T) -> &'static str,
) {
    super::pair_labeled(ui, icon, label, |ui| {
        let selected = display(*value);
        let response = egui::ComboBox::from_id_salt(label)
            .selected_text(selected)
            .truncate()
            .show_ui(ui, |ui| {
                for opt in options {
                    ui.selectable_value(value, *opt, display(*opt));
                }
            })
            .response;
        selected_text_hover(ui, response, selected, help_text);
    });
}

/// Like [`combo_str`], but each option is `(stored value, display label)`.
///
/// When `value` is empty, the closed button and open-list highlight use
/// `empty_default` / its label without writing into `value` until the user picks.
pub fn combo_str_labeled(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut String,
    options: &[(&str, &str)],
    empty_default: &str,
) {
    super::pair_labeled(ui, icon, label, |ui| {
        let empty_label = options
            .iter()
            .find(|(v, _)| *v == empty_default)
            .map(|(_, l)| *l)
            .unwrap_or(empty_default);
        let display = if value.is_empty() {
            empty_label.to_string()
        } else {
            options
                .iter()
                .find(|(v, _)| *v == value.as_str())
                .map(|(_, l)| (*l).to_string())
                .unwrap_or_else(|| value.clone())
        };
        let current = if value.is_empty() {
            empty_default.to_string()
        } else {
            value.clone()
        };
        let mut custom = None;
        if !value.is_empty() && !options.iter().any(|(v, _)| *v == value.as_str()) {
            custom = Some(value.clone());
        }
        let response = egui::ComboBox::from_id_salt(label)
            .selected_text(display.clone())
            .truncate()
            .show_ui(ui, |ui| {
                for &(stored, shown) in options {
                    if ui.selectable_label(current == stored, shown).clicked() {
                        *value = stored.to_string();
                    }
                }
                if let Some(c) = custom {
                    ui.selectable_value(value, c.clone(), c);
                }
            })
            .response;
        selected_text_hover(ui, response, &display, help_text);
    });
}

/// Callback for option (and closed-button) hover in [`searchable_combo`].
pub type ComboOptionHover<'a> = dyn FnMut(&mut egui::Ui, &egui::Response, &str) + 'a;

/// Leading icon for combo options / closed selection in [`searchable_combo`].
pub type ComboOptionIcon<'a> =
    dyn FnMut(&egui::Context, &str) -> Option<eframe::egui::TextureHandle> + 'a;

/// Display-only label transform for combo options and the closed button in
/// [`searchable_combo`] — e.g. appending a count ("Demo Program (3)").
/// Fuzzy matching and the stored `value` still use the raw option string.
pub type ComboOptionLabel<'a> = dyn FnMut(&str) -> String + 'a;

/// Popup list height for searchable combos (egui default [`Spacing::combo_height`] is only 200).
///
/// Sized from the window so long catalogs (programs, masks, macros) stay easy to browse.
/// Does not use [`Ui::available_height`] — that is usually one form row when the combo is closed.
fn searchable_combo_popup_height(ctx: &egui::Context) -> f32 {
    const MIN: f32 = 320.0;
    (ctx.content_rect().height() * 0.55).max(MIN)
}

/// Options for [`searchable_combo`].
pub struct SearchableComboOpts<'a> {
    /// Options shown in the popup list.
    pub options: &'a [String],
    /// Closed-button label while `value` is empty.
    pub empty_text: &'a str,
    /// `Some` offers an empty-string option with that label.
    pub none_label: Option<&'a str>,
    /// Fixed button/popup width; `None` = content/row width.
    pub width: Option<f32>,
    /// Per-option hover callback (also on the closed button when `value` is
    /// non-empty). Use for image/preview tooltips while browsing the popup.
    pub on_option_hover: Option<&'a mut ComboOptionHover<'a>>,
    /// Leading icon drawn on each option row and, when `closed_icon` is set,
    /// beside the closed button.
    pub option_icon: Option<&'a mut ComboOptionIcon<'a>>,
    /// Paint the selected option's icon beside the closed button too. Turn
    /// off when an adjacent control already shows the selection's icon.
    pub closed_icon: bool,
    /// Display label transform (counts, decorations); `None` shows the raw option.
    pub option_label: Option<&'a mut ComboOptionLabel<'a>>,
}

/// Labeled [`searchable_combo`]: icon pair, hover tip, optional trailing control.
///
/// Writes `value` in place. Returns whether the selection changed.
/// `trailing` is the add button on atlas membership; omit when unused.
///
/// [`super::wrap_unit`] may probe the pair body before painting it, so `opts`
/// is rebuilt each call (callbacks re-borrowed) and `id_salt` must be `Copy`.
#[expect(
    clippy::too_many_arguments,
    reason = "label chrome + SearchableComboOpts + optional trailing slot"
)]
pub fn labeled_searchable_combo(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    id_salt: impl Hash + std::fmt::Debug + Copy,
    value: &mut String,
    opts: SearchableComboOpts<'_>,
    mut trailing: Option<&mut dyn FnMut(&mut egui::Ui)>,
) -> bool {
    let before = value.clone();
    let mut opts = Some(opts);
    super::pair_labeled(ui, icon, label, |ui| {
        // wrap_unit probes with an invisible child before the real paint. Keep
        // callback slots for the visible pass; the probe only needs sizing.
        let pass = if !ui.is_visible() {
            let o = opts.as_ref().expect("labeled_searchable_combo opts");
            SearchableComboOpts {
                options: o.options,
                empty_text: o.empty_text,
                none_label: o.none_label,
                width: o.width,
                on_option_hover: None,
                option_icon: None,
                closed_icon: o.closed_icon,
                option_label: None,
            }
        } else {
            opts.take().expect("labeled_searchable_combo visible pass")
        };
        help::tip(searchable_combo(ui, id_salt, value, pass), help_text);
        if let Some(slot) = trailing.as_mut() {
            slot(ui);
        }
    });
    *value != before
}

/// Searchable combo for unbounded / growing option lists (programs, masks, macros, …).
///
/// Uses fuzzy subsequence matching. Returns the combo button response;
/// `changed` is set when the selection changes.
pub fn searchable_combo(
    ui: &mut egui::Ui,
    id_salt: impl Hash + std::fmt::Debug,
    value: &mut String,
    opts: SearchableComboOpts<'_>,
) -> egui::Response {
    let SearchableComboOpts {
        options,
        empty_text,
        none_label,
        width,
        mut on_option_hover,
        mut option_icon,
        closed_icon,
        mut option_label,
    } = opts;
    let display = if value.is_empty() {
        empty_text.to_string()
    } else {
        option_label
            .as_mut()
            .map(|f| f(value.as_str()))
            .unwrap_or_else(|| value.clone())
    };

    let orphan = (!value.is_empty() && !options.iter().any(|o| o == value)).then(|| value.clone());

    let closed_tex = closed_icon
        .then(|| {
            option_icon
                .as_mut()
                .filter(|_| !value.is_empty())
                .and_then(|cb| cb(ui.ctx(), value.as_str()))
        })
        .flatten();

    let mut changed = false;
    ui.horizontal(|ui| {
        // Must match `ComboBox::from_id_salt(salt)` → `make_persistent_id(IdSalt::new(salt))`
        // on this horizontal child `ui` (its id differs from the caller's). A mismatch
        // re-requests search focus every frame, which flickers the Android soft keyboard.
        let button_id = ui.make_persistent_id(egui::IdSalt::new(&id_salt));
        let popup_id = button_id.with("popup");
        let search_id = button_id.with("search");
        let focused_id = button_id.with("search_focused");

        if let Some(tex) = closed_tex.as_ref() {
            crate::icon_cache::paint_process_icon(ui, tex, crate::icon_cache::PROCESS_ICON_SIDE);
        }
        let mut combo = egui::ComboBox::from_id_salt(&id_salt)
            .selected_text(display.clone())
            .truncate()
            .height(searchable_combo_popup_height(ui.ctx()))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        if let Some(w) = width {
            combo = combo.width(w);
        }

        let mut ir = combo.show_ui(ui, |ui| {
            let mut search = ui
                .ctx()
                .data(|d| d.get_temp::<String>(search_id))
                .unwrap_or_default();
            // `desired_width(INFINITY)` fills the popup without advancing its
            // min_size — `available_width()` here ratchets the combo wider and
            // never shrinks back.
            let search_resp = ui.add(
                egui::TextEdit::singleline(&mut search)
                    .id(search_id)
                    .hint_text("Search…")
                    .desired_width(f32::INFINITY),
            );
            let already_focused = ui
                .ctx()
                .data(|d| d.get_temp::<bool>(focused_id))
                .unwrap_or(false);
            if !already_focused {
                search_resp.request_focus();
                ui.ctx().data_mut(|d| d.insert_temp(focused_id, true));
            }
            ui.ctx()
                .data_mut(|d| d.insert_temp(search_id, search.clone()));

            crate::widgets::section_separator(ui);

            let q = search.trim().to_ascii_lowercase();
            let mut any = false;

            if let Some(label) = none_label {
                if q.is_empty() || fuzzy_match_fold(&q, label) {
                    any = true;
                    if ui.selectable_value(value, String::new(), label).clicked() {
                        changed = true;
                        egui::Popup::close_id(ui.ctx(), popup_id);
                    }
                }
            }

            if let Some(ref custom) = orphan {
                if q.is_empty() || fuzzy_match_fold(&q, custom) {
                    any = true;
                    let resp = paint_combo_option(
                        ui,
                        value,
                        custom.as_str(),
                        custom.as_str(),
                        &mut option_icon,
                    );
                    if let Some(cb) = on_option_hover.as_mut() {
                        cb(ui, &resp, custom.as_str());
                    }
                    if resp.clicked() {
                        changed = true;
                        egui::Popup::close_id(ui.ctx(), popup_id);
                    }
                }
            }

            for opt in options {
                if !q.is_empty() && !fuzzy_match_fold(&q, opt) {
                    continue;
                }
                any = true;
                let label = option_label
                    .as_mut()
                    .map(|f| f(opt.as_str()))
                    .unwrap_or_else(|| opt.clone());
                let resp = paint_combo_option(ui, value, opt.as_str(), &label, &mut option_icon);
                if let Some(cb) = on_option_hover.as_mut() {
                    cb(ui, &resp, opt.as_str());
                }
                if resp.clicked() {
                    changed = true;
                    egui::Popup::close_id(ui.ctx(), popup_id);
                }
            }

            if !any {
                // Combo-inline: title only (full list_vacancy chrome is too tall in popups).
                let copy = crate::widgets::list_vacancy_copy(&q, "options");
                ui.weak(copy.title);
            }
        });

        if !egui::ComboBox::is_open(ui.ctx(), button_id) {
            ui.ctx().data_mut(|d| {
                d.remove::<String>(search_id);
                d.remove::<bool>(focused_id);
            });
        }

        if !value.is_empty() {
            if let Some(cb) = on_option_hover.as_mut() {
                cb(ui, &ir.response, value.as_str());
            }
        }

        ir.response = text_hover_if(
            ir.response.clone(),
            &display,
            combo_text_elided(ui, &ir.response, &display),
        );
        if changed {
            ir.response.mark_changed();
        }
        ir.response
    })
    .inner
}

fn paint_combo_option(
    ui: &mut egui::Ui,
    value: &mut String,
    option_value: &str,
    label: &str,
    option_icon: &mut Option<&mut ComboOptionIcon<'_>>,
) -> egui::Response {
    let selected = value.as_str() == option_value;
    let icon = option_icon
        .as_mut()
        .and_then(|cb| cb(ui.ctx(), option_value));
    let resp = ui
        .horizontal(|ui| {
            if let Some(tex) = icon.as_ref() {
                crate::icon_cache::paint_process_icon(
                    ui,
                    tex,
                    crate::icon_cache::PROCESS_ICON_SIDE,
                );
            }
            ui.selectable_label(selected, label)
        })
        .inner;
    if resp.clicked() {
        *value = option_value.to_string();
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_row_keeps_trailing_inside_pane_and_parent_unwidened() {
        const PANE_W: f32 = 300.0;
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(PANE_W, 200.0));
            let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
            let mut text = String::new();
            let mut button = egui::Rect::NOTHING;
            let field = fill_row(
                &mut pane_ui,
                |ui| button = ui.button("Refresh").rect,
                |ui| {
                    ui.add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY))
                        .rect
                },
            );
            assert!(
                button.right() <= pane.right() + 0.5,
                "{button:?} past {pane:?}"
            );
            assert!(
                field.right() <= button.left(),
                "{field:?} overlaps {button:?}"
            );
            assert!(
                pane_ui.min_rect().width() <= PANE_W + 0.5,
                "row widened parent to {}",
                pane_ui.min_rect().width()
            );
        })
        .drop_without_applying_deltas();
    }

    #[test]
    fn searchable_combo_focuses_search_once_and_keeps_query() {
        use egui_kittest::kittest::Queryable;

        let options = vec!["apple".to_string(), "banana".to_string()];
        let mut harness = egui_kittest::Harness::new_ui_state(
            |ui, value: &mut String| {
                searchable_combo(
                    ui,
                    "fruit",
                    value,
                    SearchableComboOpts {
                        options: &options,
                        empty_text: "Pick",
                        none_label: None,
                        width: None,
                        on_option_hover: None,
                        option_icon: None,
                        closed_icon: true,
                        option_label: None,
                    },
                );
            },
            String::new(),
        );
        harness.run();
        harness.get_by_role(egui::accesskit::Role::ComboBox).click();
        harness.run();

        // Re-requesting focus every frame interrupts IME, which flickers the Android keyboard.
        for _ in 0..3 {
            harness.step();
            let ime = harness.output().platform_output.ime.as_ref();
            assert!(ime.is_some(), "search field should own the IME");
            assert!(
                !ime.is_some_and(|i| i.should_interrupt_composition),
                "focus re-requested while the popup stays open"
            );
        }

        harness
            .get_by_role(egui::accesskit::Role::TextInput)
            .type_text("ban");
        harness.run();
        assert!(harness.query_by_label("banana").is_some());
        assert!(
            harness.query_by_label("apple").is_none(),
            "query should persist across frames"
        );
    }

    /// String fields get the same S-stroke steppers as `num_entry`: active
    /// while the text is a pure integer, steps clamped to range.
    #[test]
    fn num_text_edit_steps_and_clamps_integers() {
        use egui_kittest::kittest::Queryable;

        let mut h = egui_kittest::Harness::builder()
            .with_size([200.0, 60.0])
            .build_ui_state(
                |ui, v: &mut String| {
                    crate::widgets::num_text_edit(ui, v, 80.0, 1..=9);
                },
                "8".to_owned(),
            );
        h.run_steps(2);
        h.get_by_label("Increment").click();
        h.run_steps(2);
        assert_eq!(h.state().as_str(), "9");
        h.get_by_label("Increment").click();
        h.run_steps(2);
        assert_eq!(h.state().as_str(), "9", "clamped at range end");
        h.get_by_label("Decrement").click();
        h.run_steps(2);
        assert_eq!(h.state().as_str(), "8");
    }

    /// The trailing eraser clears the query and reports `changed`; without
    /// `clearable` no eraser exists.
    #[test]
    fn search_field_clear_button_empties_query() {
        use egui_kittest::kittest::Queryable;

        let mut h = egui_kittest::Harness::builder()
            .with_size([300.0, 40.0])
            .build_ui_state(
                |ui, v: &mut String| {
                    search_field(
                        ui,
                        v,
                        SearchFieldOpts {
                            hint: "Filter…",
                            focus_id: None,
                            clearable: true,
                            stable_id: Some(egui::Id::new("test_search")),
                            trailing: None,
                        },
                    );
                },
                "abc".to_owned(),
            );
        h.run_steps(2);
        let clear = h.get_by_label("Clear search");
        assert!(
            clear.rect().right() <= 300.0 + 1.0,
            "clear icon fell outside the search pair: {:?}",
            clear.rect()
        );
        clear.click();
        h.run_steps(2);
        assert_eq!(h.state().as_str(), "");
    }

    /// The extra icon is a slot inside the search pair, after the clear eraser,
    /// not a sibling button past the frame. Empty query hides the eraser; the
    /// trailing icon stays.
    #[test]
    fn search_trailing_icon_sits_inside_the_pair() {
        let ctx = egui::Context::default();
        let pane = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(360.0, 64.0));
        let plus = egui_phosphor::regular::PLUS;
        let clear_glyph = egui_phosphor::regular::ERASER;
        for (start, field_text) in [("query", "query"), ("", "Filter…")] {
            let mut value = start.to_owned();
            for pass in 0..2 {
                let out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(400.0, 120.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                        search_field(
                            &mut pane_ui,
                            &mut value,
                            SearchFieldOpts {
                                hint: "Filter…",
                                focus_id: None,
                                clearable: true,
                                stable_id: Some(egui::Id::new("trail_search")),
                                trailing: Some(SearchTrailing {
                                    glyph: plus,
                                    tip: "New macro",
                                    color: Some(crate::theme::MACRO_START),
                                }),
                            },
                        );
                    },
                );
                let text_rect = |needle: &str| {
                    out.shapes.iter().find_map(|clipped| match &clipped.shape {
                        egui::Shape::Text(text) if text.galley.job.text == needle => {
                            Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
                        }
                        _ => None,
                    })
                };
                let field = text_rect(field_text).unwrap_or_else(|| panic!("missing {field_text}"));
                let add = text_rect(plus).expect("trailing glyph");
                let frame = out
                    .shapes
                    .iter()
                    .find_map(|clipped| match &clipped.shape {
                        egui::Shape::Rect(rect)
                            if rect.fill.a() > 0
                                && rect.rect.contains(field.center())
                                && rect.rect.contains(add.center()) =>
                        {
                            Some(rect.rect)
                        }
                        _ => None,
                    })
                    .expect("pair frame around the field and the trailing icon");
                assert!(
                    add.left() + 1.0 >= field.right(),
                    "pass {pass} text {start:?} trailing icon overlaps the field: {field:?} {add:?}"
                );
                assert!(
                    (add.center().y - field.center().y).abs() < 2.0,
                    "pass {pass} text {start:?} trailing icon left the baseline: {field:?} {add:?}"
                );
                assert!(
                    frame.right() <= pane.right() + 1.0,
                    "pass {pass} text {start:?} pair past the pane: {frame:?}"
                );
                if start.is_empty() {
                    assert!(
                        text_rect(clear_glyph).is_none(),
                        "clear glyph shown for an empty query"
                    );
                } else {
                    let clear = text_rect(clear_glyph).expect("clear glyph");
                    assert!(
                        frame.contains(clear.center())
                            && clear.left() + 1.0 >= field.right()
                            && add.left() + 1.0 >= clear.right(),
                        "pass {pass} order should be field, clear, trailing: {field:?} {clear:?} {add:?}"
                    );
                }
                out.drop_without_applying_deltas();
            }
        }
    }

    #[test]
    fn search_trailing_click_is_reported() {
        use egui_kittest::kittest::Queryable;

        let mut h = egui_kittest::Harness::builder()
            .with_size([300.0, 40.0])
            .build_ui_state(
                |ui, frames: &mut Vec<bool>| {
                    let mut value = String::new();
                    let out = search_field(
                        ui,
                        &mut value,
                        SearchFieldOpts {
                            hint: "Filter…",
                            focus_id: None,
                            clearable: true,
                            stable_id: Some(egui::Id::new("trail_click")),
                            trailing: Some(SearchTrailing {
                                glyph: egui_phosphor::regular::PLUS,
                                tip: "New macro",
                                color: Some(crate::theme::MACRO_START),
                            }),
                        },
                    );
                    frames.push(out.trailing_clicked);
                },
                Vec::new(),
            );
        h.run_steps(2);
        let before = h.state().len();
        assert!(
            h.state().iter().all(|clicked| !clicked),
            "trailing icon clicked before the pointer press"
        );
        h.get_by_label("New macro").click();
        h.run_steps(3);
        assert!(
            h.state()[before..].contains(&true),
            "trailing icon click should be reported"
        );
    }

    /// `${var}`/text content isn't steppable — no zones are registered.
    #[test]
    fn num_text_edit_hides_steppers_for_non_integers() {
        use egui_kittest::kittest::Queryable;

        for value in ["${x}", "abc", ""] {
            let mut h = egui_kittest::Harness::builder()
                .with_size([200.0, 60.0])
                .build_ui_state(
                    |ui, v: &mut String| {
                        crate::widgets::num_text_edit(ui, v, 80.0, 1..=9);
                    },
                    value.to_owned(),
                );
            h.run_steps(2);
            assert_eq!(
                h.query_all_by_label("Increment").count(),
                0,
                "steppers shown for {value:?}"
            );
        }
    }

    /// Fill-width search must keep one width. A frame that spills past the pane
    /// turns the overflow strip on and off, and the field jumps every frame.
    #[test]
    fn search_field_width_holds_across_frames() {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);
        let pane_w = 360.0;
        for clearable_text in [String::new(), String::from("query")] {
            let mut value = clearable_text.clone();
            let mut widths = Vec::new();
            for frame in 0..8 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 400.0),
                    )),
                    time: Some(frame as f64 * 0.05),
                    predicted_dt: 0.05,
                    ..Default::default()
                };
                ctx.run_ui(input, |ui| {
                    let pane =
                        egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(pane_w, 64.0));
                    let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                    pane_ui.set_clip_rect(pane);
                    crate::widgets::show_overflow(&mut pane_ui, "search_stable", 1.0, |ui| {
                        let out = search_field(
                            ui,
                            &mut value,
                            SearchFieldOpts {
                                hint: "Filter…",
                                focus_id: None,
                                clearable: true,
                                stable_id: Some(egui::Id::new("stable_search")),
                                trailing: None,
                            },
                        );
                        widths.push(out.response.rect.width());
                    });
                    let row_right = pane_ui.min_rect().right();
                    assert!(
                        row_right <= pane.right() + 1.0,
                        "frame {frame} text {value:?} row {row_right} past {}",
                        pane.right()
                    );
                })
                .drop_without_applying_deltas();
            }
            let settled = widths[3];
            for (i, w) in widths.iter().enumerate().skip(3) {
                assert!(
                    (w - settled).abs() < 0.75,
                    "search width jitter frame {i} text {value:?}: {widths:?}"
                );
            }
        }
    }
}
