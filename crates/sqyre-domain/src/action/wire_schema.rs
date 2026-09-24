//! Shared wire-field descriptors for macros and actions.
//!
//! Powers JSON Schema generation, YAML autocomplete, and drift tests so
//! autocomplete and validation cannot disagree about allowed keys/enums.

use super::{
    ConditionOperator, ItemSortBy, ItemSortThen, LoopJumpMode, MatchGrouping, MatchMethod,
    MatchMode, MouseButton, NavPressMode, NavSelectDevice, PressState, RepeatMode,
};
use crate::variables::VariableType;
use serde_json::{json, Map, Value as JsonValue};

/// How a YAML field is typed for schema / autocomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFieldKind {
    String,
    Bool,
    Number,
    /// Untagged scalar: null | bool | number | string.
    Scalar,
    /// Coordinate / entity string ref.
    CoordRef,
    StringList,
    Enum(&'static [&'static str]),
    /// Nested object described by `nested` on [`WireField`].
    Object,
    /// Array of nested objects.
    ObjectList,
    /// Recursive action list (`subactions` / `elseactions`).
    ActionList,
}

#[derive(Debug, Clone, Copy)]
pub struct WireField {
    pub key: &'static str,
    pub kind: WireFieldKind,
    pub required: bool,
    /// Nested object fields when `kind` is [`WireFieldKind::Object`] or [`ObjectList`].
    pub nested: &'static [WireField],
}

#[derive(Debug, Clone, Copy)]
pub struct ActionWireDesc {
    pub type_key: &'static str,
    pub fields: &'static [WireField],
}

const EMPTY: &[WireField] = &[];

const CLAUSE_FIELDS: &[WireField] = &[
    WireField {
        key: "left",
        kind: WireFieldKind::Scalar,
        required: false,
        nested: EMPTY,
    },
    WireField {
        key: "operator",
        kind: WireFieldKind::Enum(OPERATOR_VALUES),
        required: false,
        nested: EMPTY,
    },
    WireField {
        key: "right",
        kind: WireFieldKind::Scalar,
        required: false,
        nested: EMPTY,
    },
];

const LIST_COLUMN_FIELDS: &[WireField] = &[
    WireField {
        key: "source",
        kind: WireFieldKind::String,
        required: false,
        nested: EMPTY,
    },
    WireField {
        key: "outputvar",
        kind: WireFieldKind::String,
        required: false,
        nested: EMPTY,
    },
    WireField {
        key: "isfile",
        kind: WireFieldKind::Bool,
        required: false,
        nested: EMPTY,
    },
    WireField {
        key: "skipblanklines",
        kind: WireFieldKind::Bool,
        required: false,
        nested: EMPTY,
    },
];

const ASSIGNMENT_FIELDS: &[WireField] = &[
    WireField {
        key: "variablename",
        kind: WireFieldKind::String,
        required: false,
        nested: EMPTY,
    },
    WireField {
        key: "value",
        kind: WireFieldKind::Scalar,
        required: false,
        nested: EMPTY,
    },
];

const VARIABLE_DECL_FIELDS: &[WireField] = &[
    WireField {
        key: "name",
        kind: WireFieldKind::String,
        required: false,
        nested: EMPTY,
    },
    WireField {
        key: "type",
        kind: WireFieldKind::Enum(VARIABLE_TYPE_VALUES),
        required: false,
        nested: EMPTY,
    },
    WireField {
        key: "initialvalue",
        kind: WireFieldKind::String,
        required: false,
        nested: EMPTY,
    },
    WireField {
        key: "description",
        kind: WireFieldKind::String,
        required: false,
        nested: EMPTY,
    },
];

const MATCH_MODE_VALUES: &[&str] = &["all", "any"];
const REPEAT_MODE_VALUES: &[&str] = &[
    "once",
    "waituntilfound",
    "waitwhilefound",
    "repeatuntilfound",
    "repeatwhilefound",
];
const MOUSE_BUTTON_VALUES: &[&str] = &["left", "right", "middle", "scroll"];
const PRESS_STATE_VALUES: &[&str] = &["up", "down", "tap"];
const LOOP_JUMP_VALUES: &[&str] = &["break", "continue"];
const MATCH_METHOD_VALUES: &[&str] = &[
    "sqdiff",
    "sqdiff_normed",
    "ccorr",
    "ccorr_normed",
    "ccoeff",
    "ccoeff_normed",
];
const ITEM_SORT_BY_VALUES: &[&str] = &["name", "footprint", "tags", "manual"];
const ITEM_SORT_THEN_VALUES: &[&str] = &[
    "name_asc",
    "name_desc",
    "footprint_large",
    "footprint_small",
    "list_order",
];
const MATCH_GROUPING_VALUES: &[&str] = &["row", "column", "none"];
const NAV_PRESS_VALUES: &[&str] = &["click", "down", "up", "hold"];
const NAV_DEVICE_VALUES: &[&str] = &["mouse", "keyboard"];
const VARIABLE_TYPE_VALUES: &[&str] = &["auto", "text", "number"];
const HOTKEY_TRIGGER_VALUES: &[&str] = &["press", "release"];

const OPERATOR_VALUES: &[&str] = &[
    "==",
    "!=",
    "<",
    "<=",
    ">",
    ">=",
    "contains",
    "starts with",
    "ends with",
    "is set",
    "is empty",
];

macro_rules! f {
    ($key:literal, $kind:expr) => {
        WireField {
            key: $key,
            kind: $kind,
            required: false,
            nested: EMPTY,
        }
    };
    (req $key:literal, $kind:expr) => {
        WireField {
            key: $key,
            kind: $kind,
            required: true,
            nested: EMPTY,
        }
    };
    ($key:literal, $kind:expr, $nested:expr) => {
        WireField {
            key: $key,
            kind: $kind,
            required: false,
            nested: $nested,
        }
    };
}

const LOOP_FIELDS: &[WireField] = &[
    f!(req "name", WireFieldKind::String),
    f!("count", WireFieldKind::Scalar),
    f!("subactions", WireFieldKind::ActionList),
];

const WHILE_FIELDS: &[WireField] = &[
    f!("name", WireFieldKind::String),
    f!("match", WireFieldKind::Enum(MATCH_MODE_VALUES)),
    f!("clauses", WireFieldKind::ObjectList, CLAUSE_FIELDS),
    f!("maxiterations", WireFieldKind::Number),
    f!("subactions", WireFieldKind::ActionList),
];

const CONDITIONAL_FIELDS: &[WireField] = &[
    f!("name", WireFieldKind::String),
    f!("match", WireFieldKind::Enum(MATCH_MODE_VALUES)),
    f!("clauses", WireFieldKind::ObjectList, CLAUSE_FIELDS),
    f!("subactions", WireFieldKind::ActionList),
    f!("elseactions", WireFieldKind::ActionList),
];

const IMAGE_SEARCH_FIELDS: &[WireField] = &[
    f!(req "name", WireFieldKind::String),
    f!("targets", WireFieldKind::StringList),
    f!("targettags", WireFieldKind::StringList),
    f!("searcharea", WireFieldKind::CoordRef),
    f!("tolerance", WireFieldKind::Number),
    f!("blur", WireFieldKind::Number),
    f!("matchmethod", WireFieldKind::Enum(MATCH_METHOD_VALUES)),
    f!("sortby", WireFieldKind::Enum(ITEM_SORT_BY_VALUES)),
    f!("sortthen", WireFieldKind::Enum(ITEM_SORT_THEN_VALUES)),
    f!("tagpriority", WireFieldKind::StringList),
    f!("repeatmode", WireFieldKind::Enum(REPEAT_MODE_VALUES)),
    f!("waittilfoundseconds", WireFieldKind::Number),
    f!("waittilfoundintervalms", WireFieldKind::Number),
    f!("maxiterations", WireFieldKind::Number),
    f!("outputxvariable", WireFieldKind::String),
    f!("outputyvariable", WireFieldKind::String),
    f!("grouping", WireFieldKind::Enum(MATCH_GROUPING_VALUES)),
    f!("horizontal", WireFieldKind::String),
    f!("vertical", WireFieldKind::String),
    f!("subactions", WireFieldKind::ActionList),
    f!("elseactions", WireFieldKind::ActionList),
];

const OCR_FIELDS: &[WireField] = &[
    f!("name", WireFieldKind::String),
    f!("target", WireFieldKind::String),
    f!("searcharea", WireFieldKind::CoordRef),
    f!("outputvariable", WireFieldKind::String),
    f!("blur", WireFieldKind::Number),
    f!("minthreshold", WireFieldKind::Number),
    f!("resize", WireFieldKind::Number),
    f!("grayscale", WireFieldKind::Bool),
    f!("thresholdotsu", WireFieldKind::Bool),
    f!("thresholdinvert", WireFieldKind::Bool),
    f!("repeatmode", WireFieldKind::Enum(REPEAT_MODE_VALUES)),
    f!("waittilfoundseconds", WireFieldKind::Number),
    f!("waittilfoundintervalms", WireFieldKind::Number),
    f!("maxiterations", WireFieldKind::Number),
    f!("outputxvariable", WireFieldKind::String),
    f!("outputyvariable", WireFieldKind::String),
    f!("grouping", WireFieldKind::Enum(MATCH_GROUPING_VALUES)),
    f!("horizontal", WireFieldKind::String),
    f!("vertical", WireFieldKind::String),
    f!("subactions", WireFieldKind::ActionList),
    f!("elseactions", WireFieldKind::ActionList),
];

const FIND_PIXEL_FIELDS: &[WireField] = &[
    f!("name", WireFieldKind::String),
    f!("searcharea", WireFieldKind::CoordRef),
    f!("targetcolor", WireFieldKind::String),
    f!("colortolerance", WireFieldKind::Number),
    f!("repeatmode", WireFieldKind::Enum(REPEAT_MODE_VALUES)),
    f!("waittilfoundseconds", WireFieldKind::Number),
    f!("waittilfoundintervalms", WireFieldKind::Number),
    f!("maxiterations", WireFieldKind::Number),
    f!("outputxvariable", WireFieldKind::String),
    f!("outputyvariable", WireFieldKind::String),
    f!("grouping", WireFieldKind::Enum(MATCH_GROUPING_VALUES)),
    f!("horizontal", WireFieldKind::String),
    f!("vertical", WireFieldKind::String),
    f!("subactions", WireFieldKind::ActionList),
    f!("elseactions", WireFieldKind::ActionList),
];

const FOREACH_ROW_FIELDS: &[WireField] = &[
    f!("name", WireFieldKind::String),
    f!("sources", WireFieldKind::ObjectList, LIST_COLUMN_FIELDS),
    f!("startrow", WireFieldKind::Scalar),
    f!("endrow", WireFieldKind::Scalar),
    f!("subactions", WireFieldKind::ActionList),
];

const FOREACH_CELL_FIELDS: &[WireField] = &[
    f!("name", WireFieldKind::String),
    f!("cells", WireFieldKind::CoordRef),
    f!("subactions", WireFieldKind::ActionList),
];

const WAIT_FIELDS: &[WireField] = &[f!("time", WireFieldKind::Scalar)];

const PAUSE_FIELDS: &[WireField] = &[
    f!("message", WireFieldKind::String),
    f!("continuekey", WireFieldKind::StringList),
    f!("passthrough", WireFieldKind::Bool),
];

const MOVE_FIELDS: &[WireField] = &[
    f!("point", WireFieldKind::CoordRef),
    f!("smooth", WireFieldKind::Bool),
    f!("smoothlow", WireFieldKind::Number),
    f!("smoothhigh", WireFieldKind::Number),
    f!("smoothdelayms", WireFieldKind::Number),
];

const CLICK_FIELDS: &[WireField] = &[
    f!(req "button", WireFieldKind::Enum(MOUSE_BUTTON_VALUES)),
    f!("state", WireFieldKind::Enum(PRESS_STATE_VALUES)),
];

const KEY_FIELDS: &[WireField] = &[
    f!(req "key", WireFieldKind::String),
    f!("state", WireFieldKind::Enum(PRESS_STATE_VALUES)),
];

const TYPE_FIELDS: &[WireField] = &[
    f!("text", WireFieldKind::String),
    f!("delayms", WireFieldKind::Number),
];

const SET_VARIABLE_FIELDS: &[WireField] = &[f!(
    "assignments",
    WireFieldKind::ObjectList,
    ASSIGNMENT_FIELDS
)];

const SAVE_VARIABLE_FIELDS: &[WireField] = &[
    f!(req "variablename", WireFieldKind::String),
    f!("destination", WireFieldKind::String),
    f!("append", WireFieldKind::Bool),
    f!("appendnewline", WireFieldKind::Bool),
];

const FOCUS_WINDOW_FIELDS: &[WireField] = &[
    f!("processpath", WireFieldKind::String),
    f!("windowtitle", WireFieldKind::String),
];

const RUN_MACRO_FIELDS: &[WireField] = &[f!("macroname", WireFieldKind::String)];

const NAVIGATE_SELECT_FIELDS: &[WireField] = &[
    f!("program", WireFieldKind::String),
    f!("atlas", WireFieldKind::String),
    f!("chordup", WireFieldKind::StringList),
    f!("chorddown", WireFieldKind::StringList),
    f!("chordleft", WireFieldKind::StringList),
    f!("chordright", WireFieldKind::StringList),
    f!("chordselect", WireFieldKind::StringList),
    f!("chordback", WireFieldKind::StringList),
    f!("wrapedges", WireFieldKind::Bool),
    f!("movecursorwithnav", WireFieldKind::Bool),
    f!("smooth", WireFieldKind::Bool),
    f!("passthrough", WireFieldKind::Bool),
    f!("holdrepeat", WireFieldKind::Bool),
    f!("selectdevice", WireFieldKind::Enum(NAV_DEVICE_VALUES)),
    f!("selectbutton", WireFieldKind::Enum(MOUSE_BUTTON_VALUES)),
    f!("selectkey", WireFieldKind::String),
    f!("selectpressmode", WireFieldKind::Enum(NAV_PRESS_VALUES)),
    f!("inatlas", WireFieldKind::String),
    f!("inrow", WireFieldKind::String),
    f!("incol", WireFieldKind::String),
    f!("incollection", WireFieldKind::String),
    f!("outputref", WireFieldKind::String),
    f!("outputatlas", WireFieldKind::String),
    f!("outputrow", WireFieldKind::String),
    f!("outputcol", WireFieldKind::String),
    f!("outputcollection", WireFieldKind::String),
    f!("subactions", WireFieldKind::ActionList),
];

const NAVIGATE_KEY_FIELDS: &[WireField] = &[
    f!("name", WireFieldKind::String),
    f!("chord", WireFieldKind::StringList),
    f!("exit", WireFieldKind::Bool),
    f!("subactions", WireFieldKind::ActionList),
];

const LOOP_JUMP_FIELDS: &[WireField] = &[f!(req "mode", WireFieldKind::Enum(LOOP_JUMP_VALUES))];

const MACRO_FIELDS: &[WireField] = &[
    f!(req "name", WireFieldKind::String),
    f!(req "root", WireFieldKind::Object),
    f!("globaldelay", WireFieldKind::Number),
    f!("keyboarddelay", WireFieldKind::Number),
    f!("mousedelay", WireFieldKind::Number),
    f!("hotkey", WireFieldKind::StringList),
    f!("hotkey_trigger", WireFieldKind::Enum(HOTKEY_TRIGGER_VALUES)),
    f!("tags", WireFieldKind::StringList),
    f!("variables", WireFieldKind::ObjectList, VARIABLE_DECL_FIELDS),
];

/// Descriptors for every wire action type (same order as [`WIRE_TYPE_KEYS`]).
pub const ACTION_WIRE_DESCS: &[ActionWireDesc] = &[
    ActionWireDesc {
        type_key: "loop",
        fields: LOOP_FIELDS,
    },
    ActionWireDesc {
        type_key: "while",
        fields: WHILE_FIELDS,
    },
    ActionWireDesc {
        type_key: "conditional",
        fields: CONDITIONAL_FIELDS,
    },
    ActionWireDesc {
        type_key: "imagesearch",
        fields: IMAGE_SEARCH_FIELDS,
    },
    ActionWireDesc {
        type_key: "ocr",
        fields: OCR_FIELDS,
    },
    ActionWireDesc {
        type_key: "findpixel",
        fields: FIND_PIXEL_FIELDS,
    },
    ActionWireDesc {
        type_key: "foreachrow",
        fields: FOREACH_ROW_FIELDS,
    },
    ActionWireDesc {
        type_key: "foreachcell",
        fields: FOREACH_CELL_FIELDS,
    },
    ActionWireDesc {
        type_key: "wait",
        fields: WAIT_FIELDS,
    },
    ActionWireDesc {
        type_key: "pause",
        fields: PAUSE_FIELDS,
    },
    ActionWireDesc {
        type_key: "move",
        fields: MOVE_FIELDS,
    },
    ActionWireDesc {
        type_key: "click",
        fields: CLICK_FIELDS,
    },
    ActionWireDesc {
        type_key: "key",
        fields: KEY_FIELDS,
    },
    ActionWireDesc {
        type_key: "type",
        fields: TYPE_FIELDS,
    },
    ActionWireDesc {
        type_key: "setvariable",
        fields: SET_VARIABLE_FIELDS,
    },
    ActionWireDesc {
        type_key: "savevariable",
        fields: SAVE_VARIABLE_FIELDS,
    },
    ActionWireDesc {
        type_key: "focuswindow",
        fields: FOCUS_WINDOW_FIELDS,
    },
    ActionWireDesc {
        type_key: "runmacro",
        fields: RUN_MACRO_FIELDS,
    },
    ActionWireDesc {
        type_key: "navigatekey",
        fields: NAVIGATE_KEY_FIELDS,
    },
    ActionWireDesc {
        type_key: "loopjump",
        fields: LOOP_JUMP_FIELDS,
    },
    ActionWireDesc {
        type_key: "navigateselect",
        fields: NAVIGATE_SELECT_FIELDS,
    },
];

/// Look up an action descriptor by wire `type` key.
pub fn action_wire_desc(type_key: &str) -> Option<&'static ActionWireDesc> {
    let key = type_key.trim().to_ascii_lowercase();
    ACTION_WIRE_DESCS.iter().find(|d| d.type_key == key)
}

/// Top-level macro document fields.
pub fn macro_wire_fields() -> &'static [WireField] {
    MACRO_FIELDS
}

/// Enum allowlists keyed by YAML field name (for autocomplete).
pub fn enum_values_for_field(field: &str) -> Option<&'static [&'static str]> {
    match field {
        "match" => Some(MATCH_MODE_VALUES),
        "repeatmode" => Some(REPEAT_MODE_VALUES),
        "button" | "selectbutton" => Some(MOUSE_BUTTON_VALUES),
        "state" => Some(PRESS_STATE_VALUES),
        "mode" => Some(LOOP_JUMP_VALUES),
        "matchmethod" => Some(MATCH_METHOD_VALUES),
        "sortby" => Some(ITEM_SORT_BY_VALUES),
        "sortthen" => Some(ITEM_SORT_THEN_VALUES),
        "grouping" => Some(MATCH_GROUPING_VALUES),
        "selectpressmode" => Some(NAV_PRESS_VALUES),
        "selectdevice" => Some(NAV_DEVICE_VALUES),
        "operator" => Some(OPERATOR_VALUES),
        "hotkey_trigger" => Some(HOTKEY_TRIGGER_VALUES),
        "type" if false => None, // action type handled separately
        _ => None,
    }
}

/// VariableDecl `type` enum values.
pub fn variable_type_values() -> &'static [&'static str] {
    VARIABLE_TYPE_VALUES
}

fn scalar_schema() -> JsonValue {
    json!({
        "anyOf": [
            { "type": "null" },
            { "type": "boolean" },
            { "type": "number" },
            { "type": "integer" },
            { "type": "string" }
        ]
    })
}

fn field_schema(field: &WireField) -> JsonValue {
    match field.kind {
        WireFieldKind::String => json!({ "type": "string" }),
        WireFieldKind::CoordRef => json!({
            "anyOf": [
                { "type": "string" },
                { "type": "null" }
            ]
        }),
        WireFieldKind::Bool => json!({ "type": "boolean" }),
        WireFieldKind::Number => json!({ "type": ["number", "integer"] }),
        WireFieldKind::Scalar => scalar_schema(),
        WireFieldKind::StringList => json!({
            "type": "array",
            "items": { "type": "string" }
        }),
        WireFieldKind::Enum(vals) => json!({
            "type": "string",
            "enum": vals
        }),
        WireFieldKind::Object => object_schema(field.nested),
        WireFieldKind::ObjectList => json!({
            "type": "array",
            "items": object_schema(field.nested)
        }),
        WireFieldKind::ActionList => json!({
            "type": "array",
            "items": { "$ref": "#/$defs/action" }
        }),
    }
}

fn object_schema(fields: &[WireField]) -> JsonValue {
    let mut props = Map::new();
    let mut required = Vec::new();
    for f in fields {
        props.insert(f.key.to_string(), field_schema(f));
        if f.required {
            required.push(JsonValue::String(f.key.to_string()));
        }
    }
    let mut obj = Map::new();
    obj.insert("type".into(), json!("object"));
    obj.insert("properties".into(), JsonValue::Object(props));
    obj.insert("additionalProperties".into(), json!(false));
    if !required.is_empty() {
        obj.insert("required".into(), JsonValue::Array(required));
    }
    JsonValue::Object(obj)
}

fn action_variant_schema(desc: &ActionWireDesc) -> JsonValue {
    let mut props = Map::new();
    props.insert("type".into(), json!({ "const": desc.type_key }));
    let mut required = vec![JsonValue::String("type".into())];
    for f in desc.fields {
        props.insert(f.key.to_string(), field_schema(f));
        if f.required {
            required.push(JsonValue::String(f.key.to_string()));
        }
    }
    json!({
        "type": "object",
        "properties": props,
        "required": required,
        "additionalProperties": false
    })
}

/// Build a draft-2020-12 JSON Schema for a single-macro YAML document.
pub fn macro_document_json_schema() -> JsonValue {
    let action_one_of: Vec<JsonValue> = ACTION_WIRE_DESCS
        .iter()
        .map(action_variant_schema)
        .collect();

    let mut macro_props = Map::new();
    let mut macro_required = Vec::new();
    for f in MACRO_FIELDS {
        let schema = match f.key {
            "root" => json!({ "$ref": "#/$defs/action" }),
            "variables" => json!({
                "type": "array",
                "items": object_schema(VARIABLE_DECL_FIELDS)
            }),
            _ => field_schema(f),
        };
        macro_props.insert(f.key.to_string(), schema);
        if f.required {
            macro_required.push(JsonValue::String(f.key.to_string()));
        }
    }

    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://sqyre.app/schemas/macro-document.schema.json",
        "title": "Sqyre macro document",
        "type": "object",
        "properties": macro_props,
        "required": macro_required,
        "additionalProperties": false,
        "$defs": {
            "action": {
                "oneOf": action_one_of
            }
        }
    })
}

/// Canonical enum strings used by wire types (for drift tests).
pub fn assert_enum_tables_match_domain() {
    assert_eq!(
        MATCH_MODE_VALUES,
        MatchMode::ALL
            .iter()
            .map(|v| v.as_str())
            .collect::<Vec<_>>()
            .as_slice()
    );
    assert_eq!(
        REPEAT_MODE_VALUES.len(),
        RepeatMode::ALL.len(),
        "RepeatMode drift"
    );
    for v in RepeatMode::ALL {
        assert!(REPEAT_MODE_VALUES.contains(&v.as_str()));
    }
    for v in MouseButton::ALL {
        assert!(MOUSE_BUTTON_VALUES.contains(&v.as_str()) || v.as_str().is_empty());
    }
    assert_eq!(PRESS_STATE_VALUES, &["up", "down", "tap"]);
    assert_eq!(PressState::Up.as_str(), "up");
    assert_eq!(PressState::Down.as_str(), "down");
    assert_eq!(PressState::Tap.as_str(), "tap");
    for v in LoopJumpMode::ALL {
        assert!(LOOP_JUMP_VALUES.contains(&v.as_str()));
    }
    for v in MatchMethod::ALL {
        let wire = match v {
            MatchMethod::Sqdiff => "sqdiff",
            MatchMethod::SqdiffNormed => "sqdiff_normed",
            MatchMethod::Ccorr => "ccorr",
            MatchMethod::CcorrNormed => "ccorr_normed",
            MatchMethod::Ccoeff => "ccoeff",
            MatchMethod::CcoeffNormed => "ccoeff_normed",
        };
        assert!(MATCH_METHOD_VALUES.contains(&wire));
    }
    for v in ItemSortBy::ALL {
        assert!(ITEM_SORT_BY_VALUES.contains(&v.as_str()));
    }
    for v in ItemSortThen::ALL {
        assert!(ITEM_SORT_THEN_VALUES.contains(&v.as_str()));
    }
    for v in MatchGrouping::ALL {
        assert!(MATCH_GROUPING_VALUES.contains(&v.as_str()));
    }
    for v in NavPressMode::ALL {
        assert!(NAV_PRESS_VALUES.contains(&v.as_str()));
    }
    for v in NavSelectDevice::ALL {
        assert!(NAV_DEVICE_VALUES.contains(&v.as_str()) || v.as_str().is_empty());
    }
    for v in VariableType::ALL {
        assert!(VARIABLE_TYPE_VALUES.contains(&v.as_str()));
    }
    for v in ConditionOperator::ALL {
        assert!(OPERATOR_VALUES.contains(&v.as_str()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WIRE_TYPE_KEYS;

    #[test]
    fn every_wire_type_has_exactly_one_desc() {
        assert_eq!(ACTION_WIRE_DESCS.len(), WIRE_TYPE_KEYS.len());
        for key in WIRE_TYPE_KEYS {
            let matches: Vec<_> = ACTION_WIRE_DESCS
                .iter()
                .filter(|d| d.type_key == *key)
                .collect();
            assert_eq!(matches.len(), 1, "type {key}");
        }
    }

    #[test]
    fn schema_lists_all_action_types() {
        let schema = macro_document_json_schema();
        let one_of = schema["$defs"]["action"]["oneOf"]
            .as_array()
            .expect("oneOf");
        assert_eq!(one_of.len(), WIRE_TYPE_KEYS.len());
        for (i, key) in WIRE_TYPE_KEYS.iter().enumerate() {
            assert_eq!(one_of[i]["properties"]["type"]["const"], *key);
            assert_eq!(one_of[i]["additionalProperties"], false);
        }
        assert_eq!(schema["additionalProperties"], false);
        assert_enum_tables_match_domain();
    }

    #[test]
    fn action_desc_lookup() {
        assert!(action_wire_desc("imagesearch").is_some());
        assert!(action_wire_desc("not-a-type").is_none());
    }
}
