package macro

// beginExecutionExpand opens every branch for the duration of a macro run so
// cursor highlights never pay per-step OpenBranch / CloseBranch churn.
func (mt *MacroTree) beginExecutionExpand() {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return
	}
	mt.preExecClosedBranches = map[string]struct{}{}
	for _, uid := range mt.collectBranchUIDs() {
		if !mt.IsBranchOpen(uid) {
			mt.preExecClosedBranches[uid] = struct{}{}
		}
	}
	mt.stopCollapseDebounce()
	mt.execOpenedBranches = nil
	mt.suppressBranchOpenScroll++
	mt.Tree.OpenAllBranches()
	mt.suppressBranchOpenScroll--
	mt.execFullyExpanded = true
	mt.Refresh()
}

// endExecutionExpand restores branches that were collapsed before execution.
func (mt *MacroTree) endExecutionExpand() {
	if !mt.execFullyExpanded {
		return
	}
	mt.suppressBranchOpenScroll++
	for uid := range mt.preExecClosedBranches {
		if mt.IsBranchOpen(uid) {
			mt.Tree.CloseBranch(uid)
		}
	}
	mt.suppressBranchOpenScroll--
	mt.preExecClosedBranches = nil
	mt.execFullyExpanded = false
	mt.execOpenedBranches = nil
	mt.scheduleClampScroll()
}

func (mt *MacroTree) collectBranchUIDs() []string {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return nil
	}
	var out []string
	var walk func(uid string)
	walk = func(uid string) {
		for _, child := range mt.ChildUIDs(uid) {
			if mt.IsBranch(child) {
				out = append(out, child)
				walk(child)
			}
		}
	}
	walk(mt.Macro.Root.GetUID())
	return out
}
