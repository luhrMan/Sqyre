//! Shared parsing / naming helpers for the data editor.

use sqyre_domain::ScalarValue;
use sqyre_persist::ProgramCatalog;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn new_overlay_button_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("btn-{ms}")
}

pub(crate) fn rgba_color(c: [u8; 4]) -> eframe::egui::Color32 {
    eframe::egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

/// `#rrggbb` for persist, or empty when it matches the theme default.
pub(crate) fn overlay_hex_or_empty(c: eframe::egui::Color32, default_hex: &str) -> String {
    let hex = sqyre_domain::format_hex_color([c.r(), c.g(), c.b(), 255]);
    if hex.eq_ignore_ascii_case(default_hex) {
        String::new()
    } else {
        hex
    }
}

pub(crate) fn scalar_to_edit(v: &ScalarValue) -> String {
    v.as_display()
}

pub(crate) fn parse_scalar(s: &str) -> ScalarValue {
    let s = s.trim();
    if s.is_empty() {
        return ScalarValue::Null;
    }
    if let Ok(i) = s.parse::<i64>() {
        return ScalarValue::Int(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return ScalarValue::Float(f);
    }
    ScalarValue::String(s.to_string())
}

pub(crate) fn parse_i32(s: &str) -> Option<i32> {
    s.trim().parse().ok()
}

pub(crate) fn unique_name(base: &str, exists: impl Fn(&str) -> bool) -> String {
    if !exists(base) {
        return base.to_string();
    }
    for i in 2..10_000 {
        let candidate = format!("{base} {i}");
        if !exists(&candidate) {
            return candidate;
        }
    }
    format!("{base} {}", uuid_simple())
}

/// Sorted unique tags across items in a program.
pub(crate) fn collect_program_item_tags(catalog: &ProgramCatalog, program: &str) -> Vec<String> {
    let mut tags: Vec<String> = catalog
        .get(program)
        .map(|p| {
            p.items
                .values()
                .flat_map(|it| it.tags.iter().cloned())
                .collect()
        })
        .unwrap_or_default();
    tags.sort();
    tags.dedup();
    tags
}

pub(crate) fn item_tag_completion_options(
    search: &str,
    on_item: &[String],
    program_tags: &[String],
    limit: usize,
) -> Vec<String> {
    let search_l = search.trim().to_lowercase();
    if search_l.is_empty() {
        return Vec::new();
    }
    program_tags
        .iter()
        .filter(|t| !on_item.iter().any(|c| c == *t))
        .filter(|t| t.to_lowercase().contains(&search_l))
        .take(limit)
        .cloned()
        .collect()
}

pub(crate) fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{n}")
}

pub(crate) fn form_coord_i32(s: &str) -> i32 {
    form_coord_literal(s).unwrap_or(0)
}

/// Parse a form coordinate as a literal number suitable for live preview capture.
/// Returns `None` for `${var}` refs and other non-numeric expressions.
pub(crate) fn form_coord_literal(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return Some(0);
    }
    if sqyre_varref::contains(s) {
        return None;
    }
    if let Ok(i) = s.parse::<i32>() {
        return Some(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Some(f as i32);
    }
    None
}

pub(crate) fn copy_image_as_png(
    src: &std::path::Path,
    dest: &std::path::Path,
) -> Result<(), String> {
    let bytes = std::fs::read(src).map_err(|e| format!("read: {e}"))?;
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        std::fs::write(dest, &bytes).map_err(|e| format!("write: {e}"))?;
        return Ok(());
    }
    let img = image::load_from_memory(&bytes).map_err(|e| format!("decode: {e}"))?;
    img.save(dest).map_err(|e| format!("save png: {e}"))
}
