//! Filesystem-safe entity names for catalog paths under `images/`.
//!
//! Rules live in [`sqyre_validate::validate_entity_name`] (Windows/Linux forbidden
//! filename characters, reserved device names, path escape).

use crate::{PersistError, Result};
use std::path::{Component, Path, PathBuf};

/// Reject names that could escape a managed directory when joined as a path component.
pub fn validate_fs_entity_name(name: &str) -> Result<()> {
    sqyre_validate::validate_entity_name(name).map_err(|e| PersistError::Message(e.to_string()))
}

pub fn is_safe_fs_entity_name(name: &str) -> bool {
    sqyre_validate::is_safe_fs_entity_name(name)
}

/// Join `name` under `base` only when `name` is a single safe path component.
pub fn confined_join(base: &Path, name: &str) -> Result<PathBuf> {
    validate_fs_entity_name(name)?;
    let joined = base.join(name);
    // Defense in depth: joined must stay a direct child of base by components.
    let rel = joined.strip_prefix(base).map_err(|_| {
        PersistError::Message(format!("path escapes base {}: {name}", base.display()))
    })?;
    let mut comps = rel.components();
    match comps.next() {
        Some(Component::Normal(_)) => {}
        _ => {
            return Err(PersistError::Message(format!(
                "invalid path component: {name}"
            )));
        }
    }
    if comps.next().is_some() {
        return Err(PersistError::Message(format!(
            "name must be a single path component: {name}"
        )));
    }
    Ok(joined)
}

/// Like [`confined_join`], but returns `base.join("__invalid__")` when unsafe
/// so read-only UI path lookups never follow traversal.
pub fn confined_join_or_invalid(base: &Path, name: &str) -> PathBuf {
    confined_join(base, name).unwrap_or_else(|_| base.join("__invalid__"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn confined_join_rejects_escape() {
        let base = Path::new("/tmp/images/icons");
        assert!(confined_join(base, "Demo").is_ok());
        assert!(confined_join(base, "../etc").is_err());
        assert!(confined_join(base, "a/b").is_err());
        assert!(confined_join(base, "").is_err());
    }
}
