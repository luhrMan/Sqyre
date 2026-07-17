//! Blank action factories for the Add Action picker.

use crate::{
    action_type_table, Action, ActionId, ActionKind, ConditionBlock, CoordinateRef,
    DetectionBranch, ListColumn, ScalarValue, DEFAULT_SMOOTH_DELAY_MS, DEFAULT_SMOOTH_HIGH,
    DEFAULT_SMOOTH_LOW,
};

/// One picker entry: label, type key, category, and a fresh blank [`Action`].
#[derive(Debug, Clone)]
pub struct ActionTemplate {
    pub label: &'static str,
    pub action_type: &'static str,
    pub category: &'static str,
}

impl ActionTemplate {
    pub fn create(&self) -> Action {
        blank_action(self.action_type).expect("known template type")
    }
}

/// All addable action kinds for the picker (21 kinds; Calculate folded into Set).
pub fn action_templates() -> Vec<ActionTemplate> {
    action_type_table()
        .iter()
        .map(|m| ActionTemplate {
            label: m.label,
            action_type: m.type_key,
            category: m.picker_category,
        })
        .collect()
}

/// Build a blank action with a fresh UID for the given type key.
pub fn blank_action(action_type: &str) -> Option<Action> {
    let kind = blank_kind(action_type)?;
    Some(Action {
        id: ActionId::new(),
        kind,
    })
}

/// Convenience for tests: wrap a kind with a fresh id.
pub fn test_action(kind: ActionKind) -> Action {
    Action {
        id: ActionId::new(),
        kind,
    }
}

/// `test_action!(ActionKind::Wait { time: ScalarValue::Int(1) })`
#[macro_export]
macro_rules! test_action {
    ($kind:expr) => {
        $crate::test_action($kind)
    };
}

fn blank_kind(action_type: &str) -> Option<ActionKind> {
    Some(match action_type.trim().to_ascii_lowercase().as_str() {
        "move" => ActionKind::Move {
            point: CoordinateRef::default(),
            smooth: true,
            smooth_low: DEFAULT_SMOOTH_LOW,
            smooth_high: DEFAULT_SMOOTH_HIGH,
            smooth_delay_ms: DEFAULT_SMOOTH_DELAY_MS,
        },
        "click" => ActionKind::Click {
            button: "left".into(),
            state: true,
        },
        "key" => ActionKind::Key {
            key: "ctrl".into(),
            state: true,
        },
        "type" => ActionKind::Type {
            text: String::new(),
            delay_ms: 0,
        },
        "wait" => ActionKind::Wait {
            time: ScalarValue::Int(0),
        },
        "pause" => ActionKind::Pause {
            message: String::new(),
            continue_key: Vec::new(),
            pass_through: false,
        },
        "focuswindow" => ActionKind::FocusWindow {
            process_path: String::new(),
            window_title: String::new(),
        },
        "runmacro" => ActionKind::RunMacro {
            macro_name: String::new(),
        },
        "conditional" => ActionKind::Conditional {
            condition: ConditionBlock::default(),
            subactions: Vec::new(),
        },
        "loop" => ActionKind::Loop {
            name: String::new(),
            count: ScalarValue::Int(1),
            subactions: Vec::new(),
        },
        "while" => ActionKind::While {
            condition: ConditionBlock::default(),
            max_iterations: 0,
            subactions: Vec::new(),
        },
        "break" => ActionKind::Break,
        "continue" => ActionKind::Continue,
        "imagesearch" => ActionKind::ImageSearch {
            name: String::new(),
            targets: Vec::new(),
            search_area: CoordinateRef::default(),
            tolerance: 0.95,
            blur: 5,
            detection: DetectionBranch::default(),
        },
        "ocr" => ActionKind::Ocr {
            name: String::new(),
            target: "template".into(),
            search_area: CoordinateRef("template search area".into()),
            output_variable: String::new(),
            blur: 0,
            min_threshold: 0,
            resize: 1.0,
            grayscale: false,
            threshold_otsu: false,
            threshold_invert: false,
            detection: DetectionBranch::default(),
        },
        "findpixel" => ActionKind::FindPixel {
            name: String::new(),
            search_area: CoordinateRef::default(),
            target_color: "ffffff".into(),
            color_tolerance: 0,
            detection: DetectionBranch::default(),
        },
        "setvariable" => ActionKind::SetVariable {
            variable_name: String::new(),
            value: serde_yaml::Value::String(String::new()),
        },
        "foreachrow" => ActionKind::ForEachRow {
            name: String::new(),
            sources: vec![ListColumn::default()],
            start_row: ScalarValue::Int(1),
            end_row: ScalarValue::Null,
            subactions: Vec::new(),
        },
        "savevariable" => ActionKind::SaveVariable {
            variable_name: String::new(),
            destination: String::new(),
            append: false,
            append_newline: false,
        },
        "navigateselect" => ActionKind::NavigateSelect(Box::default()),
        "navigatekey" => ActionKind::NavigateKey {
            name: String::new(),
            chord: Vec::new(),
            exit: false,
            subactions: Vec::new(),
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_picker_category;

    #[test]
    fn templates_cover_twenty_one_kinds() {
        let t = action_templates();
        assert_eq!(t.len(), 21);
        for tmpl in &t {
            let a = tmpl.create();
            assert_eq!(a.type_key(), tmpl.action_type);
            assert!(!a.id.is_root());
            assert_eq!(action_picker_category(tmpl.action_type), tmpl.category);
        }
    }

    #[test]
    fn blank_unknown_is_none() {
        assert!(blank_action("calculate").is_none());
        assert!(blank_action("nope").is_none());
    }

    #[test]
    fn blank_move_has_expected_defaults() {
        let a = blank_action("move").unwrap();
        match a.kind {
            ActionKind::Move {
                smooth, smooth_low, ..
            } => {
                assert!(smooth);
                assert!((smooth_low - DEFAULT_SMOOTH_LOW).abs() < f64::EPSILON);
            }
            other => panic!("expected Move, got {other:?}"),
        }
    }
}
