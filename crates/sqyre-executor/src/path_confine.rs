//! Confine macro file I/O under the configured variables directory.

use crate::error::{ExecError, Result};
use std::path::{Component, Path, PathBuf};

/// Resolve `relative` under `base`, rejecting absolute paths and `..` / prefix components.
pub(crate) fn resolve_under_dir(base: &Path, relative: &str) -> Result<PathBuf> {
    let rel = Path::new(relative);
    if rel.as_os_str().is_empty() {
        return Err(ExecError::Message(
            "file path cannot be empty".into(),
        ));
    }
    if rel.is_absolute() {
        return Err(ExecError::Message(format!(
            "absolute file paths are not allowed: {relative}"
        )));
    }
    for c in rel.components() {
        match c {
            Component::Normal(s) => {
                if s.to_string_lossy().contains('\0') {
                    return Err(ExecError::Message(format!(
                        "file path contains NUL: {relative}"
                    )));
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ExecError::Message(format!(
                    "file path escapes variables directory: {relative}"
                )));
            }
        }
    }
    Ok(base.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn resolve_under_dir_rejects_escape() {
        let base = Path::new("/tmp/vars");
        assert_eq!(
            resolve_under_dir(base, "out.txt").unwrap(),
            base.join("out.txt")
        );
        assert!(resolve_under_dir(base, "../etc/passwd").is_err());
        assert!(resolve_under_dir(base, "/etc/passwd").is_err());
        assert!(resolve_under_dir(base, "a/../../x").is_err());
        assert!(resolve_under_dir(base, "").is_err());
    }
}
