//! Macro-tree editing UX: selection, undo/redo, drag gestures, clipboard.

use crate::action_tooltip::TooltipState;
use crate::tree_history::TreeHistory;
use eframe::egui;
use sqyre_domain::ActionId;
use std::collections::{HashMap, HashSet};

/// Active pointer gesture on the macro tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeDragMode {
    #[default]
    Idle,
    Reorder,
    Scroll,
}

pub(crate) struct TreeState {
    /// Currently selected action ids in the macro tree (egui tree node ids).
    /// Empty = none; order matches TreeView (last is the primary for insert/paste).
    pub(crate) selected_actions: Vec<ActionId>,
    /// Per-macro undo/redo stacks keyed by macro name.
    pub(crate) histories: HashMap<String, TreeHistory>,
    /// Process-local action clipboard (YAML map without UIDs).
    pub(crate) clipboard: Option<serde_yaml::Mapping>,
    /// Branches that were collapsed before execution expand.
    pub(crate) pre_exec_closed: HashSet<ActionId>,
    /// True while branches are force-opened for the active run.
    pub(crate) exec_fully_expanded: bool,
    /// Last action scrolled into view for execution highlight follow.
    pub(crate) last_exec_follow: Option<ActionId>,
    /// Prior-frame icon/pill rects; used to decide reorder vs drag-scroll.
    pub(crate) drag_handles: Vec<egui::Rect>,
    /// Active pointer gesture on the macro tree (idle / reorder / drag-scroll).
    pub(crate) drag_mode: TreeDragMode,
    /// Vertical coast velocity after a drag-scroll release (points/sec).
    pub(crate) scroll_vel: f32,
    pub(crate) tooltip: TooltipState,
}

impl Default for TreeState {
    fn default() -> Self {
        Self {
            selected_actions: Vec::new(),
            histories: HashMap::new(),
            clipboard: None,
            pre_exec_closed: HashSet::new(),
            exec_fully_expanded: false,
            last_exec_follow: None,
            drag_handles: Vec::new(),
            drag_mode: TreeDragMode::Idle,
            scroll_vel: 0.0,
            tooltip: TooltipState::Hidden,
        }
    }
}
