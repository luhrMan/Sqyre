//! Action kinds and tree helpers.

mod action_serde;
mod kind;
mod tree;
mod wire_keys;

pub use kind::ActionKind;
pub use tree::{InsertSlot, TreeNodeRef};
pub use wire_keys::WIRE_TYPE_KEYS;

use crate::{CoordinateRef, ScalarValue};
use serde::{Deserialize, Serialize};
pub use crate::match_method::MatchMethod;
use uuid::Uuid;

/// Declares a C-like string enum with `ALL`, `as_str`, `try_parse`, `parse`, `Display`, `From`, and serde.
///
/// The first literal for each variant is the canonical wire/UI string. Additional
/// `| "alias"` literals are accepted by `parse` only. Parsing is case-insensitive
/// after trim; unknown values map to [`Default`].
#[macro_export]
macro_rules! string_enum {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $Name:ident {
            $(
                $(#[$variant_meta:meta])*
                $Variant:ident = $first:literal $(| $rest:literal)*
            ),+ $(,)?
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        $vis enum $Name {
            $(
                $(#[$variant_meta])*
                $Variant,
            )+
        }

        impl $Name {
            pub const ALL: &'static [Self] = &[$(Self::$Variant,)+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $first,)+
                }
            }

            pub fn try_parse(s: &str) -> Option<Self> {
                match s.trim().to_ascii_lowercase().as_str() {
                    $($first $(| $rest)* => Some(Self::$Variant),)+
                    _ => None,
                }
            }

            pub fn parse(s: &str) -> Self {
                Self::try_parse(s).unwrap_or_default()
            }
        }

        impl std::fmt::Display for $Name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl From<&str> for $Name {
            fn from(s: &str) -> Self {
                Self::parse(s)
            }
        }

        impl From<String> for $Name {
            fn from(s: String) -> Self {
                Self::parse(&s)
            }
        }

        impl serde::Serialize for $Name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $Name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = <String as serde::Deserialize>::deserialize(deserializer)?;
                Ok(Self::parse(&s))
            }
        }
    };
}

/// Runtime action identity. Empty UUID string marks the macro root loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionId(pub Uuid);

impl ActionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn root() -> Self {
        Self(Uuid::nil())
    }

    pub fn is_root(self) -> bool {
        self.0.is_nil()
    }

    pub fn as_str(self) -> String {
        self.to_string()
    }

    /// Stable tree id for the Else directory under a branch that has an else list.
    ///
    /// Xor is reversible so [`Self::else_folder_owner`] can recover the parent.
    pub fn else_folder(parent: Self) -> Self {
        Self(Uuid::from_u128(parent.0.as_u128() ^ ELSE_FOLDER_ID_XOR))
    }

    /// Inverse of [`Self::else_folder`].
    pub fn else_folder_owner(else_id: Self) -> Self {
        Self(Uuid::from_u128(else_id.0.as_u128() ^ ELSE_FOLDER_ID_XOR))
    }
}

/// Marker xor so Else folder ids never collide with normal v4 action ids in practice.
const ELSE_FOLDER_ID_XOR: u128 = 0xE15E_A11C_E000_0000_0000_0000_0000_0001;

impl<'de> Deserialize<'de> for ActionId {
    /// Fails on a malformed `uid` rather than silently minting a fresh id —
    /// a corrupt id must surface as a decode error so callers (e.g.
    /// `Database::from_yaml_with_warnings`) can warn and skip the action's
    /// macro, instead of quietly renaming an action out from under the user.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Uuid::parse_str(&s)
            .map(ActionId)
            .map_err(|e| serde::de::Error::custom(format!("invalid action id {s:?}: {e}")))
    }
}

impl std::fmt::Display for ActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_root() {
            Ok(())
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl Default for ActionId {
    fn default() -> Self {
        Self::new()
    }
}

pub const DEFAULT_SMOOTH_LOW: f64 = 0.05;
pub const DEFAULT_SMOOTH_HIGH: f64 = 0.20;
pub const DEFAULT_SMOOTH_DELAY_MS: i32 = 1;

string_enum! {
    /// How condition clauses are combined.
    pub enum MatchMode {
        #[default]
        All = "all",
        Any = "any",
    }
}

string_enum! {
    /// Image-search / OCR / find-pixel wait and repeat modes.
    ///
    /// Wait modes poll silently, then run the branch once. Repeat modes run the
    /// branch each iteration until the stop condition.
    pub enum RepeatMode {
        #[default]
        Once = "once",
        WaitUntilFound = "waituntilfound",
        WaitWhileFound = "waitwhilefound",
        RepeatUntilFound = "repeatuntilfound",
        RepeatWhileFound = "repeatwhilefound",
    }
}

string_enum! {
    /// Mouse button for click / navigate-select.
    pub enum MouseButton {
        #[default]
        Left = "left",
        Right = "right",
        Middle = "middle" | "center",
        /// Scroll-wheel click / scroll action.
        Scroll = "scroll",
    }
}

string_enum! {
    /// Exit the enclosing loop, or skip to its next iteration.
    pub enum LoopJumpMode {
        #[default]
        Break = "break",
        Continue = "continue",
    }
}

/// Press / release phase for click and key actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PressState {
    Up,
    #[default]
    Down,
    Tap,
}

impl PressState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Tap => "tap",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "up" => Self::Up,
            "tap" => Self::Tap,
            _ => Self::Down,
        }
    }

    pub const fn is_down(self) -> bool {
        matches!(self, Self::Down)
    }
}

impl std::fmt::Display for PressState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PressState {
    fn from(s: &str) -> Self {
        Self::parse(s)
    }
}

impl serde::Serialize for PressState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for PressState {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = PressState;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a press state (`up`, `down`, `tap`) or legacy bool")
            }

            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(if v { PressState::Down } else { PressState::Up })
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(PressState::parse(v))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

string_enum! {
    /// Overlay / mask geometry.
    pub enum MaskShape {
        #[default]
        Rectangle = "rectangle",
        Circle = "circle",
    }
}

/// Comparison used by a [`ConditionClause`].
///
/// Strict wire decode: an unrecognized operator fails to deserialize rather than
/// collapsing into [`Self::Equals`], so a typo cannot silently change branch logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConditionOperator {
    #[default]
    Equals,
    NotEquals,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    IsSet,
    IsEmpty,
}

impl ConditionOperator {
    pub const ALL: &'static [Self] = &[
        Self::Equals,
        Self::NotEquals,
        Self::LessThan,
        Self::LessOrEqual,
        Self::GreaterThan,
        Self::GreaterOrEqual,
        Self::Contains,
        Self::StartsWith,
        Self::EndsWith,
        Self::IsSet,
        Self::IsEmpty,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equals => "==",
            Self::NotEquals => "!=",
            Self::LessThan => "<",
            Self::LessOrEqual => "<=",
            Self::GreaterThan => ">",
            Self::GreaterOrEqual => ">=",
            Self::Contains => "contains",
            Self::StartsWith => "starts with",
            Self::EndsWith => "ends with",
            Self::IsSet => "is set",
            Self::IsEmpty => "is empty",
        }
    }

    pub fn try_parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "==" => Some(Self::Equals),
            "!=" => Some(Self::NotEquals),
            "<" => Some(Self::LessThan),
            "<=" => Some(Self::LessOrEqual),
            ">" => Some(Self::GreaterThan),
            ">=" => Some(Self::GreaterOrEqual),
            "contains" => Some(Self::Contains),
            "starts with" => Some(Self::StartsWith),
            "ends with" => Some(Self::EndsWith),
            "is set" => Some(Self::IsSet),
            "is empty" => Some(Self::IsEmpty),
            _ => None,
        }
    }

    pub fn accepted_values() -> String {
        Self::ALL
            .iter()
            .map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// True when the operator only looks at the left operand (`is set`, `is empty`).
    pub const fn is_unary(self) -> bool {
        matches!(self, Self::IsSet | Self::IsEmpty)
    }

    /// True when the operator reads the *name* of the left operand rather than its value.
    pub const fn reads_variable_name(self) -> bool {
        matches!(self, Self::IsSet)
    }
}

impl std::fmt::Display for ConditionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ConditionOperator {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_parse(s).ok_or_else(|| {
            format!(
                "unknown ConditionOperator {:?}; expected one of: {}",
                s.trim(),
                Self::accepted_values()
            )
        })
    }
}

impl serde::Serialize for ConditionOperator {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ConditionOperator {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionClause {
    #[serde(default)]
    pub left: ScalarValue,
    #[serde(default)]
    pub operator: ConditionOperator,
    #[serde(default)]
    pub right: ScalarValue,
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn is_true(b: &bool) -> bool {
    *b
}

pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}

pub(crate) fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}

pub(crate) fn is_default_image_blur(v: &i32) -> bool {
    *v == 5
}

pub(crate) fn default_image_blur() -> i32 {
    5
}

pub(crate) fn default_ocr_blur() -> i32 {
    1
}

pub(crate) fn is_default_ocr_blur(v: &i32) -> bool {
    *v == 1
}

pub(crate) fn default_resize() -> f64 {
    1.0
}

pub(crate) fn is_default_resize(v: &f64) -> bool {
    (*v - 1.0).abs() < f64::EPSILON
}

pub(crate) fn default_ocr_text() -> String {
    "ocrText".into()
}

pub(crate) fn is_default_ocr_text(s: &str) -> bool {
    s.is_empty() || s == "ocrText"
}

pub(crate) fn default_target_color() -> String {
    "ffffff".into()
}

pub(crate) fn is_default_target_color(s: &str) -> bool {
    s.is_empty() || s == "ffffff"
}

fn default_found_x() -> String {
    "foundX".into()
}

fn default_found_y() -> String {
    "foundY".into()
}

fn is_default_found_x(s: &str) -> bool {
    s.is_empty() || s == "foundX"
}

fn is_default_found_y(s: &str) -> bool {
    s.is_empty() || s == "foundY"
}

pub(crate) fn default_loop_count() -> ScalarValue {
    ScalarValue::Int(1)
}

pub(crate) fn default_wait_time() -> ScalarValue {
    ScalarValue::Int(0)
}

impl Default for ConditionClause {
    fn default() -> Self {
        Self {
            left: ScalarValue::String(String::new()),
            operator: ConditionOperator::Equals,
            right: ScalarValue::String(String::new()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaitTilFoundConfig {
    #[serde(rename = "repeatmode", default)]
    pub repeat_mode: RepeatMode,
    #[serde(
        rename = "waittilfoundseconds",
        default,
        skip_serializing_if = "is_zero_i32"
    )]
    pub wait_til_found_seconds: i32,
    #[serde(
        rename = "waittilfoundintervalms",
        default,
        skip_serializing_if = "is_zero_i32"
    )]
    pub wait_til_found_interval_ms: i32,
    #[serde(rename = "maxiterations", default, skip_serializing_if = "is_zero_i32")]
    pub max_iterations: i32,
}

impl Default for WaitTilFoundConfig {
    fn default() -> Self {
        Self {
            repeat_mode: RepeatMode::Once,
            wait_til_found_seconds: 0,
            wait_til_found_interval_ms: 0,
            max_iterations: 0,
        }
    }
}

impl WaitTilFoundConfig {
    /// Silent poll until found (or timeout).
    pub fn wait_until_found_active(&self) -> bool {
        self.repeat_mode == RepeatMode::WaitUntilFound && self.wait_til_found_seconds > 0
    }

    /// Silent poll while found (or timeout).
    pub fn wait_while_found_active(&self) -> bool {
        self.repeat_mode == RepeatMode::WaitWhileFound && self.wait_til_found_seconds > 0
    }

    /// Run the branch each pass while the target remains found.
    pub fn is_repeat_while_found(&self) -> bool {
        self.repeat_mode == RepeatMode::RepeatWhileFound
    }

    /// Run the branch each miss (when configured) until the target is found.
    pub fn is_repeat_until_found(&self) -> bool {
        self.repeat_mode == RepeatMode::RepeatUntilFound
    }

    pub fn uses_timing(&self) -> bool {
        self.repeat_mode != RepeatMode::Once
    }

    pub fn uses_max_iterations(&self) -> bool {
        self.is_repeat_while_found() || self.is_repeat_until_found()
    }

    pub fn effective_interval_ms(&self, default_ms: i32) -> i32 {
        if self.wait_til_found_interval_ms > 0 {
            self.wait_til_found_interval_ms
        } else {
            default_ms
        }
    }

    /// Max iterations for repeat modes (default 100 when unset).
    pub fn effective_max_iterations(&self) -> i32 {
        if self.max_iterations > 0 {
            self.max_iterations
        } else {
            100
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CoordinateOutputs {
    #[serde(
        rename = "outputxvariable",
        default = "default_found_x",
        skip_serializing_if = "is_default_found_x"
    )]
    pub output_x_variable: String,
    #[serde(
        rename = "outputyvariable",
        default = "default_found_y",
        skip_serializing_if = "is_default_found_y"
    )]
    pub output_y_variable: String,
}

impl CoordinateOutputs {
    pub fn defaults() -> Self {
        Self {
            output_x_variable: "foundX".into(),
            output_y_variable: "foundY".into(),
        }
    }
}

pub(crate) fn is_default_match_method(v: &MatchMethod) -> bool {
    *v == MatchMethod::CcoeffNormed
}

/// Optional match-order fields present in newer `~/.sqyre` data.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MatchOrder {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub grouping: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub horizontal: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub vertical: String,
}

/// Shared wait / coords / branch fields for ImageSearch, OCR, and FindPixel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectionBranch {
    #[serde(flatten)]
    pub wait: WaitTilFoundConfig,
    #[serde(flatten)]
    pub coords: CoordinateOutputs,
    #[serde(flatten)]
    pub order: MatchOrder,
    /// Children run once per match (the "then" branch).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subactions: Vec<Action>,
    /// Children run when nothing matched (the "else" branch).
    #[serde(rename = "elseactions", default, skip_serializing_if = "Vec::is_empty")]
    pub else_actions: Vec<Action>,
}

impl Default for DetectionBranch {
    fn default() -> Self {
        Self {
            wait: WaitTilFoundConfig::default(),
            coords: CoordinateOutputs::defaults(),
            order: MatchOrder::default(),
            subactions: Vec::new(),
            else_actions: Vec::new(),
        }
    }
}

/// Shared name / match / clauses for While and Conditional.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ConditionBlock {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(rename = "match", default)]
    pub match_mode: MatchMode,
    #[serde(default = "default_clauses")]
    pub clauses: Vec<ConditionClause>,
}

fn default_clauses() -> Vec<ConditionClause> {
    vec![ConditionClause::default()]
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ListColumn {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(
        rename = "outputvar",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub output_var: String,
    #[serde(rename = "isfile", default, skip_serializing_if = "is_false")]
    pub is_file: bool,
    #[serde(rename = "skipblanklines", default, skip_serializing_if = "is_false")]
    pub skip_blank_lines: bool,
}

/// One name/value pair inside [`ActionKind::SetVariable`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VariableAssignment {
    #[serde(
        rename = "variablename",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub variable_name: String,
    #[serde(default)]
    pub value: ScalarValue,
}

impl VariableAssignment {
    pub fn new(name: impl Into<String>, value: ScalarValue) -> Self {
        Self {
            variable_name: name.into(),
            value,
        }
    }
}

fn default_assignments() -> Vec<VariableAssignment> {
    vec![VariableAssignment::default()]
}

/// Built-in navigation chords for [`ActionKind::NavigateSelect`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NavChords {
    #[serde(rename = "chordup", default, skip_serializing_if = "Vec::is_empty")]
    pub up: Vec<String>,
    #[serde(rename = "chorddown", default, skip_serializing_if = "Vec::is_empty")]
    pub down: Vec<String>,
    #[serde(rename = "chordleft", default, skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<String>,
    #[serde(rename = "chordright", default, skip_serializing_if = "Vec::is_empty")]
    pub right: Vec<String>,
    #[serde(rename = "chordselect", default, skip_serializing_if = "Vec::is_empty")]
    pub select: Vec<String>,
    #[serde(rename = "chordback", default, skip_serializing_if = "Vec::is_empty")]
    pub back: Vec<String>,
}

impl NavChords {
    pub fn blank_defaults() -> Self {
        Self {
            up: vec!["up".into()],
            down: vec!["down".into()],
            left: vec!["left".into()],
            right: vec!["right".into()],
            select: vec!["enter".into()],
            back: vec!["esc".into()],
        }
    }
}

/// Behavior flags for Navigate Select.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavOptions {
    #[serde(
        rename = "wrapedges",
        default = "default_true",
        skip_serializing_if = "is_true"
    )]
    pub wrap_edges: bool,
    #[serde(
        rename = "movecursorwithnav",
        default = "default_true",
        skip_serializing_if = "is_true"
    )]
    pub move_cursor_with_nav: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub smooth: bool,
    #[serde(rename = "passthrough", default, skip_serializing_if = "is_false")]
    pub pass_through: bool,
    #[serde(rename = "holdrepeat", default, skip_serializing_if = "is_false")]
    pub hold_repeat: bool,
}

impl Default for NavOptions {
    fn default() -> Self {
        Self {
            wrap_edges: true,
            move_cursor_with_nav: true,
            smooth: false,
            pass_through: false,
            hold_repeat: false,
        }
    }
}

/// Press performed when the Select chord fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavSelectAction {
    #[serde(
        rename = "selectdevice",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub device: String,
    #[serde(
        rename = "selectbutton",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub button: String,
    #[serde(
        rename = "selectkey",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub key: String,
    #[serde(
        rename = "selectpressmode",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub press_mode: String,
}

impl Default for NavSelectAction {
    fn default() -> Self {
        Self {
            device: "mouse".into(),
            button: "left".into(),
            key: String::new(),
            press_mode: "click".into(),
        }
    }
}

/// Optional start / override sources for Navigate Select.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NavInputs {
    #[serde(rename = "inatlas", default, skip_serializing_if = "String::is_empty")]
    pub atlas: String,
    #[serde(rename = "inrow", default, skip_serializing_if = "String::is_empty")]
    pub row: String,
    #[serde(rename = "incol", default, skip_serializing_if = "String::is_empty")]
    pub col: String,
    /// Starting Collection within the atlas (optional).
    #[serde(
        rename = "incollection",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub collection: String,
}

/// Output variables written by Navigate Select.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NavOutputs {
    #[serde(
        rename = "outputref",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub output_ref: String,
    #[serde(
        rename = "outputatlas",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub output_atlas: String,
    #[serde(
        rename = "outputrow",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub output_row: String,
    #[serde(
        rename = "outputcol",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub output_col: String,
    /// Current Collection within the atlas.
    #[serde(
        rename = "outputcollection",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub output_collection: String,
}

/// Boxed payload for [`ActionKind::NavigateSelect`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigateSelectData {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub program: String,
    /// Atlas name within the program.
    #[serde(rename = "atlas", default, skip_serializing_if = "String::is_empty")]
    pub atlas: String,
    #[serde(flatten)]
    pub chords: NavChords,
    #[serde(flatten)]
    pub options: NavOptions,
    #[serde(flatten)]
    pub select: NavSelectAction,
    #[serde(flatten)]
    pub inputs: NavInputs,
    #[serde(flatten)]
    pub outputs: NavOutputs,
    /// Direct children should be [`ActionKind::NavigateKey`] branches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subactions: Vec<Action>,
}

impl Default for NavigateSelectData {
    fn default() -> Self {
        Self {
            program: String::new(),
            atlas: String::new(),
            chords: NavChords::blank_defaults(),
            options: NavOptions::default(),
            select: NavSelectAction::default(),
            inputs: NavInputs::default(),
            outputs: NavOutputs::default(),
            subactions: Vec::new(),
        }
    }
}

/// Runtime builtins set inside ForEachRow sub-actions (1-based row index).
pub const FOREACH_ROW_BUILTIN_ROW: &str = "Row";
/// Total line count of the driving (first) ForEachRow source.
pub const FOREACH_ROW_BUILTIN_ROW_COUNT: &str = "RowCount";

/// One node in a macro action tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    /// Runtime identity. Written as `uid` only when present on decode; never
    /// serialized by default (see `action_to_map_with_uid` inject path).
    #[serde(default, rename = "uid", skip_serializing)]
    pub id: ActionId,
    #[serde(flatten)]
    pub kind: ActionKind,
}

impl Action {
    pub fn type_key(&self) -> &'static str {
        self.kind.type_key()
    }

    pub fn is_branch(&self) -> bool {
        self.kind.is_branch()
    }

    pub fn children(&self) -> &[Action] {
        self.kind.children()
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<Action>> {
        self.kind.children_mut()
    }

    /// Else-branch children for Conditional / detection actions (`None` otherwise).
    pub fn else_children(&self) -> Option<&[Action]> {
        self.kind.else_actions()
    }

    pub fn else_children_mut(&mut self) -> Option<&mut Vec<Action>> {
        self.kind.else_actions_mut()
    }

    pub fn is_detection(&self) -> bool {
        self.kind.is_detection()
    }

    /// True when this action paints an Else folder (Conditional or detection).
    pub fn has_else_folder(&self) -> bool {
        self.kind.has_else_folder()
    }

    pub fn display_name(&self) -> String {
        self.kind.display_name()
    }

    /// If this is a key/click [`PressState::Down`], a matching `Up` action (fresh id).
    ///
    /// Used when inserting a hold so the release is paired immediately below.
    pub fn matching_release(&self) -> Option<Action> {
        match &self.kind {
            ActionKind::Click { button, state } if state.is_down() => Some(Action {
                id: ActionId::new(),
                kind: ActionKind::Click {
                    button: *button,
                    state: PressState::Up,
                },
            }),
            ActionKind::Key { key, state } if state.is_down() => Some(Action {
                id: ActionId::new(),
                kind: ActionKind::Key {
                    key: key.clone(),
                    state: PressState::Up,
                },
            }),
            _ => None,
        }
    }

    /// True when `other` is the opposite press of the same key or mouse button
    /// ([`PressState::Down`] ↔ [`PressState::Up`]; Tap does not pair).
    pub fn is_press_pair_of(&self, other: &Action) -> bool {
        match (&self.kind, &other.kind) {
            (
                ActionKind::Key {
                    key: key_a,
                    state: state_a,
                },
                ActionKind::Key {
                    key: key_b,
                    state: state_b,
                },
            ) => key_a.eq_ignore_ascii_case(key_b) && press_states_pair(*state_a, *state_b),
            (
                ActionKind::Click {
                    button: button_a,
                    state: state_a,
                },
                ActionKind::Click {
                    button: button_b,
                    state: state_b,
                },
            ) => button_a == button_b && press_states_pair(*state_a, *state_b),
            _ => false,
        }
    }

}

pub fn root_loop(subactions: Vec<Action>) -> Action {
    Action {
        id: ActionId::root(),
        kind: ActionKind::Loop {
            name: "root".into(),
            count: ScalarValue::Int(1),
            subactions,
        },
    }
}

fn press_states_pair(a: PressState, b: PressState) -> bool {
    matches!(
        (a, b),
        (PressState::Down, PressState::Up) | (PressState::Up, PressState::Down)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_state_deserializes_bool_and_string() {
        assert_eq!(
            serde_yaml::from_str::<PressState>("true").unwrap(),
            PressState::Down
        );
        assert_eq!(
            serde_yaml::from_str::<PressState>("false").unwrap(),
            PressState::Up
        );
        assert_eq!(
            serde_yaml::from_str::<PressState>("tap").unwrap(),
            PressState::Tap
        );
        assert_eq!(
            serde_yaml::from_str::<PressState>("down").unwrap(),
            PressState::Down
        );
    }

    #[test]
    fn matching_release_pairs_down_key_and_click() {
        let key_down = Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: "shift".into(),
                state: PressState::Down,
            },
        };
        let key_up = key_down.matching_release().expect("key down has release");
        assert_ne!(key_up.id, key_down.id);
        assert_eq!(
            key_up.kind,
            ActionKind::Key {
                key: "shift".into(),
                state: PressState::Up,
            }
        );

        let click_down = Action {
            id: ActionId::new(),
            kind: ActionKind::Click {
                button: MouseButton::Right,
                state: PressState::Down,
            },
        };
        let click_up = click_down
            .matching_release()
            .expect("click down has release");
        assert_eq!(
            click_up.kind,
            ActionKind::Click {
                button: MouseButton::Right,
                state: PressState::Up,
            }
        );

        let tap = Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: "a".into(),
                state: PressState::Tap,
            },
        };
        assert!(tap.matching_release().is_none());
    }

    #[test]
    fn is_press_pair_of_matches_opposite_same_key_or_button() {
        let down = Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: "Ctrl".into(),
                state: PressState::Down,
            },
        };
        let up = Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: "ctrl".into(),
                state: PressState::Up,
            },
        };
        let other_key = Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: "alt".into(),
                state: PressState::Up,
            },
        };
        assert!(down.is_press_pair_of(&up));
        assert!(up.is_press_pair_of(&down));
        assert!(!down.is_press_pair_of(&other_key));
        assert!(!down.is_press_pair_of(&down));

        let click_down = Action {
            id: ActionId::new(),
            kind: ActionKind::Click {
                button: MouseButton::Left,
                state: PressState::Down,
            },
        };
        let click_up = Action {
            id: ActionId::new(),
            kind: ActionKind::Click {
                button: MouseButton::Left,
                state: PressState::Up,
            },
        };
        assert!(click_down.is_press_pair_of(&click_up));
        assert!(!click_down.is_press_pair_of(&down));
    }

    #[test]
    fn condition_operator_strict_rejects_unknown() {
        use std::str::FromStr;
        assert_eq!(
            ConditionOperator::try_parse("=="),
            Some(ConditionOperator::Equals)
        );
        assert_eq!(
            ConditionOperator::try_parse("CONTAINS"),
            Some(ConditionOperator::Contains)
        );
        assert!(ConditionOperator::from_str("nope").is_err());
    }

    #[test]
    fn condition_operator_deserialize_rejects_unknown() {
        let err = serde_yaml::from_str::<ConditionClause>("operator: bogus\nleft: a\nright: b")
            .unwrap_err();
        assert!(
            err.to_string().contains("unknown ConditionOperator"),
            "{err}"
        );
    }

    #[test]
    fn string_enums_parse_aliases_and_defaults() {
        assert_eq!(MouseButton::parse("center"), MouseButton::Middle);
        assert_eq!(MouseButton::parse("CENTER"), MouseButton::Middle);
        assert_eq!(MouseButton::parse("nope"), MouseButton::Left);
        assert_eq!(MatchMode::parse("any"), MatchMode::Any);
        assert_eq!(
            RepeatMode::parse("repeatwhilefound"),
            RepeatMode::RepeatWhileFound
        );
        assert_eq!(
            RepeatMode::parse("waitwhilefound"),
            RepeatMode::WaitWhileFound
        );
        assert_eq!(
            RepeatMode::parse("repeatuntilfound"),
            RepeatMode::RepeatUntilFound
        );
        assert_eq!(MaskShape::parse("circle"), MaskShape::Circle);
        assert_eq!(format!("{}", MouseButton::Scroll), "scroll");
    }
    #[test]
    fn action_id_deserialize_rejects_malformed_uuid_instead_of_reminting() {
        let err = serde_yaml::from_str::<ActionId>("\"not-a-uuid\"").unwrap_err();
        assert!(err.to_string().contains("invalid action id"), "{err}");
    }

    #[test]
    fn action_id_deserialize_accepts_valid_uuid() {
        let id = ActionId::new();
        let yaml = format!("\"{id}\"");
        let decoded: ActionId = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded, id);
    }

    #[test]
    fn action_with_malformed_uid_fails_to_decode() {
        let yaml = r#"
uid: "not-a-uuid"
type: wait
time: 1
"#;
        let err = serde_yaml::from_str::<Action>(yaml).unwrap_err();
        assert!(err.to_string().contains("invalid action id"), "{err}");
    }

    #[test]
    fn root_loop_is_named_root() {
        let root = root_loop(vec![]);
        match &root.kind {
            ActionKind::Loop { name, .. } => assert_eq!(name, "root"),
            _ => panic!("expected loop"),
        }
        assert!(root.id.is_root());
    }
}
