//! Log entry filter / section grouping helpers for the action Logs window.

use sqyre_ports::{ActionLogEntry, LogFind};

/// Which entry kinds the filter passes.
#[derive(Clone, Copy)]
pub(super) struct KindFilter {
    pub(super) text: bool,
    pub(super) images: bool,
    pub(super) items: bool,
}

impl Default for KindFilter {
    fn default() -> Self {
        Self {
            text: true,
            images: true,
            items: true,
        }
    }
}

pub(super) fn counts_text(entries: &[ActionLogEntry], visible: &[usize]) -> String {
    let mut text = 0;
    let mut images = 0;
    let mut items = 0;
    for &i in visible {
        match &entries[i] {
            ActionLogEntry::Text(_) | ActionLogEntry::Finds { .. } | ActionLogEntry::Section(_) => {
                text += 1
            }
            ActionLogEntry::Image(_) => images += 1,
            ActionLogEntry::ItemPipeline { .. } => items += 1,
        }
    }
    if visible.len() == entries.len() {
        format!("{text} line(s) · {images} image(s) · {items} item(s)")
    } else {
        format!(
            "{}/{} shown · {text} lines · {images} images · {items} items",
            visible.len(),
            entries.len()
        )
    }
}

/// Filter predicate: kind toggle + fuzzy query over text / labels / finds.
pub(super) fn entry_matches(e: &ActionLogEntry, q: &str, kinds: &KindFilter) -> bool {
    match e {
        ActionLogEntry::Text(line) => kinds.text && crate::pickers::fuzzy_match_fold(q, line),
        // Section headers are structure, not content — keep them whenever text
        // lines are shown so collapsed groups still label themselves.
        ActionLogEntry::Section(_) => kinds.text,
        ActionLogEntry::Finds { text, finds } => {
            kinds.text
                && (crate::pickers::fuzzy_match_fold(q, text)
                    || finds.iter().any(|f| find_matches(f, q)))
        }
        ActionLogEntry::Image(img) => {
            kinds.images && crate::pickers::fuzzy_match_fold(q, &img.label)
        }
        ActionLogEntry::ItemPipeline {
            title,
            summary,
            details,
            finds,
            ..
        } => {
            kinds.items
                && (crate::pickers::fuzzy_match_fold(q, title)
                    || crate::pickers::fuzzy_match_fold(q, summary)
                    || details
                        .iter()
                        .any(|d| crate::pickers::fuzzy_match_fold(q, d))
                    || finds.iter().any(|f| find_matches(f, q)))
        }
    }
}

/// Fuzzy-match a find's label and detail (e.g. OCR text found inside the box).
pub(super) fn find_matches(f: &LogFind, q: &str) -> bool {
    crate::pickers::fuzzy_match_fold(q, &f.label)
        || f.detail
            .as_deref()
            .is_some_and(|d| crate::pickers::fuzzy_match_fold(q, d))
}

/// One run of filtered entries under a section header — `label: None` is the
/// leading unsectioned run (actions that never emit markers render flat).
pub(super) struct SectionChunk<'a> {
    pub(super) label: Option<&'a str>,
    /// Index of the `Section` entry — stable toggle key within this snapshot.
    pub(super) key: usize,
    /// Indices into `entries`, already filtered.
    pub(super) indices: Vec<usize>,
}

/// Group `entries` on `Section` boundaries, keeping only `visible` indices.
pub(super) fn section_chunks<'a>(
    entries: &'a [ActionLogEntry],
    visible: &[usize],
) -> Vec<SectionChunk<'a>> {
    let mut chunks = vec![SectionChunk {
        label: None,
        key: 0,
        indices: Vec::new(),
    }];
    let mut vi = 0usize;
    for (i, e) in entries.iter().enumerate() {
        let in_visible = vi < visible.len() && visible[vi] == i;
        if in_visible {
            vi += 1;
        }
        if let ActionLogEntry::Section(label) = e {
            chunks.push(SectionChunk {
                label: Some(label.as_str()),
                key: i,
                indices: Vec::new(),
            });
        } else if in_visible {
            chunks.last_mut().expect("leading chunk").indices.push(i);
        }
    }
    chunks
}

/// Entry indices of visible item pipelines (respects the active filter).
pub(super) fn item_indices(entries: &[ActionLogEntry], q: &str, kinds: &KindFilter) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.is_item_pipeline() && entry_matches(e, q, kinds))
        .map(|(i, _)| i)
        .collect()
}

/// Step `cur` through `items` by `dir` (`±1`); `None` starts at an end.
pub(super) fn step_item(items: &[usize], cur: Option<usize>, dir: i32) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    let idx = match cur.and_then(|c| items.iter().position(|&i| i == c)) {
        Some(p) => (p as i32 + dir).clamp(0, items.len() as i32 - 1) as usize,
        None if dir < 0 => items.len() - 1,
        None => 0,
    };
    Some(items[idx])
}

#[cfg(test)]
mod tests {
    use super::{entry_matches, item_indices, section_chunks, step_item, KindFilter};
    use sqyre_ports::{ActionLogEntry, LogFind, LogImage};

    fn solid(label: &str) -> LogImage {
        LogImage::from_rgba(label.to_string(), 2, 2, vec![0u8; 16]).unwrap()
    }

    fn pipeline(title: &str) -> ActionLogEntry {
        ActionLogEntry::ItemPipeline {
            title: title.into(),
            summary: "sum".into(),
            thumbnail: solid("t"),
            steps: vec![],
            details: vec!["detail-line".into()],
            finds: vec![LogFind {
                label: "Find #1".into(),
                detail: None,
                screen: (0, 0, 4, 4),
                context: None,
            }],
        }
    }

    fn kinds() -> KindFilter {
        KindFilter::default()
    }

    #[test]
    fn entry_matches_kind_toggles() {
        let text = ActionLogEntry::Text("hello".into());
        let image = ActionLogEntry::Image(solid("cap"));
        let item = pipeline("Item");
        let k = kinds();
        assert!(entry_matches(&text, "", &k));
        assert!(entry_matches(&image, "", &k));
        assert!(entry_matches(&item, "", &k));
        let mut only_text = k;
        only_text.images = false;
        only_text.items = false;
        assert!(entry_matches(&text, "", &only_text));
        assert!(!entry_matches(&image, "", &only_text));
        assert!(!entry_matches(&item, "", &only_text));
    }

    #[test]
    fn entry_matches_query_across_fields() {
        let finds = ActionLogEntry::Finds {
            text: "OCR matched".into(),
            finds: vec![LogFind {
                label: "Find #1 (\"Submit\")".into(),
                detail: Some("found \"Submit\"".into()),
                screen: (0, 0, 4, 4),
                context: None,
            }],
        };
        let k = kinds();
        assert!(entry_matches(&finds, "submit", &k), "find label searched");
        assert!(entry_matches(&finds, "ocr", &k), "text searched");
        assert!(!entry_matches(&finds, "zzzz", &k));
        let item = pipeline("Sword");
        assert!(entry_matches(&item, "detail-line", &k), "details searched");
        assert!(entry_matches(&item, "find", &k), "find labels searched");
        let img = ActionLogEntry::Image(solid("Where found"));
        assert!(entry_matches(&img, "where", &k));
    }

    #[test]
    fn item_indices_respect_filter() {
        let entries = vec![
            ActionLogEntry::Text("a".into()),
            pipeline("Sword"),
            ActionLogEntry::Text("b".into()),
            pipeline("Potion"),
        ];
        let k = kinds();
        assert_eq!(item_indices(&entries, "", &k), vec![1, 3]);
        assert_eq!(item_indices(&entries, "sword", &k), vec![1]);
    }

    #[test]
    fn step_item_walks_indices() {
        let items = vec![1, 3, 7];
        assert_eq!(step_item(&items, None, 1), Some(1));
        assert_eq!(step_item(&items, None, -1), Some(7));
        assert_eq!(step_item(&items, Some(3), 1), Some(7));
        assert_eq!(step_item(&items, Some(3), -1), Some(1));
        assert_eq!(step_item(&items, Some(7), 1), Some(7), "clamped at end");
        assert_eq!(step_item(&items, Some(99), 1), Some(1), "stale → start");
        assert_eq!(step_item(&[], None, 1), None);
    }

    #[test]
    pub(super) fn section_chunks_group_on_markers() {
        let entries = vec![
            ActionLogEntry::Text("before".into()),
            ActionLogEntry::Section("Attempt 1".into()),
            ActionLogEntry::Text("a1".into()),
            ActionLogEntry::Text("a1b".into()),
            ActionLogEntry::Section("Attempt 2".into()),
            ActionLogEntry::Text("a2".into()),
        ];
        let visible: Vec<usize> = (0..entries.len()).collect();
        let chunks = section_chunks(&entries, &visible);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].label, None);
        assert_eq!(chunks[0].indices, vec![0]);
        assert_eq!(chunks[1].label, Some("Attempt 1"));
        assert_eq!(chunks[1].indices, vec![2, 3]);
        assert_eq!(chunks[2].label, Some("Attempt 2"));
        assert_eq!(chunks[2].indices, vec![5]);
    }

    #[test]
    pub(super) fn section_chunks_respect_filter() {
        let entries = vec![
            ActionLogEntry::Section("Attempt 1".into()),
            ActionLogEntry::Text("keep".into()),
            ActionLogEntry::Image(solid("cap")),
            ActionLogEntry::Text("keep2".into()),
        ];
        // Only the two text lines are visible (images toggled off).
        let visible = vec![0, 1, 3];
        let chunks = section_chunks(&entries, &visible);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].indices, Vec::<usize>::new());
        assert_eq!(chunks[1].label, Some("Attempt 1"));
        assert_eq!(chunks[1].indices, vec![1, 3]);
    }
}
