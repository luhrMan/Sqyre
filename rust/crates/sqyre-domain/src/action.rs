//! Action kinds and tree helpers.

use crate::{CoordinateRef, ScalarValue};
use uuid::Uuid;

/// Runtime action identity. Empty UUID string marks the macro root loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionId(pub Uuid);

impl ActionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn root() -> Self {
        Self(Uuid::nil())
    }

    pub fn is_root(self) -> bool {
        self.0.is_nil()
    }

    pub fn as_str(self) -> String {
        if self.is_root() {
            String::new()
        } else {
            self.0.to_string()
        }
    }
}

impl Default for ActionId {
    fn default() -> Self {
        Self::new()
    }
}

pub const OP_EQUALS: &str = "==";
pub const MATCH_ALL: &str = "all";
pub const MATCH_ANY: &str = "any";

pub const REPEAT_ONCE: &str = "once";
pub const REPEAT_WAIT_UNTIL_FOUND: &str = "waituntilfound";
pub const REPEAT_WHILE_FOUND: &str = "repeatwhilefound";

pub const DEFAULT_SMOOTH_LOW: f64 = 0.05;
pub const DEFAULT_SMOOTH_HIGH: f64 = 0.20;
pub const DEFAULT_SMOOTH_DELAY_MS: i32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionClause {
    pub left: ScalarValue,
    pub operator: String,
    pub right: ScalarValue,
}

impl Default for ConditionClause {
    fn default() -> Self {
        Self {
            left: ScalarValue::String(String::new()),
            operator: OP_EQUALS.to_string(),
            right: ScalarValue::String(String::new()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WaitTilFoundConfig {
    pub repeat_mode: String,
    pub wait_til_found: bool,
    pub wait_til_found_seconds: i32,
    pub wait_til_found_interval_ms: i32,
    pub max_iterations: i32,
}

impl WaitTilFoundConfig {
    pub fn effective_repeat_mode(&self) -> &str {
        match self.repeat_mode.as_str() {
            REPEAT_ONCE | REPEAT_WAIT_UNTIL_FOUND | REPEAT_WHILE_FOUND => &self.repeat_mode,
            _ if self.wait_til_found => REPEAT_WAIT_UNTIL_FOUND,
            _ => REPEAT_ONCE,
        }
    }

    /// Go `WaitTilFoundConfig.Active` — retry until found (or timeout).
    pub fn wait_until_found_active(&self) -> bool {
        self.effective_repeat_mode() == REPEAT_WAIT_UNTIL_FOUND && self.wait_til_found_seconds > 0
    }

    pub fn effective_interval_ms(&self, default_ms: i32) -> i32 {
        if self.wait_til_found_interval_ms > 0 {
            self.wait_til_found_interval_ms
        } else {
            default_ms
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CoordinateOutputs {
    pub output_x_variable: String,
    pub output_y_variable: String,
}

impl CoordinateOutputs {
    pub fn defaults() -> Self {
        Self {
            output_x_variable: "foundX".into(),
            output_y_variable: "foundY".into(),
        }
    }
}

/// Optional match-order fields present in newer `~/.sqyre` data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MatchOrder {
    pub grouping: String,
    pub horizontal: String,
    pub vertical: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListColumn {
    pub source: String,
    pub output_var: String,
    pub is_file: bool,
    pub skip_blank_lines: bool,
}

/// One node in a macro action tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    pub id: ActionId,
    pub kind: ActionKind,
}

impl Action {
    pub fn type_key(&self) -> &'static str {
        self.kind.type_key()
    }

    pub fn is_branch(&self) -> bool {
        self.kind.is_branch()
    }

    pub fn children(&self) -> &[Action] {
        self.kind.children()
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<Action>> {
        self.kind.children_mut()
    }

    pub fn display_name(&self) -> String {
        self.kind.display_name()
    }

    pub fn find_by_id(&self, id: ActionId) -> Option<&Action> {
        if self.id == id {
            return Some(self);
        }
        for child in self.children() {
            if let Some(found) = child.find_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    pub fn walk<F: FnMut(&Action)>(&self, f: &mut F) {
        f(self);
        for child in self.children() {
            child.walk(f);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    Loop {
        name: String,
        count: ScalarValue,
        subactions: Vec<Action>,
    },
    While {
        name: String,
        match_mode: String,
        clauses: Vec<ConditionClause>,
        max_iterations: i32,
        subactions: Vec<Action>,
    },
    Conditional {
        name: String,
        match_mode: String,
        clauses: Vec<ConditionClause>,
        subactions: Vec<Action>,
    },
    ImageSearch {
        name: String,
        targets: Vec<String>,
        search_area: CoordinateRef,
        row_split: i32,
        col_split: i32,
        tolerance: f64,
        blur: i32,
        wait: WaitTilFoundConfig,
        coords: CoordinateOutputs,
        run_branch_on_no_find: bool,
        order: MatchOrder,
        subactions: Vec<Action>,
    },
    Ocr {
        name: String,
        target: String,
        search_area: CoordinateRef,
        output_variable: String,
        coords: CoordinateOutputs,
        wait: WaitTilFoundConfig,
        run_branch_on_no_find: bool,
        blur: i32,
        min_threshold: i32,
        resize: f64,
        grayscale: bool,
        threshold_otsu: bool,
        threshold_invert: bool,
        order: MatchOrder,
        subactions: Vec<Action>,
    },
    FindPixel {
        name: String,
        search_area: CoordinateRef,
        target_color: String,
        color_tolerance: i32,
        wait: WaitTilFoundConfig,
        coords: CoordinateOutputs,
        run_branch_on_no_find: bool,
        order: MatchOrder,
        subactions: Vec<Action>,
    },
    ForEachRow {
        name: String,
        sources: Vec<ListColumn>,
        start_row: ScalarValue,
        end_row: ScalarValue,
        subactions: Vec<Action>,
    },
    Wait {
        time: ScalarValue,
    },
    Pause {
        message: String,
        continue_key: Vec<String>,
        pass_through: bool,
    },
    Move {
        point: CoordinateRef,
        smooth: bool,
        smooth_low: f64,
        smooth_high: f64,
        smooth_delay_ms: i32,
    },
    Click {
        button: String,
        state: bool,
    },
    Key {
        key: String,
        state: bool,
    },
    Type {
        text: String,
        delay_ms: i32,
    },
    SetVariable {
        variable_name: String,
        value: serde_yaml::Value,
    },
    Calculate {
        expression: String,
        output_var: String,
    },
    SaveVariable {
        variable_name: String,
        destination: String,
        append: bool,
        append_newline: bool,
    },
    FocusWindow {
        process_path: String,
        window_title: String,
    },
    RunMacro {
        macro_name: String,
    },
    NavigateSelect {
        program: String,
        graph_name: String,
        chord_up: Vec<String>,
        chord_down: Vec<String>,
        chord_left: Vec<String>,
        chord_right: Vec<String>,
        chord_select: Vec<String>,
        chord_back: Vec<String>,
        wrap_edges: bool,
        move_cursor_with_nav: bool,
        smooth: bool,
        pass_through: bool,
        hold_repeat: bool,
        select_device: String,
        select_button: String,
        select_key: String,
        select_press_mode: String,
        in_graph: String,
        in_row: String,
        in_col: String,
        in_collection: String,
        output_ref: String,
        output_graph: String,
        output_row: String,
        output_col: String,
        output_collection: String,
    },
    Break,
    Continue,
}

impl ActionKind {
    pub fn type_key(&self) -> &'static str {
        match self {
            Self::Loop { .. } => "loop",
            Self::While { .. } => "while",
            Self::Conditional { .. } => "conditional",
            Self::ImageSearch { .. } => "imagesearch",
            Self::Ocr { .. } => "ocr",
            Self::FindPixel { .. } => "findpixel",
            Self::ForEachRow { .. } => "foreachrow",
            Self::Wait { .. } => "wait",
            Self::Pause { .. } => "pause",
            Self::Move { .. } => "move",
            Self::Click { .. } => "click",
            Self::Key { .. } => "key",
            Self::Type { .. } => "type",
            Self::SetVariable { .. } => "setvariable",
            Self::Calculate { .. } => "calculate",
            Self::SaveVariable { .. } => "savevariable",
            Self::FocusWindow { .. } => "focuswindow",
            Self::RunMacro { .. } => "runmacro",
            Self::NavigateSelect { .. } => "navigateselect",
            Self::Break => "break",
            Self::Continue => "continue",
        }
    }

    pub fn is_branch(&self) -> bool {
        matches!(
            self,
            Self::Loop { .. }
                | Self::While { .. }
                | Self::Conditional { .. }
                | Self::ImageSearch { .. }
                | Self::Ocr { .. }
                | Self::FindPixel { .. }
                | Self::ForEachRow { .. }
        )
    }

    pub fn children(&self) -> &[Action] {
        match self {
            Self::Loop { subactions, .. }
            | Self::While { subactions, .. }
            | Self::Conditional { subactions, .. }
            | Self::ImageSearch { subactions, .. }
            | Self::Ocr { subactions, .. }
            | Self::FindPixel { subactions, .. }
            | Self::ForEachRow { subactions, .. } => subactions,
            _ => &[],
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<Action>> {
        match self {
            Self::Loop { subactions, .. }
            | Self::While { subactions, .. }
            | Self::Conditional { subactions, .. }
            | Self::ImageSearch { subactions, .. }
            | Self::Ocr { subactions, .. }
            | Self::FindPixel { subactions, .. }
            | Self::ForEachRow { subactions, .. } => Some(subactions),
            _ => None,
        }
    }

    pub fn display_name(&self) -> String {
        use crate::action_type_label;
        let label = action_type_label(self.type_key());
        match self {
            Self::Loop { name, .. }
            | Self::While { name, .. }
            | Self::Conditional { name, .. }
            | Self::ImageSearch { name, .. }
            | Self::Ocr { name, .. }
            | Self::FindPixel { name, .. }
            | Self::ForEachRow { name, .. } => {
                if name.trim().is_empty() || name == "root" {
                    label.to_string()
                } else {
                    format!("{label}: {name}")
                }
            }
            Self::Wait { time } => format!("Wait {}", time.as_display()),
            Self::Move { point, .. } => format!("Move {}", point.display_label()),
            Self::Click { button, state } => {
                format!("Click {button} {}", if *state { "down" } else { "up" })
            }
            Self::Key { key, state } => {
                format!("Key {key} {}", if *state { "down" } else { "up" })
            }
            Self::Type { text, .. } => format!("Type {text}"),
            Self::SetVariable { variable_name, .. } => format!("Set {variable_name}"),
            Self::Calculate { output_var, .. } => format!("Calculate → {output_var}"),
            Self::SaveVariable {
                variable_name,
                destination,
                ..
            } => format!("Save {variable_name} → {destination}"),
            Self::FocusWindow { window_title, .. } => {
                if window_title.trim().is_empty() {
                    "Focus window".into()
                } else {
                    format!("Focus {window_title}")
                }
            }
            Self::RunMacro { macro_name } => format!("Run {macro_name}"),
            Self::NavigateSelect {
                program,
                graph_name,
                ..
            } => {
                if program.is_empty() && graph_name.is_empty() {
                    label.to_string()
                } else {
                    format!("{label}: {program} · {graph_name}")
                }
            }
            Self::Pause { message, .. } => {
                if message.trim().is_empty() {
                    label.to_string()
                } else {
                    format!("Pause: {message}")
                }
            }
            Self::Break | Self::Continue => label.to_string(),
        }
    }
}

/// Creates the empty macro root loop (`name: root`, nil UID, count 1).
pub fn root_loop(subactions: Vec<Action>) -> Action {
    Action {
        id: ActionId::root(),
        kind: ActionKind::Loop {
            name: "root".into(),
            count: ScalarValue::Int(1),
            subactions,
        },
    }
}
