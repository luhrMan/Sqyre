//! Caret context for the YAML Macro Builder: which mapping or list the caret
//! sits in, what belongs there, and how far a new line should indent.
//!
//! Expects block YAML indented with spaces. Sequences may be compact (`- item`
//! at the owning key's column, as serde_yaml writes them) or indented.

use std::collections::BTreeSet;

use sqyre_domain::{
    action_wire_desc, enum_values_for_field, macro_wire_fields, ActionWireDesc, WireField,
    WireFieldKind, ACTION_WIRE_DESCS,
};

/// One indent step. YAML forbids tab indentation.
pub(super) const INDENT: &str = "  ";

/// Catalog entity a value refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityKind {
    Item,
    Point,
    SearchArea,
    Collection,
    Program,
    Atlas,
}

/// What the caret is completing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Completion {
    /// `type:` value of an action.
    ActionType,
    /// Keys that still fit the enclosing mapping, in schema order.
    Key(Vec<&'static str>),
    Enum {
        field: String,
        values: &'static [&'static str],
    },
    Entity {
        kind: EntityKind,
        /// Sibling `program:` value (narrows atlas suggestions).
        program: Option<String>,
    },
    MacroName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct YamlContext {
    /// Byte range to replace on accept.
    pub start: usize,
    pub end: usize,
    pub completion: Completion,
    pub query: String,
    /// Text from line start up to the key (`indent` or `indent- `); stub expansion reuses it.
    pub line_prefix: String,
}

/// One parsed source line.
#[derive(Debug, Clone, Copy)]
struct Line<'a> {
    indent: usize,
    /// Starts a sequence item (`- …`).
    dash: bool,
    /// Byte column where the key or scalar starts (after any `- `).
    col: usize,
    body: &'a str,
}

impl<'a> Line<'a> {
    fn parse(raw: &'a str) -> Self {
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        let rest = &raw[indent..];
        let (dash, after) = match rest.strip_prefix('-') {
            Some(a) if a.is_empty() || a.starts_with(' ') => (true, a),
            _ => (false, rest),
        };
        let body = after.trim_start_matches(' ');
        let col = if dash {
            indent + 1 + (after.len() - body.len())
        } else {
            indent
        };
        Self {
            indent,
            dash,
            col,
            body: body.trim_end(),
        }
    }

    fn is_blank(&self) -> bool {
        (self.body.is_empty() && !self.dash) || self.body.starts_with('#')
    }

    /// `key` when the body is `key:` or `key: value`.
    fn key(&self) -> Option<&'a str> {
        let i = self.body.find(':')?;
        let key = &self.body[..i];
        let rest = &self.body[i + 1..];
        let plain = !key.is_empty() && !key.contains(' ') && !key.starts_with(['"', '\'']);
        (plain && (rest.is_empty() || rest.starts_with(' '))).then_some(key)
    }

    fn value(&self) -> &'a str {
        self.key().map_or("", |k| self.body[k.len() + 1..].trim())
    }

    /// `key:` with nothing after it owns the indented block below.
    fn opens_block(&self) -> bool {
        self.key().is_some() && self.value().is_empty()
    }
}

/// Schema for the mapping around the caret.
#[derive(Debug, Clone, Copy)]
enum Scope {
    Macro,
    /// Action mapping; `None` until `type:` names a known action.
    Action(Option<&'static ActionWireDesc>),
    Nested(&'static [WireField]),
    Unknown,
}

impl Scope {
    fn fields(self) -> Option<&'static [WireField]> {
        match self {
            Self::Macro => Some(macro_wire_fields()),
            Self::Action(desc) => desc.map(|d| d.fields),
            Self::Nested(fields) => Some(fields),
            Self::Unknown => None,
        }
    }
}

struct Mapping<'a> {
    scope: Scope,
    /// Keys already present, excluding the caret line.
    keys: Vec<&'a str>,
    program: Option<&'a str>,
}

fn all_wire_fields() -> impl Iterator<Item = &'static WireField> {
    let top = macro_wire_fields()
        .iter()
        .chain(ACTION_WIRE_DESCS.iter().flat_map(|d| d.fields));
    top.flat_map(|f| std::iter::once(f).chain(f.nested))
}

/// Field list for entries under an object-list / object key (`clauses`, `variables`, …).
fn nested_fields(key: &str) -> Option<&'static [WireField]> {
    all_wire_fields()
        .find(|f| f.key == key && !f.nested.is_empty())
        .map(|f| f.nested)
}

fn is_action_list(key: &str) -> bool {
    matches!(key, "root" | "subactions" | "elseactions")
}

/// Sequence items under `key` are mappings (actions or nested objects).
fn is_mapping_list(key: &str) -> bool {
    is_action_list(key) || nested_fields(key).is_some()
}

/// Values under `key` are a YAML sequence.
fn is_list_key(key: &str) -> bool {
    all_wire_fields().any(|f| {
        f.key == key
            && matches!(
                f.kind,
                WireFieldKind::StringList | WireFieldKind::ObjectList | WireFieldKind::ActionList
            )
    })
}

/// Line that owns the sequence item at `idx` (indented with `indent` spaces).
fn list_owner<'a>(lines: &[&'a str], idx: usize, indent: usize) -> Option<Line<'a>> {
    lines[..idx]
        .iter()
        .rev()
        .map(|raw| Line::parse(raw))
        .find(|l| !l.is_blank() && l.indent <= indent && !(l.dash && l.indent == indent))
        .filter(Line::opens_block)
}

/// Mapping whose keys sit at column `col` around line `idx`.
fn mapping_at<'a>(lines: &[&'a str], idx: usize, col: usize) -> Mapping<'a> {
    let caret = Line::parse(lines[idx]);
    let mut members: Vec<Line<'a>> = Vec::new();
    let mut start = idx;
    let mut parent = None;
    if !(caret.dash && caret.col == col) {
        for j in (0..idx).rev() {
            let l = Line::parse(lines[j]);
            if l.is_blank() || l.col > col {
                continue;
            }
            if l.col < col {
                parent = Some(l);
                break;
            }
            members.push(l);
            start = j;
            if l.dash {
                break;
            }
        }
    }
    for raw in &lines[idx + 1..] {
        let l = Line::parse(raw);
        if l.is_blank() || l.col > col {
            continue;
        }
        if l.col == col && !l.dash {
            members.push(l);
            continue;
        }
        break;
    }

    let first = Line::parse(lines[start]);
    let container = if first.dash && first.col == col {
        list_owner(lines, start, first.indent)
    } else {
        parent.filter(Line::opens_block)
    }
    .and_then(|l| l.key());
    let value_of = |key: &str| {
        members
            .iter()
            .find(|l| l.key() == Some(key))
            .map(|l| l.value().trim_matches(['"', '\'']))
    };
    let scope = match container {
        None if col == 0 && parent.is_none() => Scope::Macro,
        Some(k) if is_action_list(k) => Scope::Action(value_of("type").and_then(action_wire_desc)),
        Some(k) => nested_fields(k).map_or(Scope::Unknown, Scope::Nested),
        None => Scope::Unknown,
    };
    Mapping {
        scope,
        keys: members.iter().filter_map(Line::key).collect(),
        program: value_of("program").filter(|p| !p.is_empty()),
    }
}

fn key_completion(m: &Mapping<'_>) -> Completion {
    let mut keys: Vec<&'static str> = match m.scope {
        Scope::Action(None) => std::iter::once("type")
            .chain(
                ACTION_WIRE_DESCS
                    .iter()
                    .flat_map(|d| d.fields.iter().map(|f| f.key))
                    .collect::<BTreeSet<_>>(),
            )
            .collect(),
        Scope::Action(Some(desc)) => std::iter::once("type")
            .chain(desc.fields.iter().map(|f| f.key))
            .collect(),
        Scope::Macro | Scope::Nested(_) => m
            .scope
            .fields()
            .unwrap_or_default()
            .iter()
            .map(|f| f.key)
            .collect(),
        Scope::Unknown => all_wire_fields()
            .map(|f| f.key)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    };
    keys.retain(|k| !m.keys.contains(k));
    Completion::Key(keys)
}

fn value_completion(key: &str, m: &Mapping<'_>) -> Option<Completion> {
    if key == "type" && matches!(m.scope, Scope::Action(_) | Scope::Unknown) {
        return Some(Completion::ActionType);
    }
    let field = m
        .scope
        .fields()
        .and_then(|fs| fs.iter().find(|f| f.key == key));
    if let Some(WireField {
        kind: WireFieldKind::Enum(values),
        ..
    }) = field
    {
        return Some(Completion::Enum {
            field: key.into(),
            values,
        });
    }
    let entity = |kind| Completion::Entity {
        kind,
        program: None,
    };
    Some(match key {
        "point" => entity(EntityKind::Point),
        "searcharea" => entity(EntityKind::SearchArea),
        "cells" => entity(EntityKind::Collection),
        "program" => entity(EntityKind::Program),
        "atlas" => Completion::Entity {
            kind: EntityKind::Atlas,
            program: m.program.map(str::to_string),
        },
        "macroname" => Completion::MacroName,
        _ if field.is_none() => Completion::Enum {
            field: key.into(),
            values: enum_values_for_field(key)?,
        },
        _ => return None,
    })
}

/// Completion context at byte offset `cursor`.
pub(super) fn find_yaml_context(text: &str, cursor: usize) -> Option<YamlContext> {
    let cursor = cursor.min(text.len());
    let line_start = text[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let lines: Vec<&str> = text.split('\n').collect();
    let idx = text[..line_start].matches('\n').count();
    let raw = &text[line_start..cursor];
    let before = Line::parse(raw);
    if before.body.is_empty() || before.body.starts_with('#') {
        return None;
    }
    let line_prefix = raw[..before.col].to_string();

    if let Some(key) = before.key() {
        let after = &raw[before.col + key.len() + 1..];
        // Wait for the space after `:` so the value lands as `key: value`.
        if !after.starts_with(' ') {
            return None;
        }
        let query = after.trim_start();
        let completion = value_completion(key, &mapping_at(&lines, idx, before.col))?;
        return Some(YamlContext {
            start: cursor - query.len(),
            end: cursor,
            completion,
            query: query.to_string(),
            line_prefix,
        });
    }

    let completion = if before.dash {
        let owner = list_owner(&lines, idx, before.indent).and_then(|l| l.key());
        match owner {
            Some(k) if is_mapping_list(k) => key_completion(&mapping_at(&lines, idx, before.col)),
            Some("targets") => Completion::Entity {
                kind: EntityKind::Item,
                program: None,
            },
            _ => return None,
        }
    } else {
        key_completion(&mapping_at(&lines, idx, before.col))
    };
    Some(YamlContext {
        start: line_start + before.col,
        end: cursor,
        completion,
        query: raw[before.col..].to_string(),
        line_prefix,
    })
}

/// How Enter edits the text at a caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum EnterEdit {
    /// Insert this text (newline plus indent) at the caret.
    Insert(String),
    /// Empty `- ` item: replace `start..caret` with `indent` spaces to leave the list.
    EndList { start: usize, indent: usize },
}

/// Enter at byte offset `caret`: indent the new line to the YAML depth.
pub(super) fn enter_edit(text: &str, caret: usize) -> EnterEdit {
    let caret = caret.min(text.len());
    let line_start = text[..caret].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[caret..].find('\n').map_or(text.len(), |i| caret + i);
    let before = Line::parse(&text[line_start..caret]);
    let at_end = text[caret..line_end].trim().is_empty();
    let pad = |n: usize| " ".repeat(n);

    if before.dash && before.body.is_empty() && at_end {
        let lines: Vec<&str> = text.split('\n').collect();
        let idx = text[..line_start].matches('\n').count();
        let indent = list_owner(&lines, idx, before.indent).map_or(before.indent, |l| l.col);
        return EnterEdit::EndList {
            start: line_start,
            indent,
        };
    }
    EnterEdit::Insert(if at_end && before.opens_block() {
        if before.key().is_some_and(is_list_key) {
            format!("\n{}- ", pad(before.col))
        } else {
            format!("\n{}", pad(before.col + INDENT.len()))
        }
    } else if at_end && before.dash && before.key().is_none() {
        format!("\n{}- ", pad(before.indent))
    } else {
        format!("\n{}", pad(before.col))
    })
}

/// Indent (or dedent) every line touched by `start..end`; returns the shifted range.
pub(super) fn shift_lines(
    text: &mut String,
    start: usize,
    end: usize,
    dedent: bool,
) -> (usize, usize) {
    let first = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let mut starts = vec![first];
    starts.extend(
        text[start..end]
            .match_indices('\n')
            .map(|(i, _)| start + i + 1)
            .filter(|&p| p < end),
    );
    let (mut new_start, mut new_end) = (start, end);
    for &p in starts.iter().rev() {
        if dedent {
            let n = text[p..]
                .bytes()
                .take(INDENT.len())
                .take_while(|b| *b == b' ')
                .count();
            text.replace_range(p..p + n, "");
            if p <= start {
                new_start -= n.min(start - p);
            }
            new_end -= n.min(end - p);
        } else {
            text.insert_str(p, INDENT);
            if p <= start {
                new_start += INDENT.len();
            }
            new_end += INDENT.len();
        }
    }
    (new_start, new_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "\
name: demo
root:
  type: loop
  name: root
  subactions:
  - type: imagesearch
    name: find
    targets:
    - Shop~Potion
    - 
    searcharea: Shop~Board
    subactions:
    - type: click
      button: left
  - type: wait
    time: 5
variables:
- name: x
  type: 
";

    fn ctx_at(marker: &str) -> YamlContext {
        let at = DOC.find(marker).expect("marker") + marker.len();
        find_yaml_context(DOC, at).expect("context")
    }

    #[test]
    fn targets_suggest_items_only() {
        let at = DOC.find("    - \n").unwrap() + "    - ".len();
        let text = format!("{}Po{}", &DOC[..at], &DOC[at..]);
        let ctx = find_yaml_context(&text, at + 2).unwrap();
        assert_eq!(
            ctx.completion,
            Completion::Entity {
                kind: EntityKind::Item,
                program: None
            }
        );
        assert_eq!(ctx.query, "Po");
    }

    #[test]
    fn search_area_value_suggests_areas() {
        let ctx = ctx_at("searcharea: Sh");
        assert!(matches!(
            ctx.completion,
            Completion::Entity {
                kind: EntityKind::SearchArea,
                ..
            }
        ));
    }

    #[test]
    fn keys_scoped_to_enclosing_action() {
        let at = DOC.find("    searcharea").unwrap();
        let text = format!("{}    tol\n{}", &DOC[..at], &DOC[at..]);
        let ctx = find_yaml_context(&text, at + "    tol".len()).unwrap();
        let Completion::Key(keys) = ctx.completion else {
            panic!("expected keys");
        };
        assert!(keys.contains(&"tolerance"));
        assert!(
            !keys.contains(&"button"),
            "click field leaked into imagesearch"
        );
        assert!(!keys.contains(&"name"), "existing key suggested again");
        assert!(!keys.contains(&"targets"));
    }

    #[test]
    fn enum_comes_from_enclosing_action() {
        let ctx = ctx_at("button: le");
        assert!(matches!(
            ctx.completion,
            Completion::Enum { values, .. } if values.contains(&"left")
        ));
    }

    #[test]
    fn variable_type_is_not_action_type() {
        let at = DOC.rfind("  type: ").unwrap() + "  type: ".len();
        let ctx = find_yaml_context(DOC, at).unwrap();
        assert!(matches!(
            ctx.completion,
            Completion::Enum { values, .. } if values.contains(&"number")
        ));
    }

    #[test]
    fn action_type_value() {
        let ctx = ctx_at("  - type: wa");
        assert_eq!(ctx.completion, Completion::ActionType);
        assert_eq!(ctx.line_prefix, "  - ");
    }

    #[test]
    fn new_action_item_suggests_type_first() {
        let at = DOC.find("variables:").unwrap();
        let text = format!("{}  - ty\n{}", &DOC[..at], &DOC[at..]);
        let ctx = find_yaml_context(&text, at + "  - ty".len()).unwrap();
        let Completion::Key(keys) = ctx.completion else {
            panic!("expected keys");
        };
        assert_eq!(keys.first(), Some(&"type"));
    }

    #[test]
    fn blank_comment_and_colon_without_space_are_quiet() {
        assert!(find_yaml_context("root:\n  ", 8).is_none());
        assert!(find_yaml_context("# note", 6).is_none());
        assert!(find_yaml_context("root:\n  type:", 13).is_none());
    }

    #[test]
    fn enter_indents_to_yaml_depth() {
        let ins = |t: &str| match enter_edit(t, t.len()) {
            EnterEdit::Insert(s) => s,
            EnterEdit::EndList { .. } => panic!("unexpected end-list"),
        };
        assert_eq!(ins("root:"), "\n  ");
        assert_eq!(ins("root:\n  subactions:"), "\n  - ");
        assert_eq!(ins("  - type: wait"), "\n    ");
        assert_eq!(ins("    time: 5"), "\n    ");
        assert_eq!(ins("    targets:\n    - Shop~A"), "\n    - ");
    }

    #[test]
    fn enter_on_empty_item_leaves_list() {
        let t = "  subactions:\n  - ";
        assert_eq!(
            enter_edit(t, t.len()),
            EnterEdit::EndList {
                start: t.len() - 4,
                indent: 2
            }
        );
    }

    #[test]
    fn shift_lines_indents_and_dedents_selection() {
        let mut t = String::from("a: 1\nb: 2\nc: 3");
        let (s, e) = shift_lines(&mut t, 1, 7, false);
        assert_eq!(t, "  a: 1\n  b: 2\nc: 3");
        assert_eq!((s, e), (3, 11));
        let (s, e) = shift_lines(&mut t, s, e, true);
        assert_eq!(t, "a: 1\nb: 2\nc: 3");
        assert_eq!((s, e), (1, 7));
    }
}
