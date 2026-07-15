//! Tree/UI display helpers: summary params, pastel colors, icon glyphs.

use crate::{
    Action, ActionKind, ConditionClause, CoordinateOutputs, ScalarValue, WaitTilFoundConfig,
    MATCH_ANY,
};

/// One display parameter (Go `actions.Param`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayParam {
    pub label: String,
    pub value: String,
    pub extra: bool,
}

impl DisplayParam {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            extra: false,
        }
    }

    pub fn extra(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            extra: true,
        }
    }

    pub fn minimal(&self) -> &str {
        self.value.trim()
    }
}

/// Produced variable binding for tree/output chips (Go `VariableBinding`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableBinding {
    pub name: String,
    pub role: String,
}

impl VariableBinding {
    pub fn pill_label(&self) -> &'static str {
        match self.role.as_str() {
            "output_x" => "X",
            "output_y" => "Y",
            "length" => "Length",
            "value" => "Variable",
            _ => "Output",
        }
    }
}

/// Summary (inline) and extra (tooltip-only) params, excluding Type and empties.
pub fn split_display_params(params: &[DisplayParam]) -> (Vec<&DisplayParam>, Vec<&DisplayParam>) {
    let mut summary = Vec::new();
    let mut extra = Vec::new();
    for p in params {
        if p.label.eq_ignore_ascii_case("Type") {
            continue;
        }
        if p.minimal().is_empty() {
            continue;
        }
        if p.extra {
            extra.push(p);
        } else {
            summary.push(p);
        }
    }
    (summary, extra)
}

impl CoordinateOutputs {
    pub fn variable_bindings(&self) -> Vec<VariableBinding> {
        let mut out = Vec::new();
        if !self.output_x_variable.is_empty() {
            out.push(VariableBinding {
                name: self.output_x_variable.clone(),
                role: "output_x".into(),
            });
        }
        if !self.output_y_variable.is_empty() {
            out.push(VariableBinding {
                name: self.output_y_variable.clone(),
                role: "output_y".into(),
            });
        }
        out
    }
}

impl WaitTilFoundConfig {
    /// Go `DisplayWaitMode`.
    pub fn display_wait_mode(&self, instant_label: &str) -> String {
        if !self.wait_til_found {
            return instant_label.to_string();
        }
        if self.wait_til_found_seconds > 0 {
            format!("{} seconds or until found", self.wait_til_found_seconds)
        } else {
            format!("wait {}s", self.wait_til_found_seconds)
        }
    }
}

impl ConditionClause {
    pub fn summary(&self) -> String {
        let left = self.left.as_display();
        let right = self.right.as_display();
        match self.operator.as_str() {
            "is set" | "is empty" => format!("{left} {}", self.operator),
            _ => format!("{left} {} {right}", self.operator),
        }
    }
}

fn up_or_down(state: bool) -> &'static str {
    if state {
        "down"
    } else {
        "up"
    }
}

fn format_wait_time(t: &ScalarValue) -> String {
    match t {
        ScalarValue::Int(i) => format!("{i} ms"),
        ScalarValue::Float(f) => format!("{:.0} ms", f),
        ScalarValue::String(s) => s.clone(),
        ScalarValue::Bool(b) => format!("{} ms", if *b { 1 } else { 0 }),
        ScalarValue::Null => "0 ms".into(),
    }
}

fn yaml_display(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::Null => String::new(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

fn match_label(mode: &str) -> &'static str {
    if mode == MATCH_ANY {
        "any (OR)"
    } else {
        "all (AND)"
    }
}

fn condition_summary(match_mode: &str, clauses: &[ConditionClause]) -> String {
    let sep = if match_mode == MATCH_ANY {
        " | "
    } else {
        " & "
    };
    clauses
        .iter()
        .map(ConditionClause::summary)
        .collect::<Vec<_>>()
        .join(sep)
}

impl Action {
    /// Params for tree/tooltip display (Go `Params()`).
    pub fn display_params(&self) -> Vec<DisplayParam> {
        self.kind.display_params()
    }

    /// Tree-row params: Image Search omits Items (shown as thumbs/count instead).
    pub fn display_params_for_tree(&self) -> Vec<DisplayParam> {
        let params = self.display_params();
        if self.type_key() != "imagesearch" {
            return params;
        }
        params
            .into_iter()
            .filter(|p| !p.label.eq_ignore_ascii_case("Items"))
            .collect()
    }

    pub fn variable_bindings(&self) -> Vec<VariableBinding> {
        self.kind.variable_bindings()
    }

    /// Inline summary pill texts with output-binding labels applied.
    pub fn tree_summary_pills(&self) -> Vec<SummaryPill> {
        let params = self.display_params_for_tree();
        let (summary, _) = split_display_params(&params);
        let bindings = self.variable_bindings();
        let mut output_labels: Vec<(String, String)> = bindings
            .iter()
            .map(|b| (b.name.clone(), b.pill_label().to_string()))
            .collect();

        let mut pills = Vec::new();
        let mut consumed = std::collections::HashSet::new();
        for p in summary {
            let text = p.minimal().to_string();
            if let Some((_, label)) = output_labels.iter().find(|(n, _)| n == &text) {
                if consumed.insert(text.clone()) {
                    pills.push(SummaryPill {
                        text: text.clone(),
                        prefix: Some(label.clone()),
                    });
                }
                continue;
            }
            pills.push(SummaryPill {
                text,
                prefix: None,
            });
        }
        for (name, label) in output_labels.drain(..) {
            if consumed.insert(name.clone()) {
                pills.push(SummaryPill {
                    text: name,
                    prefix: Some(label),
                });
            }
        }
        pills
    }
}

/// One inline tree pill (value-only, or labeled output binding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryPill {
    pub text: String,
    pub prefix: Option<String>,
}

impl SummaryPill {
    pub fn display_text(&self) -> String {
        match &self.prefix {
            Some(p) => format!("{p}: {}", self.text),
            None => self.text.clone(),
        }
    }
}

impl ActionKind {
    pub fn display_params(&self) -> Vec<DisplayParam> {
        let type_key = self.type_key();
        let mut params = vec![DisplayParam::new("Type", type_key)];
        match self {
            Self::Wait { time } => {
                params.push(DisplayParam::new("Time", format_wait_time(time)));
            }
            Self::Move {
                point,
                smooth,
                smooth_low,
                smooth_high,
                smooth_delay_ms,
            } => {
                params.push(DisplayParam::new("Point", point.display_label()));
                if *smooth {
                    params.push(DisplayParam::extra("Smooth", "true"));
                    params.push(DisplayParam::extra("Smooth low", format_float(*smooth_low)));
                    params.push(DisplayParam::extra("Smooth high", format_float(*smooth_high)));
                    params.push(DisplayParam::extra(
                        "Smooth delay (ms)",
                        smooth_delay_ms.to_string(),
                    ));
                }
            }
            Self::Click { button, state } => {
                params.push(DisplayParam::new("Button", button.as_str()));
                params.push(DisplayParam::new("State", up_or_down(*state)));
            }
            Self::Key { key, state } => {
                params.push(DisplayParam::new("Key", key.as_str()));
                params.push(DisplayParam::new("State", up_or_down(*state)));
            }
            Self::Type { text, delay_ms } => {
                params.push(DisplayParam::new("Text", format!("{text:?}")));
                if *delay_ms > 0 {
                    params.push(DisplayParam::extra("Delay", format!("{delay_ms} ms")));
                }
            }
            Self::Loop { name, count, .. } => {
                params.push(DisplayParam::new("Name", name.as_str()));
                params.push(DisplayParam::new("Iterations", count.as_display()));
            }
            Self::While {
                name,
                match_mode,
                clauses,
                max_iterations,
                ..
            } => {
                params.push(DisplayParam::new("Name", name.as_str()));
                params.push(DisplayParam::new("Match", match_label(match_mode)));
                params.push(DisplayParam::new(
                    "While",
                    condition_summary(match_mode, clauses),
                ));
                if *max_iterations > 0 {
                    params.push(DisplayParam::extra(
                        "Max iterations",
                        max_iterations.to_string(),
                    ));
                }
            }
            Self::Conditional {
                name,
                match_mode,
                clauses,
                ..
            } => {
                params.push(DisplayParam::new("Name", name.as_str()));
                params.push(DisplayParam::new("Match", match_label(match_mode)));
                params.push(DisplayParam::new(
                    "If",
                    condition_summary(match_mode, clauses),
                ));
            }
            Self::ImageSearch {
                name,
                targets,
                search_area,
                tolerance,
                blur,
                wait,
                run_branch_on_no_find,
                ..
            } => {
                params.push(DisplayParam::new("Name", name.as_str()));
                params.push(DisplayParam::new("Items", targets.len().to_string()));
                params.push(DisplayParam::new("Search Area", search_area.display_label()));
                params.push(DisplayParam::extra(
                    "Wait",
                    wait.display_wait_mode("instant"),
                ));
                params.push(DisplayParam::extra("Tolerance", format_float(*tolerance)));
                params.push(DisplayParam::extra("Blur", blur.to_string()));
                if *run_branch_on_no_find {
                    params.push(DisplayParam::extra("Run on no find", "yes"));
                }
            }
            Self::Ocr {
                name,
                target,
                search_area,
                wait,
                ..
            } => {
                params.push(DisplayParam::new("Name", name.as_str()));
                params.push(DisplayParam::new("Target Text", target.as_str()));
                params.push(DisplayParam::extra(
                    "Search Area",
                    search_area.display_label(),
                ));
                params.push(DisplayParam::extra(
                    "Wait",
                    wait.display_wait_mode("instant"),
                ));
            }
            Self::FindPixel {
                name,
                search_area,
                target_color,
                color_tolerance,
                wait,
                ..
            } => {
                params.push(DisplayParam::new("Name", name.as_str()));
                params.push(DisplayParam::new("Color", target_color.as_str()));
                params.push(DisplayParam::extra(
                    "Tolerance",
                    format!("{color_tolerance}%"),
                ));
                params.push(DisplayParam::extra(
                    "Search Area",
                    search_area.display_label(),
                ));
                params.push(DisplayParam::extra(
                    "Wait",
                    wait.display_wait_mode("instant"),
                ));
            }
            Self::ForEachRow {
                name,
                sources,
                start_row,
                end_row,
                ..
            } => {
                params.push(DisplayParam::new("Name", name.as_str()));
                params.push(DisplayParam::new("Sources", sources.len().to_string()));
                if start_row.is_set() {
                    params.push(DisplayParam::extra("Start Row", start_row.as_display()));
                }
                if end_row.is_set() {
                    params.push(DisplayParam::extra("End Row", end_row.as_display()));
                }
            }
            Self::SetVariable {
                variable_name,
                value,
            } => {
                params.push(DisplayParam::new("Variable", variable_name.as_str()));
                params.push(DisplayParam::new("Value", yaml_display(value)));
            }
            Self::SaveVariable {
                variable_name,
                destination,
                append,
                ..
            } => {
                let mode = if *append { "append" } else { "overwrite" };
                params.push(DisplayParam::new("Variable", variable_name.as_str()));
                params.push(DisplayParam::new("Destination", destination.as_str()));
                params.push(DisplayParam::new("Mode", mode));
            }
            Self::FocusWindow {
                process_path,
                window_title,
            } => {
                let title = if window_title.is_empty() {
                    "not set"
                } else {
                    window_title.as_str()
                };
                let path = if process_path.is_empty() {
                    "not set"
                } else {
                    process_path.as_str()
                };
                params.push(DisplayParam::new("Title", title));
                params.push(DisplayParam::extra("App", path));
            }
            Self::RunMacro { macro_name } => {
                let target = if macro_name.is_empty() {
                    "not set"
                } else {
                    macro_name.as_str()
                };
                params.push(DisplayParam::new("Macro", target));
            }
            Self::Pause {
                message,
                continue_key,
                ..
            } => {
                let key = if continue_key.is_empty() {
                    "not set".into()
                } else {
                    continue_key.join("+")
                };
                if !message.trim().is_empty() {
                    params.push(DisplayParam::new("Message", message.as_str()));
                }
                params.push(DisplayParam::new("Continue", key));
            }
            Self::NavigateSelect {
                program,
                graph_name,
                ..
            } => {
                if !program.is_empty() {
                    params.push(DisplayParam::new("Program", program.as_str()));
                }
                if !graph_name.is_empty() {
                    params.push(DisplayParam::new("Graph", graph_name.as_str()));
                }
            }
            Self::NavigateKey { chord, exit, .. } => {
                let key = if chord.is_empty() {
                    "not set".into()
                } else {
                    chord.join("+")
                };
                params.push(DisplayParam::new("Chord", key));
                if *exit {
                    params.push(DisplayParam::new("After", "exit"));
                }
            }
            Self::Break | Self::Continue => {}
        }
        params
    }

    pub fn variable_bindings(&self) -> Vec<VariableBinding> {
        match self {
            Self::SetVariable { variable_name, .. } if !variable_name.is_empty() => {
                vec![VariableBinding {
                    name: variable_name.clone(),
                    role: "value".into(),
                }]
            }
            Self::ImageSearch { coords, .. } | Self::FindPixel { coords, .. } => {
                coords.variable_bindings()
            }
            Self::Ocr {
                coords,
                output_variable,
                ..
            } => {
                let mut out = coords.variable_bindings();
                if !output_variable.is_empty() {
                    out.push(VariableBinding {
                        name: output_variable.clone(),
                        role: "output".into(),
                    });
                }
                out
            }
            Self::ForEachRow { sources, .. } => sources
                .iter()
                .filter(|s| !s.output_var.is_empty())
                .map(|s| VariableBinding {
                    name: s.output_var.clone(),
                    role: "output".into(),
                })
                .collect(),
            Self::NavigateSelect {
                output_ref,
                output_graph,
                output_row,
                output_col,
                output_collection,
                ..
            } => {
                let mut out = Vec::new();
                for (name, role) in [
                    (output_ref, "ref"),
                    (output_graph, "graph"),
                    (output_row, "row"),
                    (output_col, "col"),
                    (output_collection, "collection"),
                ] {
                    if !name.is_empty() {
                        out.push(VariableBinding {
                            name: name.clone(),
                            role: role.into(),
                        });
                    }
                }
                out
            }
            _ => Vec::new(),
        }
    }
}

fn format_float(f: f64) -> String {
    let s = format!("{f:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Category keys for customizable macro-tree action colors (Go `ActionColorKey*`).
pub const ACTION_COLOR_KEY_MOUSE_KEYBOARD: &str = "mouse_keyboard";
pub const ACTION_COLOR_KEY_DETECTION: &str = "detection";
pub const ACTION_COLOR_KEY_VARIABLES: &str = "variables";
pub const ACTION_COLOR_KEY_MISCELLANEOUS: &str = "miscellaneous";
pub const ACTION_COLOR_KEY_WAIT: &str = "wait";
pub const ACTION_COLOR_KEY_DEFAULT: &str = "default";

/// `(key, label)` for every customizable action color group.
pub const ACTION_COLOR_CATEGORIES: &[(&str, &str)] = &[
    (ACTION_COLOR_KEY_MOUSE_KEYBOARD, "Mouse & Keyboard"),
    (ACTION_COLOR_KEY_DETECTION, "Detection"),
    (ACTION_COLOR_KEY_VARIABLES, "Variables"),
    (ACTION_COLOR_KEY_MISCELLANEOUS, "Miscellaneous"),
    (ACTION_COLOR_KEY_WAIT, "Wait"),
    (ACTION_COLOR_KEY_DEFAULT, "Default"),
];

use std::collections::HashMap;
use std::sync::RwLock;

static CUSTOM_ACTION_COLORS: RwLock<Option<HashMap<String, [u8; 4]>>> = RwLock::new(None);

fn action_color_key(action_type: &str) -> &'static str {
    let t = action_type.trim().to_ascii_lowercase();
    if t == "wait" || t == "pause" {
        return ACTION_COLOR_KEY_WAIT;
    }
    match action_color_category(&t) {
        "Mouse & Keyboard" => ACTION_COLOR_KEY_MOUSE_KEYBOARD,
        "Detection" => ACTION_COLOR_KEY_DETECTION,
        "Variables" => ACTION_COLOR_KEY_VARIABLES,
        "Miscellaneous" => ACTION_COLOR_KEY_MISCELLANEOUS,
        _ => ACTION_COLOR_KEY_DEFAULT,
    }
}

/// Sample action type used when previewing a category swatch.
pub fn sample_action_type_for_color_key(category_key: &str) -> &'static str {
    match category_key {
        ACTION_COLOR_KEY_MOUSE_KEYBOARD => "click",
        ACTION_COLOR_KEY_DETECTION => "imagesearch",
        ACTION_COLOR_KEY_VARIABLES => "setvariable",
        ACTION_COLOR_KEY_MISCELLANEOUS => "loop",
        ACTION_COLOR_KEY_WAIT => "wait",
        _ => "",
    }
}

/// Format RGBA as `#rrggbb` (alpha ignored).
pub fn format_hex_color(rgba: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2])
}

/// Store a user-chosen color for a category key.
pub fn set_custom_action_color(category_key: &str, rgba: [u8; 4]) {
    let mut guard = CUSTOM_ACTION_COLORS.write().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(category_key.to_string(), rgba);
}

/// Remove a user override for a category key.
pub fn clear_custom_action_color(category_key: &str) {
    let mut guard = CUSTOM_ACTION_COLORS.write().unwrap();
    if let Some(map) = guard.as_mut() {
        map.remove(category_key);
    }
}

/// Remove every user override.
pub fn clear_all_custom_action_colors() {
    *CUSTOM_ACTION_COLORS.write().unwrap() = None;
}

/// Category pastel color (Go `ActionPastelColor`), light/dark theme.
/// Uses a user override when one is set; otherwise the built-in pastel.
pub fn action_pastel_color(action_type: &str, is_dark: bool) -> [u8; 4] {
    let t = action_type.trim().to_ascii_lowercase();
    if t != "warning" {
        let key = action_color_key(&t);
        if let Some(c) = CUSTOM_ACTION_COLORS
            .read()
            .unwrap()
            .as_ref()
            .and_then(|m| m.get(key).copied())
        {
            return c;
        }
    }
    default_action_pastel_color(action_type, is_dark)
}

/// Built-in pastel (Go `DefaultActionPastelColor`), ignoring user overrides.
pub fn default_action_pastel_color(action_type: &str, is_dark: bool) -> [u8; 4] {
    let t = action_type.trim().to_ascii_lowercase();
    if t == "warning" {
        return if is_dark {
            [0x8A, 0x5A, 0x2A, 0xFF]
        } else {
            [0xF0, 0xC0, 0x6A, 0xFF]
        };
    }
    let is_wait = t == "wait" || t == "pause";
    let category = action_color_category(&t);

    if is_dark {
        if is_wait {
            return [0x7B, 0x4E, 0x3E, 0xFF];
        }
        return match category {
            "Mouse & Keyboard" => [0x5E, 0x6B, 0x4A, 0xFF],
            "Detection" => [0x5A, 0x4A, 0x44, 0xFF],
            "Variables" => [0x2A, 0x42, 0x54, 0xFF],
            "Miscellaneous" => [0x6A, 0x5A, 0x3F, 0xFF],
            _ => [0x5C, 0x54, 0x49, 0xFF],
        };
    }
    if is_wait {
        return [0xC9, 0x8D, 0x6A, 0xFF];
    }
    match category {
        "Mouse & Keyboard" => [0xA1, 0xB0, 0x7A, 0xFF],
        "Detection" => [0xB4, 0x9A, 0x84, 0xFF],
        "Variables" => [0x5E, 0x8F, 0xB0, 0xFF],
        "Miscellaneous" => [0xB8, 0x9A, 0x6A, 0xFF],
        _ => [0xB2, 0xA4, 0x8E, 0xFF],
    }
}

/// Nested `${var}` chip fill (Go `DefaultNestedVarRefColor`).
pub fn nested_var_ref_color(is_dark: bool) -> [u8; 4] {
    if is_dark {
        [0x46, 0x62, 0x78, 0xFF]
    } else {
        [0x9E, 0xC4, 0xE3, 0xFF]
    }
}

fn action_color_category(action_type: &str) -> &'static str {
    match action_type {
        "move" | "click" | "key" | "type" => "Mouse & Keyboard",
        "imagesearch" | "ocr" | "findpixel" => "Detection",
        "setvariable" | "foreachrow" | "savevariable" => "Variables",
        "wait" | "pause" | "focuswindow" | "runmacro" | "loop" | "while" | "conditional"
        | "break" | "continue" | "navigateselect" | "navigatekey" => "Miscellaneous",
        _ => "",
    }
}

/// Compact glyph for the type icon badge (egui stand-in for Fyne theme icons).
///
/// Glyphs must exist in egui's default proportional fonts (Ubuntu-Light, Hack,
/// NotoEmoji, emoji-icon-font). Mathematical alphanumeric / obscure symbols
/// render as empty boxes (tofu).
pub fn action_icon_glyph(action: &Action) -> &'static str {
    match &action.kind {
        ActionKind::Click { state, .. } => {
            if *state {
                "⬇"
            } else {
                "⬆"
            }
        }
        ActionKind::Move { .. } => "➔",
        ActionKind::Key { state, .. } => {
            if *state {
                "⬇"
            } else {
                "⬆"
            }
        }
        ActionKind::Type { .. } => "⌨",
        ActionKind::Wait { .. } => "⏱",
        ActionKind::Pause { .. } => "⏸",
        ActionKind::FocusWindow { .. } => "👁",
        ActionKind::RunMacro { .. } => "▶",
        ActionKind::Conditional { .. } => "?",
        ActionKind::Loop { .. } | ActionKind::While { .. } => "↻",
        ActionKind::Break => "⏹",
        ActionKind::Continue => "⏭",
        ActionKind::SetVariable { .. } => "x",
        ActionKind::SaveVariable { .. } => "💾",
        ActionKind::ForEachRow { .. } => "☰",
        ActionKind::Ocr { .. } => "🔤",
        ActionKind::ImageSearch { .. } => "🔍",
        ActionKind::FindPixel { .. } => "🎨",
        ActionKind::NavigateSelect { .. } => "⌖",
        ActionKind::NavigateKey { .. } => "⎇",
    }
}

/// Parse `#RGB` / `#RRGGBB` / `#AARRGGBB` (Go FindPixel normalize) → RGBA.
pub fn parse_hex_color(hex: &str) -> Option<[u8; 4]> {
    let mut h = hex.trim().trim_start_matches('#').to_ascii_lowercase();
    if h.len() == 8 {
        h = h[2..].to_string(); // drop leading alpha when present
    }
    if h.len() == 3 {
        let r = u8::from_str_radix(&h[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&h[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&h[2..3].repeat(2), 16).ok()?;
        return Some([r, g, b, 255]);
    }
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some([r, g, b, 255])
}

/// True when the pill value looks like a `${name}` / `{name}` var ref.
pub fn looks_like_var_ref(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("${") && t.ends_with('}'))
        || (t.starts_with('{') && t.ends_with('}') && !t.starts_with("${"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{root_loop, ActionId, CoordinateRef};

    #[test]
    fn wait_summary_includes_time_ms() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(250),
            },
        };
        let pills = a.tree_summary_pills();
        assert_eq!(pills.len(), 1);
        assert_eq!(pills[0].text, "250 ms");
    }

    #[test]
    fn image_search_tree_omits_items() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec!["a".into(), "b".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.9,
                blur: 0,
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs::defaults(),
                run_branch_on_no_find: false,
                order: Default::default(),
                subactions: vec![],
            },
        };
        let tree = a.display_params_for_tree();
        assert!(!tree.iter().any(|p| p.label == "Items"));
        let full = a.display_params();
        assert!(full.iter().any(|p| p.label == "Items" && p.value == "2"));
    }

    #[test]
    fn pastel_wait_differs_from_mouse() {
        clear_all_custom_action_colors();
        let wait = action_pastel_color("wait", false);
        let click = action_pastel_color("click", false);
        assert_ne!(wait, click);
    }

    #[test]
    fn custom_action_color_overrides_builtin() {
        clear_all_custom_action_colors();
        let custom = [0x11, 0x22, 0x33, 0xFF];
        set_custom_action_color(ACTION_COLOR_KEY_MOUSE_KEYBOARD, custom);
        assert_eq!(action_pastel_color("click", false), custom);
        assert_ne!(
            action_pastel_color("click", false),
            default_action_pastel_color("click", false)
        );
        clear_custom_action_color(ACTION_COLOR_KEY_MOUSE_KEYBOARD);
        assert_eq!(
            action_pastel_color("click", false),
            default_action_pastel_color("click", false)
        );
        clear_all_custom_action_colors();
    }

    #[test]
    fn parse_hex_strips_alpha() {
        assert_eq!(parse_hex_color("#ff112233"), Some([0x11, 0x22, 0x33, 255]));
        assert_eq!(parse_hex_color("aabbcc"), Some([0xaa, 0xbb, 0xcc, 255]));
    }

    #[test]
    fn root_loop_is_named_root() {
        let root = root_loop(vec![]);
        match &root.kind {
            ActionKind::Loop { name, .. } => assert_eq!(name, "root"),
            _ => panic!("expected loop"),
        }
        assert!(root.id.is_root());
    }

    #[test]
    fn set_variable_uses_binding_prefix() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                variable_name: "count".into(),
                value: serde_yaml::Value::Number(1.into()),
            },
        };
        let pills = a.tree_summary_pills();
        assert!(pills.iter().any(|p| p.prefix.as_deref() == Some("Variable")
            && p.text == "count"));
    }

    #[test]
    fn looks_like_var_ref_detects_dollar_and_braces() {
        assert!(looks_like_var_ref("${foo}"));
        assert!(looks_like_var_ref("  {bar}  "));
        assert!(!looks_like_var_ref("plain"));
        assert!(!looks_like_var_ref("${unclosed"));
        assert!(!looks_like_var_ref("{"));
    }

    #[test]
    fn split_filters_type_empty_and_splits_extra() {
        let params = vec![
            DisplayParam::new("Type", "move"),
            DisplayParam::new("Point", "Prog~A"),
            DisplayParam::new("Empty", "  "),
            DisplayParam::extra("Smooth", "true"),
        ];
        let (summary, extra) = split_display_params(&params);
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].label, "Point");
        assert_eq!(extra.len(), 1);
        assert_eq!(extra[0].label, "Smooth");
    }

    #[test]
    fn move_smooth_extras_are_tooltip_only() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Move {
                point: CoordinateRef("Prog~Spot".into()),
                smooth: true,
                smooth_low: 0.05,
                smooth_high: 0.2,
                smooth_delay_ms: 1,
            },
        };
        let params = a.display_params();
        let (summary, extra) = split_display_params(&params);
        assert!(summary.iter().any(|p| p.label == "Point"));
        assert!(extra.iter().any(|p| p.label == "Smooth"));
        assert!(extra.iter().any(|p| p.label == "Smooth low"));
    }

    #[test]
    fn conditional_summary_joins_clauses() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Conditional {
                name: "gate".into(),
                match_mode: MATCH_ANY.into(),
                clauses: vec![
                    ConditionClause {
                        left: ScalarValue::String("${a}".into()),
                        operator: "==".into(),
                        right: ScalarValue::String("1".into()),
                    },
                    ConditionClause {
                        left: ScalarValue::String("${b}".into()),
                        operator: "is set".into(),
                        right: ScalarValue::Null,
                    },
                ],
                subactions: vec![],
            },
        };
        let pills = a.tree_summary_pills();
        let if_pill = pills.iter().find(|p| p.text.contains('|')).expect("If pill");
        assert!(if_pill.text.contains("${a} == 1"));
        assert!(if_pill.text.contains("${b} is set"));
        assert!(pills.iter().any(|p| p.text.contains("any (OR)")));
    }

    #[test]
    fn find_pixel_shows_color_and_bindings() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::FindPixel {
                name: "red".into(),
                search_area: CoordinateRef("Prog~Box".into()),
                target_color: "#ff0000".into(),
                color_tolerance: 10,
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs {
                    output_x_variable: "px".into(),
                    output_y_variable: "py".into(),
                },
                run_branch_on_no_find: false,
                order: Default::default(),
                subactions: vec![],
            },
        };
        let pills = a.tree_summary_pills();
        assert!(pills.iter().any(|p| p.text == "#ff0000"));
        assert!(pills.iter().any(|p| p.prefix.as_deref() == Some("X") && p.text == "px"));
        assert!(pills.iter().any(|p| p.prefix.as_deref() == Some("Y") && p.text == "py"));
        assert_eq!(action_icon_glyph(&a), "🎨");
    }

    #[test]
    fn set_binding_uses_value_role() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                variable_name: "sum".into(),
                value: serde_yaml::Value::String("1+2".into()),
            },
        };
        let pills = a.tree_summary_pills();
        assert!(pills.iter().any(|p| {
            p.prefix.as_deref() == Some("Variable") && p.text == "sum"
        }));
    }

    #[test]
    fn pastel_dark_differs_from_light_and_glyphs_flip_with_state() {
        assert_ne!(
            action_pastel_color("click", false),
            action_pastel_color("click", true)
        );
        assert_ne!(nested_var_ref_color(false), nested_var_ref_color(true));
        let down = Action {
            id: ActionId::new(),
            kind: ActionKind::Click {
                button: "left".into(),
                state: true,
            },
        };
        let up = Action {
            id: ActionId::new(),
            kind: ActionKind::Click {
                button: "left".into(),
                state: false,
            },
        };
        assert_eq!(action_icon_glyph(&down), "⬇");
        assert_eq!(action_icon_glyph(&up), "⬆");
    }

    #[test]
    fn action_icon_glyphs_avoid_tofu_codepoints() {
        // These previously used Mathematical Alphanumeric / obscure symbols
        // missing from egui's proportional fonts (empty boxes).
        let cases = [
            (
                ActionKind::Type {
                    text: String::new(),
                    delay_ms: 0,
                },
                "⌨",
            ),
            (
                ActionKind::FocusWindow {
                    process_path: String::new(),
                    window_title: String::new(),
                },
                "👁",
            ),
            (
                ActionKind::SetVariable {
                    variable_name: "a".into(),
                    value: serde_yaml::Value::String("1+2".into()),
                },
                "x",
            ),
            (
                ActionKind::ForEachRow {
                    name: String::new(),
                    sources: vec![],
                    start_row: ScalarValue::Null,
                    end_row: ScalarValue::Null,
                    subactions: vec![],
                },
                "☰",
            ),
            (
                ActionKind::Ocr {
                    name: String::new(),
                    target: String::new(),
                    search_area: CoordinateRef(String::new()),
                    output_variable: String::new(),
                    coords: CoordinateOutputs::default(),
                    wait: WaitTilFoundConfig::default(),
                    run_branch_on_no_find: false,
                    blur: 0,
                    min_threshold: 0,
                    resize: 1.0,
                    grayscale: false,
                    threshold_otsu: false,
                    threshold_invert: false,
                    order: Default::default(),
                    subactions: vec![],
                },
                "🔤",
            ),
            (
                ActionKind::ImageSearch {
                    name: String::new(),
                    targets: vec![],
                    search_area: CoordinateRef(String::new()),
                    tolerance: 0.0,
                    blur: 0,
                    wait: WaitTilFoundConfig::default(),
                    coords: CoordinateOutputs::defaults(),
                    run_branch_on_no_find: false,
                    order: Default::default(),
                    subactions: vec![],
                },
                "🔍",
            ),
        ];
        for (kind, want) in cases {
            let a = Action {
                id: ActionId::new(),
                kind,
            };
            assert_eq!(action_icon_glyph(&a), want, "{}", a.type_key());
        }
    }

    #[test]
    fn parse_hex_short_form_and_display_text() {
        assert_eq!(parse_hex_color("#abc"), Some([0xaa, 0xbb, 0xcc, 255]));
        assert_eq!(parse_hex_color("not-hex"), None);
        let pill = SummaryPill {
            text: "x".into(),
            prefix: Some("X".into()),
        };
        assert_eq!(pill.display_text(), "X: x");
    }

    #[test]
    fn wait_display_mode_and_clause_summary() {
        let mut wait = WaitTilFoundConfig::default();
        assert_eq!(wait.display_wait_mode("instant"), "instant");
        wait.wait_til_found = true;
        wait.wait_til_found_seconds = 5;
        assert_eq!(
            wait.display_wait_mode("instant"),
            "5 seconds or until found"
        );
        let clause = ConditionClause {
            left: ScalarValue::String("name".into()),
            operator: "contains".into(),
            right: ScalarValue::String("foo".into()),
        };
        assert_eq!(clause.summary(), "name contains foo");
    }
}
