//! Action type icon glyphs for the macro tree.

use sqyre_domain::{action_icon, Action, ActionKind};

pub fn action_icon_glyph(action: &Action) -> &'static str {
    match &action.kind {
        ActionKind::Click { state, .. } | ActionKind::Key { state, .. } => {
            if *state {
                "⬇"
            } else {
                "⬆"
            }
        }
        _ => action_icon(action.type_key()),
    }
}

/// True when the pill value looks like a `${name}` / `{name}` var ref.
pub fn looks_like_var_ref(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("${") && t.ends_with('}'))
        || (t.starts_with('{') && t.ends_with('}') && !t.starts_with("${"))
}
