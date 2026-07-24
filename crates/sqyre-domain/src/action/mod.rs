//! Action kinds and tree helpers.

mod action_serde;
mod wire_keys;

pub use wire_keys::WIRE_TYPE_KEYS;

use crate::{CoordinateRef, ScalarValue};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Declares a C-like string enum with `as_str`, `parse`, `Display`, `From`, and serde.
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
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $first,)+
                }
            }

            pub fn parse(s: &str) -> Self {
                match s.trim().to_ascii_lowercase().as_str() {
                    $($first $(| $rest)* => Self::$Variant,)+
                    _ => Self::default(),
                }
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
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Uuid::parse_str(&s)
            .map(ActionId)
            .unwrap_or_else(|_| ActionId::new()))
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

pub const OP_EQUALS: &str = "==";

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionClause {
    #[serde(default)]
    pub left: ScalarValue,
    #[serde(default = "default_equals_op")]
    pub operator: String,
    #[serde(default)]
    pub right: ScalarValue,
}

fn default_equals_op() -> String {
    OP_EQUALS.to_string()
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
            operator: OP_EQUALS.to_string(),
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

/// OpenCV-style template match method for [`ActionKind::ImageSearch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateMatchMethod {
    Sqdiff,
    SqdiffNormed,
    Ccorr,
    CcorrNormed,
    Ccoeff,
    #[default]
    CcoeffNormed,
}

impl TemplateMatchMethod {
    pub const ALL: [Self; 6] = [
        Self::Sqdiff,
        Self::SqdiffNormed,
        Self::Ccorr,
        Self::CcorrNormed,
        Self::Ccoeff,
        Self::CcoeffNormed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Sqdiff => "SQDIFF",
            Self::SqdiffNormed => "SQDIFF_NORMED",
            Self::Ccorr => "CCORR",
            Self::CcorrNormed => "CCORR_NORMED",
            Self::Ccoeff => "CCOEFF",
            Self::CcoeffNormed => "CCOEFF_NORMED",
        }
    }

    pub fn higher_is_better(self) -> bool {
        !matches!(self, Self::Sqdiff | Self::SqdiffNormed)
    }

    pub fn is_normed(self) -> bool {
        matches!(
            self,
            Self::SqdiffNormed | Self::CcorrNormed | Self::CcoeffNormed
        )
    }
}

pub(crate) fn is_default_match_method(v: &TemplateMatchMethod) -> bool {
    *v == TemplateMatchMethod::CcoeffNormed
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
    #[serde(rename = "ingraph", default, skip_serializing_if = "String::is_empty")]
    pub graph: String,
    #[serde(rename = "inrow", default, skip_serializing_if = "String::is_empty")]
    pub row: String,
    #[serde(rename = "incol", default, skip_serializing_if = "String::is_empty")]
    pub col: String,
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
        rename = "outputgraph",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub output_graph: String,
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
    #[serde(
        rename = "graphname",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub graph_name: String,
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
            graph_name: String::new(),
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

    /// Resolve a tree node id to either a real action or an Else folder.
    pub fn resolve_tree_id(&self, id: ActionId) -> Option<TreeNodeRef> {
        if self.id == id {
            return Some(TreeNodeRef::Action(id));
        }
        if self.find_by_id(id).is_some() {
            return Some(TreeNodeRef::Action(id));
        }
        let owner = ActionId::else_folder_owner(id);
        let owner_has_else = if owner == self.id {
            self.has_else_folder()
        } else {
            self.find_by_id(owner).is_some_and(Action::has_else_folder)
        };
        if owner_has_else && ActionId::else_folder(owner) == id {
            return Some(TreeNodeRef::ElseFolder { parent_id: owner });
        }
        None
    }

    pub fn find_by_id(&self, id: ActionId) -> Option<&Action> {
        if self.id == id {
            return Some(self);
        }
        for child in self.children() {
            if let Some(found) = child.find_by_id(id) {
                return Some(found);
            }
        }
        if let Some(else_kids) = self.else_children() {
            for child in else_kids {
                if let Some(found) = child.find_by_id(id) {
                    return Some(found);
                }
            }
        }
        None
    }

    pub fn find_by_id_mut(&mut self, id: ActionId) -> Option<&mut Action> {
        if self.id == id {
            return Some(self);
        }
        let path = self.find_child_path(id)?;
        Some(Self::follow_child_path_mut(self, &path))
    }

    /// Path of `(in_else_list, index)` steps from this node to a descendant.
    fn find_child_path(&self, id: ActionId) -> Option<Vec<(bool, usize)>> {
        for (i, child) in self.children().iter().enumerate() {
            if child.id == id {
                return Some(vec![(false, i)]);
            }
            if let Some(mut sub) = child.find_child_path(id) {
                sub.insert(0, (false, i));
                return Some(sub);
            }
        }
        if let Some(else_kids) = self.else_children() {
            for (i, child) in else_kids.iter().enumerate() {
                if child.id == id {
                    return Some(vec![(true, i)]);
                }
                if let Some(mut sub) = child.find_child_path(id) {
                    sub.insert(0, (true, i));
                    return Some(sub);
                }
            }
        }
        None
    }

    fn follow_child_path_mut<'a>(node: &'a mut Action, path: &[(bool, usize)]) -> &'a mut Action {
        let mut cur = node;
        for &(in_else, index) in path {
            let list = if in_else {
                cur.else_children_mut().expect("else path")
            } else {
                cur.children_mut().expect("then path")
            };
            cur = &mut list[index];
        }
        cur
    }

    /// Remove a descendant by id (not self). Returns the detached node.
    pub fn remove_by_id(&mut self, id: ActionId) -> Option<Action> {
        let path = self.find_child_path(id)?;
        Self::remove_at_path(self, &path)
    }

    fn remove_at_path(node: &mut Action, path: &[(bool, usize)]) -> Option<Action> {
        let [(in_else, index)] = path else {
            let (in_else, index) = path[0];
            let child = {
                let list = if in_else {
                    node.else_children_mut()?
                } else {
                    node.children_mut()?
                };
                &mut list[index]
            };
            return Self::remove_at_path(child, &path[1..]);
        };
        let list = if *in_else {
            node.else_children_mut()?
        } else {
            node.children_mut()?
        };
        Some(list.remove(*index))
    }

    /// True if `id` is this node or any descendant (then or else).
    pub fn contains_id(&self, id: ActionId) -> bool {
        self.find_by_id(id).is_some()
    }

    /// Parent id of `id` when it is a descendant of this node (not self).
    ///
    /// Else-branch children report the detection action as parent (not the Else folder sentinel).
    pub fn find_parent_id(&self, id: ActionId) -> Option<ActionId> {
        for child in self.children() {
            if child.id == id {
                return Some(self.id);
            }
            if let Some(p) = child.find_parent_id(id) {
                return Some(p);
            }
        }
        if let Some(else_kids) = self.else_children() {
            for child in else_kids {
                if child.id == id {
                    return Some(self.id);
                }
                if let Some(p) = child.find_parent_id(id) {
                    return Some(p);
                }
            }
        }
        None
    }

    fn child_list_mut_for_insert(
        &mut self,
        parent_id: ActionId,
    ) -> Result<&mut Vec<Action>, String> {
        match self.resolve_tree_id(parent_id) {
            Some(TreeNodeRef::ElseFolder { parent_id: owner }) => {
                let parent = if owner == self.id {
                    self
                } else {
                    self.find_by_id_mut(owner)
                        .ok_or_else(|| format!("parent action {owner} not found"))?
                };
                parent
                    .else_children_mut()
                    .ok_or_else(|| "else drop target has no else branch".to_string())
            }
            Some(TreeNodeRef::Action(aid)) => {
                let parent = if aid == self.id {
                    self
                } else {
                    self.find_by_id_mut(aid)
                        .ok_or_else(|| format!("parent action {aid} not found"))?
                };
                parent
                    .children_mut()
                    .ok_or_else(|| "drop target is not a branch".to_string())
            }
            None => Err(format!("parent action {parent_id} not found")),
        }
    }

    /// Insert `child` into the children of `parent_id` at `slot`.
    ///
    /// `parent_id` may be an Else folder sentinel ([`ActionId::else_folder`]).
    pub fn insert_at(
        &mut self,
        parent_id: ActionId,
        slot: InsertSlot,
        child: Action,
    ) -> Result<(), String> {
        let children = self.child_list_mut_for_insert(parent_id)?;
        match slot {
            InsertSlot::First => children.insert(0, child),
            InsertSlot::Last => children.push(child),
            InsertSlot::Before(sib) => {
                let i = children
                    .iter()
                    .position(|c| c.id == sib)
                    .ok_or_else(|| "before-sibling not found".to_string())?;
                children.insert(i, child);
            }
            InsertSlot::After(sib) => {
                let i = children
                    .iter()
                    .position(|c| c.id == sib)
                    .ok_or_else(|| "after-sibling not found".to_string())?;
                children.insert(i + 1, child);
            }
        }
        Ok(())
    }

    /// Move `source_id` under `parent_id` at `slot`. Rejects self-drops and
    /// dropping a node into its own descendant.
    pub fn move_action(
        &mut self,
        source_id: ActionId,
        parent_id: ActionId,
        slot: InsertSlot,
    ) -> Result<(), String> {
        self.move_actions(&[source_id], parent_id, slot)
    }

    /// Move several nodes under `parent_id` at `slot`, preserving `source_ids` order.
    ///
    /// Removes every source first, then inserts so sequential Before/After slots do not
    /// reverse relative order. Skips root / Else-folder sentinels and duplicate ids.
    pub fn move_actions(
        &mut self,
        source_ids: &[ActionId],
        parent_id: ActionId,
        slot: InsertSlot,
    ) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        let mut sources: Vec<ActionId> = Vec::new();
        for &id in source_ids {
            if id.is_root()
                || matches!(
                    self.resolve_tree_id(id),
                    Some(TreeNodeRef::ElseFolder { .. })
                )
                || !seen.insert(id)
            {
                continue;
            }
            sources.push(id);
        }
        if sources.is_empty() {
            return Ok(());
        }

        let parent_for_check = match self.resolve_tree_id(parent_id) {
            Some(TreeNodeRef::ElseFolder { parent_id }) => parent_id,
            _ => parent_id,
        };
        for &source_id in &sources {
            if source_id == parent_id {
                return Err("cannot drop onto self".into());
            }
            if let Some(src) = self.find_by_id(source_id) {
                if src.contains_id(parent_for_check) {
                    return Err("cannot drop into own descendant".into());
                }
            }
        }

        // Dropping Before/After a source that is itself moving is a no-op for a
        // single-node move; with multiple sources, resolve to a non-moving sibling.
        let slot = match slot {
            InsertSlot::Before(id) | InsertSlot::After(id) if sources.contains(&id) => {
                if sources.len() == 1 && sources[0] == id {
                    return Ok(());
                }
                resolve_slot_around_moving_sources(self, parent_id, slot, &sources)?
            }
            other => other,
        };

        let mut nodes = Vec::with_capacity(sources.len());
        for source_id in sources {
            let node = self
                .remove_by_id(source_id)
                .ok_or_else(|| format!("source action {source_id} not found"))?;
            nodes.push(node);
        }

        match slot {
            InsertSlot::First => {
                for node in nodes.into_iter().rev() {
                    self.insert_at(parent_id, InsertSlot::First, node)?;
                }
            }
            InsertSlot::Last => {
                for node in nodes {
                    self.insert_at(parent_id, InsertSlot::Last, node)?;
                }
            }
            InsertSlot::Before(sib) => {
                let mut anchor = InsertSlot::Before(sib);
                for node in nodes {
                    let id = node.id;
                    self.insert_at(parent_id, anchor, node)?;
                    anchor = InsertSlot::After(id);
                }
            }
            InsertSlot::After(sib) => {
                let mut anchor = InsertSlot::After(sib);
                for node in nodes {
                    let id = node.id;
                    self.insert_at(parent_id, anchor, node)?;
                    anchor = InsertSlot::After(id);
                }
            }
        }
        Ok(())
    }

    pub fn walk<F: FnMut(&Action)>(&self, f: &mut F) {
        f(self);
        for child in self.children() {
            child.walk(f);
        }
        if let Some(else_kids) = self.else_children() {
            for child in else_kids {
                child.walk(f);
            }
        }
    }

    pub fn walk_mut<F: FnMut(&mut Action)>(&mut self, f: &mut F) {
        f(self);
        if let Some(children) = self.children_mut() {
            for child in children.iter_mut() {
                child.walk_mut(f);
            }
        }
        if let Some(else_kids) = self.else_children_mut() {
            for child in else_kids.iter_mut() {
                child.walk_mut(f);
            }
        }
    }
}

/// When the drop marker is Before/After a node that is also moving, pick a
/// stable sibling (or First/Last) that will still exist after sources detach.
fn resolve_slot_around_moving_sources(
    root: &Action,
    parent_id: ActionId,
    slot: InsertSlot,
    sources: &[ActionId],
) -> Result<InsertSlot, String> {
    let children: Vec<ActionId> = {
        // Read-only walk of the insert list (same resolution as insert_at).
        let list = match root.resolve_tree_id(parent_id) {
            Some(TreeNodeRef::ElseFolder { parent_id: owner }) => {
                let parent = if owner == root.id {
                    root
                } else {
                    root.find_by_id(owner)
                        .ok_or_else(|| format!("parent action {owner} not found"))?
                };
                parent
                    .else_children()
                    .ok_or_else(|| "else drop target has no else branch".to_string())?
            }
            Some(TreeNodeRef::Action(aid)) => {
                let parent = if aid == root.id {
                    root
                } else {
                    root.find_by_id(aid)
                        .ok_or_else(|| format!("parent action {aid} not found"))?
                };
                if !parent.is_branch() {
                    return Err("drop target is not a branch".into());
                }
                parent.children()
            }
            None => return Err(format!("parent action {parent_id} not found")),
        };
        list.iter().map(|c| c.id).collect()
    };
    let source_set: std::collections::HashSet<_> = sources.iter().copied().collect();
    let (anchor_id, after) = match slot {
        InsertSlot::Before(id) => (id, false),
        InsertSlot::After(id) => (id, true),
        other => return Ok(other),
    };
    let Some(idx) = children.iter().position(|&id| id == anchor_id) else {
        return Err("drop sibling not found".into());
    };
    if after {
        // Find first non-moving sibling after the anchor block of movers.
        let mut i = idx + 1;
        while i < children.len() && source_set.contains(&children[i]) {
            i += 1;
        }
        if i < children.len() {
            // Insert before that survivor so movers land where the marker was.
            Ok(InsertSlot::Before(children[i]))
        } else {
            // Scan backward for a non-moving sibling to place After.
            let mut j = idx;
            while j > 0 && source_set.contains(&children[j - 1]) {
                j -= 1;
            }
            if j > 0 && !source_set.contains(&children[j - 1]) {
                Ok(InsertSlot::After(children[j - 1]))
            } else if !source_set.contains(&children[0]) && children[0] != anchor_id {
                Ok(InsertSlot::Before(children[0]))
            } else {
                Ok(InsertSlot::Last)
            }
        }
    } else {
        // Before(anchor): find last non-moving sibling before the mover block.
        let mut i = idx;
        while i > 0 && source_set.contains(&children[i - 1]) {
            i -= 1;
        }
        if i > 0 {
            Ok(InsertSlot::After(children[i - 1]))
        } else {
            let mut j = idx;
            while j < children.len() && source_set.contains(&children[j]) {
                j += 1;
            }
            if j < children.len() {
                Ok(InsertSlot::Before(children[j]))
            } else {
                Ok(InsertSlot::First)
            }
        }
    }
}

/// Tree selection / drop target: a real action, or an Else folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNodeRef {
    Action(ActionId),
    ElseFolder { parent_id: ActionId },
}

/// Insertion slot relative to a parent directory (mirrors egui_ltreeview DirPosition).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertSlot {
    First,
    Last,
    Before(ActionId),
    After(ActionId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    Loop {
        name: String,
        count: ScalarValue,
        subactions: Vec<Action>,
    },
    While {
        condition: ConditionBlock,
        max_iterations: i32,
        subactions: Vec<Action>,
    },
    Conditional {
        condition: ConditionBlock,
        subactions: Vec<Action>,
        else_actions: Vec<Action>,
    },
    ImageSearch {
        name: String,
        targets: Vec<String>,
        search_area: CoordinateRef,
        tolerance: f64,
        blur: i32,
        match_method: TemplateMatchMethod,
        detection: DetectionBranch,
    },
    Ocr {
        name: String,
        target: String,
        search_area: CoordinateRef,
        output_variable: String,
        blur: i32,
        min_threshold: i32,
        resize: f64,
        grayscale: bool,
        threshold_otsu: bool,
        threshold_invert: bool,
        detection: DetectionBranch,
    },
    FindPixel {
        name: String,
        search_area: CoordinateRef,
        target_color: String,
        color_tolerance: i32,
        detection: DetectionBranch,
    },
    ForEachRow {
        name: String,
        sources: Vec<ListColumn>,
        start_row: ScalarValue,
        end_row: ScalarValue,
        subactions: Vec<Action>,
    },
    Wait {
        time: ScalarValue,
    },
    Pause {
        message: String,
        continue_key: Vec<String>,
        pass_through: bool,
    },
    Move {
        point: CoordinateRef,
        smooth: bool,
        smooth_low: f64,
        smooth_high: f64,
        smooth_delay_ms: i32,
    },
    Click {
        button: MouseButton,
        state: PressState,
    },
    Key {
        key: String,
        state: PressState,
    },
    Type {
        text: String,
        delay_ms: i32,
    },
    SetVariable {
        assignments: Vec<VariableAssignment>,
    },
    SaveVariable {
        variable_name: String,
        destination: String,
        append: bool,
        append_newline: bool,
    },
    FocusWindow {
        process_path: String,
        window_title: String,
    },
    RunMacro {
        macro_name: String,
    },
    /// Interactive grid navigator. Built-in chords move / select / back; each
    /// [`NavigateKey`] child is a user-defined chord that runs its branch.
    NavigateSelect(Box<NavigateSelectData>),
    /// User-defined key branch under [`ActionKind::NavigateSelect`].
    NavigateKey {
        name: String,
        chord: Vec<String>,
        /// When true, leave the parent Navigate Select after children finish.
        exit: bool,
        subactions: Vec<Action>,
    },
    LoopJump {
        mode: LoopJumpMode,
    },
}

impl ActionKind {
    /// Default instance for a YAML/wire type key (`"wait"`, `"imagesearch"`, …).
    pub fn from_type_key(key: &str) -> Option<Self> {
        crate::blank::blank_kind(key)
    }

    // `type_key` is generated in `wire_keys` from the wire-key registry.

    pub fn is_branch(&self) -> bool {
        matches!(
            self,
            Self::Loop { .. }
                | Self::While { .. }
                | Self::Conditional { .. }
                | Self::ImageSearch { .. }
                | Self::Ocr { .. }
                | Self::FindPixel { .. }
                | Self::ForEachRow { .. }
                | Self::NavigateSelect(_)
                | Self::NavigateKey { .. }
        )
    }

    pub fn is_detection(&self) -> bool {
        matches!(
            self,
            Self::ImageSearch { .. } | Self::Ocr { .. } | Self::FindPixel { .. }
        )
    }

    pub fn has_else_folder(&self) -> bool {
        matches!(
            self,
            Self::Conditional { .. }
                | Self::ImageSearch { .. }
                | Self::Ocr { .. }
                | Self::FindPixel { .. }
        )
    }

    pub fn else_actions(&self) -> Option<&[Action]> {
        match self {
            Self::Conditional { else_actions, .. } => Some(else_actions),
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(&detection.else_actions),
            _ => None,
        }
    }

    pub fn else_actions_mut(&mut self) -> Option<&mut Vec<Action>> {
        match self {
            Self::Conditional { else_actions, .. } => Some(else_actions),
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(&mut detection.else_actions),
            _ => None,
        }
    }

    pub fn detection(&self) -> Option<&DetectionBranch> {
        match self {
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(detection),
            _ => None,
        }
    }

    pub fn detection_mut(&mut self) -> Option<&mut DetectionBranch> {
        match self {
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(detection),
            _ => None,
        }
    }

    pub fn children(&self) -> &[Action] {
        match self {
            Self::Loop { subactions, .. }
            | Self::While { subactions, .. }
            | Self::Conditional { subactions, .. }
            | Self::ForEachRow { subactions, .. }
            | Self::NavigateKey { subactions, .. } => subactions,
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => &detection.subactions,
            Self::NavigateSelect(data) => &data.subactions,
            _ => &[],
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<Action>> {
        match self {
            Self::Loop { subactions, .. }
            | Self::While { subactions, .. }
            | Self::Conditional { subactions, .. }
            | Self::ForEachRow { subactions, .. }
            | Self::NavigateKey { subactions, .. } => Some(subactions),
            Self::ImageSearch { detection, .. }
            | Self::Ocr { detection, .. }
            | Self::FindPixel { detection, .. } => Some(&mut detection.subactions),
            Self::NavigateSelect(data) => Some(&mut data.subactions),
            _ => None,
        }
    }

    pub fn display_name(&self) -> String {
        use crate::action_type_label;
        let label = action_type_label(self.type_key());
        match self {
            Self::Loop { name, .. }
            | Self::ImageSearch { name, .. }
            | Self::Ocr { name, .. }
            | Self::FindPixel { name, .. }
            | Self::ForEachRow { name, .. } => named_branch_label(label, name),
            Self::While { condition, .. } | Self::Conditional { condition, .. } => {
                named_branch_label(label, &condition.name)
            }
            Self::NavigateKey { name, chord, .. } => {
                let chord_s = if chord.is_empty() {
                    "…".to_string()
                } else {
                    chord.join("+")
                };
                if name.trim().is_empty() {
                    format!("{label} [{chord_s}]")
                } else {
                    format!("{label}: {name} [{chord_s}]")
                }
            }
            Self::Wait { time } => format!("Wait {}", time.as_display()),
            Self::Move { point, .. } => format!("Move {}", point.display_label()),
            Self::Click { button, state } => {
                format!("Click {button} {state}")
            }
            Self::Key { key, state } => {
                format!("Key {key} {state}")
            }
            Self::Type { text, .. } => format!("Type {text}"),
            Self::SetVariable { assignments } => {
                let names: Vec<&str> = assignments
                    .iter()
                    .map(|a| a.variable_name.as_str())
                    .filter(|n| !n.is_empty())
                    .collect();
                if names.is_empty() {
                    label.to_string()
                } else {
                    format!("Set {}", names.join(", "))
                }
            }
            Self::SaveVariable {
                variable_name,
                destination,
                ..
            } => format!("Save {variable_name} → {destination}"),
            Self::FocusWindow { window_title, .. } => {
                if window_title.trim().is_empty() {
                    "Focus window".into()
                } else {
                    format!("Focus {window_title}")
                }
            }
            Self::RunMacro { macro_name } => format!("Run {macro_name}"),
            Self::NavigateSelect(data) => {
                if data.program.is_empty() && data.graph_name.is_empty() {
                    label.to_string()
                } else {
                    format!("{label}: {} · {}", data.program, data.graph_name)
                }
            }
            Self::Pause { message, .. } => {
                if message.trim().is_empty() {
                    label.to_string()
                } else {
                    format!("Pause: {message}")
                }
            }
            Self::LoopJump { mode } => match mode {
                LoopJumpMode::Break => "Break".into(),
                LoopJumpMode::Continue => "Continue".into(),
            },
        }
    }
}

fn named_branch_label(label: &str, name: &str) -> String {
    if name.trim().is_empty() || name == "root" {
        label.to_string()
    } else {
        format!("{label}: {name}")
    }
}

/// Creates the empty macro root loop (`name: root`, nil UID, count 1).
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
    fn detection_else_insert_and_walk() {
        let detection_id = ActionId::new();
        let then_id = ActionId::new();
        let else_id = ActionId::new();
        let mut root = root_loop(vec![Action {
            id: detection_id,
            kind: ActionKind::FindPixel {
                name: String::new(),
                search_area: Default::default(),
                target_color: "#fff".into(),
                color_tolerance: 0,
                detection: DetectionBranch {
                    subactions: vec![wait(then_id)],
                    ..Default::default()
                },
            },
        }]);
        root.insert_at(
            ActionId::else_folder(detection_id),
            InsertSlot::Last,
            wait(else_id),
        )
        .unwrap();
        assert!(root.find_by_id(else_id).is_some());
        assert_eq!(
            root.find_parent_id(else_id),
            Some(detection_id),
            "else children report detection as parent"
        );
        let mut seen = Vec::new();
        root.walk(&mut |a| seen.push(a.id));
        assert!(seen.contains(&else_id));
        assert!(matches!(
            root.resolve_tree_id(ActionId::else_folder(detection_id)),
            Some(TreeNodeRef::ElseFolder { parent_id: id }) if id == detection_id
        ));
    }

    #[test]
    fn conditional_else_insert_and_run_path() {
        let cond_id = ActionId::new();
        let else_id = ActionId::new();
        let mut root = root_loop(vec![Action {
            id: cond_id,
            kind: ActionKind::Conditional {
                condition: ConditionBlock::default(),
                subactions: Vec::new(),
                else_actions: Vec::new(),
            },
        }]);
        root.insert_at(
            ActionId::else_folder(cond_id),
            InsertSlot::First,
            wait(else_id),
        )
        .unwrap();
        match &root.children()[0].kind {
            ActionKind::Conditional { else_actions, .. } => {
                assert_eq!(else_actions.len(), 1);
                assert_eq!(else_actions[0].id, else_id);
            }
            other => panic!("expected Conditional, got {other:?}"),
        }
    }

    fn wait(id: ActionId) -> Action {
        Action {
            id,
            kind: ActionKind::Wait {
                time: ScalarValue::Int(1),
            },
        }
    }

    #[test]
    fn move_action_reorders_siblings() {
        let a = ActionId::new();
        let b = ActionId::new();
        let c = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b), wait(c)]);
        root.move_action(c, ActionId::root(), InsertSlot::Before(a))
            .unwrap();
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![c, a, b]);
    }

    #[test]
    fn move_actions_preserves_order_after_sibling() {
        let a = ActionId::new();
        let b = ActionId::new();
        let c = ActionId::new();
        let d = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b), wait(c), wait(d)]);
        root.move_actions(&[b, c], ActionId::root(), InsertSlot::After(d))
            .unwrap();
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![a, d, b, c]);
    }

    #[test]
    fn move_actions_preserves_order_before_sibling() {
        let a = ActionId::new();
        let b = ActionId::new();
        let c = ActionId::new();
        let d = ActionId::new();
        let mut root = root_loop(vec![wait(a), wait(b), wait(c), wait(d)]);
        root.move_actions(&[c, d], ActionId::root(), InsertSlot::Before(a))
            .unwrap();
        let ids: Vec<_> = root.children().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![c, d, a, b]);
    }

    #[test]
    fn move_action_rejects_into_self_descendant() {
        let branch_id = ActionId::new();
        let child_id = ActionId::new();
        let mut root = root_loop(vec![Action {
            id: branch_id,
            kind: ActionKind::Loop {
                name: "inner".into(),
                count: ScalarValue::Int(1),
                subactions: vec![wait(child_id)],
            },
        }]);
        assert!(root
            .move_action(branch_id, branch_id, InsertSlot::Last)
            .is_err());
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
