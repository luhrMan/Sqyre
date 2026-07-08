package macro

import (
	"time"

	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
)

func (mt *MacroTree) previewVisibleRowUIDs() []string {
	vis := mt.dragVisible
	if len(vis) == 0 {
		vis = mt.visibleRowUIDs()
	}
	src := mt.dragSrcUID
	filtered := make([]string, 0, len(vis))
	for _, uid := range vis {
		if uid != src {
			filtered = append(filtered, uid)
		}
	}
	insertAt := mt.previewInsertIndex(filtered)
	out := make([]string, 0, len(filtered)+1)
	out = append(out, filtered[:insertAt]...)
	out = append(out, src)
	out = append(out, filtered[insertAt:]...)
	return out
}

func (mt *MacroTree) previewInsertIndex(vis []string) int {
	switch mt.dropMode {
	case dropBefore:
		if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
			return i
		}
	case dropAfter:
		if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
			if mt.IsBranch(mt.dropTargetUID) && mt.IsBranchOpen(mt.dropTargetUID) {
				return mt.lastVisibleDescendantIndexInList(vis, mt.dropTargetUID) + 1
			}
			return i + 1
		}
	case dropIntoStart, dropIntoEnd:
		if mt.IsBranch(mt.dropTargetUID) && !mt.IsBranchOpen(mt.dropTargetUID) {
			if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
				return i
			}
		}
		if mt.dropMode == dropIntoStart {
			if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
				return i + 1
			}
		}
		if i := indexOfString(vis, mt.dropTargetUID); i >= 0 {
			return mt.lastVisibleDescendantIndexInList(vis, mt.dropTargetUID) + 1
		}
	}
	return len(vis)
}

func (mt *MacroTree) lastVisibleDescendantIndexInList(vis []string, branchUID string) int {
	idx := indexOfString(vis, branchUID)
	if idx < 0 {
		return idx
	}
	last := idx
	for i := idx + 1; i < len(vis); i++ {
		if mt.isDescendantOf(vis[i], branchUID) {
			last = i
			continue
		}
		break
	}
	return last
}

func (mt *MacroTree) cancelDragPreviewDebounce() {
	if mt.dragPreviewTimer != nil {
		mt.dragPreviewTimer.Stop()
		mt.dragPreviewTimer = nil
	}
}

func (mt *MacroTree) scheduleDragPreview() {
	if !mt.dragActive {
		return
	}
	if mt.dragPreviewInTree && mt.dropValid && mt.dragPreviewKey == mt.dropFingerprint() {
		return
	}
	mt.cancelDragPreviewDebounce()
	if !mt.dropValid {
		if mt.dragPreviewInTree {
			mt.revertDragPreview()
		}
		return
	}
	key := mt.dropFingerprint()
	if mt.dragPreviewInTree && mt.dragPreviewKey != key {
		mt.revertDragPreview()
	}
	scheduled := key
	debounce := mt.dragPreviewDebounceDuration()
	mt.dragPreviewTimer = time.AfterFunc(debounce, func() {
		fyne.Do(func() {
			mt.applyDragPreview(scheduled)
		})
	})
}

func (mt *MacroTree) dragPreviewDebounceDuration() time.Duration {
	app := fyne.CurrentApp()
	if app == nil {
		return time.Duration(config.DefaultDragPreviewDebounceMs) * time.Millisecond
	}
	ms := app.Preferences().IntWithFallback(config.PrefDragPreviewDebounceMs, config.DefaultDragPreviewDebounceMs)
	if ms < config.MinDragPreviewDebounceMs {
		ms = config.DefaultDragPreviewDebounceMs
	}
	return time.Duration(ms) * time.Millisecond
}

func (mt *MacroTree) applyDragPreview(key string) {
	mt.dragPreviewTimer = nil
	if !mt.dragActive || !mt.dropValid || mt.dropFingerprint() != key {
		return
	}
	if mt.dragPreviewInTree && mt.dragPreviewKey == key {
		return
	}
	prevParentUID := mt.draggedNodeParentUID()
	if !mt.relocateDraggedNode(false) {
		return
	}
	newParentUID := mt.dropParentUID()
	mt.dragPreviewInTree = true
	mt.dragPreviewKey = key
	mt.refreshAfterDragLayout(mt.dragMutationNeedsFlush(prevParentUID, newParentUID))
	if mt.dragSrcUID != "" {
		mt.withPreservedScroll(func() {
			mt.invalidateRowCache(mt.dragSrcUID)
			mt.RefreshItem(mt.dragSrcUID)
		})
	}
	mt.updateDropIndicator()
}

func (mt *MacroTree) restoreDragOrigin() bool {
	node := mt.Macro.Root.GetAction(mt.dragSrcUID)
	if node == nil || mt.dragOrigin.parent == nil {
		return false
	}
	if cur := node.GetParent(); cur != nil {
		cur.RemoveSubAction(node)
	}
	subs := mt.dragOrigin.parent.GetSubActions()
	idx := min(max(mt.dragOrigin.index, 0), len(subs))
	newSubs := make([]actions.ActionInterface, 0, len(subs)+1)
	newSubs = append(newSubs, subs[:idx]...)
	newSubs = append(newSubs, node)
	newSubs = append(newSubs, subs[idx:]...)
	mt.dragOrigin.parent.SetSubActions(newSubs)
	node.SetParent(mt.dragOrigin.parent)
	return true
}

func (mt *MacroTree) revertDragPreview() {
	if !mt.dragPreviewInTree {
		return
	}
	prevParentUID := mt.draggedNodeParentUID()
	originParentUID := mt.dragOriginParentUID()
	mt.restoreDragOrigin()
	mt.dragPreviewInTree = false
	mt.dragPreviewKey = ""
	mt.refreshAfterDragLayout(mt.dragMutationNeedsFlush(prevParentUID, originParentUID))
	if mt.dragSrcUID != "" {
		mt.withPreservedScroll(func() {
			mt.invalidateRowCache(mt.dragSrcUID)
			mt.RefreshItem(mt.dragSrcUID)
		})
	}
}

func (mt *MacroTree) captureDragOrigin(node actions.ActionInterface) {
	parent := node.GetParent()
	if parent == nil {
		return
	}
	mt.dragOrigin = dragOrigin{
		parent: parent,
		index:  indexOfAction(parent.GetSubActions(), node.GetUID()),
	}
}

func (mt *MacroTree) draggedNodeParentUID() string {
	node := mt.Macro.Root.GetAction(mt.dragSrcUID)
	if node == nil {
		return ""
	}
	if p := node.GetParent(); p != nil {
		return p.GetUID()
	}
	return ""
}

func (mt *MacroTree) dragOriginParentUID() string {
	if mt.dragOrigin.parent == nil {
		return ""
	}
	return mt.dragOrigin.parent.GetUID()
}

func (mt *MacroTree) dropParentUID() string {
	if mt.dropParent == nil {
		return ""
	}
	return mt.dropParent.GetUID()
}

// dragMutationNeedsFlush reports whether the Fyne tree must rebuild row depth
// after a drag mutation. Refresh alone is enough when only sibling order changes
// under the same parent at the same indent level.
func (mt *MacroTree) dragMutationNeedsFlush(prevParentUID, newParentUID string) bool {
	if prevParentUID != newParentUID {
		return true
	}
	originUID := mt.dragOriginParentUID()
	if originUID != "" && originUID != newParentUID {
		return true
	}
	return mt.childIndentDepthForParentUID(prevParentUID) != mt.childIndentDepthForParentUID(newParentUID)
}

func (mt *MacroTree) childIndentDepthForParentUID(parentUID string) int {
	if mt.Macro == nil || mt.Macro.Root == nil || parentUID == "" {
		return 0
	}
	if parentUID == mt.Macro.Root.GetUID() {
		return 0
	}
	return mt.rowIndentDepth(parentUID) + 1
}

func (mt *MacroTree) childIndentDepth(parent actions.AdvancedActionInterface) int {
	if parent == nil || mt.Macro == nil || mt.Macro.Root == nil {
		return 0
	}
	return mt.childIndentDepthForParentUID(parent.GetUID())
}

func (mt *MacroTree) insertIndentDepth() int {
	if mt.dropParent == nil {
		return 0
	}
	switch mt.dropMode {
	case dropIntoStart, dropIntoEnd:
		if mt.IsBranch(mt.dropTargetUID) && !mt.IsBranchOpen(mt.dropTargetUID) {
			return mt.rowIndentDepth(mt.dropTargetUID) + 1
		}
		return mt.childIndentDepth(mt.dropParent)
	default:
		return mt.childIndentDepth(mt.dropParent)
	}
}

func (mt *MacroTree) dropInsertIndex(subs []actions.ActionInterface) int {
	switch mt.dropMode {
	case dropIntoStart:
		return 0
	case dropIntoEnd:
		return len(subs)
	case dropBefore:
		if i := indexOfAction(subs, mt.dropTargetUID); i >= 0 {
			return i
		}
	case dropAfter:
		if i := indexOfAction(subs, mt.dropTargetUID); i >= 0 {
			return i + 1
		}
	}
	return len(subs)
}

func (mt *MacroTree) dragLayoutNeedsFlush(prevParentUID, newParentUID string) bool {
	return mt.dragMutationNeedsFlush(prevParentUID, newParentUID)
}

func (mt *MacroTree) refreshAfterDragLayout(flushDepth bool) {
	mt.withPreservedScroll(func() {
		mt.suppressBranchOpenScroll++
		if flushDepth {
			mt.flushNodeCache()
		} else {
			mt.Refresh()
		}
		mt.suppressBranchOpenScroll--
	})
	mt.dragVisible = mt.visibleRowUIDs()
}
