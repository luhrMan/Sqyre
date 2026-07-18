//! Variable-reference grammar: `${name}` and `{name}` (brace form only when not
//! preceded by `$`).

use std::collections::HashSet;

/// One plain-text or variable-reference segment of a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub is_ref: bool,
    pub name: String,
}

#[derive(Clone)]
struct Match {
    start: usize,
    end: usize,
    name: String,
}

/// Collect `${…}` and bare `{…}` matches (bare braces skip `$` prefixes).
fn find_all_refs(text: &str) -> Vec<Match> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(end) = bytes[i + 2..]
                .iter()
                .position(|&b| b == b'}')
                .map(|p| i + 2 + p)
            {
                let name = &text[i + 2..end];
                if !name.is_empty() {
                    out.push(Match {
                        start: i,
                        end: end + 1,
                        name: name.to_string(),
                    });
                    i = end + 1;
                    continue;
                }
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'{' {
            if i > 0 && bytes[i - 1] == b'$' {
                i += 1;
                continue;
            }
            if let Some(end) = bytes[i + 1..]
                .iter()
                .position(|&b| b == b'}')
                .map(|p| i + 1 + p)
            {
                let name = &text[i + 1..end];
                if !name.is_empty() {
                    out.push(Match {
                        start: i,
                        end: end + 1,
                        name: name.to_string(),
                    });
                    i = end + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    // Dollar refs already skip their interior; bare braces that fall inside a
    // dollar span are dropped by overlap filter for segments/names consistency.
    drop_overlapping(out)
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

/// Reports whether `text` contains a `${name}` or `{name}` reference.
pub fn contains(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    !find_all_refs(text).is_empty()
}

/// Distinct raw variable names referenced in `text` (untrimmed).
pub fn names(text: &str) -> Vec<String> {
    let mut set = HashSet::new();
    for m in find_all_refs(text) {
        set.insert(m.name);
    }
    set.into_iter().collect()
}

/// Splits `text` into plain and reference segments in document order.
pub fn segments(text: &str) -> Vec<Segment> {
    if text.is_empty() {
        return Vec::new();
    }
    let matches = find_all_refs(text);
    if matches.is_empty() {
        return vec![Segment {
            text: text.to_string(),
            is_ref: false,
            name: String::new(),
        }];
    }
    let mut segs = Vec::new();
    let mut last = 0;
    for m in &matches {
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
    find_all_refs(text)
        .iter()
        .any(|m| m.name.trim().eq_ignore_ascii_case(want))
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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

    #[test]
    fn dollar_inner_brace_not_double_counted() {
        // `${foo}` must not also yield a bare `{foo}` name.
        let mut n = names("pre ${foo} end");
        n.sort();
        assert_eq!(n, vec!["foo"]);
    }

    /// Identifier-ish names: start with a letter, then alnum/underscore.
    fn arb_var_name() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{0,16}")
            .unwrap()
            .prop_filter("non-empty", |s| !s.is_empty())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn segments_join_to_original(
            prefix in ".*",
            name in arb_var_name(),
            suffix in ".*",
            dollar in any::<bool>(),
        ) {
            // Avoid accidental new refs in prefix/suffix by stripping braces.
            let prefix = prefix.replace(['{', '}'], "");
            let suffix = suffix.replace(['{', '}'], "");
            let text = if dollar {
                format!("{prefix}${{{name}}}{suffix}")
            } else {
                format!("{prefix}{{{name}}}{suffix}")
            };
            let joined: String = segments(&text).into_iter().map(|s| s.text).collect();
            prop_assert!(contains(&text));
            prop_assert!(references(&text, &name));
            prop_assert_eq!(joined, text);
        }

        #[test]
        fn rename_roundtrip_preserves_style(
            name in arb_var_name(),
            new_name in arb_var_name(),
            dollar in any::<bool>(),
        ) {
            prop_assume!(name != new_name);
            let text = if dollar {
                format!("x=${{{name}}} y")
            } else {
                format!("x={{{name}}} y")
            };
            let renamed = rename(&text, &name, &new_name);
            let expect_new = if dollar {
                format!("${{{new_name}}}")
            } else {
                format!("{{{new_name}}}")
            };
            let expect_old = if dollar {
                format!("${{{name}}}")
            } else {
                format!("{{{name}}}")
            };
            prop_assert!(renamed.contains(&expect_new));
            prop_assert!(!renamed.contains(&expect_old));
            let back = rename(&renamed, &new_name, &name);
            prop_assert_eq!(back, text);
        }

        #[test]
        fn plain_text_has_no_refs(s in "[^{$}]{0,64}") {
            prop_assert!(!contains(&s));
            prop_assert!(names(&s).is_empty());
            let segs = segments(&s);
            if s.is_empty() {
                prop_assert!(segs.is_empty());
            } else {
                prop_assert_eq!(segs.len(), 1);
                prop_assert!(!segs[0].is_ref);
                prop_assert_eq!(&segs[0].text, &s);
            }
        }
    }
}
