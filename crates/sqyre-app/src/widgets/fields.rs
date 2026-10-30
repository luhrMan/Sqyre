//! Labeled form field helpers shared by action edit, variables, and data editor.

use crate::action_tooltip::help;
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
    super::wrap_unit(ui, |ui| {
        help::label(ui, icon, label);
        help::tip(
            ui.add(egui::TextEdit::singleline(value).desired_width(opts.width)),
            help_text,
        );
    });
}

/// Width for a content-sized Name edit: hugs the text (or `hint` while empty),
/// grows as the text fills, and stops at the row's visible width — the
/// singleline scrolls inside the box past that.
pub fn name_edit_width(ui: &egui::Ui, text: &str, hint: &str) -> f32 {
    let shown = if text.trim().is_empty() { hint } else { text };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let text_w = ui
        .painter()
        .layout_no_wrap(shown.to_owned(), font, egui::Color32::WHITE)
        .size()
        .x;
    // TextEdit margin (8) + frame stroke (~2) + caret/selection slop.
    let w = text_w + 18.0;
    let min = ui.spacing().interact_size.y * 4.0;
    w.clamp(min, super::visible_width(ui).max(min))
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
pub fn num_text_field(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut String,
    range: RangeInclusive<i32>,
) {
    num_text_field_opts(
        ui,
        icon,
        label,
        help_text,
        value,
        NumFieldOpts {
            width: W_TEXT,
            range,
        },
    );
}

/// [`num_text_field`] with explicit options.
pub fn num_text_field_opts(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    help_text: &str,
    value: &mut String,
    opts: NumFieldOpts,
) {
    super::wrap_unit(ui, |ui| {
        help::label(ui, icon, label);
        help::tip(
            num_text_edit(ui, value, opts.width, opts.range.clone()),
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
    /// Trailing ✕ that empties the field while it has text.
    pub clearable: bool,
    /// Pinned field id — needed when a sibling drawn before the field (e.g.
    /// the toggling ✕) would shift the auto id and drop keyboard focus.
    pub stable_id: Option<egui::Id>,
}

/// Outcome of [`search_field`].
pub struct SearchFieldResp {
    /// The field's response — the magnifier's when the field drops for width.
    /// `has_focus` / `request_focus` / `surrender_focus` apply to it.
    pub response: egui::Response,
    /// Query text changed this frame (edit or ✕ clear — `value` is already updated).
    pub changed: bool,
}

/// Magnifier + fill-width filter field (+ optional trailing ✕) — the one
/// search row for lists, pickers, dialogs, and panels.
///
/// Emits a self-contained row: a plain `ui.horizontal` normally, a `fill_row`
/// when `clearable` so the ✕ pins right while the field fills. Wrap it in
/// [`super::show_overflow`] or another `fill_row` slot when siblings share the
/// row.
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
    } = opts;
    // `body` holds the `&mut String`, so snapshot emptiness before it is built.
    let has_text = !value.is_empty();
    let mut body = |ui: &mut egui::Ui| -> egui::Response {
        let icon = ui
            .label(egui::RichText::new(
                egui_phosphor::regular::MAGNIFYING_GLASS,
            ))
            .on_hover_text("Search");
        icon.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Search".to_owned())
        });
        // egui never allocates a TextEdit narrower than 24px (`MIN_WIDTH`), so
        // below that the field's rect underlaps what follows it. A sliver-wide
        // field is unusable anyway — drop it and keep the icon.
        if ui.available_width() < 24.0 {
            return icon;
        }
        let mut edit = egui::TextEdit::singleline(value).desired_width(f32::INFINITY);
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
    };
    if !clearable {
        let response = ui.horizontal(&mut body).inner;
        return SearchFieldResp {
            changed: response.changed(),
            response,
        };
    }
    let mut cleared = false;
    let response = fill_row(
        ui,
        |ui| {
            // The ✕ only exists while the field has text; it is drawn before
            // the field in id space (right_to_left), hence `stable_id`.
            if has_text {
                cleared = crate::widgets::cancel_icon_button(ui, "Clear search").clicked();
            }
        },
        |ui| body(ui),
    );
    if cleared {
        value.clear();
    }
    SearchFieldResp {
        changed: cleared || response.changed(),
        response,
    }
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
    super::wrap_unit(ui, |ui| {
        help::label(ui, icon, label);
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
    super::wrap_unit(ui, |ui| {
        help::label(ui, icon, label);
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
    super::wrap_unit(ui, |ui| {
        help::label(ui, icon, label);
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
    super::wrap_unit(ui, |ui| {
        help::label(ui, icon, label);
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
    /// Leading icon drawn on each option row and beside the closed button.
    pub option_icon: Option<&'a mut ComboOptionIcon<'a>>,
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
    } = opts;
    let display = if value.is_empty() {
        empty_text.to_string()
    } else {
        value.clone()
    };

    let orphan = (!value.is_empty() && !options.iter().any(|o| o == value)).then(|| value.clone());

    let closed_icon = option_icon
        .as_mut()
        .filter(|_| !value.is_empty())
        .and_then(|cb| cb(ui.ctx(), value.as_str()));

    let mut changed = false;
    ui.horizontal(|ui| {
        // Must match `ComboBox::from_id_salt(salt)` → `make_persistent_id(IdSalt::new(salt))`
        // on this horizontal child `ui` (its id differs from the caller's). A mismatch
        // re-requests search focus every frame, which flickers the Android soft keyboard.
        let button_id = ui.make_persistent_id(egui::IdSalt::new(&id_salt));
        let popup_id = button_id.with("popup");
        let search_id = button_id.with("search");
        let focused_id = button_id.with("search_focused");

        if let Some(tex) = closed_icon.as_ref() {
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
                let resp =
                    paint_combo_option(ui, value, opt.as_str(), opt.as_str(), &mut option_icon);
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
                    num_text_edit(ui, v, 80.0, 1..=9);
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

    /// The trailing ✕ clears the query and reports `changed`; without
    /// `clearable` no ✕ exists.
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
                        },
                    );
                },
                "abc".to_owned(),
            );
        h.run_steps(2);
        h.get_by_label("Clear search").click();
        h.run_steps(2);
        assert_eq!(h.state().as_str(), "");
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
                        num_text_edit(ui, v, 80.0, 1..=9);
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
}
