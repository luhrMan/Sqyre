//! Action type icon glyphs for the macro tree.

use sqyre_domain::{action_icon, Action, ActionKind, LoopJumpMode, PressState};

pub fn action_icon_glyph(action: &Action) -> &'static str {
    match &action.kind {
        ActionKind::Click { state, .. } | ActionKind::Key { state, .. } => match state {
            PressState::Down => "⬇",
            PressState::Up => "⬆",
            PressState::Tap => "↕",
        },
        ActionKind::LoopJump { mode } => match mode {
            LoopJumpMode::Break => "⏹",
            LoopJumpMode::Continue => "⏭",
        },
        _ => action_icon(action.type_key()),
    }
}

/// True when the whole (trimmed) pill value is a single `${name}` / `{name}`
/// var ref, using the real `sqyre_varref` expansion grammar (identifier-like
/// names only, escapes like `$${name}` excluded) rather than brace shape alone.
pub fn looks_like_var_ref(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    match sqyre_varref::segments(t).as_slice() {
        [seg] => seg.is_ref && seg.text.len() == t.len(),
        _ => false,
    }
}
