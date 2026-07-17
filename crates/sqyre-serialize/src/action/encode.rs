//! ActionKind → YAML mapping.

use crate::helpers::*;
use crate::Result;
use serde_yaml::{Mapping, Sequence, Value};
use sqyre_domain::{
    Action, ActionKind, ConditionClause, CoordinateOutputs,
    DetectionBranch, ListColumn, MatchOrder, RepeatMode,
    WaitTilFoundConfig, DEFAULT_SMOOTH_DELAY_MS, DEFAULT_SMOOTH_HIGH, DEFAULT_SMOOTH_LOW,
};

use super::action_to_map;

pub(super) fn encode_kind(kind: &ActionKind) -> Result<Mapping> {
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
pub(super) fn subactions_to_seq(subs: &[Action]) -> Result<Value> {
    let mut seq = Sequence::new();
    for sub in subs {
        seq.push(Value::Mapping(action_to_map(sub)?));
    }
    Ok(Value::Sequence(seq))
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
