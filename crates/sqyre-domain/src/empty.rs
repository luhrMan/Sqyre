//! User-visible empty / missing value labels (display only).
//!
//! Wire format, serde keys, and stored values are unchanged. Use these constants
//! for every UI empty-value role; do not invent local synonyms.

/// Missing catalog or coordinate reference (e.g. unset Move point).
pub const EMPTY_UNSET: &str = "(unset)";

/// Optional control: no entity selected (combos, empty target lists).
pub const EMPTY_NONE: &str = "(none)";

/// Empty non-catalog scalar or hotkey (toolbar meta, Focus title, Continue key, …).
pub const EMPTY_NOT_SET: &str = "Not set";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CoordinateRef;

    #[test]
    fn glossary_roles_are_distinct() {
        assert_eq!(EMPTY_UNSET, "(unset)");
        assert_eq!(EMPTY_NONE, "(none)");
        assert_eq!(EMPTY_NOT_SET, "Not set");
        assert_ne!(EMPTY_UNSET, EMPTY_NONE);
        assert_ne!(EMPTY_UNSET, EMPTY_NOT_SET);
        assert_ne!(EMPTY_NONE, EMPTY_NOT_SET);
        assert_eq!(CoordinateRef::UNSET_LABEL, EMPTY_UNSET);
    }
}
