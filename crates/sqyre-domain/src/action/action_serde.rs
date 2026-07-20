//! Serde for [`ActionKind`]: untagged wire variants with a `type` discriminant.
//!
//! Internally tagged enums cannot use `#[serde(flatten)]`, so each variant is a
//! struct that includes `type` plus flattened detection/condition/nav fields.

use super::{
    default_assignments, default_image_blur, default_loop_count, default_ocr_blur,
    default_ocr_text, default_resize, default_target_color, default_true, default_wait_time,
    is_default_image_blur, is_default_ocr_blur, is_default_ocr_text, is_default_resize,
    is_default_target_color, is_false, is_true, is_zero_i32, Action, ActionKind, ConditionBlock,
    CoordinateRef, DetectionBranch, ListColumn, MouseButton, NavigateSelectData, ScalarValue,
    VariableAssignment, DEFAULT_SMOOTH_DELAY_MS, DEFAULT_SMOOTH_HIGH, DEFAULT_SMOOTH_LOW,
};
use serde::{Deserialize, Serialize};

macro_rules! type_tag {
    ($name:ident, $rename:literal) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
        enum $name {
            #[serde(rename = $rename)]
            Tag,
        }
    };
}

type_tag!(TagLoop, "loop");
type_tag!(TagWhile, "while");
type_tag!(TagConditional, "conditional");
type_tag!(TagImageSearch, "imagesearch");
type_tag!(TagOcr, "ocr");
type_tag!(TagFindPixel, "findpixel");
type_tag!(TagForEachRow, "foreachrow");
type_tag!(TagWait, "wait");
type_tag!(TagPause, "pause");
type_tag!(TagMove, "move");
type_tag!(TagClick, "click");
type_tag!(TagKey, "key");
type_tag!(TagType, "type");
type_tag!(TagSetVariable, "setvariable");
type_tag!(TagSaveVariable, "savevariable");
type_tag!(TagFocusWindow, "focuswindow");
type_tag!(TagRunMacro, "runmacro");
type_tag!(TagNavigateSelect, "navigateselect");
type_tag!(TagNavigateKey, "navigatekey");
type_tag!(TagBreak, "break");
type_tag!(TagContinue, "continue");

fn is_default_smooth_low(v: &f64) -> bool {
    (*v - DEFAULT_SMOOTH_LOW).abs() < f64::EPSILON
}
fn is_default_smooth_high(v: &f64) -> bool {
    (*v - DEFAULT_SMOOTH_HIGH).abs() < f64::EPSILON
}
fn is_default_smooth_delay(v: &i32) -> bool {
    *v == DEFAULT_SMOOTH_DELAY_MS
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ActionKindWire {
    Loop {
        #[serde(rename = "type")]
        type_: TagLoop,
        name: String,
        #[serde(default = "default_loop_count")]
        count: ScalarValue,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subactions: Vec<Action>,
    },
    While {
        #[serde(rename = "type")]
        type_: TagWhile,
        #[serde(flatten)]
        condition: ConditionBlock,
        #[serde(rename = "maxiterations", default, skip_serializing_if = "is_zero_i32")]
        max_iterations: i32,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subactions: Vec<Action>,
    },
    Conditional {
        #[serde(rename = "type")]
        type_: TagConditional,
        #[serde(flatten)]
        condition: ConditionBlock,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subactions: Vec<Action>,
    },
    ImageSearch {
        #[serde(rename = "type")]
        type_: TagImageSearch,
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        targets: Vec<String>,
        #[serde(rename = "searcharea", default)]
        search_area: CoordinateRef,
        #[serde(default)]
        tolerance: f64,
        #[serde(
            default = "default_image_blur",
            skip_serializing_if = "is_default_image_blur"
        )]
        blur: i32,
        #[serde(flatten)]
        detection: DetectionBranch,
    },
    Ocr {
        #[serde(rename = "type")]
        type_: TagOcr,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        name: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        target: String,
        #[serde(rename = "searcharea", default)]
        search_area: CoordinateRef,
        #[serde(
            rename = "outputvariable",
            default = "default_ocr_text",
            skip_serializing_if = "is_default_ocr_text"
        )]
        output_variable: String,
        #[serde(
            default = "default_ocr_blur",
            skip_serializing_if = "is_default_ocr_blur"
        )]
        blur: i32,
        #[serde(rename = "minthreshold", default, skip_serializing_if = "is_zero_i32")]
        min_threshold: i32,
        #[serde(default = "default_resize", skip_serializing_if = "is_default_resize")]
        resize: f64,
        #[serde(default = "default_true", skip_serializing_if = "is_true")]
        grayscale: bool,
        #[serde(rename = "thresholdotsu", default, skip_serializing_if = "is_false")]
        threshold_otsu: bool,
        #[serde(rename = "thresholdinvert", default, skip_serializing_if = "is_false")]
        threshold_invert: bool,
        #[serde(flatten)]
        detection: DetectionBranch,
    },
    FindPixel {
        #[serde(rename = "type")]
        type_: TagFindPixel,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        name: String,
        #[serde(rename = "searcharea", default)]
        search_area: CoordinateRef,
        #[serde(
            rename = "targetcolor",
            default = "default_target_color",
            skip_serializing_if = "is_default_target_color"
        )]
        target_color: String,
        #[serde(
            rename = "colortolerance",
            default,
            skip_serializing_if = "is_zero_i32"
        )]
        color_tolerance: i32,
        #[serde(flatten)]
        detection: DetectionBranch,
    },
    ForEachRow {
        #[serde(rename = "type")]
        type_: TagForEachRow,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        sources: Vec<ListColumn>,
        #[serde(
            rename = "startrow",
            default,
            skip_serializing_if = "ScalarValue::is_null"
        )]
        start_row: ScalarValue,
        #[serde(
            rename = "endrow",
            default,
            skip_serializing_if = "ScalarValue::is_null"
        )]
        end_row: ScalarValue,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subactions: Vec<Action>,
    },
    Wait {
        #[serde(rename = "type")]
        type_: TagWait,
        #[serde(default = "default_wait_time")]
        time: ScalarValue,
    },
    Pause {
        #[serde(rename = "type")]
        type_: TagPause,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        message: String,
        #[serde(rename = "continuekey", default, skip_serializing_if = "Vec::is_empty")]
        continue_key: Vec<String>,
        #[serde(rename = "passthrough", default, skip_serializing_if = "is_false")]
        pass_through: bool,
    },
    Move {
        #[serde(rename = "type")]
        type_: TagMove,
        #[serde(default)]
        point: CoordinateRef,
        #[serde(default, skip_serializing_if = "is_false")]
        smooth: bool,
        #[serde(
            rename = "smoothlow",
            default = "default_smooth_low",
            skip_serializing_if = "is_default_smooth_low"
        )]
        smooth_low: f64,
        #[serde(
            rename = "smoothhigh",
            default = "default_smooth_high",
            skip_serializing_if = "is_default_smooth_high"
        )]
        smooth_high: f64,
        #[serde(
            rename = "smoothdelayms",
            default = "default_smooth_delay",
            skip_serializing_if = "is_default_smooth_delay"
        )]
        smooth_delay_ms: i32,
    },
    Click {
        #[serde(rename = "type")]
        type_: TagClick,
        button: MouseButton,
        #[serde(default)]
        state: bool,
    },
    Key {
        #[serde(rename = "type")]
        type_: TagKey,
        key: String,
        #[serde(default)]
        state: bool,
    },
    Type {
        #[serde(rename = "type")]
        type_: TagType,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        text: String,
        #[serde(rename = "delayms", default, skip_serializing_if = "is_zero_i32")]
        delay_ms: i32,
    },
    SetVariable {
        #[serde(rename = "type")]
        type_: TagSetVariable,
        #[serde(default = "default_assignments", skip_serializing_if = "Vec::is_empty")]
        assignments: Vec<VariableAssignment>,
    },
    SaveVariable {
        #[serde(rename = "type")]
        type_: TagSaveVariable,
        #[serde(rename = "variablename")]
        variable_name: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        destination: String,
        #[serde(default, skip_serializing_if = "is_false")]
        append: bool,
        #[serde(rename = "appendnewline", default, skip_serializing_if = "is_false")]
        append_newline: bool,
    },
    FocusWindow {
        #[serde(rename = "type")]
        type_: TagFocusWindow,
        #[serde(
            rename = "processpath",
            default,
            skip_serializing_if = "String::is_empty"
        )]
        process_path: String,
        #[serde(
            rename = "windowtitle",
            default,
            skip_serializing_if = "String::is_empty"
        )]
        window_title: String,
    },
    RunMacro {
        #[serde(rename = "type")]
        type_: TagRunMacro,
        #[serde(
            rename = "macroname",
            default,
            skip_serializing_if = "String::is_empty"
        )]
        macro_name: String,
    },
    NavigateSelect {
        #[serde(rename = "type")]
        type_: TagNavigateSelect,
        #[serde(flatten)]
        data: Box<NavigateSelectData>,
    },
    NavigateKey {
        #[serde(rename = "type")]
        type_: TagNavigateKey,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        chord: Vec<String>,
        #[serde(default, skip_serializing_if = "is_false")]
        exit: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subactions: Vec<Action>,
    },
    Break {
        #[serde(rename = "type")]
        type_: TagBreak,
    },
    Continue {
        #[serde(rename = "type")]
        type_: TagContinue,
    },
}

fn default_smooth_low() -> f64 {
    DEFAULT_SMOOTH_LOW
}
fn default_smooth_high() -> f64 {
    DEFAULT_SMOOTH_HIGH
}
fn default_smooth_delay() -> i32 {
    DEFAULT_SMOOTH_DELAY_MS
}

impl From<ActionKindWire> for ActionKind {
    fn from(w: ActionKindWire) -> Self {
        match w {
            ActionKindWire::Loop {
                name,
                mut count,
                subactions,
                ..
            } => {
                if name == "root" {
                    count = ScalarValue::Int(1);
                }
                Self::Loop {
                    name,
                    count,
                    subactions,
                }
            }
            ActionKindWire::While {
                condition,
                max_iterations,
                subactions,
                ..
            } => Self::While {
                condition,
                max_iterations,
                subactions,
            },
            ActionKindWire::Conditional {
                condition,
                subactions,
                ..
            } => Self::Conditional {
                condition,
                subactions,
            },
            ActionKindWire::ImageSearch {
                name,
                targets,
                search_area,
                tolerance,
                blur,
                detection,
                ..
            } => Self::ImageSearch {
                name,
                targets,
                search_area,
                tolerance,
                blur,
                detection,
            },
            ActionKindWire::Ocr {
                name,
                target,
                search_area,
                output_variable,
                mut blur,
                min_threshold,
                resize,
                grayscale,
                threshold_otsu,
                threshold_invert,
                detection,
                ..
            } => {
                if blur < 1 {
                    blur = 1;
                }
                Self::Ocr {
                    name,
                    target,
                    search_area,
                    output_variable,
                    blur,
                    min_threshold,
                    resize,
                    grayscale,
                    threshold_otsu,
                    threshold_invert,
                    detection,
                }
            }
            ActionKindWire::FindPixel {
                name,
                search_area,
                target_color,
                mut color_tolerance,
                detection,
                ..
            } => {
                if !(0..=100).contains(&color_tolerance) {
                    color_tolerance = 0;
                }
                Self::FindPixel {
                    name,
                    search_area,
                    target_color,
                    color_tolerance,
                    detection,
                }
            }
            ActionKindWire::ForEachRow {
                name,
                sources,
                start_row,
                end_row,
                subactions,
                ..
            } => Self::ForEachRow {
                name,
                sources,
                start_row,
                end_row,
                subactions,
            },
            ActionKindWire::Wait { time, .. } => Self::Wait { time },
            ActionKindWire::Pause {
                message,
                continue_key,
                pass_through,
                ..
            } => Self::Pause {
                message,
                continue_key,
                pass_through,
            },
            ActionKindWire::Move {
                point,
                smooth,
                smooth_low,
                smooth_high,
                smooth_delay_ms,
                ..
            } => Self::Move {
                point,
                smooth,
                smooth_low: if smooth {
                    smooth_low
                } else {
                    DEFAULT_SMOOTH_LOW
                },
                smooth_high: if smooth {
                    smooth_high
                } else {
                    DEFAULT_SMOOTH_HIGH
                },
                smooth_delay_ms: if smooth {
                    smooth_delay_ms
                } else {
                    DEFAULT_SMOOTH_DELAY_MS
                },
            },
            ActionKindWire::Click { button, state, .. } => Self::Click { button, state },
            ActionKindWire::Key { key, state, .. } => Self::Key { key, state },
            ActionKindWire::Type { text, delay_ms, .. } => Self::Type { text, delay_ms },
            ActionKindWire::SetVariable { assignments, .. } => Self::SetVariable { assignments },
            ActionKindWire::SaveVariable {
                variable_name,
                destination,
                append,
                append_newline,
                ..
            } => Self::SaveVariable {
                variable_name,
                destination,
                append,
                append_newline,
            },
            ActionKindWire::FocusWindow {
                process_path,
                window_title,
                ..
            } => Self::FocusWindow {
                process_path,
                window_title,
            },
            ActionKindWire::RunMacro { macro_name, .. } => Self::RunMacro { macro_name },
            ActionKindWire::NavigateSelect { data, .. } => Self::NavigateSelect(data),
            ActionKindWire::NavigateKey {
                name,
                chord,
                exit,
                subactions,
                ..
            } => Self::NavigateKey {
                name,
                chord,
                exit,
                subactions,
            },
            ActionKindWire::Break { .. } => Self::Break,
            ActionKindWire::Continue { .. } => Self::Continue,
        }
    }
}

impl From<&ActionKind> for ActionKindWire {
    fn from(kind: &ActionKind) -> Self {
        match kind {
            ActionKind::Loop {
                name,
                count,
                subactions,
            } => Self::Loop {
                type_: TagLoop::Tag,
                name: name.clone(),
                count: count.clone(),
                subactions: subactions.clone(),
            },
            ActionKind::While {
                condition,
                max_iterations,
                subactions,
            } => Self::While {
                type_: TagWhile::Tag,
                condition: condition.clone(),
                max_iterations: *max_iterations,
                subactions: subactions.clone(),
            },
            ActionKind::Conditional {
                condition,
                subactions,
            } => Self::Conditional {
                type_: TagConditional::Tag,
                condition: condition.clone(),
                subactions: subactions.clone(),
            },
            ActionKind::ImageSearch {
                name,
                targets,
                search_area,
                tolerance,
                blur,
                detection,
            } => Self::ImageSearch {
                type_: TagImageSearch::Tag,
                name: name.clone(),
                targets: targets.clone(),
                search_area: search_area.clone(),
                tolerance: *tolerance,
                blur: *blur,
                detection: detection.clone(),
            },
            ActionKind::Ocr {
                name,
                target,
                search_area,
                output_variable,
                blur,
                min_threshold,
                resize,
                grayscale,
                threshold_otsu,
                threshold_invert,
                detection,
            } => Self::Ocr {
                type_: TagOcr::Tag,
                name: name.clone(),
                target: target.clone(),
                search_area: search_area.clone(),
                output_variable: output_variable.clone(),
                blur: *blur,
                min_threshold: *min_threshold,
                resize: *resize,
                grayscale: *grayscale,
                threshold_otsu: *threshold_otsu,
                threshold_invert: *threshold_invert,
                detection: detection.clone(),
            },
            ActionKind::FindPixel {
                name,
                search_area,
                target_color,
                color_tolerance,
                detection,
            } => Self::FindPixel {
                type_: TagFindPixel::Tag,
                name: name.clone(),
                search_area: search_area.clone(),
                target_color: target_color.clone(),
                color_tolerance: *color_tolerance,
                detection: detection.clone(),
            },
            ActionKind::ForEachRow {
                name,
                sources,
                start_row,
                end_row,
                subactions,
            } => Self::ForEachRow {
                type_: TagForEachRow::Tag,
                name: name.clone(),
                sources: sources.clone(),
                start_row: start_row.clone(),
                end_row: end_row.clone(),
                subactions: subactions.clone(),
            },
            ActionKind::Wait { time } => Self::Wait {
                type_: TagWait::Tag,
                time: time.clone(),
            },
            ActionKind::Pause {
                message,
                continue_key,
                pass_through,
            } => Self::Pause {
                type_: TagPause::Tag,
                message: message.clone(),
                continue_key: continue_key.clone(),
                pass_through: *pass_through,
            },
            ActionKind::Move {
                point,
                smooth,
                smooth_low,
                smooth_high,
                smooth_delay_ms,
            } => Self::Move {
                type_: TagMove::Tag,
                point: point.clone(),
                smooth: *smooth,
                smooth_low: *smooth_low,
                smooth_high: *smooth_high,
                smooth_delay_ms: *smooth_delay_ms,
            },
            ActionKind::Click { button, state } => Self::Click {
                type_: TagClick::Tag,
                button: *button,
                state: *state,
            },
            ActionKind::Key { key, state } => Self::Key {
                type_: TagKey::Tag,
                key: key.clone(),
                state: *state,
            },
            ActionKind::Type { text, delay_ms } => Self::Type {
                type_: TagType::Tag,
                text: text.clone(),
                delay_ms: *delay_ms,
            },
            ActionKind::SetVariable { assignments } => Self::SetVariable {
                type_: TagSetVariable::Tag,
                assignments: assignments.clone(),
            },
            ActionKind::SaveVariable {
                variable_name,
                destination,
                append,
                append_newline,
            } => Self::SaveVariable {
                type_: TagSaveVariable::Tag,
                variable_name: variable_name.clone(),
                destination: destination.clone(),
                append: *append,
                append_newline: *append_newline,
            },
            ActionKind::FocusWindow {
                process_path,
                window_title,
            } => Self::FocusWindow {
                type_: TagFocusWindow::Tag,
                process_path: process_path.clone(),
                window_title: window_title.clone(),
            },
            ActionKind::RunMacro { macro_name } => Self::RunMacro {
                type_: TagRunMacro::Tag,
                macro_name: macro_name.clone(),
            },
            ActionKind::NavigateSelect(data) => Self::NavigateSelect {
                type_: TagNavigateSelect::Tag,
                data: data.clone(),
            },
            ActionKind::NavigateKey {
                name,
                chord,
                exit,
                subactions,
            } => Self::NavigateKey {
                type_: TagNavigateKey::Tag,
                name: name.clone(),
                chord: chord.clone(),
                exit: *exit,
                subactions: subactions.clone(),
            },
            ActionKind::Break => Self::Break {
                type_: TagBreak::Tag,
            },
            ActionKind::Continue => Self::Continue {
                type_: TagContinue::Tag,
            },
        }
    }
}

impl Serialize for ActionKind {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ActionKindWire::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ActionKind {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ActionKindWire::deserialize(deserializer).map(Into::into)
    }
}
