//! YAML mapping → ActionKind.

use crate::helpers::*;
use crate::{Result, SerializeError};
use serde_yaml::{Mapping, Value};
use sqyre_domain::{
    Action, ActionKind, ConditionBlock, ConditionClause, CoordinateOutputs,
    DetectionBranch, ListColumn, MatchMode, MatchOrder, MouseButton, RepeatMode, ScalarValue,
    WaitTilFoundConfig, DEFAULT_SMOOTH_DELAY_MS, DEFAULT_SMOOTH_HIGH, DEFAULT_SMOOTH_LOW,
};

use super::action_from_map;

fn decode_detection(raw: &Mapping) -> Result<DetectionBranch> {
    Ok(DetectionBranch {
        wait: decode_wait(raw),
        coords: decode_coords(raw),
        run_branch_on_no_find: bool_from_map(raw, "runbranchonnofind"),
        order: decode_order(raw),
        subactions: decode_subactions(raw)?,
    })
}
fn decode_wait(raw: &Mapping) -> WaitTilFoundConfig {
    WaitTilFoundConfig {
        repeat_mode: RepeatMode::parse(&string_from_map(raw, "repeatmode")),
        wait_til_found_seconds: optional_int(raw, "waittilfoundseconds").unwrap_or(0),
        wait_til_found_interval_ms: optional_int(raw, "waittilfoundintervalms").unwrap_or(0),
        max_iterations: optional_int(raw, "maxiterations").unwrap_or(0),
    }
}

fn decode_coords(raw: &Mapping) -> CoordinateOutputs {
    let mut c = CoordinateOutputs::defaults();
    let x = string_from_map(raw, "outputxvariable");
    let y = string_from_map(raw, "outputyvariable");
    if !x.is_empty() {
        c.output_x_variable = x;
    }
    if !y.is_empty() {
        c.output_y_variable = y;
    }
    c
}

fn decode_chord(raw: &Mapping, key: &str) -> Vec<String> {
    map_get(raw, key)
        .map(string_slice_from_value)
        .unwrap_or_default()
}

fn decode_nav_chords(raw: &Mapping) -> sqyre_domain::NavChords {
    sqyre_domain::NavChords {
        up: decode_chord(raw, "chordup"),
        down: decode_chord(raw, "chorddown"),
        left: decode_chord(raw, "chordleft"),
        right: decode_chord(raw, "chordright"),
        select: decode_chord(raw, "chordselect"),
        back: decode_chord(raw, "chordback"),
    }
}

fn decode_nav_options(raw: &Mapping) -> sqyre_domain::NavOptions {
    sqyre_domain::NavOptions {
        wrap_edges: bool_from_map(raw, "wrapedges"),
        move_cursor_with_nav: bool_from_map(raw, "movecursorwithnav"),
        smooth: bool_from_map(raw, "smooth"),
        pass_through: bool_from_map(raw, "passthrough"),
        hold_repeat: bool_from_map(raw, "holdrepeat"),
    }
}

fn decode_nav_select(raw: &Mapping) -> sqyre_domain::NavSelectAction {
    sqyre_domain::NavSelectAction {
        device: string_from_map(raw, "selectdevice"),
        button: string_from_map(raw, "selectbutton"),
        key: string_from_map(raw, "selectkey"),
        press_mode: string_from_map(raw, "selectpressmode"),
    }
}

fn decode_nav_inputs(raw: &Mapping) -> sqyre_domain::NavInputs {
    sqyre_domain::NavInputs {
        graph: string_from_map(raw, "ingraph"),
        row: string_from_map(raw, "inrow"),
        col: string_from_map(raw, "incol"),
        collection: string_from_map(raw, "incollection"),
    }
}

fn decode_nav_outputs(raw: &Mapping) -> sqyre_domain::NavOutputs {
    sqyre_domain::NavOutputs {
        output_ref: string_from_map(raw, "outputref"),
        output_graph: string_from_map(raw, "outputgraph"),
        output_row: string_from_map(raw, "outputrow"),
        output_col: string_from_map(raw, "outputcol"),
        output_collection: string_from_map(raw, "outputcollection"),
    }
}

fn decode_order(raw: &Mapping) -> MatchOrder {
    MatchOrder {
        grouping: string_from_map(raw, "grouping"),
        horizontal: string_from_map(raw, "horizontal"),
        vertical: string_from_map(raw, "vertical"),
    }
}
pub(super) fn decode_subactions(raw: &Mapping) -> Result<Vec<Action>> {
    let Some(Value::Sequence(seq)) = map_get(raw, "subactions") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(seq.len());
    for (i, item) in seq.iter().enumerate() {
        let map =
            as_mapping(item).map_err(|e| SerializeError::msg(format!("subactions[{i}]: {e}")))?;
        out.push(
            action_from_map(map)
                .map_err(|e| SerializeError::msg(format!("subactions[{i}]: {e}")))?,
        );
    }
    Ok(out)
}
fn decode_clauses(raw: &Mapping) -> Vec<ConditionClause> {
    let Some(Value::Sequence(seq)) = map_get(raw, "clauses") else {
        return vec![ConditionClause::default()];
    };
    let mut out = Vec::new();
    for item in seq {
        let Some(m) = item.as_mapping() else {
            continue;
        };
        let mut op = string_from_map(m, "operator");
        if op.is_empty() {
            op = "==".into();
        }
        out.push(ConditionClause {
            left: scalar_from_value(map_get(m, "left")),
            operator: op,
            right: scalar_from_value(map_get(m, "right")),
        });
    }
    if out.is_empty() {
        out.push(ConditionClause::default());
    }
    out
}

fn decode_sources(raw: &Mapping) -> Vec<ListColumn> {
    let Some(Value::Sequence(seq)) = map_get(raw, "sources") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in seq {
        let Some(m) = item.as_mapping() else {
            continue;
        };
        out.push(ListColumn {
            source: string_from_map(m, "source"),
            output_var: string_from_map(m, "outputvar"),
            is_file: bool_from_map(m, "isfile"),
            skip_blank_lines: bool_from_map(m, "skipblanklines"),
        });
    }
    out
}
pub(super) fn decode_kind(raw: &Mapping, type_name: &str) -> Result<ActionKind> {
    match type_name {
        "loop" => {
            let name = expect_string(raw, "name")
                .map_err(|e| SerializeError::msg(format!("action type loop: {e}")))?;
            let count = match map_get(raw, "count") {
                None | Some(Value::Null) => ScalarValue::Int(1),
                Some(v) => ScalarValue::from_yaml(v),
            };
            Ok(ActionKind::Loop {
                name: name.clone(),
                count: if name == "root" {
                    ScalarValue::Int(1)
                } else {
                    count
                },
                subactions: decode_subactions(raw)?,
            })
        }
        "while" => Ok(ActionKind::While {
            condition: ConditionBlock {
                name: string_from_map(raw, "name"),
                match_mode: MatchMode::parse(&string_from_map(raw, "match")),
                clauses: decode_clauses(raw),
            },
            max_iterations: optional_int(raw, "maxiterations").unwrap_or(0),
            subactions: decode_subactions(raw)?,
        }),
        "conditional" => Ok(ActionKind::Conditional {
            condition: ConditionBlock {
                name: string_from_map(raw, "name"),
                match_mode: MatchMode::parse(&string_from_map(raw, "match")),
                clauses: decode_clauses(raw),
            },
            subactions: decode_subactions(raw)?,
        }),
        "imagesearch" => {
            let name = expect_string(raw, "name")
                .map_err(|e| SerializeError::msg(format!("action type imagesearch: {e}")))?;
            let targets = {
                let from_seq = map_get(raw, "targets")
                    .map(string_slice_from_value)
                    .unwrap_or_default();
                if !from_seq.is_empty() {
                    from_seq
                } else {
                    // Legacy singular `target` string.
                    let one = string_from_map(raw, "target");
                    if one.is_empty() {
                        Vec::new()
                    } else {
                        vec![one]
                    }
                }
            };
            let blur = optional_int(raw, "blur").unwrap_or(5);
            let tolerance = map_get(raw, "tolerance")
                .map(float_from_value)
                .unwrap_or(0.0);
            Ok(ActionKind::ImageSearch {
                name,
                targets,
                search_area: parse_coordinate_ref(map_get(raw, "searcharea")),
                tolerance,
                blur,
                detection: decode_detection(raw)?,
            })
        }
        "ocr" => {
            let name = string_from_map(raw, "name");
            let target = string_from_map(raw, "target");
            let mut blur = optional_int(raw, "blur").unwrap_or(1);
            if blur < 1 {
                blur = 1;
            }
            Ok(ActionKind::Ocr {
                name,
                target,
                search_area: parse_coordinate_ref(map_get(raw, "searcharea")),
                output_variable: {
                    let v = string_from_map(raw, "outputvariable");
                    if v.is_empty() {
                        "ocrText".into()
                    } else {
                        v
                    }
                },
                blur,
                min_threshold: optional_int(raw, "minthreshold").unwrap_or(0),
                resize: map_get(raw, "resize")
                    .map(float_from_value)
                    .unwrap_or(1.0),
                grayscale: map_get(raw, "grayscale")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                threshold_otsu: bool_from_map(raw, "thresholdotsu"),
                threshold_invert: bool_from_map(raw, "thresholdinvert"),
                detection: decode_detection(raw)?,
            })
        }
        "findpixel" => {
            let mut color = string_from_map(raw, "targetcolor");
            if color.is_empty() {
                color = "ffffff".into();
            }
            let mut tol = optional_int(raw, "colortolerance").unwrap_or(0);
            if !(0..=100).contains(&tol) {
                tol = 0;
            }
            Ok(ActionKind::FindPixel {
                name: string_from_map(raw, "name"),
                search_area: parse_coordinate_ref(map_get(raw, "searcharea")),
                target_color: color,
                color_tolerance: tol,
                detection: decode_detection(raw)?,
            })
        }
        "foreachrow" => Ok(ActionKind::ForEachRow {
            name: string_from_map(raw, "name"),
            sources: decode_sources(raw),
            start_row: scalar_from_value(map_get(raw, "startrow")),
            end_row: scalar_from_value(map_get(raw, "endrow")),
            subactions: decode_subactions(raw)?,
        }),
        "wait" => Ok(ActionKind::Wait {
            time: match map_get(raw, "time") {
                None | Some(Value::Null) => ScalarValue::Int(0),
                Some(v) => ScalarValue::from_yaml(v),
            },
        }),
        "pause" => Ok(ActionKind::Pause {
            message: string_from_map(raw, "message"),
            continue_key: map_get(raw, "continuekey")
                .map(string_slice_from_value)
                .unwrap_or_default(),
            pass_through: bool_from_map(raw, "passthrough"),
        }),
        "move" => {
            let smooth = bool_from_map(raw, "smooth");
            Ok(ActionKind::Move {
                point: parse_coordinate_ref(map_get(raw, "point")),
                smooth,
                smooth_low: map_get(raw, "smoothlow")
                    .map(float_from_value)
                    .unwrap_or(if smooth { DEFAULT_SMOOTH_LOW } else { 0.0 }),
                smooth_high: map_get(raw, "smoothhigh")
                    .map(float_from_value)
                    .unwrap_or(if smooth { DEFAULT_SMOOTH_HIGH } else { 0.0 }),
                smooth_delay_ms: optional_int(raw, "smoothdelayms").unwrap_or(if smooth {
                    DEFAULT_SMOOTH_DELAY_MS
                } else {
                    0
                }),
            })
        }
        "click" => {
            let button = expect_string(raw, "button")
                .map_err(|e| SerializeError::msg(format!("action type click: {e}")))?;
            match button.as_str() {
                "left" | "right" | "center" | "middle" | "scroll" => {}
                other => {
                    return Err(SerializeError::msg(format!(
                        "action type click: field \"button\": unknown button \"{other}\""
                    )));
                }
            }
            Ok(ActionKind::Click {
                button: MouseButton::parse(&button),
                state: bool_from_map(raw, "state"),
            })
        }
        "key" => Ok(ActionKind::Key {
            key: expect_string(raw, "key")
                .map_err(|e| SerializeError::msg(format!("action type key: {e}")))?,
            state: expect_bool(raw, "state")
                .map_err(|e| SerializeError::msg(format!("action type key: {e}")))?,
        }),
        "type" => Ok(ActionKind::Type {
            text: string_from_map(raw, "text"),
            delay_ms: optional_int(raw, "delayms").unwrap_or(0),
        }),
        "setvariable" => Ok(ActionKind::SetVariable {
            variable_name: expect_string(raw, "variablename")
                .map_err(|e| SerializeError::msg(format!("action type setvariable: {e}")))?,
            value: map_get(raw, "value")
                .cloned()
                .unwrap_or(Value::Null),
        }),
        "savevariable" => Ok(ActionKind::SaveVariable {
            variable_name: expect_string(raw, "variablename")
                .map_err(|e| SerializeError::msg(format!("action type savevariable: {e}")))?,
            destination: expect_string(raw, "destination")
                .map_err(|e| SerializeError::msg(format!("action type savevariable: {e}")))?,
            append: bool_from_map(raw, "append"),
            append_newline: bool_from_map(raw, "appendnewline"),
        }),
        "focuswindow" => Ok(ActionKind::FocusWindow {
            process_path: string_from_map(raw, "processpath"),
            window_title: string_from_map(raw, "windowtitle"),
        }),
        "runmacro" => Ok(ActionKind::RunMacro {
            macro_name: string_from_map(raw, "macroname"),
        }),
        "navigateselect" => Ok(ActionKind::NavigateSelect(Box::new(
            sqyre_domain::NavigateSelectData {
                program: string_from_map(raw, "program"),
                graph_name: string_from_map(raw, "graphname"),
                chords: decode_nav_chords(raw),
                options: decode_nav_options(raw),
                select: decode_nav_select(raw),
                inputs: decode_nav_inputs(raw),
                outputs: decode_nav_outputs(raw),
                subactions: decode_subactions(raw)?,
            },
        ))),
        "navigatekey" => Ok(ActionKind::NavigateKey {
            name: string_from_map(raw, "name"),
            chord: map_get(raw, "chord")
                .map(string_slice_from_value)
                .unwrap_or_default(),
            exit: bool_from_map(raw, "exit"),
            subactions: decode_subactions(raw)?,
        }),
        "break" => Ok(ActionKind::Break),
        "continue" => Ok(ActionKind::Continue),
        other => Err(SerializeError::msg(format!("unknown action type {other}"))),
    }
}

