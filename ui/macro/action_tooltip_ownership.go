package macro

// Action tooltips use process-global ownership so that only one view tooltip and
// one edit (pinned) tooltip are live at a time across every tree row.
var (
	activeActionViewTooltip *actionDisplayTooltipHover
	activeActionEditTooltip *actionDisplayTooltipHover
)

// ResetActionTooltipOwnershipForTesting clears global action tooltip ownership (tests only).
func ResetActionTooltipOwnershipForTesting() {
	activeActionViewTooltip = nil
	activeActionEditTooltip = nil
}

func actionTooltipEditPinnedByOther(h *actionDisplayTooltipHover) bool {
	return activeActionEditTooltip != nil && activeActionEditTooltip != h
}

func dismissOtherActionViewTooltip(h *actionDisplayTooltipHover) {
	if activeActionViewTooltip == nil || activeActionViewTooltip == h {
		return
	}
	prev := activeActionViewTooltip
	activeActionViewTooltip = nil
	prev.hideViewTooltip()
}

func claimActionViewTooltip(h *actionDisplayTooltipHover) {
	dismissOtherActionViewTooltip(h)
	activeActionViewTooltip = h
}

func releaseActionViewTooltip(h *actionDisplayTooltipHover) {
	if activeActionViewTooltip == h {
		activeActionViewTooltip = nil
	}
}

func claimActionEditTooltip(h *actionDisplayTooltipHover) {
	dismissOtherActionViewTooltip(h)
	activeActionEditTooltip = h
	releaseActionViewTooltip(h)
}

func releaseActionEditTooltip(h *actionDisplayTooltipHover) {
	if activeActionEditTooltip == h {
		activeActionEditTooltip = nil
	}
}

func dismissActiveActionTooltips() {
	if activeActionViewTooltip != nil {
		h := activeActionViewTooltip
		activeActionViewTooltip = nil
		h.hideViewTooltip()
	}
	if activeActionEditTooltip != nil {
		h := activeActionEditTooltip
		activeActionEditTooltip = nil
		h.hideTooltip()
	}
}

func (h *actionDisplayTooltipHover) actionTooltipsSuppressed() bool {
	return h.rowBody != nil && h.rowBody.tree != nil && h.rowBody.tree.dragActive
}
