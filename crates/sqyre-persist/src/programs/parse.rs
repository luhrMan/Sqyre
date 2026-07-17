//! YAML parse helpers for program entities.

use super::types::*;
use crate::Result;
use serde_yaml::{Mapping, Value};
use sqyre_domain::{MaskShape, ScalarValue};
use std::collections::BTreeMap;

pub(super) fn parse_program(name: &str, v: &Value) -> Result<ProgramData> {
    let mut data = ProgramData {
        name: name.to_string(),
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return Ok(data);
    };

    if let Some(n) = map
        .get(Value::String("name".into()))
        .and_then(|x| x.as_str())
    {
        data.name = n.to_string();
    }
    if let Some(p) = map
        .get(Value::String("processpath".into()))
        .and_then(|x| x.as_str())
    {
        data.process_path = p.to_string();
    }
    if let Some(t) = map
        .get(Value::String("windowtitle".into()))
        .and_then(|x| x.as_str())
    {
        data.window_title = t.to_string();
    }

    if let Some(Value::Mapping(items)) = map.get(Value::String("items".into())) {
        for (ik, iv) in items {
            let iname = ik.as_str().unwrap_or("").to_string();
            if iname.is_empty() {
                continue;
            }
            data.items.insert(iname.clone(), parse_item(&iname, iv));
        }
    }

    if let Some(Value::Mapping(coords)) = map.get(Value::String("coordinates".into())) {
        for (rk, rv) in coords {
            let res = rk.as_str().unwrap_or("").to_string();
            if res.is_empty() {
                continue;
            }
            let Some(block) = rv.as_mapping() else {
                continue;
            };
            if let Some(Value::Mapping(pts)) = block.get(Value::String("points".into())) {
                let mut m = BTreeMap::new();
                for (pk, pv) in pts {
                    let pname = pk.as_str().unwrap_or("").to_string();
                    if pname.is_empty() {
                        continue;
                    }
                    m.insert(pname.clone(), parse_point(&pname, pv));
                }
                data.points.insert(res.clone(), m);
            }
            if let Some(Value::Mapping(sas)) = block.get(Value::String("searchareas".into())) {
                let mut m = BTreeMap::new();
                for (sk, sv) in sas {
                    let sname = sk.as_str().unwrap_or("").to_string();
                    if sname.is_empty() {
                        continue;
                    }
                    m.insert(sname.clone(), parse_search_area(&sname, sv));
                }
                data.search_areas.insert(res, m);
            }
        }
    }

    if let Some(Value::Mapping(masks)) = map.get(Value::String("masks".into())) {
        for (mk, mv) in masks {
            let mname = mk.as_str().unwrap_or("").to_string();
            if mname.is_empty() {
                continue;
            }
            data.masks.insert(mname.clone(), parse_mask(&mname, mv));
        }
    }

    if let Some(Value::Mapping(cols)) = map.get(Value::String("collections".into())) {
        for (ck, cv) in cols {
            let cname = ck.as_str().unwrap_or("").to_string();
            if cname.is_empty() {
                continue;
            }
            data.collections
                .insert(cname.clone(), parse_collection(&cname, cv));
        }
    }

    Ok(data)
}

pub(super) fn parse_item(name: &str, v: &Value) -> ProgramItem {
    let mut item = ProgramItem {
        name: name.to_string(),
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return item;
    };
    if let Some(n) = map
        .get(Value::String("name".into()))
        .and_then(|x| x.as_str())
    {
        item.name = n.to_string();
    }
    if let Some(m) = map
        .get(Value::String("mask".into()))
        .and_then(|x| x.as_str())
    {
        item.mask = m.to_string();
    }
    if let Some(n) = yaml_i64(map.get(Value::String("stackmax".into()))) {
        item.stack_max = n as i32;
    }
    if let Some(Value::Sequence(g)) = map.get(Value::String("gridsize".into())) {
        if g.len() >= 2 {
            item.grid_cols = yaml_i64(Some(&g[0])).unwrap_or(0) as i32;
            item.grid_rows = yaml_i64(Some(&g[1])).unwrap_or(0) as i32;
        }
    }
    if let Some(Value::Sequence(tags)) = map.get(Value::String("tags".into())) {
        item.tags = tags
            .iter()
            .filter_map(|t| t.as_str().map(str::to_string))
            .collect();
    }
    item
}

pub(super) fn parse_point(name: &str, v: &Value) -> ProgramPoint {
    let mut pt = ProgramPoint {
        name: name.to_string(),
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return pt;
    };
    if let Some(n) = map
        .get(Value::String("name".into()))
        .and_then(|x| x.as_str())
    {
        pt.name = n.to_string();
    }
    pt.x = scalar_field(map.get(Value::String("x".into())));
    pt.y = scalar_field(map.get(Value::String("y".into())));
    pt
}

pub(super) fn parse_search_area(name: &str, v: &Value) -> ProgramSearchArea {
    let mut sa = ProgramSearchArea {
        name: name.to_string(),
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return sa;
    };
    if let Some(n) = map
        .get(Value::String("name".into()))
        .and_then(|x| x.as_str())
    {
        sa.name = n.to_string();
    }
    sa.left_x = scalar_field(map.get(Value::String("leftx".into())));
    sa.top_y = scalar_field(map.get(Value::String("topy".into())));
    sa.right_x = scalar_field(map.get(Value::String("rightx".into())));
    sa.bottom_y = scalar_field(map.get(Value::String("bottomy".into())));
    sa
}

pub(super) fn parse_mask(name: &str, v: &Value) -> ProgramMask {
    let mut mask = ProgramMask {
        name: name.to_string(),
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return mask;
    };
    if let Some(n) = map
        .get(Value::String("name".into()))
        .and_then(|x| x.as_str())
    {
        mask.name = n.to_string();
    }
    if let Some(s) = map
        .get(Value::String("shape".into()))
        .and_then(|x| x.as_str())
    {
        mask.shape = MaskShape::parse(s);
    }
    mask.center_x = yaml_string_field(map.get(Value::String("centerx".into())), "50");
    mask.center_y = yaml_string_field(map.get(Value::String("centery".into())), "50");
    mask.base = yaml_string_field(map.get(Value::String("base".into())), "");
    mask.height = yaml_string_field(map.get(Value::String("height".into())), "");
    mask.radius = yaml_string_field(map.get(Value::String("radius".into())), "");
    if let Some(Value::Bool(b)) = map.get(Value::String("inverse".into())) {
        mask.inverse = *b;
    }
    mask
}

pub(super) fn parse_collection(name: &str, v: &Value) -> ProgramCollection {
    let mut col = ProgramCollection {
        name: name.to_string(),
        rows: 1,
        cols: 1,
        ..Default::default()
    };
    let Some(map) = v.as_mapping() else {
        return col;
    };
    if let Some(n) = map
        .get(Value::String("name".into()))
        .and_then(|x| x.as_str())
    {
        col.name = n.to_string();
    }
    if let Some(s) = map
        .get(Value::String("searcharea".into()))
        .and_then(|x| x.as_str())
    {
        col.search_area = s.to_string();
    }
    if let Some(n) = yaml_i64(map.get(Value::String("rows".into()))) {
        col.rows = n.max(1) as i32;
    }
    if let Some(n) = yaml_i64(map.get(Value::String("cols".into()))) {
        col.cols = n.max(1) as i32;
    }
    col
}

pub(super) fn yaml_string_field(v: Option<&Value>, default: &str) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Null) | None => default.to_string(),
        Some(other) => format!("{other:?}"),
    }
}

pub(super) fn scalar_field(v: Option<&Value>) -> ScalarValue {
    match v {
        Some(val) => ScalarValue::from_yaml(val),
        None => ScalarValue::Null,
    }
}

pub(super) fn yaml_i64(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}
