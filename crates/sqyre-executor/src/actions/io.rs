//! Variable I/O actions: SetVariable, SaveVariable.

use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::{resolve_set_variable_value, ActionId, Macro, ScalarValue};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn execute_set_variable(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    variable_name: &str,
    value: &ScalarValue,
    macro_: &mut Macro,
) -> Result<()> {
    let scalar = resolve_set_variable_value(value, macro_).map_err(ExecError::Message)?;
    exec.log(
        action_id,
        format!("Set: {variable_name} = {}", scalar.as_display()),
    );
    macro_.variables.set(variable_name, scalar);
    Ok(())
}

/// Save a variable to clipboard or a file under `variables_dir`.
pub(crate) fn execute_save_variable(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    variable_name: &str,
    destination: &str,
    append: bool,
    append_newline: bool,
    macro_: &Macro,
) -> Result<()> {
    let val = macro_
        .variables
        .get(variable_name)
        .ok_or_else(|| ExecError::Message(format!("variable {variable_name} not found")))?;
    let val_str = val.as_display();

    if destination == "clipboard" {
        exec.deps
            .automation
            .write_clipboard(&val_str)
            .map_err(ExecError::Message)?;
        exec.log(
            action_id,
            format!("SaveVariable: {variable_name} → clipboard"),
        );
        return Ok(());
    }

    let base = exec.deps.variables_dir.ok_or_else(|| {
        ExecError::Message("save variable: variables directory not configured".into())
    })?;
    let file_path = if Path::new(destination).is_absolute() {
        PathBuf::from(destination)
    } else {
        base.join(destination)
    };
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            ExecError::Message(format!(
                "failed to create directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    save_to_file(&val_str, &file_path, append, append_newline)?;
    exec.log(
        action_id,
        format!(
            "SaveVariable: {variable_name} → {} ({})",
            file_path.display(),
            if append { "append" } else { "overwrite" }
        ),
    );
    Ok(())
}

fn save_to_file(value: &str, path: &Path, append: bool, append_newline: bool) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .map_err(|e| {
            ExecError::Message(format!(
                "failed to save variable to file {}: {e}",
                path.display()
            ))
        })?;
    file.write_all(value.as_bytes())
        .map_err(|e| ExecError::Message(format!("failed to write {}: {e}", path.display())))?;
    if append_newline {
        file.write_all(b"\n").map_err(|e| {
            ExecError::Message(format!("failed to write newline {}: {e}", path.display()))
        })?;
    }
    Ok(())
}
