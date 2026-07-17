use crate::helpers::*;
use crate::{Result, SerializeError};
use serde_yaml::{Mapping, Sequence, Value};
use sqyre_domain::{
    Action, ActionId, ActionKind, ConditionBlock, ConditionClause, CoordinateOutputs,
    DetectionBranch, ListColumn, MatchMode, MatchOrder, MouseButton, RepeatMode, ScalarValue,
    WaitTilFoundConfig, DEFAULT_SMOOTH_DELAY_MS, DEFAULT_SMOOTH_HIGH, DEFAULT_SMOOTH_LOW,
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
            condition,
            max_iterations,
            subactions,
        } => {
            insert_str(&mut m, "name", &condition.name);
            insert_str(&mut m, "match", condition.match_mode.as_str());
            if *max_iterations > 0 {
                insert_i32(&mut m, "maxiterations", *max_iterations);
            }
            insert(&mut m, "clauses", clauses_to_seq(&condition.clauses));
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::Conditional {
            condition,
            subactions,
        } => {
            insert_str(&mut m, "name", &condition.name);
            insert_str(&mut m, "match", condition.match_mode.as_str());
            insert(&mut m, "clauses", clauses_to_seq(&condition.clauses));
            insert(&mut m, "subactions", subactions_to_seq(subactions)?);
        }
        ActionKind::ImageSearch {
            name,
            targets,
            search_area,
            tolerance,
            blur,
            detection,
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
            write_detection(&mut m, detection)?;
        }
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
        } => {
            insert_str(&mut m, "name", name);
            insert_str(&mut m, "target", target);
            insert(&mut m, "searcharea", coordinate_ref_to_value(search_area));
            if !output_variable.is_empty() {
                insert_str(&mut m, "outputvariable", output_variable);
            }
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
            write_detection(&mut m, detection)?;
        }
        ActionKind::FindPixel {
            name,
            search_area,
            target_color,
            color_tolerance,
            detection,
        } => {
            insert_str(&mut m, "name", name);
            insert(&mut m, "searcharea", coordinate_ref_to_value(search_area));
            insert_str(&mut m, "targetcolor", target_color);
            insert_i32(&mut m, "colortolerance", *color_tolerance);
            write_detection(&mut m, detection)?;
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
            insert_str(&mut m, "button", button.as_str());
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
        ActionKind::NavigateSelect(data) => {
            if !data.program.is_empty() {
                insert_str(&mut m, "program", &data.program);
            }
            if !data.graph_name.is_empty() {
                insert_str(&mut m, "graphname", &data.graph_name);
            }
            write_nav_chords(&mut m, &data.chords);
            write_nav_options(&mut m, &data.options);
            write_nav_select(&mut m, &data.select);
            write_nav_inputs(&mut m, &data.inputs);
            write_nav_outputs(&mut m, &data.outputs);
            insert(&mut m, "subactions", subactions_to_seq(&data.subactions)?);
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

fn write_nav_chords(m: &mut Mapping, c: &sqyre_domain::NavChords) {
    write_chord(m, "chordup", &c.up);
    write_chord(m, "chorddown", &c.down);
    write_chord(m, "chordleft", &c.left);
    write_chord(m, "chordright", &c.right);
    write_chord(m, "chordselect", &c.select);
    write_chord(m, "chordback", &c.back);
}

fn write_nav_options(m: &mut Mapping, o: &sqyre_domain::NavOptions) {
    if o.wrap_edges {
        insert_bool(m, "wrapedges", true);
    }
    if o.move_cursor_with_nav {
        insert_bool(m, "movecursorwithnav", true);
    }
    if o.smooth {
        insert_bool(m, "smooth", true);
    }
    if o.pass_through {
        insert_bool(m, "passthrough", true);
    }
    if o.hold_repeat {
        insert_bool(m, "holdrepeat", true);
    }
}

fn write_nav_select(m: &mut Mapping, s: &sqyre_domain::NavSelectAction) {
    if !s.device.is_empty() {
        insert_str(m, "selectdevice", &s.device);
    }
    if !s.button.is_empty() {
        insert_str(m, "selectbutton", &s.button);
    }
    if !s.key.is_empty() {
        insert_str(m, "selectkey", &s.key);
    }
    if !s.press_mode.is_empty() {
        insert_str(m, "selectpressmode", &s.press_mode);
    }
}

fn write_nav_inputs(m: &mut Mapping, i: &sqyre_domain::NavInputs) {
    if !i.graph.is_empty() {
        insert_str(m, "ingraph", &i.graph);
    }
    if !i.row.is_empty() {
        insert_str(m, "inrow", &i.row);
    }
    if !i.col.is_empty() {
        insert_str(m, "incol", &i.col);
    }
    if !i.collection.is_empty() {
        insert_str(m, "incollection", &i.collection);
    }
}

fn write_nav_outputs(m: &mut Mapping, o: &sqyre_domain::NavOutputs) {
    if !o.output_ref.is_empty() {
        insert_str(m, "outputref", &o.output_ref);
    }
    if !o.output_graph.is_empty() {
        insert_str(m, "outputgraph", &o.output_graph);
    }
    if !o.output_row.is_empty() {
        insert_str(m, "outputrow", &o.output_row);
    }
    if !o.output_col.is_empty() {
        insert_str(m, "outputcol", &o.output_col);
    }
    if !o.output_collection.is_empty() {
        insert_str(m, "outputcollection", &o.output_collection);
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

fn write_detection(m: &mut Mapping, d: &DetectionBranch) -> Result<()> {
    write_wait(m, &d.wait);
    write_coords(m, &d.coords);
    write_order(m, &d.order);
    if d.run_branch_on_no_find {
        insert_bool(m, "runbranchonnofind", true);
    }
    insert(m, "subactions", subactions_to_seq(&d.subactions)?);
    Ok(())
}

fn decode_detection(raw: &Mapping) -> Result<DetectionBranch> {
    Ok(DetectionBranch {
        wait: decode_wait(raw),
        coords: decode_coords(raw),
        run_branch_on_no_find: bool_from_map(raw, "runbranchonnofind"),
        order: decode_order(raw),
        subactions: decode_subactions(raw)?,
    })
}

fn write_wait(m: &mut Mapping, w: &WaitTilFoundConfig) {
    let mode = w.repeat_mode;
    insert_str(m, "repeatmode", mode.as_str());
    if mode == RepeatMode::WaitUntilFound || w.wait_til_found_seconds > 0 {
        insert_i32(m, "waittilfoundseconds", w.wait_til_found_seconds);
    }
    if w.wait_til_found_interval_ms > 0 {
        insert_i32(m, "waittilfoundintervalms", w.wait_til_found_interval_ms);
    }
    if mode == RepeatMode::WhileFound {
        let max = if w.max_iterations > 0 {
            w.max_iterations
        } else {
            100
        };
        insert_i32(m, "maxiterations", max);
    }
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

fn subactions_to_seq(subs: &[Action]) -> Result<Value> {
    let mut seq = Sequence::new();
    for sub in subs {
        seq.push(Value::Mapping(action_to_map(sub)?));
    }
    Ok(Value::Sequence(seq))
}

fn decode_subactions(raw: &Mapping) -> Result<Vec<Action>> {
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

fn decode_kind(raw: &Mapping, type_name: &str) -> Result<ActionKind> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;
    use sqyre_domain::{root_loop, CoordinateRef, NavigateSelectData, ScalarValue};

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
            kind: ActionKind::NavigateSelect(Box::new(NavigateSelectData {
                program: "P".into(),
                graph_name: "bag".into(),
                chords: sqyre_domain::NavChords {
                    up: vec!["up".into()],
                    down: vec!["down".into()],
                    left: vec![],
                    right: vec![],
                    select: vec!["enter".into()],
                    back: vec!["esc".into()],
                },
                options: sqyre_domain::NavOptions {
                    wrap_edges: true,
                    move_cursor_with_nav: true,
                    smooth: false,
                    pass_through: false,
                    hold_repeat: true,
                },
                select: sqyre_domain::NavSelectAction {
                    device: "mouse".into(),
                    button: "left".into(),
                    key: String::new(),
                    press_mode: "click".into(),
                },
                inputs: sqyre_domain::NavInputs::default(),
                outputs: sqyre_domain::NavOutputs {
                    output_ref: "ref".into(),
                    output_graph: String::new(),
                    output_row: "r".into(),
                    output_col: "c".into(),
                    output_collection: String::new(),
                },
                subactions: vec![branch],
            })),
        };
        let nav_id = nav.id;
        let m = action_to_map_with_uid(&nav).unwrap();
        let restored = action_from_map(&m).unwrap();
        assert_eq!(restored.id, nav_id);
        assert!(restored.is_branch());
        match &restored.kind {
            ActionKind::NavigateSelect(data) => {
                assert_eq!(data.program, "P");
                assert!(data.options.wrap_edges);
                assert!(data.options.hold_repeat);
                assert_eq!(data.subactions.len(), 1);
                assert_eq!(data.subactions[0].id, branch_id);
                match &data.subactions[0].kind {
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
    fn blank_action_kinds_roundtrip_encode_decode() {
        use sqyre_domain::action_templates;
        for tmpl in action_templates() {
            let action = tmpl.create();
            let encoded = action_to_map(&action).unwrap();
            let decoded = action_from_map(&encoded).unwrap();
            assert_eq!(
                decoded.type_key(),
                tmpl.action_type,
                "type_key mismatch for {}",
                tmpl.action_type
            );
            // Codec applies defaults on decode (empty optional fields). A second
            // encode→decode must be idempotent.
            let reencoded = action_to_map(&decoded).unwrap();
            let redecoded = action_from_map(&reencoded).unwrap();
            assert_eq!(
                decoded.kind, redecoded.kind,
                "encode/decode not idempotent for {}",
                tmpl.action_type
            );
        }
    }

    #[test]
    fn decode_rejects_missing_type() {
        let mut m = Mapping::new();
        insert_str(&mut m, "name", "x");
        let err = action_from_map(&m).unwrap_err();
        assert!(err.to_string().contains("type"), "{err}");
    }

    #[test]
    fn decode_rejects_unknown_type() {
        let mut m = Mapping::new();
        insert_str(&mut m, "type", "notarealaction");
        let err = action_from_map(&m).unwrap_err();
        assert!(
            err.to_string().to_ascii_lowercase().contains("unknown")
                || err.to_string().contains("notarealaction"),
            "{err}"
        );
    }

    #[test]
    fn image_search_populated_fields_roundtrip() {
        use sqyre_domain::{MatchOrder, RepeatMode, WaitTilFoundConfig};
        let action = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "find sword".into(),
                targets: vec!["Game~Sword".into(), "Game~Shield".into()],
                search_area: CoordinateRef("Game~Arena".into()),
                tolerance: 0.87,
                blur: 3,
                detection: DetectionBranch {
                    wait: WaitTilFoundConfig {
                        repeat_mode: RepeatMode::WaitUntilFound,
                        wait_til_found_seconds: 12,
                        wait_til_found_interval_ms: 250,
                        max_iterations: 0,
                    },
                    coords: CoordinateOutputs {
                        output_x_variable: "sx".into(),
                        output_y_variable: "sy".into(),
                    },
                    run_branch_on_no_find: true,
                    order: MatchOrder::default(),
                    subactions: vec![Action {
                        id: ActionId::new(),
                        kind: ActionKind::Click {
                            button: "left".into(),
                            state: true,
                        },
                    }],
                },
            },
        };
        let map = action_to_map_with_uid(&action).unwrap();
        assert!(!map_get(&map, "detection").is_some());
        assert_eq!(
            string_from_map(&map, "repeatmode"),
            RepeatMode::WaitUntilFound.as_str()
        );
        assert_eq!(string_from_map(&map, "outputxvariable"), "sx");
        assert!(map_get(&map, "subactions").is_some());
        let back = action_from_map(&map).unwrap();
        assert_eq!(back.id, action.id);
        assert_eq!(back.kind, action.kind);
    }

    #[test]
    fn while_found_preserves_max_iterations() {
        use sqyre_domain::{MatchOrder, RepeatMode, WaitTilFoundConfig};
        let action = Action {
            id: ActionId::new(),
            kind: ActionKind::FindPixel {
                name: "loop".into(),
                search_area: CoordinateRef("Game~Arena".into()),
                target_color: "#ffffff".into(),
                color_tolerance: 2,
                detection: DetectionBranch {
                    wait: WaitTilFoundConfig {
                        repeat_mode: RepeatMode::WhileFound,
                        wait_til_found_seconds: 0,
                        wait_til_found_interval_ms: 100,
                        max_iterations: 40,
                    },
                    coords: CoordinateOutputs::defaults(),
                    run_branch_on_no_find: false,
                    order: MatchOrder::default(),
                    subactions: vec![],
                },
            },
        };
        let back = action_from_map(&action_to_map(&action).unwrap()).unwrap();
        match back.kind {
            ActionKind::FindPixel { detection, .. } => {
                assert_eq!(detection.wait.repeat_mode, RepeatMode::WhileFound);
                assert_eq!(detection.wait.max_iterations, 40);
                assert_eq!(detection.wait.wait_til_found_interval_ms, 100);
            }
            other => panic!("expected FindPixel, got {other:?}"),
        }
    }

    #[test]
    fn image_search_accepts_legacy_singular_target() {
        let yaml = r#"
type: imagesearch
name: find
target: Game~Sword
searcharea: Game~Arena
tolerance: 0.9
blur: 3
repeatmode: once
"#;
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let m = value.as_mapping().unwrap();
        let action = action_from_map(m).unwrap();
        match action.kind {
            ActionKind::ImageSearch {
                targets,
                search_area,
                tolerance,
                blur,
                ..
            } => {
                assert_eq!(targets, vec!["Game~Sword".to_string()]);
                assert_eq!(search_area.as_str(), "Game~Arena");
                assert!((tolerance - 0.9).abs() < 1e-9);
                assert_eq!(blur, 3);
            }
            other => panic!("expected ImageSearch, got {other:?}"),
        }
    }

    #[test]
    fn image_search_accepts_nested_searcharea_mapping() {
        let yaml = r#"
type: imagesearch
name: find
targets: [Game~A, Game~B]
searcharea:
  name: Game~Box
tolerance: 0.95
"#;
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let action = action_from_map(value.as_mapping().unwrap()).unwrap();
        match action.kind {
            ActionKind::ImageSearch {
                targets,
                search_area,
                ..
            } => {
                assert_eq!(targets.len(), 2);
                assert_eq!(search_area.as_str(), "Game~Box");
            }
            other => panic!("expected ImageSearch, got {other:?}"),
        }
    }
}
