//! OCR target-match mode enum shared by the wire format and the matcher.

use serde::{Deserialize, Serialize};

/// How the OCR action decides whether recognized text satisfies its Text target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcrMatchMode {
    /// Recognized words must equal the target exactly.
    Exact = 0,
    /// A recognized word may contain the target as a substring
    /// ("networks" hits "network"), but never the reverse.
    #[default]
    Contains = 1,
    /// Either side may be a substantial fragment of the other —
    /// tolerates clipped reads like "networ" for "network".
    Fragment = 2,
    /// Target is a regular expression matched against the recognized text.
    Regex = 3,
}

impl OcrMatchMode {
    pub const ALL: [Self; 4] = [Self::Exact, Self::Contains, Self::Fragment, Self::Regex];

    /// Wire/debug name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Contains => "contains",
            Self::Fragment => "fragment",
            Self::Regex => "regex",
        }
    }

    /// Everyday name for the Match combo.
    pub fn ui_label(self) -> &'static str {
        match self {
            Self::Exact => "Exact",
            Self::Contains => "Contains",
            Self::Fragment => "Loose fragment",
            Self::Regex => "Regex",
        }
    }

    /// Short hover hint for the combo option.
    pub fn ui_hint(self) -> &'static str {
        match self {
            Self::Exact => "Recognized words must equal the target exactly — no partial words.",
            Self::Contains => {
                "A recognized word may contain the target (\"networks\" hits \"network\")."
            }
            Self::Fragment => {
                "Tolerates clipped reads (\"networ\" hits \"network\"). Most forgiving."
            }
            Self::Regex => "Target is a regular expression matched against the recognized text.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_uses_snake_case_wire_names() {
        let yaml = serde_yaml::to_string(&OcrMatchMode::Fragment).unwrap();
        assert_eq!(yaml.trim(), "fragment");
        let back: OcrMatchMode = serde_yaml::from_str("contains").unwrap();
        assert_eq!(back, OcrMatchMode::Contains);
    }

    #[test]
    fn every_mode_has_ui_text() {
        for m in OcrMatchMode::ALL {
            assert!(!m.ui_label().is_empty());
            assert!(!m.ui_hint().is_empty());
        }
    }
}
