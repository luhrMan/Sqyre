//! YAML encode helpers for program entities.

use super::types::*;
use serde_yaml::{Mapping, Value};
use sqyre_domain::MaskShape;
use std::collections::BTreeMap;

pub(super) fn encode_program(data: &ProgramData, previous: &Mapping) -> Value {
    let mut map = Mapping::new();
    map.insert(
        Value::String("name".into()),
        Value::String(data.name.clone()),
    );
    if !data.process_path.is_empty() {
        map.insert(
            Value::String("processpath".into()),
            Value::String(data.process_path.clone()),
        );
    }
    if !data.window_title.is_empty() {
        map.insert(
            Value::String("windowtitle".into()),
            Value::String(data.window_title.clone()),
        );
    }

    let mut items = Mapping::new();
    for (k, item) in &data.items {
        items.insert(Value::String(k.clone()), encode_item(item));
    }
    map.insert(Value::String("items".into()), Value::Mapping(items));

    // Union resolution keys from points and search_areas.
    let mut res_keys: BTreeMap<String, ()> = BTreeMap::new();
    for k in data.points.keys() {
        res_keys.insert(k.clone(), ());
    }
    for k in data.search_areas.keys() {
        res_keys.insert(k.clone(), ());
    }
    let mut coords = Mapping::new();
    for res in res_keys.keys() {
        let mut block = Mapping::new();
        let mut pts = Mapping::new();
        if let Some(m) = data.points.get(res) {
            for (k, pt) in m {
                pts.insert(Value::String(k.clone()), encode_point(pt));
            }
        }
        block.insert(Value::String("points".into()), Value::Mapping(pts));
        let mut sas = Mapping::new();
        if let Some(m) = data.search_areas.get(res) {
            for (k, sa) in m {
                sas.insert(Value::String(k.clone()), encode_search_area(sa));
            }
        }
        block.insert(Value::String("searchareas".into()), Value::Mapping(sas));
        coords.insert(Value::String(res.clone()), Value::Mapping(block));
    }
    map.insert(Value::String("coordinates".into()), Value::Mapping(coords));

    let mut masks = Mapping::new();
    for (k, mask) in &data.masks {
        masks.insert(Value::String(k.clone()), encode_mask(mask));
    }
    map.insert(Value::String("masks".into()), Value::Mapping(masks));

    let mut collections = Mapping::new();
    for (k, col) in &data.collections {
        collections.insert(Value::String(k.clone()), encode_collection(col));
    }
    map.insert(
        Value::String("collections".into()),
        Value::Mapping(collections),
    );

    // Preserve unknown keys from previous YAML (not typed catalog fields).
    for (k, v) in previous {
        let Some(key) = k.as_str() else {
            continue;
        };
        match key {
            "name" | "processpath" | "windowtitle" | "items" | "coordinates" | "masks"
            | "collections" => {}
            _ => {
                map.insert(k.clone(), v.clone());
            }
        }
    }

    Value::Mapping(map)
}

pub(super) fn encode_item(item: &ProgramItem) -> Value {
    let mut map = Mapping::new();
    map.insert(
        Value::String("name".into()),
        Value::String(item.name.clone()),
    );
    map.insert(
        Value::String("mask".into()),
        Value::String(item.mask.clone()),
    );
    map.insert(
        Value::String("stackmax".into()),
        Value::Number(item.stack_max.into()),
    );
    map.insert(
        Value::String("gridsize".into()),
        Value::Sequence(vec![
            Value::Number(item.grid_cols.into()),
            Value::Number(item.grid_rows.into()),
        ]),
    );
    let tags: Vec<Value> = item.tags.iter().map(|t| Value::String(t.clone())).collect();
    map.insert(Value::String("tags".into()), Value::Sequence(tags));
    Value::Mapping(map)
}

pub(super) fn encode_point(pt: &ProgramPoint) -> Value {
    let mut map = Mapping::new();
    map.insert(Value::String("name".into()), Value::String(pt.name.clone()));
    map.insert(Value::String("x".into()), pt.x.to_yaml());
    map.insert(Value::String("y".into()), pt.y.to_yaml());
    Value::Mapping(map)
}

pub(super) fn encode_search_area(sa: &ProgramSearchArea) -> Value {
    let mut map = Mapping::new();
    map.insert(Value::String("name".into()), Value::String(sa.name.clone()));
    map.insert(Value::String("leftx".into()), sa.left_x.to_yaml());
    map.insert(Value::String("topy".into()), sa.top_y.to_yaml());
    map.insert(Value::String("rightx".into()), sa.right_x.to_yaml());
    map.insert(Value::String("bottomy".into()), sa.bottom_y.to_yaml());
    Value::Mapping(map)
}

pub(super) fn encode_mask(mask: &ProgramMask) -> Value {
    let mut map = Mapping::new();
    map.insert(
        Value::String("name".into()),
        Value::String(mask.name.clone()),
    );
    map.insert(
        Value::String("shape".into()),
        Value::String(mask.shape.as_str().into()),
    );
    map.insert(
        Value::String("centerx".into()),
        Value::String(mask.center_x.clone()),
    );
    map.insert(
        Value::String("centery".into()),
        Value::String(mask.center_y.clone()),
    );
    map.insert(
        Value::String("base".into()),
        Value::String(mask.base.clone()),
    );
    map.insert(
        Value::String("height".into()),
        Value::String(mask.height.clone()),
    );
    map.insert(
        Value::String("radius".into()),
        Value::String(mask.radius.clone()),
    );
    map.insert(Value::String("inverse".into()), Value::Bool(mask.inverse));
    Value::Mapping(map)
}

pub(super) fn encode_collection(col: &ProgramCollection) -> Value {
    let mut map = Mapping::new();
    map.insert(
        Value::String("name".into()),
        Value::String(col.name.clone()),
    );
    map.insert(
        Value::String("searcharea".into()),
        Value::String(col.search_area.clone()),
    );
    map.insert(Value::String("rows".into()), Value::Number(col.rows.into()));
    map.insert(Value::String("cols".into()), Value::Number(col.cols.into()));
    Value::Mapping(map)
}
