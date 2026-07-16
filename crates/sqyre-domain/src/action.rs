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

#[derive(Debug, Clone, PartialEq)]
pub struct WaitTilFoundConfig {
    pub repeat_mode: String,
    pub wait_til_found_seconds: i32,
    pub wait_til_found_interval_ms: i32,
    pub max_iterations: i32,
}

impl Default for WaitTilFoundConfig {
    fn default() -> Self {
        Self {
            repeat_mode: REPEAT_ONCE.to_string(),
            wait_til_found_seconds: 0,
            wait_til_found_interval_ms: 0,
            max_iterations: 0,
        }
    }
}

impl WaitTilFoundConfig {
    pub fn effective_repeat_mode(&self) -> &str {
        match self.repeat_mode.as_str() {
            REPEAT_ONCE | REPEAT_WAIT_UNTIL_FOUND | REPEAT_WHILE_FOUND => &self.repeat_mode,
            _ => REPEAT_ONCE,
        }
    }

    /// Retry until found (or timeout).
    pub fn wait_until_found_active(&self) -> bool {
        self.effective_repeat_mode() == REPEAT_WAIT_UNTIL_FOUND && self.wait_til_found_seconds > 0
    }

    /// When true, repeat while the target remains found.
    pub fn is_repeat_while_found(&self) -> bool {
        self.effective_repeat_mode() == REPEAT_WHILE_FOUND
    }

    pub fn effective_interval_ms(&self, default_ms: i32) -> i32 {
        if self.wait_til_found_interval_ms > 0 {
            self.wait_til_found_interval_ms
        } else {
            default_ms
        }
    }

    /// Max iterations for wait-until-found (default 100 when unset).
    pub fn effective_max_iterations(&self) -> i32 {
        if self.max_iterations > 0 {
            self.max_iterations
        } else {
            100
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

/// Runtime builtins set inside ForEachRow sub-actions (1-based row index).
pub const FOREACH_ROW_BUILTIN_ROW: &str = "Row";
/// Total line count of the driving (first) ForEachRow source.
pub const FOREACH_ROW_BUILTIN_ROW_COUNT: &str = "RowCount";

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

    pub fn find_by_id_mut(&mut self, id: ActionId) -> Option<&mut Action> {
        if self.id == id {
            return Some(self);
        }
        Self::find_descendant_mut(self, id)
    }

    fn find_descendant_mut(node: &mut Action, id: ActionId) -> Option<&mut Action> {
        let children = node.children_mut()?;
        for child in children.iter_mut() {
            if child.id == id {
                return Some(child);
            }
            if let Some(found) = Self::find_descendant_mut(child, id) {
                return Some(found);
            }
        }
        None
    }

    /// Remove a descendant by id (not self). Returns the detached node.
    pub fn remove_by_id(&mut self, id: ActionId) -> Option<Action> {
        let children = self.children_mut()?;
        if let Some(i) = children.iter().position(|c| c.id == id) {
            return Some(children.remove(i));
        }
        for child in children.iter_mut() {
            if let Some(found) = child.remove_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// True if `id` is this node or any descendant.
    pub fn contains_id(&self, id: ActionId) -> bool {
        self.find_by_id(id).is_some()
    }

    /// Insert `child` into the children of `parent_id` at `slot`.
    pub fn insert_at(
        &mut self,
        parent_id: ActionId,
        slot: InsertSlot,
        child: Action,
    ) -> Result<(), String> {
        let parent = self
            .find_by_id_mut(parent_id)
            .ok_or_else(|| format!("parent action {} not found", parent_id.as_str()))?;
        let children = parent
            .children_mut()
            .ok_or_else(|| "drop target is not a branch".to_string())?;
        match slot {
            InsertSlot::First => children.insert(0, child),
            InsertSlot::Last => children.push(child),
            InsertSlot::Before(sib) => {
                let i = children
                    .iter()
                    .position(|c| c.id == sib)
                    .ok_or_else(|| "before-sibling not found".to_string())?;
                children.insert(i, child);
            }
            InsertSlot::After(sib) => {
                let i = children
                    .iter()
                    .position(|c| c.id == sib)
                    .ok_or_else(|| "after-sibling not found".to_string())?;
                children.insert(i + 1, child);
            }
        }
        Ok(())
    }

    /// Move `source_id` under `parent_id` at `slot`. Rejects self-drops and
    /// dropping a node into its own descendant.
    pub fn move_action(
        &mut self,
        source_id: ActionId,
        parent_id: ActionId,
        slot: InsertSlot,
    ) -> Result<(), String> {
        if source_id == parent_id {
            return Err("cannot drop onto self".into());
        }
        if let Some(src) = self.find_by_id(source_id) {
            if src.contains_id(parent_id) {
                return Err("cannot drop into own descendant".into());
            }
        }
        match slot {
            InsertSlot::Before(id) | InsertSlot::After(id) if id == source_id => {
                return Ok(());
            }
            _ => {}
        }
        let node = self
            .remove_by_id(source_id)
            .ok_or_else(|| format!("source action {} not found", source_id.as_str()))?;
        self.insert_at(parent_id, slot, node)
    }

    pub fn walk<F: FnMut(&Action)>(&self, f: &mut F) {
        f(self);
        for child in self.children() {
            child.walk(f);
        }
    }

    pub fn walk_mut<F: FnMut(&mut Action)>(&mut self, f: &mut F) {
        f(self);
        if let Some(children) = self.children_mut() {
            for child in children.iter_mut() {
                child.walk_mut(f);
            }
        }
    }
}

/// Insertion slot relative to a parent directory (mirrors egui_ltreeview DirPosition).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertSlot {
    First,
    Last,
    Before(ActionId),
    After(ActionId),
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
    /// Interactive grid navigator. Built-in chords move / select / back; each
    /// [`NavigateKey`] child is a user-defined chord that runs its branch.
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
        /// Direct children should be [`ActionKind::NavigateKey`] branches.
        subactions: Vec<Action>,
    },
    /// User-defined key branch under [`ActionKind::NavigateSelect`].
    NavigateKey {
        name: String,
        chord: Vec<String>,
        /// When true, leave the parent Navigate Select after children finish.
        exit: bool,
        subactions: Vec<Action>,
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
            Self::SaveVariable { .. } => "savevariable",
            Self::FocusWindow { .. } => "focuswindow",
            Self::RunMacro { .. } => "runmacro",
            Self::NavigateSelect { .. } => "navigateselect",
            Self::NavigateKey { .. } => "navigatekey",
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
                | Self::NavigateSelect { .. }
                | Self::NavigateKey { .. }
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
            | Self::ForEachRow { subactions, .. }
            | Self::NavigateSelect { subactions, .. }
            | Self::NavigateKey { subactions, .. } => subactions,
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
            | Self::ForEachRow { subactions, .. }
            | Self::NavigateSelect { subactions, .. }
            | Self::NavigateKey { subactions, .. } => Some(subactions),
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
            Self::NavigateKey { name, chord, .. } => {
                let chord_s = if chord.is_empty() {
                    "…".to_string()
                } else {
                    chord.join("+")
                };
                if name.trim().is_empty() {
                    format!("{label} [{chord_s}]")
                } else {
                    format!("{label}: {name} [{chord_s}]")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn wait(id: ActionId) -> Action {
        Action {
            id,
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        }
    }

    #[test]
    fn move_action_reorders_siblings() {
        let a = ActionId::new();
        let b = ActionId::new();
        let c = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b), wait(c)]);
        root.move_action(c, ActionId::root(), InsertSlot::Before(a))
            .unwrap();
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![c, a, b]);
    }

    #[test]
    fn move_action_rejects_into_self_descendant() {
        let branch_id = ActionId::new();
        let child_id = ActionId::new();
        let mut root = root_loop(vec![Action {
            id: branch_id,
            kind: ActionKind::Loop {
                name: "inner".into(),
                count: ScalarValue::Int(1),
                subactions: vec![wait(child_id)],
            },
        }]);
        assert!(root
            .move_action(branch_id, branch_id, InsertSlot::Last)
            .is_err());
    }
}
