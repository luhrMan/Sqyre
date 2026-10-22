//! Action kind enum and tree-shape helpers.

use super::{
    Action, ConditionBlock, CoordinateRef, DetectionBranch, ItemSortBy, ItemSortThen, ListColumn,
    LoopJumpMode, MatchMethod, MouseButton, NavigateSelectData, PressState, ScalarValue,
    VariableAssignment,
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
        /// Catalog item tags: every item with any of these tags is searched
        /// (union with [`targets`](Self::ImageSearch::targets), resolved at run time).
        target_tags: Vec<String>,
        search_area: CoordinateRef,
        tolerance: f64,
        blur: i32,
        match_method: MatchMethod,
        /// Primary order for matching items and for the Items grid display.
        sort_by: ItemSortBy,
        /// Within-group order after [`sort_by`](Self::ImageSearch::sort_by).
        sort_then: ItemSortThen,
        /// Tag priority when sort_by is Tags.
        tag_priority: Vec<String>,
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
    /// Runs nested actions once per 1×1 cell in a Collection range.
    ForEachCell {
        name: String,
        /// Must be a Collection cell range (`Program~Name@r1,c1-r2,c2`).
        cells: CoordinateRef,
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
        /// Focus the window that held focus before the run's last Focus Window
        /// instead of `process_path` / `window_title` (kept so the toggle can flip back).
        previous: bool,
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
                | Self::ForEachCell { .. }
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
            | Self::ForEachCell { subactions, .. }
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
            | Self::ForEachCell { subactions, .. }
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
            | Self::ForEachRow { name, .. }
            | Self::ForEachCell { name, .. } => named_branch_label(label, name),
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
            Self::FocusWindow { previous: true, .. } => "Focus previous window".into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blank::blank_kind;

    fn kind(key: &str) -> ActionKind {
        blank_kind(key).unwrap_or_else(|| panic!("no blank kind for {key}"))
    }

    const BRANCH_KEYS: &[&str] = &[
        "loop",
        "while",
        "conditional",
        "imagesearch",
        "ocr",
        "findpixel",
        "foreachrow",
        "foreachcell",
        "navigateselect",
        "navigatekey",
    ];

    const LEAF_KEYS: &[&str] = &[
        "wait",
        "pause",
        "move",
        "click",
        "key",
        "type",
        "setvariable",
        "savevariable",
        "focuswindow",
        "runmacro",
        "loopjump",
    ];

    #[test]
    fn is_branch_matches_branch_kinds_only() {
        for key in BRANCH_KEYS {
            assert!(kind(key).is_branch(), "{key}");
        }
        for key in LEAF_KEYS {
            assert!(!kind(key).is_branch(), "{key}");
        }
    }

    #[test]
    fn detection_flags_only_searches() {
        for key in ["imagesearch", "ocr", "findpixel"] {
            let mut k = kind(key);
            assert!(k.is_detection(), "{key}");
            assert!(k.has_else_folder(), "{key}");
            assert!(k.detection().is_some(), "{key}");
            assert!(k.detection_mut().is_some(), "{key}");
            assert!(k.else_actions().is_some(), "{key}");
            assert!(k.else_actions_mut().is_some(), "{key}");
        }
        for key in ["conditional", "loop", "while", "wait"] {
            let mut k = kind(key);
            assert!(!k.is_detection(), "{key}");
            assert!(k.detection().is_none(), "{key}");
            assert!(k.detection_mut().is_none(), "{key}");
        }
        // Conditional is not a detection but still exposes an else folder.
        let conditional = kind("conditional");
        assert!(!conditional.is_detection());
        assert!(conditional.has_else_folder());
        assert!(conditional.else_actions().is_some());
        assert!(!kind("loop").has_else_folder());
        assert!(kind("loop").else_actions().is_none());
        assert!(kind("wait").else_actions_mut().is_none());
    }

    #[test]
    fn else_actions_mut_writes_into_kind() {
        let mut k = kind("conditional");
        k.else_actions_mut()
            .unwrap()
            .push(crate::test_action(kind("wait")));
        assert_eq!(k.else_actions().unwrap().len(), 1);

        let mut search = kind("findpixel");
        search
            .else_actions_mut()
            .unwrap()
            .push(crate::test_action(kind("wait")));
        assert_eq!(search.else_actions().unwrap().len(), 1);
    }

    #[test]
    fn children_accessors_cover_each_container() {
        // Subactions containers (blanks may seed children, e.g. ForEachCell).
        for key in [
            "loop",
            "while",
            "conditional",
            "foreachrow",
            "foreachcell",
            "navigatekey",
        ] {
            let mut k = kind(key);
            let before = k.children().len();
            k.children_mut()
                .unwrap_or_else(|| panic!("{key} children_mut"))
                .push(crate::test_action(kind("wait")));
            assert_eq!(k.children().len(), before + 1, "{key}");
        }
        // Detection-backed containers.
        for key in ["imagesearch", "ocr", "findpixel"] {
            let mut k = kind(key);
            let before = k.children().len();
            k.children_mut()
                .unwrap_or_else(|| panic!("{key} children_mut"))
                .push(crate::test_action(kind("wait")));
            assert_eq!(k.children().len(), before + 1, "{key}");
        }
        // NavigateSelect children live on the boxed data.
        let mut nav = kind("navigateselect");
        nav.children_mut()
            .unwrap()
            .push(crate::test_action(kind("wait")));
        assert_eq!(nav.children().len(), 1);
        // Leaf kinds expose no children.
        let mut wait = kind("wait");
        assert!(wait.children().is_empty());
        assert!(wait.children_mut().is_none());
    }

    #[test]
    fn display_name_navigate_key_variants() {
        let named = ActionKind::NavigateKey {
            name: "Inspect".into(),
            chord: vec!["ctrl".into(), "i".into()],
            exit: false,
            subactions: Vec::new(),
        };
        assert!(named.display_name().contains("Inspect [ctrl+i]"));

        let unnamed = ActionKind::NavigateKey {
            name: String::new(),
            chord: vec!["f9".into()],
            exit: false,
            subactions: Vec::new(),
        };
        assert!(unnamed.display_name().ends_with("[f9]"));
        assert!(!unnamed.display_name().contains(':'));

        let no_chord = ActionKind::NavigateKey {
            name: "X".into(),
            chord: Vec::new(),
            exit: false,
            subactions: Vec::new(),
        };
        assert!(no_chord.display_name().contains('…'));
    }

    #[test]
    fn display_name_leaf_kinds() {
        assert_eq!(
            kind("wait").display_name(),
            format!("Wait {}", ScalarValue::Int(0).as_display())
        );
        let click = kind("click").display_name();
        assert!(click.starts_with("Click "), "{click}");
        let key = kind("key").display_name();
        assert!(key.starts_with("Key "), "{key}");
        assert_eq!(
            ActionKind::FocusWindow {
                process_path: String::new(),
                window_title: String::new(),
                previous: true,
            }
            .display_name(),
            "Focus previous window"
        );
        assert_eq!(
            ActionKind::FocusWindow {
                process_path: String::new(),
                window_title: "Game".into(),
                previous: false,
            }
            .display_name(),
            "Focus Game"
        );
        assert_eq!(
            ActionKind::FocusWindow {
                process_path: String::new(),
                window_title: String::new(),
                previous: false,
            }
            .display_name(),
            "Focus window"
        );
        let run = kind("runmacro").display_name();
        assert!(run.starts_with("Run "), "{run}");
        assert_eq!(
            ActionKind::LoopJump {
                mode: LoopJumpMode::Break
            }
            .display_name(),
            "Break"
        );
        assert_eq!(
            ActionKind::LoopJump {
                mode: LoopJumpMode::Continue
            }
            .display_name(),
            "Continue"
        );
        // Pause with/without message.
        assert!(!kind("pause").display_name().contains(':'));
        assert_eq!(
            ActionKind::Pause {
                message: "check map".into(),
                continue_key: Vec::new(),
                pass_through: false,
            }
            .display_name(),
            "Pause: check map"
        );
        // SetVariable lists assigned names, falls back to the bare label.
        assert!(!kind("setvariable").display_name().contains("Set ,"));
        assert_eq!(
            ActionKind::SetVariable {
                assignments: vec![VariableAssignment {
                    variable_name: "score".into(),
                    value: ScalarValue::Int(1),
                }]
            }
            .display_name(),
            "Set score"
        );
    }

    #[test]
    fn display_name_named_branches() {
        // Empty name → bare label; named → "Label: name".
        let bare = kind("loop").display_name();
        assert!(!bare.contains(':'), "{bare}");
        let named = ActionKind::Loop {
            name: "farm".into(),
            count: ScalarValue::Int(3),
            subactions: Vec::new(),
        }
        .display_name();
        assert!(named.ends_with(": farm"), "{named}");

        // While/Conditional label from the condition name.
        let cond = ActionKind::Conditional {
            condition: ConditionBlock {
                name: "hp low".into(),
                ..Default::default()
            },
            subactions: Vec::new(),
            else_actions: Vec::new(),
        }
        .display_name();
        assert!(cond.ends_with(": hp low"), "{cond}");

        // NavigateSelect shows program · atlas when set.
        let nav = ActionKind::NavigateSelect(Box::new(NavigateSelectData {
            program: "Game".into(),
            atlas: "inv".into(),
            ..Default::default()
        }))
        .display_name();
        assert!(nav.ends_with(": Game · inv"), "{nav}");
        let nav_empty = kind("navigateselect").display_name();
        assert!(!nav_empty.contains('·'), "{nav_empty}");
    }
}
