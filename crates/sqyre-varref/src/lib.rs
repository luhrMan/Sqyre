//! Variable-reference grammar: `${name}` and `{name}` (brace form only when not
//! preceded by `$`).

use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

/// One plain-text or variable-reference segment of a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub is_ref: bool,
    pub name: String,
}

fn dollar_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\$\{([^}]+)\}").expect("dollar pattern"))
}

fn brace_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{([^}]+)\}").expect("brace pattern"))
}

/// Reports whether `text` contains a `${name}` or `{name}` reference.
pub fn contains(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    if dollar_pattern().is_match(text) {
        return true;
    }
    !find_brace_refs(text).is_empty()
}

/// Distinct raw variable names referenced in `text` (untrimmed).
pub fn names(text: &str) -> Vec<String> {
    let mut set = HashSet::new();
    for caps in dollar_pattern().captures_iter(text) {
        if let Some(m) = caps.get(1) {
            set.insert(m.as_str().to_string());
        }
    }
    for caps in brace_pattern().captures_iter(text) {
        if let Some(m) = caps.get(1) {
            set.insert(m.as_str().to_string());
        }
    }
    set.into_iter().collect()
}

/// Splits `text` into plain and reference segments in document order.
pub fn segments(text: &str) -> Vec<Segment> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut matches = Vec::new();
    for caps in dollar_pattern().captures_iter(text) {
        let full = caps.get(0).unwrap();
        let name = caps.get(1).unwrap().as_str().to_string();
        matches.push(Match {
            start: full.start(),
            end: full.end(),
            name,
        });
    }
    matches.extend(find_brace_refs(text));
    if matches.is_empty() {
        return vec![Segment {
            text: text.to_string(),
            is_ref: false,
            name: String::new(),
        }];
    }
    matches.sort_by_key(|m| m.start);
    let filtered = drop_overlapping(matches);
    let mut segs = Vec::new();
    let mut last = 0;
    for m in &filtered {
        if m.start > last {
            segs.push(Segment {
                text: text[last..m.start].to_string(),
                is_ref: false,
                name: String::new(),
            });
        }
        segs.push(Segment {
            text: text[m.start..m.end].to_string(),
            is_ref: true,
            name: m.name.clone(),
        });
        last = m.end;
    }
    if last < text.len() {
        segs.push(Segment {
            text: text[last..].to_string(),
            is_ref: false,
            name: String::new(),
        });
    }
    segs
}

/// Whether `text` references `name` (case-insensitive; whitespace inside braces OK).
pub fn references(text: &str, name: &str) -> bool {
    if text.is_empty() || name.is_empty() {
        return false;
    }
    let want = name.trim();
    names(text)
        .iter()
        .any(|n| n.trim().eq_ignore_ascii_case(want))
}

/// Replaces `${old}` / `{old}` with `new_name`, preserving brace style.
pub fn rename(s: &str, old_name: &str, new_name: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }
    let want = old_name.trim();
    let segs = segments(s);
    let mut out = String::with_capacity(s.len());
    for seg in segs {
        if !seg.is_ref || !seg.name.trim().eq_ignore_ascii_case(want) {
            out.push_str(&seg.text);
            continue;
        }
        // Preserve `$` vs bare-brace style from the original segment text.
        if seg.text.starts_with("${") {
            out.push_str("${");
            out.push_str(new_name);
            out.push('}');
        } else {
            out.push('{');
            out.push_str(new_name);
            out.push('}');
        }
    }
    out
}

#[derive(Clone)]
struct Match {
    start: usize,
    end: usize,
    name: String,
}

fn find_brace_refs(text: &str) -> Vec<Match> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        if i > 0 && bytes[i - 1] == b'$' {
            i += 1;
            continue;
        }
        let Some(end) = bytes[i + 1..].iter().position(|&b| b == b'}').map(|p| i + 1 + p)
        else {
            i += 1;
            continue;
        };
        let name = &text[i + 1..end];
        if name.is_empty() {
            i += 1;
            continue;
        }
        out.push(Match {
            start: i,
            end: end + 1,
            name: name.to_string(),
        });
        i = end;
        i += 1;
    }
    out
}

fn drop_overlapping(matches: Vec<Match>) -> Vec<Match> {
    let mut filtered = Vec::new();
    for m in matches {
        let overlap = filtered
            .iter()
            .any(|prev: &Match| m.start >= prev.start && m.end <= prev.end);
        if !overlap {
            filtered.push(m);
        }
    }
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_dollar_and_brace() {
        assert!(contains("wait ${delay}"));
        assert!(contains("value {x}"));
        assert!(!contains("plain"));
        assert!(!contains("${")); // incomplete
    }

    #[test]
    fn names_dedups() {
        let mut n = names("${a} and {a} and ${b}");
        n.sort();
        assert_eq!(n, vec!["a", "b"]);
    }

    #[test]
    fn segments_skip_inner_brace_of_dollar() {
        let segs = segments("pre ${foo} mid {bar} end");
        let refs: Vec<_> = segs.iter().filter(|s| s.is_ref).collect();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].name, "foo");
        assert_eq!(refs[1].name, "bar");
    }

    #[test]
    fn references_case_insensitive() {
        assert!(references("${Delay}", "delay"));
        assert!(references(" { Delay } ", "delay"));
        assert!(!references("${other}", "delay"));
    }

    #[test]
    fn rename_preserves_style() {
        assert_eq!(rename("x=${Old} y={Old}", "old", "new"), "x=${new} y={new}");
    }
}
