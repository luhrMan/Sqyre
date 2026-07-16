use crate::helpers::*;
use crate::{Result, SerializeError};
use serde_yaml::{Mapping, Sequence, Value};
use sqyre_domain::{
    Action, ActionId, ActionKind, ConditionClause, CoordinateOutputs, ListColumn, MatchOrder,
    ScalarValue, WaitTilFoundConfig, DEFAULT_SMOOTH_DELAY_MS, DEFAULT_SMOOTH_HIGH,
    DEFAULT_SMOOTH_LOW, MATCH_ALL, REPEAT_ONCE, REPEAT_WAIT_UNTIL_FOUND, REPEAT_WHILE_FOUND,
};
use uuid::Uuid;

/// Encode an action (and subtree) to a YAML mapping.
pub fn action_to_map(action: &Action) -> Result<Mapping> {
    let m = encode_kind(&action.kind)?;
    let _ = &action.id;
    Ok(m)
}

/// Encode including `uid` on every node (for undo/clipboard snapshots).
///
/// Normal [`action_to_map`] omits UIDs so copy/paste gets fresh identities;
/// undo must restore stable IDs, so this walks the live tree and injects them.
pub fn action_to_map_with_uid(action: &Action) -> Result<Mapping> {
    let mut m = action_to_map(action)?;
    inject_action_uid(&mut m, action);
    Ok(m)
}

fn inject_action_uid(m: &mut Mapping, action: &Action) {
    if !action.id.is_root() {
        insert_str(m, "uid", action.id.as_str());
    }
    let Some(Value::Sequence(seq)) = m.get_mut(Value::String("subactions".into())) else {
        return;
    };
    for (i, child) in action.children().iter().enumerate() {
        if let Some(Value::Mapping(sub)) = seq.get_mut(i) {
            inject_action_uid(sub, child);
        }
    }
}

/// Decode an action from a YAML mapping. Assigns a new UID unless `uid` is set.
pub fn action_from_map(raw: &Mapping) -> Result<Action> {
    let type_name = string_from_map(raw, "type");
    if type_name.is_empty() {
        return Err(SerializeError::msg("missing field \"type\""));
    }
    let kind = decode_kind(raw, &type_name)?;
    let id = restore_uid(raw, &type_name, &kind);
    Ok(Action { id, kind })
}

fn restore_uid(raw: &Mapping, type_name: &str, kind: &ActionKind) -> ActionId {
    let uid = string_from_map(raw, "uid");
    if !uid.is_empty() {
        if let Ok(u) = Uuid::parse_str(&uid) {
            return ActionId(u);
        }
    }
    if type_name == "loop" {
        if let ActionKind::Loop { name, .. } = kind {
            if name == "root" {
                return ActionId::root();
            }
        }
    }
    ActionId::new()
}

fn encode_kind(kind: &ActionKind) -> Result<Mapping> {
    let mut m = Mapping::new();
    insert_str(&mut m, "type", kind.type_key());
    match kind {
        ActionKind::Loop {
            name,
            count,
            subactions,
        } => {
            insert_str(&mut m, "name", name);
            insert(&mut m, "count", count.to_yaml());
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::While {
            name,
            match_mode,
            clauses,
            max_iterations,
            subactions,
        } => {
            insert_str(&mut m, "name", name);
            insert_str(
                &mut m,
                "match",
                if match_mode == "any" { "any" } else { MATCH_ALL },
            );
            if *max_iterations > 0 {
                insert_i32(&mut m, "maxiterations", *max_iterations);
            }
            insert(&mut m, "clauses", clauses_to_seq(clauses));
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::Conditional {
            name,
            match_mode,
            clauses,
            subactions,
        } => {
            insert_str(&mut m, "name", name);
            insert_str(
                &mut m,
                "match",
                if match_mode == "any" { "any" } else { MATCH_ALL },
            );
            insert(&mut m, "clauses", clauses_to_seq(clauses));
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::ImageSearch {
            name,
            targets,
            search_area,
            tolerance,
            blur,
            wait,
            coords,
            run_branch_on_no_find,
            order,
            subactions,
        } => {
            insert_str(&mut m, "name", name);
            insert(
                &mut m,
                "targets",
                Value::Sequence(targets.iter().cloned().map(Value::String).collect()),
            );
            insert(&mut m, "searcharea", coordinate_ref_to_value(search_area));
            insert_f64(&mut m, "tolerance", *tolerance);
            insert_i32(&mut m, "blur", *blur);
            write_wait(&mut m, wait);
            write_coords(&mut m, coords);
            write_order(&mut m, order);
            if *run_branch_on_no_find {
                insert_bool(&mut m, "runbranchonnofind", true);
            }
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::Ocr {
            name,
            target,
            search_area,
            output_variable,
            coords,
            wait,
            run_branch_on_no_find,
            blur,
            min_threshold,
            resize,
            grayscale,
            threshold_otsu,
            threshold_invert,
            order,
            subactions,
        } => {
            insert_str(&mut m, "name", name);
            insert_str(&mut m, "target", target);
            insert(&mut m, "searcharea", coordinate_ref_to_value(search_area));
            if !output_variable.is_empty() {
                insert_str(&mut m, "outputvariable", output_variable);
            }
            write_wait(&mut m, wait);
            write_coords(&mut m, coords);
            write_order(&mut m, order);
            if !*grayscale {
                insert_bool(&mut m, "grayscale", false);
            }
            if *blur != 1 {
                insert_i32(&mut m, "blur", *blur);
            }
            if *min_threshold != 0 {
                insert_i32(&mut m, "minthreshold", *min_threshold);
            }
            if (*resize - 1.0).abs() > f64::EPSILON {
                insert_f64(&mut m, "resize", *resize);
            }
            if *threshold_otsu {
                insert_bool(&mut m, "thresholdotsu", true);
            }
            if *threshold_invert {
                insert_bool(&mut m, "thresholdinvert", true);
            }
            if *run_branch_on_no_find {
                insert_bool(&mut m, "runbranchonnofind", true);
            }
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::FindPixel {
            name,
            search_area,
            target_color,
            color_tolerance,
            wait,
            coords,
            run_branch_on_no_find,
            order,
            subactions,
        } => {
            insert_str(&mut m, "name", name);
            insert(&mut m, "searcharea", coordinate_ref_to_value(search_area));
            insert_str(&mut m, "targetcolor", target_color);
            insert_i32(&mut m, "colortolerance", *color_tolerance);
            write_wait(&mut m, wait);
            write_coords(&mut m, coords);
            write_order(&mut m, order);
            if *run_branch_on_no_find {
                insert_bool(&mut m, "runbranchonnofind", true);
            }
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::ForEachRow {
            name,
            sources,
            start_row,
            end_row,
            subactions,
        } => {
            insert_str(&mut m, "name", name);
            insert(&mut m, "sources", list_columns_to_seq(sources));
            if start_row.is_set() {
                insert(&mut m, "startrow", start_row.to_yaml());
            }
            if end_row.is_set() {
                insert(&mut m, "endrow", end_row.to_yaml());
            }
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::Wait { time } => {
            insert(&mut m, "time", time.to_yaml());
        }
        ActionKind::Pause {
            message,
            continue_key,
            pass_through,
        } => {
            if !message.is_empty() {
                insert_str(&mut m, "message", message);
            }
            if !continue_key.is_empty() {
                insert(
                    &mut m,
                    "continuekey",
                    Value::Sequence(continue_key.iter().cloned().map(Value::String).collect()),
                );
            }
            if *pass_through {
                insert_bool(&mut m, "passthrough", true);
            }
        }
        ActionKind::Move {
            point,
            smooth,
            smooth_low,
            smooth_high,
            smooth_delay_ms,
        } => {
            insert(&mut m, "point", coordinate_ref_to_value(point));
            insert_bool(&mut m, "smooth", *smooth);
            if *smooth {
                let low = if *smooth_low > 0.0 {
                    *smooth_low
                } else {
                    DEFAULT_SMOOTH_LOW
                };
                let high = if *smooth_high > 0.0 {
                    *smooth_high
                } else {
                    DEFAULT_SMOOTH_HIGH
                };
                let delay = if *smooth_delay_ms > 0 {
                    *smooth_delay_ms
                } else {
                    DEFAULT_SMOOTH_DELAY_MS
                };
                insert_f64(&mut m, "smoothlow", low);
                insert_f64(&mut m, "smoothhigh", high);
                insert_i32(&mut m, "smoothdelayms", delay);
            }
        }
        ActionKind::Click { button, state } => {
            insert_str(&mut m, "button", button);
            insert_bool(&mut m, "state", *state);
        }
        ActionKind::Key { key, state } => {
            insert_str(&mut m, "key", key);
            insert_bool(&mut m, "state", *state);
        }
        ActionKind::Type { text, delay_ms } => {
            insert_str(&mut m, "text", text);
            insert_i32(&mut m, "delayms", *delay_ms);
        }
        ActionKind::SetVariable {
            variable_name,
            value,
        } => {
            insert_str(&mut m, "variablename", variable_name);
            insert(&mut m, "value", value.clone());
        }
        ActionKind::SaveVariable {
            variable_name,
            destination,
            append,
            append_newline,
        } => {
            insert_str(&mut m, "variablename", variable_name);
            insert_str(&mut m, "destination", destination);
            insert_bool(&mut m, "append", *append);
            insert_bool(&mut m, "appendnewline", *append_newline);
        }
        ActionKind::FocusWindow {
            process_path,
            window_title,
        } => {
            insert_str(&mut m, "processpath", process_path);
            insert_str(&mut m, "windowtitle", window_title);
        }
        ActionKind::RunMacro { macro_name } => {
            insert_str(&mut m, "macroname", macro_name);
        }
        ActionKind::NavigateSelect {
            program,
            graph_name,
            chord_up,
            chord_down,
            chord_left,
            chord_right,
            chord_select,
            chord_back,
            wrap_edges,
            move_cursor_with_nav,
            smooth,
            pass_through,
            hold_repeat,
            select_device,
            select_button,
            select_key,
            select_press_mode,
            in_graph,
            in_row,
            in_col,
            in_collection,
            output_ref,
            output_graph,
            output_row,
            output_col,
            output_collection,
            subactions,
        } => {
            if !program.is_empty() {
                insert_str(&mut m, "program", program);
            }
            if !graph_name.is_empty() {
                insert_str(&mut m, "graphname", graph_name);
            }
            write_chord(&mut m, "chordup", chord_up);
            write_chord(&mut m, "chorddown", chord_down);
            write_chord(&mut m, "chordleft", chord_left);
            write_chord(&mut m, "chordright", chord_right);
            write_chord(&mut m, "chordselect", chord_select);
            write_chord(&mut m, "chordback", chord_back);
            if *wrap_edges {
                insert_bool(&mut m, "wrapedges", true);
            }
            if *move_cursor_with_nav {
                insert_bool(&mut m, "movecursorwithnav", true);
            }
            if *smooth {
                insert_bool(&mut m, "smooth", true);
            }
            if *pass_through {
                insert_bool(&mut m, "passthrough", true);
            }
            if *hold_repeat {
                insert_bool(&mut m, "holdrepeat", true);
            }
            if !select_device.is_empty() {
                insert_str(&mut m, "selectdevice", select_device);
            }
            if !select_button.is_empty() {
                insert_str(&mut m, "selectbutton", select_button);
            }
            if !select_key.is_empty() {
                insert_str(&mut m, "selectkey", select_key);
            }
            if !select_press_mode.is_empty() {
                insert_str(&mut m, "selectpressmode", select_press_mode);
            }
            if !in_graph.is_empty() {
                insert_str(&mut m, "ingraph", in_graph);
            }
            if !in_row.is_empty() {
                insert_str(&mut m, "inrow", in_row);
            }
            if !in_col.is_empty() {
                insert_str(&mut m, "incol", in_col);
            }
            if !in_collection.is_empty() {
                insert_str(&mut m, "incollection", in_collection);
            }
            if !output_ref.is_empty() {
                insert_str(&mut m, "outputref", output_ref);
            }
            if !output_graph.is_empty() {
                insert_str(&mut m, "outputgraph", output_graph);
            }
            if !output_row.is_empty() {
                insert_str(&mut m, "outputrow", output_row);
            }
            if !output_col.is_empty() {
                insert_str(&mut m, "outputcol", output_col);
            }
            if !output_collection.is_empty() {
                insert_str(&mut m, "outputcollection", output_collection);
            }
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::NavigateKey {
            name,
            chord,
            exit,
            subactions,
        } => {
            if !name.is_empty() {
                insert_str(&mut m, "name", name);
            }
            write_chord(&mut m, "chord", chord);
            if *exit {
                insert_bool(&mut m, "exit", true);
            }
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::Break | ActionKind::Continue => {}
    }
    Ok(m)
}

fn write_chord(m: &mut Mapping, key: &str, keys: &[String]) {
    if !keys.is_empty() {
        insert(
            m,
            key,
            Value::Sequence(keys.iter().cloned().map(Value::String).collect()),
        );
    }
}

fn write_coords(m: &mut Mapping, c: &CoordinateOutputs) {
    if !c.output_x_variable.is_empty() {
        insert_str(m, "outputxvariable", &c.output_x_variable);
    }
    if !c.output_y_variable.is_empty() {
        insert_str(m, "outputyvariable", &c.output_y_variable);
    }
}

fn write_order(m: &mut Mapping, o: &MatchOrder) {
    if !o.grouping.is_empty() {
        insert_str(m, "grouping", &o.grouping);
    }
    if !o.horizontal.is_empty() {
        insert_str(m, "horizontal", &o.horizontal);
    }
    if !o.vertical.is_empty() {
        insert_str(m, "vertical", &o.vertical);
    }
}

fn write_wait(m: &mut Mapping, w: &WaitTilFoundConfig) {
    let mode = w.effective_repeat_mode().to_string();
    insert_str(m, "repeatmode", &mode);
    if mode == REPEAT_WAIT_UNTIL_FOUND {
        insert_i32(m, "waittilfoundseconds", w.wait_til_found_seconds);
    } else if w.wait_til_found_seconds > 0 {
        insert_i32(m, "waittilfoundseconds", w.wait_til_found_seconds);
    }
    if w.wait_til_found_interval_ms > 0 {
        insert_i32(m, "waittilfoundintervalms", w.wait_til_found_interval_ms);
    }
    if mode == REPEAT_WHILE_FOUND {
        let max = if w.max_iterations > 0 {
            w.max_iterations
        } else {
            100
        };
        insert_i32(m, "maxiterations", max);
    }
}

fn decode_wait(raw: &Mapping) -> WaitTilFoundConfig {
    let mut out = WaitTilFoundConfig {
        repeat_mode: string_from_map(raw, "repeatmode"),
        wait_til_found_seconds: optional_int(raw, "waittilfoundseconds").unwrap_or(0),
        wait_til_found_interval_ms: optional_int(raw, "waittilfoundintervalms").unwrap_or(0),
        max_iterations: optional_int(raw, "maxiterations").unwrap_or(0),
    };
    if out.repeat_mode.is_empty() {
        out.repeat_mode = REPEAT_ONCE.to_string();
    }
    out
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

fn decode_order(raw: &Mapping) -> MatchOrder {
    MatchOrder {
        grouping: string_from_map(raw, "grouping"),
        horizontal: string_from_map(raw, "horizontal"),
        vertical: string_from_map(raw, "vertical"),
    }
}

fn subactions_to_seq(subs: &[Action]) -> Result<Value> {
    let mut seq = Sequence::new();
    for sub in subs {
        seq.push(Value::Mapping(action_to_map(sub)?));
    }
    Ok(Value::Sequence(seq))
}

fn decode_subactions(raw: &Mapping) -> Result<Vec<Action>> {
    let Some(Value::Sequence(seq)) = raw.get(Value::String("subactions".into())) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(seq.len());
    for (i, item) in seq.iter().enumerate() {
        let map = as_mapping(item).map_err(|e| {
            SerializeError::msg(format!("subactions[{i}]: {e}"))
        })?;
        out.push(action_from_map(map).map_err(|e| {
            SerializeError::msg(format!("subactions[{i}]: {e}"))
        })?);
    }
    Ok(out)
}

fn clauses_to_seq(clauses: &[ConditionClause]) -> Value {
    let mut seq = Sequence::new();
    for c in clauses {
        let mut m = Mapping::new();
        insert(&mut m, "left", c.left.to_yaml());
        insert_str(&mut m, "operator", &c.operator);
        insert(&mut m, "right", c.right.to_yaml());
        seq.push(Value::Mapping(m));
    }
    Value::Sequence(seq)
}

fn decode_clauses(raw: &Mapping) -> Vec<ConditionClause> {
    let Some(Value::Sequence(seq)) = raw.get(Value::String("clauses".into())) else {
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
            left: scalar_from_value(m.get(Value::String("left".into()))),
            operator: op,
            right: scalar_from_value(m.get(Value::String("right".into()))),
        });
    }
    if out.is_empty() {
        out.push(ConditionClause::default());
    }
    out
}

fn list_columns_to_seq(cols: &[ListColumn]) -> Value {
    let mut seq = Sequence::new();
    for c in cols {
        let mut m = Mapping::new();
        insert_str(&mut m, "source", &c.source);
        insert_str(&mut m, "outputvar", &c.output_var);
        if c.is_file {
            insert_bool(&mut m, "isfile", true);
        }
        if c.skip_blank_lines {
            insert_bool(&mut m, "skipblanklines", true);
        }
        seq.push(Value::Mapping(m));
    }
    Value::Sequence(seq)
}

fn decode_sources(raw: &Mapping) -> Vec<ListColumn> {
    let Some(Value::Sequence(seq)) = raw.get(Value::String("sources".into())) else {
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

fn decode_kind(raw: &Mapping, type_name: &str) -> Result<ActionKind> {
    match type_name {
        "loop" => {
            let name = expect_string(raw, "name")
                .map_err(|e| SerializeError::msg(format!("action type loop: {e}")))?;
            let count = match raw.get(Value::String("count".into())) {
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
            name: string_from_map(raw, "name"),
            match_mode: string_from_map(raw, "match"),
            clauses: decode_clauses(raw),
            max_iterations: optional_int(raw, "maxiterations").unwrap_or(0),
            subactions: decode_subactions(raw)?,
        }),
        "conditional" => Ok(ActionKind::Conditional {
            name: string_from_map(raw, "name"),
            match_mode: string_from_map(raw, "match"),
            clauses: decode_clauses(raw),
            subactions: decode_subactions(raw)?,
        }),
        "imagesearch" => {
            let name = expect_string(raw, "name")
                .map_err(|e| SerializeError::msg(format!("action type imagesearch: {e}")))?;
            let targets = raw
                .get(Value::String("targets".into()))
                .map(string_slice_from_value)
                .unwrap_or_default();
            let blur = optional_int(raw, "blur").unwrap_or(5);
            let tolerance = raw
                .get(Value::String("tolerance".into()))
                .map(float_from_value)
                .unwrap_or(0.0);
            Ok(ActionKind::ImageSearch {
                name,
                targets,
                search_area: parse_coordinate_ref(raw.get(Value::String("searcharea".into()))),
                tolerance,
                blur,
                wait: decode_wait(raw),
                coords: decode_coords(raw),
                run_branch_on_no_find: bool_from_map(raw, "runbranchonnofind"),
                order: decode_order(raw),
                subactions: decode_subactions(raw)?,
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
                search_area: parse_coordinate_ref(raw.get(Value::String("searcharea".into()))),
                output_variable: {
                    let v = string_from_map(raw, "outputvariable");
                    if v.is_empty() {
                        "ocrText".into()
                    } else {
                        v
                    }
                },
                coords: decode_coords(raw),
                wait: decode_wait(raw),
                run_branch_on_no_find: bool_from_map(raw, "runbranchonnofind"),
                blur,
                min_threshold: optional_int(raw, "minthreshold").unwrap_or(0),
                resize: raw
                    .get(Value::String("resize".into()))
                    .map(float_from_value)
                    .unwrap_or(1.0),
                grayscale: raw
                    .get(Value::String("grayscale".into()))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                threshold_otsu: bool_from_map(raw, "thresholdotsu"),
                threshold_invert: bool_from_map(raw, "thresholdinvert"),
                order: decode_order(raw),
                subactions: decode_subactions(raw)?,
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
                search_area: parse_coordinate_ref(raw.get(Value::String("searcharea".into()))),
                target_color: color,
                color_tolerance: tol,
                wait: decode_wait(raw),
                coords: decode_coords(raw),
                run_branch_on_no_find: bool_from_map(raw, "runbranchonnofind"),
                order: decode_order(raw),
                subactions: decode_subactions(raw)?,
            })
        }
        "foreachrow" => Ok(ActionKind::ForEachRow {
            name: string_from_map(raw, "name"),
            sources: decode_sources(raw),
            start_row: scalar_from_value(raw.get(Value::String("startrow".into()))),
            end_row: scalar_from_value(raw.get(Value::String("endrow".into()))),
            subactions: decode_subactions(raw)?,
        }),
        "wait" => Ok(ActionKind::Wait {
            time: match raw.get(Value::String("time".into())) {
                None | Some(Value::Null) => ScalarValue::Int(0),
                Some(v) => ScalarValue::from_yaml(v),
            },
        }),
        "pause" => Ok(ActionKind::Pause {
            message: string_from_map(raw, "message"),
            continue_key: raw
                .get(Value::String("continuekey".into()))
                .map(string_slice_from_value)
                .unwrap_or_default(),
            pass_through: bool_from_map(raw, "passthrough"),
        }),
        "move" => {
            let smooth = bool_from_map(raw, "smooth");
            Ok(ActionKind::Move {
                point: parse_coordinate_ref(raw.get(Value::String("point".into()))),
                smooth,
                smooth_low: raw
                    .get(Value::String("smoothlow".into()))
                    .map(float_from_value)
                    .unwrap_or(if smooth { DEFAULT_SMOOTH_LOW } else { 0.0 }),
                smooth_high: raw
                    .get(Value::String("smoothhigh".into()))
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
                "left" | "right" | "center" | "scroll" => {}
                other => {
                    return Err(SerializeError::msg(format!(
                        "action type click: field \"button\": unknown button \"{other}\""
                    )));
                }
            }
            Ok(ActionKind::Click {
                button,
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
            value: raw
                .get(Value::String("value".into()))
                .cloned()
                .unwrap_or(Value::Null),
        }),
        // Legacy calculate actions load as Set (expression → value, outputvar → variablename).
        "calculate" => Ok(ActionKind::SetVariable {
            variable_name: expect_string(raw, "outputvar")
                .map_err(|e| SerializeError::msg(format!("action type calculate: {e}")))?,
            value: Value::String(
                expect_string(raw, "expression")
                    .map_err(|e| SerializeError::msg(format!("action type calculate: {e}")))?,
            ),
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
        "navigateselect" => Ok(ActionKind::NavigateSelect {
            program: string_from_map(raw, "program"),
            graph_name: string_from_map(raw, "graphname"),
            chord_up: raw
                .get(Value::String("chordup".into()))
                .map(string_slice_from_value)
                .unwrap_or_default(),
            chord_down: raw
                .get(Value::String("chorddown".into()))
                .map(string_slice_from_value)
                .unwrap_or_default(),
            chord_left: raw
                .get(Value::String("chordleft".into()))
                .map(string_slice_from_value)
                .unwrap_or_default(),
            chord_right: raw
                .get(Value::String("chordright".into()))
                .map(string_slice_from_value)
                .unwrap_or_default(),
            chord_select: raw
                .get(Value::String("chordselect".into()))
                .map(string_slice_from_value)
                .unwrap_or_default(),
            chord_back: raw
                .get(Value::String("chordback".into()))
                .map(string_slice_from_value)
                .unwrap_or_default(),
            wrap_edges: bool_from_map(raw, "wrapedges"),
            move_cursor_with_nav: bool_from_map(raw, "movecursorwithnav"),
            smooth: bool_from_map(raw, "smooth"),
            pass_through: bool_from_map(raw, "passthrough"),
            hold_repeat: bool_from_map(raw, "holdrepeat"),
            select_device: string_from_map(raw, "selectdevice"),
            select_button: string_from_map(raw, "selectbutton"),
            select_key: string_from_map(raw, "selectkey"),
            select_press_mode: string_from_map(raw, "selectpressmode"),
            in_graph: string_from_map(raw, "ingraph"),
            in_row: string_from_map(raw, "inrow"),
            in_col: string_from_map(raw, "incol"),
            in_collection: string_from_map(raw, "incollection"),
            output_ref: string_from_map(raw, "outputref"),
            output_graph: string_from_map(raw, "outputgraph"),
            output_row: string_from_map(raw, "outputrow"),
            output_col: string_from_map(raw, "outputcol"),
            output_collection: string_from_map(raw, "outputcollection"),
            subactions: decode_subactions(raw)?,
        }),
        "navigatekey" => Ok(ActionKind::NavigateKey {
            name: string_from_map(raw, "name"),
            chord: raw
                .get(Value::String("chord".into()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, ScalarValue};

    #[test]
    fn action_to_map_with_uid_preserves_nested_uids() {
        let child = Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        };
        let child_id = child.id;
        let nested = Action {
            id: ActionId::new(),
            kind: ActionKind::Loop {
                name: "inner".into(),
                count: ScalarValue::Int(1),
                subactions: vec![child],
            },
        };
        let nested_id = nested.id;
        let root = root_loop(vec![nested]);
        let m = action_to_map_with_uid(&root).unwrap();
        let restored = action_from_map(&m).unwrap();
        assert_eq!(restored.children()[0].id, nested_id);
        assert_eq!(restored.children()[0].children()[0].id, child_id);
    }

    #[test]
    fn navigate_select_with_key_branch_roundtrips() {
        let kid = Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        };
        let branch = Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateKey {
                name: "Inspect".into(),
                chord: vec!["i".into()],
                exit: true,
                subactions: vec![kid],
            },
        };
        let branch_id = branch.id;
        let nav = Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateSelect {
                program: "P".into(),
                graph_name: "bag".into(),
                chord_up: vec!["up".into()],
                chord_down: vec!["down".into()],
                chord_left: vec![],
                chord_right: vec![],
                chord_select: vec!["enter".into()],
                chord_back: vec!["esc".into()],
                wrap_edges: true,
                move_cursor_with_nav: true,
                smooth: false,
                pass_through: false,
                hold_repeat: true,
                select_device: "mouse".into(),
                select_button: "left".into(),
                select_key: String::new(),
                select_press_mode: "click".into(),
                in_graph: String::new(),
                in_row: String::new(),
                in_col: String::new(),
                in_collection: String::new(),
                output_ref: "ref".into(),
                output_graph: String::new(),
                output_row: "r".into(),
                output_col: "c".into(),
                output_collection: String::new(),
                subactions: vec![branch],
            },
        };
        let nav_id = nav.id;
        let m = action_to_map_with_uid(&nav).unwrap();
        let restored = action_from_map(&m).unwrap();
        assert_eq!(restored.id, nav_id);
        assert!(restored.is_branch());
        match &restored.kind {
            ActionKind::NavigateSelect {
                program,
                wrap_edges,
                hold_repeat,
                subactions,
                ..
            } => {
                assert_eq!(program, "P");
                assert!(*wrap_edges);
                assert!(*hold_repeat);
                assert_eq!(subactions.len(), 1);
                assert_eq!(subactions[0].id, branch_id);
                match &subactions[0].kind {
                    ActionKind::NavigateKey {
                        name,
                        chord,
                        exit,
                        subactions: kids,
                    } => {
                        assert_eq!(name, "Inspect");
                        assert_eq!(chord, &vec!["i".to_string()]);
                        assert!(*exit);
                        assert_eq!(kids.len(), 1);
                    }
                    other => panic!("expected NavigateKey, got {other:?}"),
                }
            }
            other => panic!("expected NavigateSelect, got {other:?}"),
        }
    }

    #[test]
    fn legacy_calculate_decodes_as_set() {
        use serde_yaml::{Mapping, Value};

        let mut m = Mapping::new();
        m.insert(
            Value::String("type".into()),
            Value::String("calculate".into()),
        );
        m.insert(
            Value::String("expression".into()),
            Value::String("1+2".into()),
        );
        m.insert(
            Value::String("outputvar".into()),
            Value::String("sum".into()),
        );
        let action = action_from_map(&m).unwrap();
        match action.kind {
            ActionKind::SetVariable {
                variable_name,
                value,
            } => {
                assert_eq!(variable_name, "sum");
                assert_eq!(value, Value::String("1+2".into()));
            }
            other => panic!("expected SetVariable, got {other:?}"),
        }
    }
}
