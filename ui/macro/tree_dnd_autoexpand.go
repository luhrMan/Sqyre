package macro

import (
	"time"

	"fyne.io/fyne/v2"
)

const branchOpenDebounceMs = 200

// Edge auto-scroll tuning.
const (
	autoScrollIntervalMs = 16
	autoScrollSpeed      = 8
)

func (mt *MacroTree) branchOpenCandidate(k int) string {
	if (mt.dropMode == dropIntoStart || mt.dropMode == dropIntoEnd) &&
		mt.IsBranch(mt.dropTargetUID) && !mt.IsBranchOpen(mt.dropTargetUID) &&
		mt.dropTargetUID != mt.dragSrcUID && !mt.isDescendantOf(mt.dropTargetUID, mt.dragSrcUID) {
		return mt.dropTargetUID
	}
	if k >= 0 && k < len(mt.dragVisible) {
		uid := mt.dragVisible[k]
		if mt.IsBranch(uid) && !mt.IsBranchOpen(uid) &&
			uid != mt.dragSrcUID && !mt.isDescendantOf(uid, mt.dragSrcUID) {
			return uid
		}
	}
	return ""
}

func (mt *MacroTree) updateBranchOpenDebounce(k int) {
	uid := mt.branchOpenCandidate(k)
	if uid == "" {
		mt.cancelAutoExpand()
		return
	}
	mt.cancelAutoExpand()
	mt.autoExpandUID = uid
	mt.autoExpandTimer = time.AfterFunc(branchOpenDebounceMs*time.Millisecond, func() {
		fyne.Do(func() {
			mt.doAutoExpand(uid)
		})
	})
}

func (mt *MacroTree) updateAutoScroll(pointerY float32) {
	viewH := mt.Size().Height
	if viewH <= 0 {
		mt.setAutoScroll(0)
		return
	}
	rowH, _ := mt.dragMetrics()
	margin := rowH
	top := mt.dragTreeTop
	switch {
	case pointerY < top+margin:
		mt.setAutoScroll(-1)
	case pointerY > top+viewH-margin:
		mt.setAutoScroll(1)
	default:
		mt.setAutoScroll(0)
	}
}

func (mt *MacroTree) setAutoScroll(dir int) {
	if mt.autoScrollDir == dir {
		return
	}
	mt.autoScrollDir = dir
	if dir == 0 {
		mt.stopAutoScroll()
		return
	}
	if mt.autoScrollStop != nil {
		return
	}
	stop := make(chan struct{})
	mt.autoScrollStop = stop
	go func() {
		ticker := time.NewTicker(autoScrollIntervalMs * time.Millisecond)
		defer ticker.Stop()
		for {
			select {
			case <-stop:
				return
			case <-ticker.C:
				fyne.Do(mt.autoScrollStep)
			}
		}
	}()
}

func (mt *MacroTree) stopAutoScroll() {
	if mt.autoScrollStop != nil {
		close(mt.autoScrollStop)
		mt.autoScrollStop = nil
	}
	mt.autoScrollDir = 0
}

func (mt *MacroTree) autoScrollStep() {
	if !mt.dragActive || mt.autoScrollDir == 0 {
		return
	}
	scroll, ok := treeScrollOffsetY(&mt.Tree)
	if !ok {
		return
	}
	maxOff := mt.openTreeContentHeight() - mt.Size().Height
	if maxOff < 0 {
		maxOff = 0
	}
	newOff := scroll + float32(mt.autoScrollDir)*autoScrollSpeed
	if newOff < 0 {
		newOff = 0
	}
	if newOff > maxOff {
		newOff = maxOff
	}
	if newOff == scroll {
		return
	}
	mt.ScrollToOffset(newOff)
	mt.resolveDropAt(mt.dragLastPointerY)
}

func (mt *MacroTree) doAutoExpand(uid string) {
	mt.autoExpandUID = ""
	mt.autoExpandTimer = nil
	if !mt.dragActive || !mt.IsBranch(uid) || mt.IsBranchOpen(uid) {
		return
	}
	mt.suppressBranchOpenScroll++
	mt.OpenBranch(uid)
	mt.suppressBranchOpenScroll--
	if !mt.wasOpenAtDragStart(uid) {
		if mt.dragAutoOpenedBranches == nil {
			mt.dragAutoOpenedBranches = map[string]struct{}{}
		}
		mt.dragAutoOpenedBranches[uid] = struct{}{}
	}
	mt.dragVisible = mt.visibleRowUIDs()
	mt.dropIndicatorKey = "" // branch open changes preview slot
	mt.updateDropIndicator()
	mt.resolveDropAt(mt.dragLastPointerY)
	mt.scheduleDragPreview()
}

func (mt *MacroTree) cancelAutoExpand() {
	if mt.autoExpandTimer != nil {
		mt.autoExpandTimer.Stop()
		mt.autoExpandTimer = nil
	}
	mt.autoExpandUID = ""
}

func (mt *MacroTree) initDragBranchState() {
	mt.dragStartOpenBranches = map[string]struct{}{}
	for _, uid := range mt.collectOpenBranchUIDs() {
		mt.dragStartOpenBranches[uid] = struct{}{}
	}
	mt.dragAutoOpenedBranches = nil
}

func (mt *MacroTree) finishDragBranchState() {
	mt.collapseDragAutoOpenedBranchesExcept(mt.dragBranchesToKeepAfterDrop())
	mt.dragStartOpenBranches = nil
	mt.dragAutoOpenedBranches = nil
}

func (mt *MacroTree) dragBranchesToKeepAfterDrop() map[string]struct{} {
	keep := map[string]struct{}{}
	if !mt.dropValid || mt.dragAutoOpenedBranches == nil {
		return keep
	}
	for uid := range mt.dragAutoOpenedBranches {
		if mt.dropMode == dropIntoStart || mt.dropMode == dropIntoEnd {
			if mt.dropTargetUID == uid {
				keep[uid] = struct{}{}
			}
		}
		if mt.dropParent != nil {
			pUID := mt.dropParent.GetUID()
			if pUID == uid || mt.isDescendantOf(pUID, uid) {
				keep[uid] = struct{}{}
			}
		}
	}
	return keep
}

func (mt *MacroTree) wasOpenAtDragStart(uid string) bool {
	if mt.dragStartOpenBranches == nil {
		return mt.IsBranchOpen(uid)
	}
	_, ok := mt.dragStartOpenBranches[uid]
	return ok
}

func (mt *MacroTree) dragBranchesToKeepOpen(k int) map[string]struct{} {
	keep := map[string]struct{}{}
	if mt.dragAutoOpenedBranches == nil {
		if cand := mt.branchOpenCandidate(k); cand != "" {
			keep[cand] = struct{}{}
		}
		if mt.autoExpandUID != "" {
			keep[mt.autoExpandUID] = struct{}{}
		}
		return keep
	}

	var rowUID string
	if k >= 0 && k < len(mt.dragVisible) {
		rowUID = mt.dragVisible[k]
	}

	for uid := range mt.dragAutoOpenedBranches {
		if rowUID != "" && (rowUID == uid || mt.isDescendantOf(rowUID, uid)) {
			keep[uid] = struct{}{}
		}
		if mt.dropParent != nil {
			pUID := mt.dropParent.GetUID()
			if pUID == uid || mt.isDescendantOf(pUID, uid) {
				keep[uid] = struct{}{}
			}
		}
		if mt.dropTargetUID != "" && (mt.dropTargetUID == uid || mt.isDescendantOf(mt.dropTargetUID, uid)) {
			keep[uid] = struct{}{}
		}
		if (mt.dropMode == dropIntoStart || mt.dropMode == dropIntoEnd) && mt.dropTargetUID == uid {
			keep[uid] = struct{}{}
		}
	}

	if cand := mt.branchOpenCandidate(k); cand != "" {
		keep[cand] = struct{}{}
	}
	if mt.autoExpandUID != "" {
		keep[mt.autoExpandUID] = struct{}{}
	}
	return keep
}

func (mt *MacroTree) syncDragAutoOpenedBranches(k int) bool {
	if mt.dragAutoOpenedBranches == nil {
		return false
	}
	return mt.collapseDragAutoOpenedBranchesExcept(mt.dragBranchesToKeepOpen(k))
}

func (mt *MacroTree) collapseDragAutoOpenedBranchesExcept(keep map[string]struct{}) bool {
	if mt.dragAutoOpenedBranches == nil {
		return false
	}
	changed := false
	for uid := range mt.dragAutoOpenedBranches {
		if keep != nil {
			if _, ok := keep[uid]; ok {
				continue
			}
		}
		if mt.IsBranchOpen(uid) {
			mt.suppressBranchOpenScroll++
			mt.CloseBranch(uid)
			mt.suppressBranchOpenScroll--
			changed = true
		}
		delete(mt.dragAutoOpenedBranches, uid)
	}
	if mt.autoExpandUID != "" {
		if keep == nil {
			mt.cancelAutoExpand()
		} else if _, ok := keep[mt.autoExpandUID]; !ok {
			mt.cancelAutoExpand()
		}
	}
	return changed
}
