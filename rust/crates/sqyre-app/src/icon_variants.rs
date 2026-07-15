//! Icon variant filesystem helpers (Go `IconVariantService`).

use sqyre_domain::PROGRAM_DELIMITER;
use sqyre_persist::ProgramCatalog;
use sqyre_vision::invalidate_search_templates_under;
use std::path::{Path, PathBuf};

const MAX_VARIANTS: usize = 100;
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Debug, Clone)]
pub struct VariantExistsError {
    pub variant_name: String,
}

impl std::fmt::Display for VariantExistsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "variant '{}' already exists", self.variant_name)
    }
}

impl std::error::Error for VariantExistsError {}

pub fn variant_path(catalog: &ProgramCatalog, program: &str, item: &str, variant: &str) -> PathBuf {
    let dir = catalog.icons_dir(program);
    if variant.is_empty() {
        dir.join(format!("{item}.png"))
    } else {
        dir.join(format!("{item}{PROGRAM_DELIMITER}{variant}.png"))
    }
}

pub fn variant_names(catalog: &ProgramCatalog, program: &str, item: &str) -> Vec<String> {
    let target = format!("{program}{PROGRAM_DELIMITER}{item}");
    let mut names: Vec<String> = catalog
        .variant_paths(&target)
        .into_iter()
        .map(|p| variant_name_from_path(&p, item))
        .collect();
    names.sort();
    names.dedup();
    names
}

pub fn variant_name_from_path(path: &Path, item: &str) -> String {
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return String::new();
    };
    if stem == item {
        return String::new();
    }
    let prefix = format!("{item}{PROGRAM_DELIMITER}");
    stem.strip_prefix(&prefix)
        .unwrap_or(stem)
        .to_string()
}

pub fn validate_png_file(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("file path cannot be empty".into());
    }
    let meta = std::fs::metadata(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "file does not exist".into()
        } else {
            format!("failed to access file: {e}")
        }
    })?;
    if !meta.is_file() {
        return Err("path is not a regular file".into());
    }
    let mut file = std::fs::File::open(path).map_err(|e| format!("failed to open file: {e}"))?;
    let mut header = [0u8; 8];
    use std::io::Read;
    let n = file
        .read(&mut header)
        .map_err(|e| format!("failed to read file header: {e}"))?;
    if n < 8 {
        return Err("file too small to be a valid PNG".into());
    }
    if header != PNG_SIGNATURE {
        return Err("file is not a valid PNG (invalid header signature)".into());
    }
    Ok(())
}

fn sanitize_variant_name(name: &str) -> Result<String, String> {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("invalid variant name: contains path separators".into());
    }
    let base = Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    if base.is_empty() {
        return Err("variant name cannot be empty".into());
    }
    Ok(base)
}

#[derive(Debug)]
pub enum AddVariantError {
    Exists(VariantExistsError),
    Other(String),
}

impl std::fmt::Display for AddVariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exists(e) => write!(f, "{e}"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

pub fn add_variant(
    catalog: &ProgramCatalog,
    program: &str,
    item: &str,
    variant_name: &str,
    source: &Path,
) -> Result<String, AddVariantError> {
    if program.is_empty() || item.is_empty() {
        return Err(AddVariantError::Other(
            "program name and item name cannot be empty".into(),
        ));
    }
    validate_png_file(source).map_err(AddVariantError::Other)?;
    let existing = variant_names(catalog, program, item);
    let name = if existing.is_empty() {
        "Original".to_string()
    } else {
        let n = sanitize_variant_name(variant_name).map_err(AddVariantError::Other)?;
        if n.is_empty() {
            return Err(AddVariantError::Other(
                "variant name cannot be empty".into(),
            ));
        }
        n
    };
    if existing.contains(&name) {
        return Err(AddVariantError::Exists(VariantExistsError {
            variant_name: name,
        }));
    }
    if existing.len() >= MAX_VARIANTS {
        return Err(AddVariantError::Other(format!(
            "maximum variant limit ({MAX_VARIANTS}) reached for item '{item}'"
        )));
    }
    let dest_dir = catalog.icons_dir(program);
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| AddVariantError::Other(format!("create icons dir: {e}")))?;
    let dest = variant_path(catalog, program, item, &name);
    std::fs::copy(source, &dest).map_err(|e| AddVariantError::Other(format!("copy file: {e}")))?;
    invalidate_item_templates(catalog, program, item);
    Ok(name)
}

pub fn overwrite_variant(
    catalog: &ProgramCatalog,
    program: &str,
    item: &str,
    variant_name: &str,
    source: &Path,
) -> Result<(), String> {
    if program.is_empty() || item.is_empty() || variant_name.is_empty() {
        return Err("program name, item name, and variant name cannot be empty".into());
    }
    validate_png_file(source)?;
    let name = sanitize_variant_name(variant_name)?;
    let dest_dir = catalog.icons_dir(program);
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("create icons dir: {e}"))?;
    let dest = variant_path(catalog, program, item, &name);
    std::fs::copy(source, &dest).map_err(|e| format!("copy file: {e}"))?;
    invalidate_item_templates(catalog, program, item);
    Ok(())
}

pub fn delete_variant(
    catalog: &ProgramCatalog,
    program: &str,
    item: &str,
    variant_name: &str,
) -> Result<(), String> {
    if program.is_empty() || item.is_empty() {
        return Err("program name and item name cannot be empty".into());
    }
    if variant_name == "Original" {
        return Err("cannot delete the 'Original' variant".into());
    }
    let path = variant_path(catalog, program, item, variant_name);
    match std::fs::remove_file(&path) {
        Ok(()) => {
            invalidate_item_templates(catalog, program, item);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            invalidate_item_templates(catalog, program, item);
            Ok(())
        }
        Err(e) => Err(format!("failed to delete variant file: {e}")),
    }
}

fn invalidate_item_templates(catalog: &ProgramCatalog, program: &str, item: &str) {
    let prefix = catalog.icons_dir(program).join(item);
    invalidate_search_templates_under(&prefix);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_persist::ProgramCatalog;
    use tempfile::tempdir;

    fn catalog_with_icons(root: &Path) -> ProgramCatalog {
        let mut c = ProgramCatalog::default();
        c.set_images_root(Some(root.to_path_buf()));
        c
    }

    #[test]
    fn add_first_variant_forced_original() {
        let dir = tempdir().unwrap();
        let cat = catalog_with_icons(dir.path());
        let icons = cat.icons_dir("Prog");
        std::fs::create_dir_all(&icons).unwrap();
        let src = dir.path().join("src.png");
        std::fs::write(&src, PNG_SIGNATURE).unwrap();
        // Need a valid enough PNG - signature alone may fail later loads, but validate only checks header
        let name = add_variant(&cat, "Prog", "Sword", "ignored", &src).unwrap();
        assert_eq!(name, "Original");
        assert!(variant_path(&cat, "Prog", "Sword", "Original").is_file());
    }
}
