//! The program name is always a macro tag. It is derived from the catalog key
//! and is not stored in [`super::ProgramData::tags`].

/// Drop stored tags that duplicate `name` (the implicit name tag).
pub(super) fn strip(tags: &mut Vec<String>, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    tags.retain(|tag| tag.trim() != name);
}

/// Name tag first, then `extra`, skipping blanks and duplicates.
pub(super) fn with_name(name: &str, extra: &[String]) -> Vec<String> {
    let name = name.trim();
    let mut out = Vec::with_capacity(extra.len() + 1);
    if !name.is_empty() {
        out.push(name.to_string());
    }
    for tag in extra {
        let tag = tag.trim();
        if tag.is_empty() || out.iter().any(|existing| existing == tag) {
            continue;
        }
        out.push(tag.to_string());
    }
    out
}
