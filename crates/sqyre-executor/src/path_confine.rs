//! Confine macro file I/O under the configured variables directory.

use crate::error::{ExecError, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Resolve `relative` under `base`, rejecting absolute paths, `..` / prefix
/// components, and any symlink encountered while walking down to the final
/// component. Rejecting symlinks (rather than only checking the final
/// canonicalized path) keeps this safe even when the target file does not
/// yet exist, which is the common case for `SaveVariable`.
pub(crate) fn resolve_under_dir(base: &Path, relative: &str) -> Result<PathBuf> {
    let rel = Path::new(relative);
    if rel.as_os_str().is_empty() {
        return Err(ExecError::Message("file path cannot be empty".into()));
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

    let mut resolved = base.to_path_buf();
    for c in rel.components() {
        if let Component::Normal(s) = c {
            resolved.push(s);
            let is_symlink = fs::symlink_metadata(&resolved)
                .map(|meta| meta.file_type().is_symlink())
                .unwrap_or(false);
            if is_symlink {
                return Err(ExecError::Message(format!(
                    "file path traverses a symlink: {relative}"
                )));
            }
        }
    }
    Ok(resolved)
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

    #[cfg(unix)]
    #[test]
    fn resolve_under_dir_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().expect("tempdir");
        let base = tmp.path().join("variables");
        fs::create_dir_all(&base).expect("create base");

        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).expect("create outside dir");
        fs::write(outside.join("passwd"), b"secret").expect("write outside file");

        let link = base.join("link");
        symlink(&outside, &link).expect("create symlink");

        // Escaping through a symlinked directory component must be rejected.
        assert!(resolve_under_dir(&base, "link/passwd").is_err());

        // A symlinked file at the leaf position must also be rejected.
        let leaf_link = base.join("leaf_link");
        symlink(outside.join("passwd"), &leaf_link).expect("create leaf symlink");
        assert!(resolve_under_dir(&base, "leaf_link").is_err());

        // Non-symlinked paths under base still resolve normally.
        assert_eq!(
            resolve_under_dir(&base, "real.txt").unwrap(),
            base.join("real.txt")
        );
    }
}
