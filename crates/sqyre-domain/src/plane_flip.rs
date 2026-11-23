//! Template plane-flip enum for Image Search / Match Check.

use serde::{Deserialize, Serialize};

/// Mirror the template across one or both axes before matching.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaneFlip {
    #[default]
    None,
    Horizontal,
    Vertical,
    Both,
}

impl PlaneFlip {
    pub const ALL: [Self; 4] = [Self::None, Self::Horizontal, Self::Vertical, Self::Both];

    /// Everyday name for the Plane flip combo.
    pub fn ui_label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
            Self::Both => "Both",
        }
    }

    /// Short hover hint for the combo option.
    pub fn ui_hint(self) -> &'static str {
        match self {
            Self::None => "Search with the template as stored (no mirror).",
            Self::Horizontal => "Mirror the template left↔right before searching.",
            Self::Vertical => "Mirror the template top↔bottom before searching.",
            Self::Both => "Mirror the template on both axes before searching.",
        }
    }

    #[inline]
    pub fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    #[inline]
    pub fn flip_horizontal(self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }

    #[inline]
    pub fn flip_vertical(self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_uses_snake_case_wire_names() {
        let yaml = serde_yaml::to_string(&PlaneFlip::Horizontal).unwrap();
        assert_eq!(yaml.trim(), "horizontal");
        let back: PlaneFlip = serde_yaml::from_str("both").unwrap();
        assert_eq!(back, PlaneFlip::Both);
    }

    #[test]
    fn every_mode_has_ui_text() {
        for m in PlaneFlip::ALL {
            assert!(!m.ui_label().is_empty());
            assert!(!m.ui_hint().is_empty());
        }
    }

    #[test]
    fn both_flips_both_axes() {
        assert!(PlaneFlip::Both.flip_horizontal());
        assert!(PlaneFlip::Both.flip_vertical());
        assert!(!PlaneFlip::None.flip_horizontal());
        assert!(!PlaneFlip::None.flip_vertical());
    }
}
