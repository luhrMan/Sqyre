use sqyre_capture::WindowInfo;

pub fn query_matches_name_or_tags(q: &str, name: &str, tags: &[String]) -> bool {
    if q.is_empty() {
        return true;
    }
    fuzzy_match_fold(q, name) || tags.iter().any(|t| fuzzy_match_fold(q, t))
}

/// Subsequence fuzzy match: each needle char appears in order in haystack.
pub fn fuzzy_match_fold(needle: &str, haystack: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let mut hay = haystack.chars().flat_map(|c| c.to_lowercase());
    'outer: for nc in needle.chars().flat_map(|c| c.to_lowercase()) {
        for hc in hay.by_ref() {
            if hc == nc {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

pub(crate) fn query_matches_window(q: &str, w: &WindowInfo) -> bool {
    if q.is_empty() {
        return true;
    }
    fuzzy_match_fold(q, &w.title)
        || fuzzy_match_fold(q, &w.process_name)
        || fuzzy_match_fold(q, &w.process_path)
}
