//! Data Editor tab / section / search-scope enums.

/// Top-level Data Editor section (tab bar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorSection {
    Programs,
    Items,
    Coordinates,
    Tools,
}

impl EditorSection {
    /// Tabs under this section, in menu order.
    pub(super) fn subtabs(self) -> &'static [EditorTab] {
        match self {
            Self::Programs => &[EditorTab::Programs, EditorTab::Overlay],
            Self::Items => &[EditorTab::Items, EditorTab::Masks],
            Self::Coordinates => &[
                EditorTab::Points,
                EditorTab::SearchAreas,
                EditorTab::Collections,
                EditorTab::Atlases,
            ],
            Self::Tools => &[EditorTab::ScreenCap, EditorTab::MatchCheck],
        }
    }

    /// Tools (Screen Cap / Match Check) are desktop-only.
    pub(super) fn is_available(self) -> bool {
        {
            let _ = self;
            true
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum EditorTab {
    #[default]
    Programs,
    Items,
    Masks,
    ScreenCap,
    MatchCheck,
    Points,
    SearchAreas,
    Collections,
    Atlases,
    Overlay,
}

impl EditorTab {
    /// User-visible tab title (never camelCase enum / wire names).
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Programs => "Programs",
            Self::Items => "Items",
            Self::Masks => "Masks",
            Self::ScreenCap => "Screen Cap",
            Self::MatchCheck => "Match Check",
            Self::Points => "Points",
            Self::SearchAreas => "Search Areas",
            Self::Collections => "Collections",
            Self::Atlases => "Atlases",
            Self::Overlay => "Overlay",
        }
    }

    /// Semantic glyph prefixed onto the tab combo face. Entity tabs use the
    /// canonical [`crate::model_icons`] glyph so the same model looks the same
    /// app-wide.
    pub(super) fn icon(self) -> &'static str {
        use egui_phosphor::regular::*;
        match self {
            Self::Programs => crate::model_icons::PROGRAM,
            Self::Items => crate::model_icons::ITEM,
            Self::Masks => crate::model_icons::MASK,
            Self::ScreenCap => CAMERA,
            Self::MatchCheck => MAGNIFYING_GLASS,
            Self::Points => crate::model_icons::POINT,
            Self::SearchAreas => crate::model_icons::SEARCH_AREA,
            Self::Collections => crate::model_icons::COLLECTION,
            Self::Atlases => crate::model_icons::ATLAS,
            Self::Overlay => crate::model_icons::OVERLAY,
        }
    }

    /// Closed-combo / menu face: phosphor glyph + human title.
    pub(crate) fn choice_label(self) -> String {
        format!("{} {}", self.icon(), self.label())
    }

    /// Capture / match tools that need a desktop runtime.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "helper retained for tests / editor-only builds")
    )]
    pub(crate) fn is_desktop_only(self) -> bool {
        matches!(self, Self::ScreenCap | Self::MatchCheck)
    }

    /// Entity noun for this tab (`item “foo”` in the delete confirmation,
    /// `New item` in the footer button's a11y name). `None` when the tab has
    /// no creatable/deletable entity kind.
    pub(super) fn entity_noun(self) -> Option<&'static str> {
        match self {
            Self::Programs => Some("program"),
            Self::Items => Some("item"),
            Self::Points => Some("point"),
            Self::SearchAreas => Some("search area"),
            Self::Masks => Some("mask"),
            Self::Collections => Some("collection"),
            Self::Atlases => Some("atlas"),
            Self::Overlay => Some("overlay button"),
            Self::ScreenCap | Self::MatchCheck => None,
        }
    }
}

/// Which programs the left list / header search covers on entity tabs.
///
/// The Programs tab always lists programs, so it has no scope choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SearchScope {
    /// Flat list of the selected program's entities (default).
    #[default]
    SelectedProgram,
    /// All listed programs grouped under collapsible headers.
    AllPrograms,
}

impl SearchScope {
    pub(crate) const ALL: &'static [SearchScope] = &[Self::SelectedProgram, Self::AllPrograms];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::SelectedProgram => "Selected program",
            Self::AllPrograms => "All programs",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EditorTab;

    #[test]
    fn editor_tab_labels_are_human_titles() {
        assert_eq!(EditorTab::ScreenCap.label(), "Screen Cap");
        assert_eq!(EditorTab::MatchCheck.label(), "Match Check");
        assert_eq!(EditorTab::SearchAreas.label(), "Search Areas");
        assert_eq!(EditorTab::Programs.label(), "Programs");
    }

    #[test]
    fn choice_label_prefixes_icon_onto_title() {
        let face = EditorTab::SearchAreas.choice_label();
        assert!(face.starts_with(EditorTab::SearchAreas.icon()));
        assert!(face.ends_with("Search Areas"));
    }

    #[test]
    fn screen_cap_and_match_check_are_desktop_only() {
        assert!(EditorTab::ScreenCap.is_desktop_only());
        assert!(EditorTab::MatchCheck.is_desktop_only());
        assert!(!EditorTab::Items.is_desktop_only());
        assert!(!EditorTab::Overlay.is_desktop_only());
    }
}
