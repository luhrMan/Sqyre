//! Action kind enum and tree-shape helpers.

use super::{
    Action, ConditionBlock, CoordinateRef, DetectionBranch, ListColumn, LoopJumpMode, MatchMethod,
    MouseButton, NavigateSelectData, PressState, ScalarValue, VariableAssignment,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    Loop {
        name: String,
        count: ScalarValue,
        subactions: Vec<Action>,
    },
    While {
        condition: ConditionBlock,
        max_iterations: i32,
        subactions: Vec<Action>,
    },
    Conditional {
        condition: ConditionBlock,
        subactions: Vec<Action>,
        else_actions: Vec<Action>,
    },
    ImageSearch {
        name: String,
        targets: Vec<String>,
        search_area: CoordinateRef,
        tolerance: f64,
        blur: i32,
        match_method: MatchMethod,
        detection: DetectionBranch,
    },
    Ocr {
        name: String,
        target: String,
        search_area: CoordinateRef,
        output_variable: String,
        blur: i32,
        min_threshold: i32,
        resize: f64,
        grayscale: bool,
        threshold_otsu: bool,
        threshold_invert: bool,
        detection: DetectionBranch,
    },
    FindPixel {
        name: String,
        search_area: CoordinateRef,
        target_color: String,
        color_tolerance: i32,
        detection: DetectionBranch,
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
        button: MouseButton,
        state: PressState,
    },
    Key {
        key: String,
        state: PressState,
    },
    Type {
        text: String,
        delay_ms: i32,
    },
    SetVariable {
        assignments: Vec<VariableAssignment>,
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
    NavigateSelect(Box<NavigateSelectData>),
    /// User-defined key branch under [`ActionKind::NavigateSelect`].
    NavigateKey {
        name: String,
        chord: Vec<String>,
        /// When true, leave the parent Navigate Select after children finish.
        exit: bool,
        subactions: Vec<Action>,
    },
    LoopJump {
        mode: LoopJumpMode,
    },
}

impl ActionKind {
    /// Default instance for a YAML/wire type key (`"wait"`, `"imagesearch"`, …).
    pub fn from_type_key(key: &str) -> Option<Self> {
        crate::blank::blank_kind(key)
    }

    // `type_key` is generated in `wire_keys` from the wire-key registry.

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
                | Self::NavigateSelect(_)
                | Self::NavigateKey { .. }
        )
    }

    pub fn is_detection(&self) -> bool {
        matches!(
            self,
            Self::ImageSearch { .. } | Self::Ocr { .. } | Self::FindPixel { .. }
        )
    }

    pub fn has_else_folder(&self) -> bool {
        matches!(
            self,
            Self::Conditional { .. }
                | Self::ImageSearch { .. }
                | Self::Ocr { .. }
                | Self::FindPixel { .. }
        )
    }

    pub fn else_actions(&self) -> Option<&[Action]> {
        match self {
            Self::Conditional { else_actions, .. } => Some(else_actions),
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(&detection.else_actions),
            _ => None,
        }
    }

    pub fn else_actions_mut(&mut self) -> Option<&mut Vec<Action>> {
        match self {
            Self::Conditional { else_actions, .. } => Some(else_actions),
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(&mut detection.else_actions),
            _ => None,
        }
    }

    pub fn detection(&self) -> Option<&DetectionBranch> {
        match self {
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(detection),
            _ => None,
        }
    }

    pub fn detection_mut(&mut self) -> Option<&mut DetectionBranch> {
        match self {
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(detection),
            _ => None,
        }
    }

    pub fn children(&self) -> &[Action] {
        match self {
            Self::Loop { subactions, .. }
            | Self::While { subactions, .. }
            | Self::Conditional { subactions, .. }
            | Self::ForEachRow { subactions, .. }
            | Self::NavigateKey { subactions, .. } => subactions,
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => &detection.subactions,
            Self::NavigateSelect(data) => &data.subactions,
            _ => &[],
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<Action>> {
        match self {
            Self::Loop { subactions, .. }
            | Self::While { subactions, .. }
            | Self::Conditional { subactions, .. }
            | Self::ForEachRow { subactions, .. }
            | Self::NavigateKey { subactions, .. } => Some(subactions),
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(&mut detection.subactions),
            Self::NavigateSelect(data) => Some(&mut data.subactions),
            _ => None,
        }
    }

    pub fn display_name(&self) -> String {
        use crate::action_type_label;
        let label = action_type_label(self.type_key());
        match self {
            Self::Loop { name, .. }
            | Self::ImageSearch { name, .. }
            | Self::Ocr { name, .. }
            | Self::FindPixel { name, .. }
            | Self::ForEachRow { name, .. } => named_branch_label(label, name),
            Self::While { condition, .. } | Self::Conditional { condition, .. } => {
                named_branch_label(label, &condition.name)
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
                format!("Click {button} {state}")
            }
            Self::Key { key, state } => {
                format!("Key {key} {state}")
            }
            Self::Type { text, .. } => format!("Type {text}"),
            Self::SetVariable { assignments } => {
                let names: Vec<&str> = assignments
                    .iter()
                    .map(|a| a.variable_name.as_str())
                    .filter(|n| !n.is_empty())
                    .collect();
                if names.is_empty() {
                    label.to_string()
                } else {
                    format!("Set {}", names.join(", "))
                }
            }
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
            Self::NavigateSelect(data) => {
                if data.program.is_empty() && data.atlas.is_empty() {
                    label.to_string()
                } else {
                    format!("{label}: {} · {}", data.program, data.atlas)
                }
            }
            Self::Pause { message, .. } => {
                if message.trim().is_empty() {
                    label.to_string()
                } else {
                    format!("Pause: {message}")
                }
            }
            Self::LoopJump { mode } => match mode {
                LoopJumpMode::Break => "Break".into(),
                LoopJumpMode::Continue => "Continue".into(),
            },
        }
    }
}

fn named_branch_label(label: &str, name: &str) -> String {
    if name.trim().is_empty() || name == "root" {
        label.to_string()
    } else {
        format!("{label}: {name}")
    }
}
