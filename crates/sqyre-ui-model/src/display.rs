//! Display params and tree summary pills for actions.

use sqyre_domain::{
    Action, ActionKind, ConditionClause, MatchMode, RepeatMode, ScalarValue, WaitTilFoundConfig,
};

/// One display parameter.
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

/// Wait / repeat mode summary shown in the tree and tooltips.
pub trait WaitDisplay {
    fn display_wait_mode(&self, instant_label: &str) -> String;
}

impl WaitDisplay for WaitTilFoundConfig {
    fn display_wait_mode(&self, instant_label: &str) -> String {
        match self.repeat_mode {
            RepeatMode::WaitUntilFound => {
                if self.wait_til_found_seconds > 0 {
                    format!("{} seconds or until found", self.wait_til_found_seconds)
                } else {
                    format!("wait {}s", self.wait_til_found_seconds)
                }
            }
            RepeatMode::WhileFound => {
                if self.wait_til_found_seconds > 0 {
                    format!("repeat while found ({}s)", self.wait_til_found_seconds)
                } else {
                    "repeat while found".to_string()
                }
            }
            RepeatMode::Once => instant_label.to_string(),
        }
    }
}

/// One-line condition clause summary (e.g. `${a} == 1`).
pub trait ConditionDisplay {
    fn summary(&self) -> String;
}

impl ConditionDisplay for ConditionClause {
    fn summary(&self) -> String {
        let left = self.left.as_display();
        let right = self.right.as_display();
        match self.operator.as_str() {
            "is set" | "is empty" => format!("{left} {}", self.operator),
            _ => format!("{left} {} {right}", self.operator),
        }
    }
}

fn press_state_label(state: sqyre_domain::PressState) -> &'static str {
    state.as_str()
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

fn yaml_display(v: &ScalarValue) -> String {
    v.as_display()
}

fn match_label(mode: MatchMode) -> &'static str {
    match mode {
        MatchMode::Any => "any (OR)",
        MatchMode::All => "all (AND)",
    }
}

fn condition_summary(match_mode: MatchMode, clauses: &[ConditionClause]) -> String {
    let sep = match match_mode {
        MatchMode::Any => " | ",
        MatchMode::All => " & ",
    };
    clauses
        .iter()
        .map(ConditionClause::summary)
        .collect::<Vec<_>>()
        .join(sep)
}

fn format_float(f: f64) -> String {
    let s = format!("{f:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Params for tree/tooltip display, plus derived tree pills.
pub trait ActionDisplay {
    /// Params for tree/tooltip display.
    fn display_params(&self) -> Vec<DisplayParam>;

    /// Tree-row params: Image Search omits Items (shown as thumbs/count instead).
    fn display_params_for_tree(&self) -> Vec<DisplayParam>;

    /// Inline summary pill texts (primary params only; no output bindings).
    fn tree_summary_pills(&self) -> Vec<SummaryPill>;
}

impl ActionDisplay for Action {
    fn display_params(&self) -> Vec<DisplayParam> {
        self.kind.display_params()
    }

    fn display_params_for_tree(&self) -> Vec<DisplayParam> {
        let params = self.display_params();
        if self.type_key() != "imagesearch" {
            return params;
        }
        params
            .into_iter()
            .filter(|p| !p.label.eq_ignore_ascii_case("Items"))
            .collect()
    }

    fn tree_summary_pills(&self) -> Vec<SummaryPill> {
        let params = self.display_params_for_tree();
        let (summary, _) = split_display_params(&params);
        // Prefix when a summary value is also the action's set-variable name;
        // do not append produced outputs (X/Y, OCR text, nav refs, etc.).
        let binding_labels: std::collections::HashMap<String, String> = self
            .variable_bindings()
            .into_iter()
            .filter(|b| matches!(b.role, sqyre_domain::BindingRole::Value))
            .map(|b| {
                let label = b.role.pill_label().to_string();
                (b.name, label)
            })
            .collect();

        summary
            .into_iter()
            .map(|p| {
                let text = p.minimal().to_string();
                let prefix = binding_labels.get(&text).cloned();
                SummaryPill { text, prefix }
            })
            .collect()
    }
}

/// Internal: params for a single action kind (delegated to by [`ActionDisplay`]).
trait ActionKindDisplay {
    fn display_params(&self) -> Vec<DisplayParam>;
}

impl ActionKindDisplay for ActionKind {
    fn display_params(&self) -> Vec<DisplayParam> {
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
                    params.push(DisplayParam::extra(
                        "Smooth high",
                        format_float(*smooth_high),
                    ));
                    params.push(DisplayParam::extra(
                        "Smooth delay (ms)",
                        smooth_delay_ms.to_string(),
                    ));
                }
            }
            Self::Click { button, state } => {
                params.push(DisplayParam::new("Button", button.as_str()));
                params.push(DisplayParam::new("State", press_state_label(*state)));
            }
            Self::Key { key, state } => {
                params.push(DisplayParam::new("Key", key.as_str()));
                params.push(DisplayParam::new("State", press_state_label(*state)));
            }
            Self::Type { text, delay_ms } => {
                params.push(DisplayParam::new("Text", format!("{text:?}")));
                if *delay_ms > 0 {
                    params.push(DisplayParam::extra("Delay", format!("{delay_ms} ms")));
                }
            }
            Self::Loop { name, count, .. } => {
                params.push(DisplayParam::new("Iterations", count.as_display()));
                params.push(DisplayParam::new("Name", name.as_str()));
            }
            Self::While {
                condition,
                max_iterations,
                ..
            } => {
                params.push(DisplayParam::new("Name", condition.name.as_str()));
                params.push(DisplayParam::new(
                    "Match",
                    match_label(condition.match_mode),
                ));
                params.push(DisplayParam::extra(
                    "While",
                    condition_summary(condition.match_mode, &condition.clauses),
                ));
                if *max_iterations > 0 {
                    params.push(DisplayParam::extra(
                        "Max iterations",
                        max_iterations.to_string(),
                    ));
                }
            }
            Self::Conditional { condition, .. } => {
                params.push(DisplayParam::new("Name", condition.name.as_str()));
                params.push(DisplayParam::new(
                    "Match",
                    match_label(condition.match_mode),
                ));
                params.push(DisplayParam::extra(
                    "If",
                    condition_summary(condition.match_mode, &condition.clauses),
                ));
            }
            Self::ImageSearch {
                name,
                targets,
                search_area,
                tolerance,
                blur,
                detection,
            } => {
                params.push(DisplayParam::new("Name", name.as_str()));
                params.push(DisplayParam::new("Items", targets.len().to_string()));
                params.push(DisplayParam::extra(
                    "Search Area",
                    search_area.display_label(),
                ));
                params.push(DisplayParam::extra(
                    "Wait",
                    detection.wait.display_wait_mode("instant"),
                ));
                params.push(DisplayParam::extra("Tolerance", format_float(*tolerance)));
                params.push(DisplayParam::extra("Blur", blur.to_string()));
                if detection.run_branch_on_no_find {
                    params.push(DisplayParam::extra("Run on no find", "yes"));
                }
            }
            Self::Ocr {
                name,
                target,
                search_area,
                detection,
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
                    detection.wait.display_wait_mode("instant"),
                ));
            }
            Self::FindPixel {
                name,
                search_area,
                target_color,
                color_tolerance,
                detection,
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
                    detection.wait.display_wait_mode("instant"),
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
            Self::SetVariable { assignments } => {
                for a in assignments {
                    if a.variable_name.is_empty() {
                        continue;
                    }
                    params.push(DisplayParam::new("Variable", a.variable_name.as_str()));
                    params.push(DisplayParam::new("Value", yaml_display(&a.value)));
                }
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
            Self::NavigateSelect(data) => {
                if !data.program.is_empty() {
                    params.push(DisplayParam::new("Program", data.program.as_str()));
                }
                if !data.graph_name.is_empty() {
                    params.push(DisplayParam::new("Graph", data.graph_name.as_str()));
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
}
