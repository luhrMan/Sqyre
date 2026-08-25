//! Validation helpers for macro entries and actions.

use sqyre_domain::{
    collect_known_variable_names, evaluate_expression, parse_hex_color, Action, ActionKind,
    ConditionBlock, ConditionClause, CoordinateRef, Macro, NavChords, ScalarValue, VariableStore,
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidateError {
    #[error("name cannot be empty")]
    EmptyName,
    #[error("invalid variable: {0}")]
    InvalidVariable(String),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, ValidateError>;

/// Characters illegal in Windows filenames. Linux forbids `/` and NUL (included here /
/// via the control-char check). Used for any catalog name that becomes a path component
/// under `images/` (programs, items, search areas, masks, collections, icon variants, …).
pub const FS_FORBIDDEN_FILENAME_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// True when `name` is safe to use as a single path component under a managed directory.
///
/// Rejects empty/whitespace names, `.` / `..`, Windows/Linux forbidden filename characters,
/// absolute forms, trailing `.`, reserved Windows device names, and control chars so catalog
/// keys cannot escape `images/` via join + `remove_dir_all` / rename, and cannot produce
/// empty or extensionless files (e.g. `:` Alternate Data Streams on Windows).
pub fn is_safe_fs_entity_name(name: &str) -> bool {
    validate_entity_name(name).is_ok()
}

pub fn validate_entity_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ValidateError::EmptyName);
    }
    if name == "." || name == ".." {
        return Err(ValidateError::Message(
            "name cannot be \".\" or \"..\"".into(),
        ));
    }
    if name.starts_with('/') || name.starts_with('\\') {
        return Err(ValidateError::Message(
            "name cannot be an absolute path".into(),
        ));
    }
    if let Some(c) = name
        .chars()
        .find(|c| *c == '\0' || c.is_control() || FS_FORBIDDEN_FILENAME_CHARS.contains(c))
    {
        return Err(ValidateError::Message(format!(
            "name cannot contain forbidden file character {c:?} (disallowed: < > : \" / \\ | ? * and controls)"
        )));
    }
    // Windows strips / rejects trailing periods in file names.
    if name.ends_with('.') {
        return Err(ValidateError::Message(
            "name cannot end with a period".into(),
        ));
    }
    if is_windows_reserved_device_name(name) {
        return Err(ValidateError::Message(format!(
            "name {name:?} is a reserved Windows device name"
        )));
    }
    Ok(())
}

/// `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9` (optionally with an extension).
fn is_windows_reserved_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name);
    matches!(
        stem.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

pub fn parse_positive_i32(s: &str) -> Result<i32> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ValidateError::Message("must be a positive integer".into()));
    }
    let v: i32 = s
        .parse()
        .map_err(|_| ValidateError::Message(format!("must be a positive integer: {s:?}")))?;
    if v <= 0 {
        return Err(ValidateError::Message(format!(
            "must be a positive integer: {s:?}"
        )));
    }
    Ok(v)
}

pub fn parse_non_negative_i32(s: &str) -> Result<i32> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ValidateError::Message(
            "must be a non-negative integer".into(),
        ));
    }
    let v: i32 = s
        .parse()
        .map_err(|_| ValidateError::Message(format!("must be a non-negative integer: {s:?}")))?;
    if v < 0 {
        return Err(ValidateError::Message(format!(
            "must be a non-negative integer: {s:?}"
        )));
    }
    Ok(v)
}

/// Item grid cols/rows > 0 and stack_max ≥ 0.
pub fn validate_item_grid_fields(cols: &str, rows: &str, stack_max: &str) -> Result<()> {
    parse_positive_i32(cols).map_err(|e| ValidateError::Message(format!("cols: {e}")))?;
    parse_positive_i32(rows).map_err(|e| ValidateError::Message(format!("rows: {e}")))?;
    parse_non_negative_i32(stack_max)
        .map_err(|e| ValidateError::Message(format!("stack max: {e}")))?;
    Ok(())
}

/// When all four coords are numeric literals, require positive width/height.
/// Variable refs (`${…}`) skip the bounds check.
pub fn validate_search_area_literal_bounds(
    left: &str,
    top: &str,
    right: &str,
    bottom: &str,
) -> Result<()> {
    let Some(lx) = parse_coord_literal(left) else {
        return Ok(());
    };
    let Some(ty) = parse_coord_literal(top) else {
        return Ok(());
    };
    let Some(rx) = parse_coord_literal(right) else {
        return Ok(());
    };
    let Some(by) = parse_coord_literal(bottom) else {
        return Ok(());
    };
    let (lx, rx) = if lx <= rx { (lx, rx) } else { (rx, lx) };
    let (ty, by) = if ty <= by { (ty, by) } else { (by, ty) };
    let w = rx - lx;
    let h = by - ty;
    if w <= 0 || h <= 0 {
        return Err(ValidateError::Message(format!(
            "invalid search area (width={w} height={h}); need positive dimensions"
        )));
    }
    if w > 1 << 16 || h > 1 << 16 {
        return Err(ValidateError::Message(format!(
            "search area dimensions too large ({w}x{h})"
        )));
    }
    Ok(())
}

fn parse_coord_literal(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() || sqyre_varref::contains(s) {
        return None;
    }
    if let Ok(i) = s.parse::<i32>() {
        return Some(i);
    }
    s.parse::<f64>().ok().map(|f| f as i32)
}

pub fn validate_variable_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ValidateError::EmptyName);
    }
    if name.contains(['$', '{', '}']) {
        return Err(ValidateError::InvalidVariable(
            "must not contain $, {, or }".into(),
        ));
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(ValidateError::InvalidVariable(
            "must not contain control characters".into(),
        ));
    }
    if sqyre_domain::is_reserved_runtime_variable_name(name) {
        return Err(ValidateError::InvalidVariable(format!(
            "{name:?} is a reserved runtime builtin name"
        )));
    }
    Ok(())
}

/// True when `name` looks like an expression rather than a plain identifier
/// (assignment-name check; arithmetic detection for names is broader — see
/// [`looks_like_arithmetic`]).
pub fn looks_like_expression(name: &str) -> bool {
    let t = name.trim();
    if t.is_empty() {
        return false;
    }
    t.contains(['+', '-', '*', '/', '(', ')', '%']) || sqyre_varref::contains(t)
}

pub fn validate_variable_assignment_name(name: &str) -> Result<()> {
    validate_variable_name(name)?;
    if looks_like_expression(name) {
        return Err(ValidateError::InvalidVariable(
            "must be a simple variable name, not an expression".into(),
        ));
    }
    Ok(())
}

/// Outcome of validating a variable entry value in the UI.
/// Warnings (unknown `${var}`) do not block submit; errors do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryValidation {
    pub warning: String,
    pub error: String,
}

impl EntryValidation {
    pub fn blocks_submit(&self) -> bool {
        !self.error.is_empty()
    }
}

/// Warning when text references undefined variables.
pub fn unknown_variable_warning(text: &str, macro_: Option<&Macro>) -> String {
    let Some(macro_) = macro_ else {
        return String::new();
    };
    if text.trim().is_empty() {
        return String::new();
    }
    let known = collect_known_variable_names(macro_);
    let mut unknown: Vec<String> = Vec::new();
    for name in sqyre_varref::names(text) {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if !sqyre_domain::is_known_variable(&known, name) {
            unknown.push(name.to_string());
        }
    }
    unknown.sort();
    unknown.dedup();
    match unknown.as_slice() {
        [] => String::new(),
        [one] => format!("unknown variable {one:?}"),
        many => format!("unknown variables: {}", many.join(", ")),
    }
}

/// Whether text will be evaluated as arithmetic at runtime (re-exported from domain).
pub use sqyre_domain::looks_like_arithmetic;

/// Parse/evaluate with placeholders for missing vars.
/// Does not mutate the caller's runtime store — works on a scratch clone.
/// When `macro_` is `None`, still validates literal/arithmetic structure on an empty scratch
/// (so `"1 + "` blocks even with no active macro).
fn validate_expression_structure(expr: &str, macro_: Option<&Macro>) -> Result<()> {
    if expr.trim().is_empty() {
        return Ok(());
    }

    let mut vars = match macro_ {
        Some(m) => scratch_variables(m),
        None => VariableStore::new(),
    };
    // Seed missing refs as 0 so structure (not unknown-var) is what we check.
    for name in sqyre_varref::names(expr) {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if vars.get(name).is_none() {
            vars.set(name, ScalarValue::Int(0));
        }
    }
    evaluate_expression(expr, &vars).map_err(|e| ValidateError::Message(e.to_string()))?;
    Ok(())
}

/// Runtime variables for `macro_`, falling back to declared initial values for any
/// decl not yet present in its live store (e.g. never-initialized macros).
fn scratch_variables(macro_: &Macro) -> VariableStore {
    let mut vars = VariableStore::new();
    for d in &macro_.variable_decls {
        let name = d.name.trim();
        if name.is_empty() || d.initial_value.trim().is_empty() {
            continue;
        }
        vars.set(name, d.initial_stored_value());
    }
    for (name, val) in macro_.variables.iter() {
        vars.set(name, val.clone());
    }
    vars
}

/// Live expression preview for Set-value editing.
///
/// Returns `Ok("")` for empty input; `Ok("= …")` when all refs have values;
/// `Ok("valid (result depends on runtime values)")` when refs are missing/unknown;
/// `Err` when the expression is structurally invalid.
pub fn preview_calculate(expr: &str, macro_: &Macro) -> std::result::Result<String, String> {
    if expr.trim().is_empty() {
        return Ok(String::new());
    }

    let mut vars = scratch_variables(macro_);

    let mut runtime_dependent = false;
    for name in sqyre_varref::names(expr) {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if vars.get(name).is_none() {
            vars.set(name, ScalarValue::Int(0));
            runtime_dependent = true;
        }
    }

    let res = evaluate_expression(expr, &vars).map_err(|e| e.to_string())?;
    if runtime_dependent || !unknown_variable_warning(expr, Some(macro_)).is_empty() {
        return Ok("valid (result depends on runtime values)".into());
    }
    Ok(format!("= {}", format_preview_number(res)))
}

fn format_preview_number(f: f64) -> String {
    // Format floats with default precision (no trailing zeros).
    if f.fract() == 0.0 && f.is_finite() && f.abs() <= i64::MAX as f64 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}

/// Set-variable value: plain text allowed; invalid arithmetic blocks.
pub fn validate_set_variable_value(text: &str, macro_: Option<&Macro>) -> EntryValidation {
    if text.trim().is_empty() {
        return EntryValidation::default();
    }
    let mut v = EntryValidation {
        warning: unknown_variable_warning(text, macro_),
        error: String::new(),
    };
    if looks_like_arithmetic(text) {
        if let Err(e) = validate_expression_structure(text, macro_) {
            v.error = e.to_string();
        }
    }
    v
}

/// Warning-only check for `${variable}` references.
pub fn validate_variable_references(text: &str, macro_: Option<&Macro>) -> EntryValidation {
    EntryValidation {
        warning: unknown_variable_warning(text, macro_),
        error: String::new(),
    }
}

/// Numeric field: empty, literal number, or valid arithmetic.
pub fn validate_numeric_expression(text: &str, macro_: Option<&Macro>) -> EntryValidation {
    if text.trim().is_empty() {
        return EntryValidation::default();
    }
    let mut v = EntryValidation {
        warning: unknown_variable_warning(text, macro_),
        error: String::new(),
    };
    if let Err(e) = validate_expression_structure(text, macro_) {
        v.error = e.to_string();
    }
    v
}

fn variable_binding_label(name: &str, role: sqyre_domain::BindingRole) -> String {
    role.validate_label(name)
}

fn yaml_string_value(v: &sqyre_domain::ScalarValue) -> Option<&str> {
    match v {
        sqyre_domain::ScalarValue::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn validate_continue_key(keys: &[String]) -> Result<()> {
    sqyre_domain::validate_continue_key(keys)
        .map(|_| ())
        .map_err(|e| ValidateError::Message(e.to_string()))
}

fn validate_coordinate_ref(label: &str, field: &str, coord: &CoordinateRef) -> Result<()> {
    if coord.is_empty() {
        return Err(ValidateError::Message(format!(
            "{label}: set a {field} before saving"
        )));
    }
    // Catalog refs are `program~entity` (or legacy bare names), resolved at runtime —
    // not math expressions. Do not run expression evaluation on them.
    Ok(())
}

fn require_search_area(label: &str, search_area: &CoordinateRef) -> Result<()> {
    if search_area.is_empty() {
        return Err(ValidateError::Message(format!(
            "{label}: set a search area"
        )));
    }
    Ok(())
}

fn validate_scalar_field(label: &str, value: &ScalarValue, macro_: Option<&Macro>) -> Result<()> {
    if let ScalarValue::String(s) = value {
        let v = validate_numeric_expression(s, macro_);
        if v.blocks_submit() {
            return Err(ValidateError::Message(format!("{label}: {}", v.error)));
        }
    }
    Ok(())
}

fn validate_scalar_expression_field(
    label: &str,
    value: &ScalarValue,
    macro_: Option<&Macro>,
) -> Result<()> {
    if let Some(text) = yaml_string_value(value) {
        if looks_like_arithmetic(text) {
            validate_expression_structure(text, macro_)
                .map_err(|e| ValidateError::Message(format!("{label}: {e}")))?;
        }
    }
    Ok(())
}

fn validate_condition_block(
    label: &str,
    block: &ConditionBlock,
    macro_: Option<&Macro>,
) -> Result<()> {
    if block.clauses.is_empty() {
        return Err(ValidateError::Message(format!(
            "{label}: add at least one condition clause"
        )));
    }
    for (i, clause) in block.clauses.iter().enumerate() {
        let clause_label = if block.clauses.len() == 1 {
            label.to_string()
        } else {
            format!("{label} clause {}", i + 1)
        };
        validate_condition_clause(&clause_label, clause, macro_)?;
    }
    Ok(())
}

fn validate_condition_clause(
    label: &str,
    clause: &ConditionClause,
    macro_: Option<&Macro>,
) -> Result<()> {
    let left = clause.left.as_display();
    if left.trim().is_empty() {
        return Err(ValidateError::Message(format!(
            "{label}: left operand cannot be empty"
        )));
    }
    if !clause.operator.is_unary() {
        let right = clause.right.as_display();
        if right.trim().is_empty() {
            return Err(ValidateError::Message(format!(
                "{label}: right operand required for `{}`",
                clause.operator
            )));
        }
    }
    validate_scalar_expression_field(&format!("{label} left"), &clause.left, macro_)?;
    if !clause.operator.is_unary() {
        validate_scalar_expression_field(&format!("{label} right"), &clause.right, macro_)?;
    }
    Ok(())
}

fn validate_nav_chords(label: &str, chords: &NavChords) -> Result<()> {
    validate_chord_keys(&format!("{label} up"), &chords.up)?;
    validate_chord_keys(&format!("{label} down"), &chords.down)?;
    validate_chord_keys(&format!("{label} left"), &chords.left)?;
    validate_chord_keys(&format!("{label} right"), &chords.right)?;
    validate_chord_keys(&format!("{label} select"), &chords.select)?;
    Ok(())
}

fn validate_chord_keys(label: &str, keys: &[String]) -> Result<()> {
    if keys.is_empty() || keys.iter().all(|k| k.trim().is_empty()) {
        return Err(ValidateError::Message(format!(
            "{label}: record a chord before saving"
        )));
    }
    Ok(())
}

fn validate_target_color(label: &str, target_color: &str) -> Result<()> {
    if target_color.trim().is_empty() {
        return Err(ValidateError::Message(format!(
            "{label}: set a target color"
        )));
    }
    if !sqyre_varref::contains(target_color) && parse_hex_color(target_color).is_none() {
        return Err(ValidateError::Message(format!(
            "{label}: invalid color {target_color:?} (use #RRGGBB or variable ref)"
        )));
    }
    Ok(())
}

/// How strictly to validate an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionValidationMode {
    /// Fields that must be sound to keep the action in the tree (edit Save,
    /// paste, undo). Incomplete-but-editable cases (e.g. image search with no
    /// target items or search area) are allowed; the tree surfaces those via
    /// [`validate_action`].
    Persist,
    /// Full readiness check for run gates and per-row tree diagnostics.
    Complete,
}

/// Checks minimum fields required to run an action (and for tree diagnostics).
///
/// Image search with no target items or search area fails here so the macro tree
/// can show the error; use [`validate_action_persist`] when applying an edit Save.
///
/// `macro_` enables Set expression structure checks; when
/// `None`, those structure checks are skipped (empty-expression / name rules
/// still apply).
pub fn validate_action(action: &Action, macro_: Option<&Macro>) -> Result<()> {
    validate_action_with(action, macro_, ActionValidationMode::Complete)
}

/// Like [`validate_action`], but allows incomplete image-search targets and an
/// unset search area so the user can save and fix them later (tree still flags
/// via [`validate_action`]).
pub fn validate_action_persist(action: &Action, macro_: Option<&Macro>) -> Result<()> {
    validate_action_with(action, macro_, ActionValidationMode::Persist)
}

fn validate_action_with(
    action: &Action,
    macro_: Option<&Macro>,
    mode: ActionValidationMode,
) -> Result<()> {
    for b in action.variable_bindings() {
        if b.name.trim().is_empty() {
            continue;
        }
        validate_variable_assignment_name(&b.name).map_err(|e| {
            ValidateError::Message(format!("{}: {e}", variable_binding_label(&b.name, b.role)))
        })?;
    }

    match &action.kind {
        ActionKind::Loop { count, .. } => {
            validate_scalar_field("loop count", count, macro_)?;
        }
        ActionKind::While { condition, .. } => {
            validate_condition_block("while", condition, macro_)?;
        }
        ActionKind::Conditional { condition, .. } => {
            validate_condition_block("conditional", condition, macro_)?;
        }
        ActionKind::ImageSearch {
            targets,
            search_area,
            detection,
            ..
        } => {
            validate_wait_config("image search", &detection.wait)?;
            if mode == ActionValidationMode::Complete {
                let mut issues = Vec::new();
                if search_area.is_empty() {
                    issues.push("set a search area");
                }
                if targets.is_empty() || targets.iter().all(|t| t.trim().is_empty()) {
                    issues.push("add at least one target item");
                }
                if !issues.is_empty() {
                    return Err(ValidateError::Message(format!(
                        "image search: {}",
                        issues.join("; ")
                    )));
                }
            }
        }
        ActionKind::Ocr {
            search_area,
            detection,
            blur,
            min_threshold,
            resize,
            ..
        } => {
            validate_wait_config("ocr", &detection.wait)?;
            if mode == ActionValidationMode::Complete {
                require_search_area("ocr", search_area)?;
            }
            if *blur < 0 {
                return Err(ValidateError::Message(
                    "ocr: blur cannot be negative".into(),
                ));
            }
            if !(*min_threshold >= 0 && *min_threshold <= 255) {
                return Err(ValidateError::Message(
                    "ocr: min threshold must be between 0 and 255".into(),
                ));
            }
            if *resize <= 0.0 {
                return Err(ValidateError::Message(
                    "ocr: resize must be positive".into(),
                ));
            }
        }
        ActionKind::FindPixel {
            search_area,
            target_color,
            detection,
            ..
        } => {
            validate_target_color("find pixel", target_color)?;
            validate_wait_config("find pixel", &detection.wait)?;
            if mode == ActionValidationMode::Complete {
                require_search_area("find pixel", search_area)?;
            }
        }
        ActionKind::ForEachRow {
            sources,
            start_row,
            end_row,
            ..
        } => {
            if sources.is_empty() || sources.iter().all(|s| s.source.trim().is_empty()) {
                return Err(ValidateError::Message(
                    "for each row: add at least one source column".into(),
                ));
            }
            validate_scalar_field("for each row start row", start_row, macro_)?;
            validate_scalar_field("for each row end row", end_row, macro_)?;
        }
        ActionKind::Wait { time } => {
            validate_scalar_field("wait time", time, macro_)?;
        }
        ActionKind::Pause { continue_key, .. } => {
            validate_continue_key(continue_key)?;
        }
        ActionKind::Move { point, .. } => {
            validate_coordinate_ref("move", "point", point)?;
        }
        ActionKind::Click { .. } => {}
        ActionKind::Key { key, .. } => {
            if key.trim().is_empty() {
                return Err(ValidateError::Message(
                    "key: record a key before saving".into(),
                ));
            }
        }
        ActionKind::Type { delay_ms, .. } => {
            if *delay_ms < 0 {
                return Err(ValidateError::Message(
                    "type: delay cannot be negative".into(),
                ));
            }
        }
        ActionKind::SetVariable { assignments } => {
            if assignments.is_empty() {
                return Err(ValidateError::Message(
                    "set variable: add at least one assignment".into(),
                ));
            }
            for a in assignments {
                validate_variable_assignment_name(&a.variable_name)
                    .map_err(|e| ValidateError::Message(format!("set variable: {e}")))?;
                if let Some(text) = yaml_string_value(&a.value) {
                    let v = validate_set_variable_value(text, macro_);
                    if v.blocks_submit() {
                        return Err(ValidateError::Message(format!("set variable: {}", v.error)));
                    }
                }
            }
        }
        ActionKind::SaveVariable {
            variable_name,
            destination,
            ..
        } => {
            if variable_name.trim().is_empty() {
                return Err(ValidateError::Message(
                    "save variable: choose a variable".into(),
                ));
            }
            validate_variable_assignment_name(variable_name)
                .map_err(|e| ValidateError::Message(format!("save variable: {e}")))?;
            if destination.trim().is_empty() {
                return Err(ValidateError::Message(
                    "save variable: set a destination path or clipboard".into(),
                ));
            }
        }
        ActionKind::FocusWindow {
            process_path,
            window_title,
        } => {
            if process_path.trim().is_empty() {
                return Err(ValidateError::Message(
                    "focus window: set an executable path".into(),
                ));
            }
            if window_title.trim().is_empty() {
                return Err(ValidateError::Message(
                    "focus window: set a window title".into(),
                ));
            }
        }
        ActionKind::RunMacro { macro_name } => {
            if macro_name.trim().is_empty() {
                return Err(ValidateError::Message(
                    "run macro: choose a macro name".into(),
                ));
            }
        }
        ActionKind::NavigateSelect(data) => {
            validate_nav_chords("navigate select", &data.chords)?;
        }
        ActionKind::NavigateKey { chord, .. } => {
            validate_chord_keys("navigate key", chord)?;
        }
        ActionKind::LoopJump { .. } => {}
    }
    Ok(())
}

fn validate_wait_config(label: &str, wait: &sqyre_domain::WaitTilFoundConfig) -> Result<()> {
    use sqyre_domain::RepeatMode;
    let needs_timeout = matches!(
        wait.repeat_mode,
        RepeatMode::WaitUntilFound | RepeatMode::WaitWhileFound
    );
    if needs_timeout && wait.timeout().is_none() {
        return Err(ValidateError::Message(format!(
            "{label}: wait modes require a positive timeout (seconds)"
        )));
    }
    if wait.wait_til_found_interval_ms < 0 {
        return Err(ValidateError::Message(format!(
            "{label}: wait interval cannot be negative"
        )));
    }
    Ok(())
}

/// Recursively validate `action` and every descendant via then/else children.
pub fn validate_action_tree(action: &Action, macro_: Option<&Macro>) -> Result<()> {
    validate_action_tree_with(action, macro_, ActionValidationMode::Complete)
}

/// Like [`validate_action_tree`] using [`validate_action_persist`] at each node.
pub fn validate_action_tree_persist(action: &Action, macro_: Option<&Macro>) -> Result<()> {
    validate_action_tree_with(action, macro_, ActionValidationMode::Persist)
}

fn validate_action_tree_with(
    action: &Action,
    macro_: Option<&Macro>,
    mode: ActionValidationMode,
) -> Result<()> {
    validate_action_with(action, macro_, mode)?;
    for child in action.children() {
        validate_action_tree_with(child, macro_, mode)?;
    }
    if let Some(else_kids) = action.else_children() {
        for child in else_kids {
            validate_action_tree_with(child, macro_, mode)?;
        }
    }
    Ok(())
}

/// Validate a macro's name and full action tree (for load / pre-run gates).
pub fn validate_macro(macro_: &Macro) -> Result<()> {
    validate_entity_name(&macro_.name)
        .map_err(|e| ValidateError::Message(format!("macro name {:?}: {e}", macro_.name)))?;
    validate_action_tree(&macro_.root, Some(macro_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{
        ActionId, CoordinateRef, PressState, ScalarValue, VariableAssignment, VariableDecl,
        VariableType,
    };

    #[test]
    fn entity_name_rejects_path_escape() {
        assert!(validate_entity_name("Demo").is_ok());
        assert!(validate_entity_name("Item Name").is_ok());
        assert!(validate_entity_name("Item-v2").is_ok());
        assert!(validate_entity_name("").is_err());
        assert!(validate_entity_name(".").is_err());
        assert!(validate_entity_name("..").is_err());
        assert!(validate_entity_name("../etc").is_err());
        assert!(validate_entity_name("a/b").is_err());
        assert!(validate_entity_name("a\\b").is_err());
        assert!(validate_entity_name("/abs").is_err());
        assert!(validate_entity_name("C:foo").is_err());
        assert!(validate_entity_name("has\0nul").is_err());
    }

    #[test]
    fn entity_name_rejects_windows_linux_forbidden_chars() {
        for c in FS_FORBIDDEN_FILENAME_CHARS {
            let name = format!("item{c}name");
            assert!(
                validate_entity_name(&name).is_err(),
                "expected reject for {c:?} in {name:?}"
            );
        }
        // Colon mid-name previously slipped past the drive-prefix-only check and
        // produced 0-byte / extensionless ScreenCap files via Windows ADS.
        assert!(validate_entity_name("Potion:Red").is_err());
        assert!(validate_entity_name("a<b").is_err());
        assert!(validate_entity_name("a>b").is_err());
        assert!(validate_entity_name("a\"b").is_err());
        assert!(validate_entity_name("a|b").is_err());
        assert!(validate_entity_name("a?b").is_err());
        assert!(validate_entity_name("a*b").is_err());
        assert!(validate_entity_name("ends.").is_err());
        assert!(validate_entity_name("CON").is_err());
        assert!(validate_entity_name("nul.txt").is_err());
        assert!(validate_entity_name("com1").is_err());
        assert!(validate_entity_name("LPT9").is_err());
        assert!(validate_entity_name("console").is_ok());
        assert!(validate_entity_name("Item.v2").is_ok());
    }

    #[test]
    fn variable_name_rejects_braces() {
        assert!(validate_variable_name("${x}").is_err());
        assert!(validate_variable_name("ok").is_ok());
    }

    #[test]
    fn variable_name_rejects_reserved_builtins() {
        assert!(validate_variable_name("StackMax").is_err());
        assert!(validate_variable_name("Row").is_err());
        assert!(validate_variable_name("monitor1Width").is_err());
        assert!(validate_variable_name("Monitor12Height").is_err());
        assert!(validate_variable_name("myStackMax").is_ok());
    }

    #[test]
    fn assignment_rejects_expression() {
        assert!(validate_variable_assignment_name("a+1").is_err());
    }

    #[test]
    fn item_grid_and_search_area_bounds() {
        assert!(validate_item_grid_fields("2", "3", "0").is_ok());
        assert!(validate_item_grid_fields("0", "3", "0").is_err());
        assert!(validate_item_grid_fields("2", "3", "-1").is_err());
        assert!(validate_search_area_literal_bounds("0", "0", "10", "10").is_ok());
        assert!(validate_search_area_literal_bounds("10", "10", "10", "10").is_err());
        assert!(validate_search_area_literal_bounds("${a}", "0", "10", "10").is_ok());
    }

    fn pause(keys: &[&str]) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Pause {
                message: String::new(),
                continue_key: keys.iter().map(|s| (*s).to_string()).collect(),
                pass_through: false,
            },
        }
    }

    fn key(k: &str) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: k.into(),
                state: PressState::Down,
            },
        }
    }

    fn set_var(name: &str, value: sqyre_domain::ScalarValue) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                assignments: vec![VariableAssignment::new(name, value)],
            },
        }
    }

    #[test]
    fn validate_action_pause_requires_continue_key() {
        assert!(validate_action(&pause(&[]), None).is_err());
    }

    #[test]
    fn validate_action_tree_walks_children() {
        let root = Action {
            id: ActionId::new(),
            kind: ActionKind::Loop {
                name: "root".into(),
                count: ScalarValue::Int(1),
                subactions: vec![key("a"), key("")],
            },
        };
        assert!(validate_action(&root, None).is_ok());
        let err = validate_action_tree(&root, None).unwrap_err();
        assert!(err.to_string().contains("key:"), "{err}");
    }

    #[test]
    fn validate_action_key_requires_key() {
        assert!(validate_action(&key(""), None).is_err());
    }

    #[test]
    fn validate_action_set_allows_empty_value() {
        assert!(validate_action(
            &set_var("out", sqyre_domain::ScalarValue::String(String::new())),
            None
        )
        .is_ok());
    }

    #[test]
    fn validate_action_set_variable_requires_name() {
        assert!(validate_action(
            &set_var("", sqyre_domain::ScalarValue::String("1".into())),
            None
        )
        .is_err());
    }

    #[test]
    fn validate_action_set_valid_expression() {
        let mut m = Macro::new("test", 0, vec![]);
        m.init_runtime_variables();
        assert!(validate_action(
            &set_var("sum", sqyre_domain::ScalarValue::String("1 + 2".into())),
            Some(&m)
        )
        .is_ok());
    }

    #[test]
    fn validate_action_set_rejects_malformed_expression() {
        let mut m = Macro::new("test", 0, vec![]);
        m.init_runtime_variables();
        let err = validate_action(
            &set_var("sum", sqyre_domain::ScalarValue::String("1 + ".into())),
            Some(&m),
        )
        .unwrap_err();
        assert!(err.to_string().contains("set variable:"), "{err}");
    }

    #[test]
    fn validate_set_variable_value_examples() {
        let mut m = Macro::new("t", 0, vec![]);
        m.variable_decls.push(VariableDecl {
            name: "x".into(),
            type_: VariableType::Number,
            initial_value: "5".into(),
            description: String::new(),
        });
        m.init_runtime_variables();

        assert!(!validate_set_variable_value("hello", Some(&m)).blocks_submit());
        assert!(!validate_set_variable_value("1+${x}", Some(&m)).blocks_submit());
        let missing = validate_set_variable_value("${missing}", Some(&m));
        assert!(!missing.blocks_submit());
        assert!(!missing.warning.is_empty());
        assert!(validate_set_variable_value("1 + ", Some(&m)).blocks_submit());
    }

    #[test]
    fn preview_calculate_examples() {
        use sqyre_domain::{root_loop, Action, ActionId, ActionKind, VariableAssignment};

        let mut m = Macro::new("t", 0, vec![]);
        m.variable_decls.push(VariableDecl {
            name: "count".into(),
            type_: VariableType::Number,
            initial_value: "5".into(),
            description: String::new(),
        });
        m.variable_decls.push(VariableDecl {
            name: "label".into(),
            type_: VariableType::Text,
            initial_value: String::new(),
            description: String::new(),
        });
        m.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                assignments: vec![VariableAssignment::new(
                    "result",
                    sqyre_domain::ScalarValue::String("0".into()),
                )],
            },
        }]);
        m.init_runtime_variables();

        assert_eq!(preview_calculate("", &m).unwrap(), "");
        assert_eq!(preview_calculate("2 + 3", &m).unwrap(), "= 5");
        assert_eq!(preview_calculate("${count} * 2", &m).unwrap(), "= 10");
        assert_eq!(
            preview_calculate("${label} + 1", &m).unwrap(),
            "valid (result depends on runtime values)"
        );
        assert_eq!(
            preview_calculate("${result} + 1", &m).unwrap(),
            "valid (result depends on runtime values)"
        );
        assert_eq!(
            preview_calculate("${missing} + 1", &m).unwrap(),
            "valid (result depends on runtime values)"
        );
    }

    #[test]
    fn validate_action_rejects_bad_variable_name() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::SetVariable {
                assignments: vec![VariableAssignment::new(
                    "a+b",
                    sqyre_domain::ScalarValue::String("1+1".into()),
                )],
            },
        };
        let err = validate_action(&a, None).unwrap_err();
        assert!(err.to_string().contains("variable \"a+b\""), "{err}");
    }

    #[test]
    fn validate_numeric_without_macro_still_checks_structure() {
        assert!(!validate_numeric_expression("100", None).blocks_submit());
        assert!(!validate_numeric_expression("1+2", None).blocks_submit());
        assert!(validate_numeric_expression("1 + ", None).blocks_submit());
        assert!(!validate_numeric_expression("${x}", None).blocks_submit());
    }

    #[test]
    fn validate_variable_references_warns_only() {
        let mut m = Macro::new("t", 0, vec![]);
        m.variable_decls.push(VariableDecl {
            name: "x".into(),
            type_: VariableType::Number,
            initial_value: "5".into(),
            description: String::new(),
        });
        m.init_runtime_variables();
        let ok = validate_variable_references("${x}", Some(&m));
        assert!(!ok.blocks_submit());
        assert!(ok.warning.is_empty());
        let warn = validate_variable_references("${missing}", Some(&m));
        assert!(!warn.blocks_submit());
        assert!(!warn.warning.is_empty());
    }

    #[test]
    fn validate_image_search_requires_target_and_wait_timeout() {
        use sqyre_domain::{RepeatMode, WaitTilFoundConfig};
        let empty = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: String::new(),
                targets: vec![],
                search_area: Default::default(),
                tolerance: 0.95,
                blur: 5,
                match_method: Default::default(),
                detection: Default::default(),
            },
        };
        assert!(validate_action(&empty, None)
            .unwrap_err()
            .to_string()
            .contains("target"));
        // Persist allows saving without items; tree/run still flag via Complete.
        assert!(validate_action_persist(&empty, None).is_ok());

        let bad_wait = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: String::new(),
                targets: vec!["Game~Item".into()],
                search_area: Default::default(),
                tolerance: 0.95,
                blur: 5,
                match_method: Default::default(),
                detection: sqyre_domain::DetectionBranch {
                    wait: WaitTilFoundConfig {
                        repeat_mode: RepeatMode::WaitUntilFound,
                        wait_til_found_seconds: 0.0,
                        wait_til_found_interval_ms: 0,
                        max_iterations: 0,
                    },
                    ..Default::default()
                },
            },
        };
        assert!(validate_action(&bad_wait, None)
            .unwrap_err()
            .to_string()
            .contains("timeout"));
        assert!(validate_action_persist(&bad_wait, None)
            .unwrap_err()
            .to_string()
            .contains("timeout"));

        // Empty targets must not mask a bad wait on persist.
        let empty_bad_wait = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: String::new(),
                targets: vec![],
                search_area: Default::default(),
                tolerance: 0.95,
                blur: 5,
                match_method: Default::default(),
                detection: sqyre_domain::DetectionBranch {
                    wait: WaitTilFoundConfig {
                        repeat_mode: RepeatMode::WaitUntilFound,
                        wait_til_found_seconds: 0.0,
                        wait_til_found_interval_ms: 0,
                        max_iterations: 0,
                    },
                    ..Default::default()
                },
            },
        };
        assert!(validate_action_persist(&empty_bad_wait, None)
            .unwrap_err()
            .to_string()
            .contains("timeout"));
    }

    #[test]
    fn validate_find_pixel_requires_color() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::FindPixel {
                name: String::new(),
                search_area: Default::default(),
                target_color: String::new(),
                color_tolerance: 0,
                detection: Default::default(),
            },
        };
        assert!(validate_action(&a, None)
            .unwrap_err()
            .to_string()
            .contains("color"));
    }

    #[test]
    fn validate_navigate_key_requires_chord() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::NavigateKey {
                name: String::new(),
                chord: vec![],
                exit: false,
                subactions: vec![],
            },
        };
        assert!(validate_action(&a, None)
            .unwrap_err()
            .to_string()
            .contains("chord"));
    }

    #[test]
    fn validate_move_requires_point() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Move {
                point: CoordinateRef(String::new()),
                smooth: false,
                smooth_low: 0.05,
                smooth_high: 0.2,
                smooth_delay_ms: 1,
            },
        };
        assert!(validate_action(&a, None)
            .unwrap_err()
            .to_string()
            .contains("point"));
    }

    #[test]
    fn validate_move_accepts_catalog_point_ref() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Move {
                point: CoordinateRef("General~Windows".into()),
                smooth: false,
                smooth_low: 0.05,
                smooth_high: 0.2,
                smooth_delay_ms: 1,
            },
        };
        assert!(validate_action(&a, None).is_ok());
    }

    #[test]
    fn validate_detection_requires_search_area_on_complete() {
        let image = Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: String::new(),
                targets: vec!["Game~Item".into()],
                search_area: Default::default(),
                tolerance: 0.95,
                blur: 5,
                match_method: Default::default(),
                detection: Default::default(),
            },
        };
        assert!(validate_action_persist(&image, None).is_ok());
        assert!(validate_action(&image, None)
            .unwrap_err()
            .to_string()
            .contains("search area"));

        let ocr = Action {
            id: ActionId::new(),
            kind: ActionKind::Ocr {
                name: String::new(),
                target: "ok".into(),
                search_area: Default::default(),
                output_variable: String::new(),
                blur: 0,
                min_threshold: 0,
                resize: 1.0,
                grayscale: false,
                threshold_otsu: false,
                threshold_invert: false,
                detection: Default::default(),
            },
        };
        assert!(validate_action_persist(&ocr, None).is_ok());
        assert!(validate_action(&ocr, None)
            .unwrap_err()
            .to_string()
            .contains("search area"));

        let pixel = Action {
            id: ActionId::new(),
            kind: ActionKind::FindPixel {
                name: String::new(),
                search_area: Default::default(),
                target_color: "ffffff".into(),
                color_tolerance: 0,
                detection: Default::default(),
            },
        };
        assert!(validate_action_persist(&pixel, None).is_ok());
        assert!(validate_action(&pixel, None)
            .unwrap_err()
            .to_string()
            .contains("search area"));
    }

    #[test]
    fn validate_run_macro_requires_name() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::RunMacro {
                macro_name: String::new(),
            },
        };
        assert!(validate_action(&a, None)
            .unwrap_err()
            .to_string()
            .contains("macro name"));
    }

    #[test]
    fn validate_conditional_requires_clause_operands() {
        let a = Action {
            id: ActionId::new(),
            kind: ActionKind::Conditional {
                condition: sqyre_domain::ConditionBlock {
                    name: "c".into(),
                    match_mode: sqyre_domain::MatchMode::All,
                    clauses: vec![sqyre_domain::ConditionClause {
                        left: ScalarValue::String(String::new()),
                        operator: sqyre_domain::ConditionOperator::Equals,
                        right: ScalarValue::String("x".into()),
                    }],
                },
                subactions: vec![],
                else_actions: vec![],
            },
        };
        assert!(validate_action(&a, None)
            .unwrap_err()
            .to_string()
            .contains("left operand"));
    }
}
