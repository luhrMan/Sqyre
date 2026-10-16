//! Event-level pre-parse guard for untrusted YAML.
//!
//! `serde_yaml` 0.9 fully expands anchors/aliases while materializing `Value`,
//! so a tiny document can amplify into gigabytes ("billion laughs"). Deeply
//! nested input can also overflow the stack during the recursive `Value`
//! build. Scanning the raw libyaml event stream first rejects dangerous input
//! before any node is materialized:
//!
//! - alias events (`*x`) are rejected outright — Sqyre never emits them
//! - mapping/sequence nesting is capped at [`crate::MAX_YAML_NESTING_DEPTH`]
//! - total events are bounded by input size; without aliases the count cannot
//!   exceed the ratio enforced here, so legitimate documents are unaffected

use crate::{Result, SerializeError, MAX_YAML_NESTING_DEPTH};
use libyaml_safer::{EventData, Parser};

/// Scan `yaml` at the libyaml event level. `Err` means the text must not be
/// handed to `serde_yaml::from_str` / `from_value`.
pub fn check_yaml_safety(yaml: &str) -> Result<()> {
    let mut parser = Parser::new();
    parser.set_input(yaml.as_bytes());
    // Structural events (stream/doc/collection markers) inflate the event
    // count by a small factor over source bytes — worst case ~2.5× for tiny
    // documents — so 4× cannot trip on alias-free input.
    let max_events = yaml.len().saturating_mul(4).saturating_add(64);
    let mut events = 0usize;
    let mut depth = 0usize;
    loop {
        let event = parser
            .parse()
            .map_err(|e| SerializeError::msg(format!("yaml parse error: {e}")))?;
        events += 1;
        if events > max_events {
            return Err(SerializeError::msg(
                "yaml event budget exceeded (possible alias expansion)",
            ));
        }
        match event.data {
            EventData::Alias { .. } => {
                return Err(SerializeError::msg(
                    "yaml aliases are not allowed in imported documents",
                ));
            }
            EventData::SequenceStart { .. } | EventData::MappingStart { .. } => {
                depth += 1;
                if depth > MAX_YAML_NESTING_DEPTH {
                    return Err(SerializeError::msg(format!(
                        "yaml nesting too deep (max {MAX_YAML_NESTING_DEPTH})"
                    )));
                }
            }
            EventData::SequenceEnd | EventData::MappingEnd => {
                depth = depth.saturating_sub(1);
            }
            EventData::StreamEnd => return Ok(()),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_alias_events() {
        let yaml = "a: &x [1, 2, 3]\nb: *x\n";
        let err = check_yaml_safety(yaml).unwrap_err();
        assert!(err.to_string().contains("aliases"), "{err}");
    }

    #[test]
    fn rejects_billion_laughs_bomb() {
        // Classic expansion bomb: each level aliases the previous list twice.
        let mut yaml = String::from("a: &a [\"lol\"]\n");
        let mut prev = "a";
        for name in ["b", "c", "d", "e", "f", "g", "h"] {
            yaml.push_str(&format!("{name}: &{name} [*{prev}, *{prev}]\n"));
            prev = name;
        }
        let err = check_yaml_safety(&yaml).unwrap_err();
        assert!(err.to_string().contains("aliases"), "{err}");
    }

    #[test]
    fn accepts_alias_free_documents() {
        check_yaml_safety("name: demo\nroot:\n  type: loop\n  count: 1\n").unwrap();
        check_yaml_safety("macros: {}\nprograms: {}\n").unwrap();
        // `&`/`*` inside quoted scalars are text, not YAML indicators.
        check_yaml_safety("name: \"a & b * c !d\"\n").unwrap();
    }

    #[test]
    fn rejects_deep_nesting_before_materialization() {
        let yaml = format!(
            "{}1{}",
            "[".repeat(MAX_YAML_NESTING_DEPTH + 20),
            "]".repeat(MAX_YAML_NESTING_DEPTH + 20)
        );
        let err = check_yaml_safety(&yaml).unwrap_err();
        assert!(err.to_string().contains("nesting too deep"), "{err}");
    }

    #[test]
    fn reports_parse_errors() {
        let err = check_yaml_safety("a: [unterminated").unwrap_err();
        assert!(err.to_string().contains("yaml parse error"), "{err}");
    }
}
