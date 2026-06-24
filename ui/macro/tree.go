package macro

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/serialize"
	"Sqyre/internal/uiutil"
	"image/color"
	"math"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// OnOpenActionDialog is called when the user taps an action's icon to edit it.
// If non-nil, the tree will open the action dialog from this callback.
type OnOpenActionDialogFunc func(action actions.ActionInterface)

type MacroTree struct {
	widget.Tree
	Macro              *models.Macro
	SelectedNode       string
	OnOpenActionDialog OnOpenActionDialogFunc

	// cursorUID is the single action currently executing (moving highlight).
	cursorUID string
	// fills maps an action UID to its horizontal fill fraction (0..1) for
	// container actions (Image Search, Run Macro, For Each) that span steps.
	fills map[string]float64
	// highlightOnlyRefresh marks tree rows where only the overlay changed so
	// UpdateNode can skip rebuilding icons and display widgets.
	highlightOnlyRefresh map[string]struct{}
	// nodeBoundUID records which action each tree row object last displayed.
	// Highlight-only refresh must not skip content rebuild when Fyne recycles
	// a row object for a different uid or before the row was ever populated.
	nodeBoundUID map[fyne.CanvasObject]string
	// lastScrollUID avoids repeated ScrollTo for the same action during execution.
	lastScrollUID string
	// collapseDebounce batches branch collapse while the cursor moves quickly.
	collapseDebounce *time.Timer
	// execOpenedBranches tracks branches expanded during execution so collapse
	// can close them without walking the entire tree.
	execOpenedBranches map[string]struct{}
}

const collapseDebounceMs = 150

// Highlight colors for the active-action execution overlay.
var (
	highlightSimpleColor = color.NRGBA{R: 90, G: 160, B: 240, A: 70}
	highlightFillColor   = color.NRGBA{R: 90, G: 200, B: 130, A: 90}
)

// fillLayout draws a full-width "simple" highlight rectangle (objects[0]) and a
// fractional-width "fill" rectangle (objects[1]) that grows left-to-right.
type fillLayout struct {
	fraction float64
}

func (l *fillLayout) MinSize(objects []fyne.CanvasObject) fyne.Size {
	return fyne.NewSize(0, 0)
}

func (l *fillLayout) Layout(objects []fyne.CanvasObject, size fyne.Size) {
	if len(objects) < 2 {
		return
	}
	simple := objects[0]
	fill := objects[1]
	simple.Resize(size)
	simple.Move(fyne.NewPos(0, 0))
	w := size.Width * float32(l.fraction)
	if w < 0 {
		w = 0
	} else if w > size.Width {
		w = size.Width
	}
	fill.Resize(fyne.NewSize(w, size.Height))
	fill.Move(fyne.NewPos(0, 0))
}

func macroTreeActionColor(action actions.ActionInterface) color.Color {
	return actions.ActionPastelColor(action.GetType())
}

func NewMacroTree(m *models.Macro) *MacroTree {
	t := &MacroTree{fills: map[string]float64{}}
	t.ExtendBaseWidget(t)
	t.Macro = m
	t.setTree()

	return t
}

// SetCursor moves the single "currently executing" highlight to uid (or clears
// it when uid is empty). Must be called on the Fyne UI thread.
func (mt *MacroTree) SetCursor(uid string) {
	old := mt.cursorUID
	if old == uid {
		return
	}
	mt.cursorUID = uid
	if old != "" {
		mt.markHighlightRefresh(old)
		mt.RefreshItem(old)
	}
	if uid != "" {
		mt.openAncestorBranches(uid)
		mt.markHighlightRefresh(uid)
		mt.RefreshItem(uid)
		targetUID := uid
		fyne.Do(func() {
			if mt.cursorUID == targetUID && mt.lastScrollUID != targetUID {
				mt.ScrollTo(targetUID)
				mt.lastScrollUID = targetUID
			}
		})
	} else {
		mt.lastScrollUID = ""
	}
	// Always debounce-collapse: ancestor paths can match while a previously
	// opened child branch (e.g. LoopInner) should close when the cursor moves
	// to a sibling under the same parent.
	mt.scheduleCollapseStale()
}

// SetFill sets the horizontal fill fraction (0..1) for a container action and
// reveals it the first time. Must be called on the Fyne UI thread.
func (mt *MacroTree) SetFill(uid string, fraction float64) {
	if uid == "" {
		return
	}
	if mt.fills == nil {
		mt.fills = map[string]float64{}
	}
	prev, existed := mt.fills[uid]
	if existed && fillNearlyEqual(prev, fraction) {
		return
	}
	mt.fills[uid] = fraction
	if !existed {
		mt.openAncestorBranches(uid)
		mt.scheduleCollapseStale()
	}
	mt.markHighlightRefresh(uid)
	mt.RefreshItem(uid)
}

// ClearHighlight removes any highlight (fill or cursor) on a single action.
func (mt *MacroTree) ClearHighlight(uid string) {
	changed := false
	if _, ok := mt.fills[uid]; ok {
		delete(mt.fills, uid)
		changed = true
	}
	if mt.cursorUID == uid {
		mt.cursorUID = ""
		changed = true
	}
	if changed {
		mt.markHighlightRefresh(uid)
		mt.RefreshItem(uid)
	}
	mt.stopCollapseDebounce()
	mt.collapseStaleBranches()
}

// ClearAllHighlights removes every execution highlight from the tree.
func (mt *MacroTree) ClearAllHighlights() {
	affected := make([]string, 0, len(mt.fills)+1)
	for k := range mt.fills {
		affected = append(affected, k)
	}
	if mt.cursorUID != "" {
		affected = append(affected, mt.cursorUID)
	}
	mt.fills = map[string]float64{}
	mt.cursorUID = ""
	mt.lastScrollUID = ""
	mt.execOpenedBranches = nil
	for _, k := range affected {
		mt.markHighlightRefresh(k)
		mt.RefreshItem(k)
	}
	mt.stopCollapseDebounce()
	mt.collapseStaleBranches()
}

// openAncestorBranches expands parent branches so uid is visible in the tree.
func (mt *MacroTree) openAncestorBranches(uid string) {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return
	}
	node := mt.Macro.Root.GetAction(uid)
	if node == nil {
		return
	}
	rootUID := mt.Macro.Root.GetUID()
	var ancestors []string
	for p := node.GetParent(); p != nil && p.GetUID() != rootUID; p = p.GetParent() {
		ancestors = append(ancestors, p.GetUID())
	}
	for i := len(ancestors) - 1; i >= 0; i-- {
		a := ancestors[i]
		if !mt.IsBranchOpen(a) {
			mt.OpenBranch(a)
			mt.trackExecOpened(a)
		}
	}
}

// revealNode expands ancestor branches and scrolls so uid is visible.
func (mt *MacroTree) revealNode(uid string) {
	mt.openAncestorBranches(uid)
	if mt.lastScrollUID != uid {
		mt.ScrollTo(uid)
		mt.lastScrollUID = uid
	}
}

// ancestorUIDs returns parent branch UIDs from the macro root down to uid's parent.
func (mt *MacroTree) ancestorUIDs(uid string) []string {
	if mt.Macro == nil || mt.Macro.Root == nil || uid == "" {
		return nil
	}
	node := mt.Macro.Root.GetAction(uid)
	if node == nil {
		return nil
	}
	rootUID := mt.Macro.Root.GetUID()
	var ancestors []string
	for p := node.GetParent(); p != nil && p.GetUID() != rootUID; p = p.GetParent() {
		ancestors = append(ancestors, p.GetUID())
	}
	return ancestors
}

func fillNearlyEqual(a, b float64) bool {
	return math.Abs(a-b) < 0.001
}

func (mt *MacroTree) trackExecOpened(uid string) {
	if uid == "" {
		return
	}
	if mt.execOpenedBranches == nil {
		mt.execOpenedBranches = map[string]struct{}{}
	}
	mt.execOpenedBranches[uid] = struct{}{}
}

func (mt *MacroTree) markHighlightRefresh(uid string) {
	if uid == "" {
		return
	}
	if mt.highlightOnlyRefresh == nil {
		mt.highlightOnlyRefresh = map[string]struct{}{}
	}
	mt.highlightOnlyRefresh[uid] = struct{}{}
}

func (mt *MacroTree) consumeHighlightRefresh(uid string) bool {
	if mt.highlightOnlyRefresh == nil {
		return false
	}
	_, ok := mt.highlightOnlyRefresh[uid]
	if ok {
		delete(mt.highlightOnlyRefresh, uid)
	}
	return ok
}

func (mt *MacroTree) markNodeBound(obj fyne.CanvasObject, uid string) {
	if mt.nodeBoundUID == nil {
		mt.nodeBoundUID = map[fyne.CanvasObject]string{}
	}
	mt.nodeBoundUID[obj] = uid
}

func (mt *MacroTree) nodeObjectShowsUID(obj fyne.CanvasObject, uid string) bool {
	if mt.nodeBoundUID == nil {
		return false
	}
	return mt.nodeBoundUID[obj] == uid
}

func (mt *MacroTree) applyHighlightOverlay(uid string, hlBg *fyne.Container) {
	fl := hlBg.Layout.(*fillLayout)
	hlSimple := hlBg.Objects[0].(*canvas.Rectangle)
	hlFill := hlBg.Objects[1].(*canvas.Rectangle)
	if frac, ok := mt.fills[uid]; ok {
		fl.fraction = frac
		hlFill.Show()
		hlSimple.Hide()
	} else if uid == mt.cursorUID {
		fl.fraction = 0
		hlSimple.Show()
		hlFill.Hide()
	} else {
		fl.fraction = 0
		hlSimple.Hide()
		hlFill.Hide()
	}
	hlBg.Refresh()
}

func (mt *MacroTree) scheduleCollapseStale() {
	if mt.collapseDebounce != nil {
		mt.collapseDebounce.Stop()
	}
	mt.collapseDebounce = time.AfterFunc(collapseDebounceMs*time.Millisecond, func() {
		fyne.Do(func() {
			mt.collapseDebounce = nil
			mt.collapseStaleBranches()
		})
	})
}

func (mt *MacroTree) stopCollapseDebounce() {
	if mt.collapseDebounce != nil {
		mt.collapseDebounce.Stop()
		mt.collapseDebounce = nil
	}
}

// branchesToKeepOpen returns branch UIDs that must stay expanded for the current
// cursor position and any in-progress container fill highlights.
func (mt *MacroTree) branchesToKeepOpen() map[string]bool {
	keep := map[string]bool{}
	addAncestors := func(uid string) {
		for _, a := range mt.ancestorUIDs(uid) {
			keep[a] = true
		}
	}
	if mt.cursorUID != "" {
		addAncestors(mt.cursorUID)
		if mt.IsBranch(mt.cursorUID) {
			keep[mt.cursorUID] = true
		}
	}
	for fillUID := range mt.fills {
		addAncestors(fillUID)
		if mt.IsBranch(fillUID) {
			keep[fillUID] = true
		}
	}
	return keep
}

// collapseStaleBranches closes branches opened during execution that no longer
// contain the active highlight.
func (mt *MacroTree) collapseStaleBranches() {
	if mt.execOpenedBranches == nil {
		return
	}
	keep := mt.branchesToKeepOpen()
	for uid := range mt.execOpenedBranches {
		if keep[uid] {
			continue
		}
		if mt.IsBranchOpen(uid) {
			mt.CloseBranch(uid)
		}
		mt.untrackExecOpenedBranch(uid)
	}
}

func (mt *MacroTree) untrackExecOpenedBranch(uid string) {
	if mt.execOpenedBranches == nil {
		return
	}
	for openUID := range mt.execOpenedBranches {
		if openUID == uid || mt.isDescendantOf(openUID, uid) {
			delete(mt.execOpenedBranches, openUID)
		}
	}
}

func (mt *MacroTree) isDescendantOf(childUID, ancestorUID string) bool {
	if mt.Macro == nil || mt.Macro.Root == nil || childUID == "" || ancestorUID == "" {
		return false
	}
	node := mt.Macro.Root.GetAction(childUID)
	if node == nil {
		return false
	}
	for p := node.GetParent(); p != nil; p = p.GetParent() {
		if p.GetUID() == ancestorUID {
			return true
		}
	}
	return false
}

func (mt *MacroTree) moveNode(selectedUID string, up bool) {
	node := mt.Macro.Root.GetAction(selectedUID)
	if node == nil || node.GetParent() == nil {
		return
	}

	parent := node.GetParent()
	psa := parent.GetSubActions()
	index := -1
	for i, child := range psa {
		if child == node {
			index = i
			break
		}
	}

	if up && index > 0 {
		psa[index-1], psa[index] = psa[index], psa[index-1]
		mt.Select(psa[index-1].GetUID())
	} else if !up && index < len(psa)-1 {
		psa[index], psa[index+1] = psa[index+1], psa[index]
		mt.Select(psa[index+1].GetUID())
	}
	mt.Refresh()
}

func (mt *MacroTree) setTree() {
	mt.ChildUIDs = func(uid string) []string {
		if aa, ok := mt.Macro.Root.GetAction(uid).(actions.AdvancedActionInterface); ok {
			sa := aa.GetSubActions()
			childIDs := make([]string, len(sa))
			for i, child := range sa {
				childIDs[i] = child.GetUID()
			}
			return childIDs
		}

		return []string{}
	}
	mt.IsBranch = func(uid string) bool {
		node := mt.Macro.Root.GetAction(uid)
		_, ok := node.(actions.AdvancedActionInterface)
		return ok
	}
	const treeItemIconSize = 24
	mt.CreateNode = func(branch bool) fyne.CanvasObject {
		actionIconBtn := ttwidget.NewButtonWithIcon("", theme.ErrorIcon(), nil)
		actionIconBtn.Importance = widget.LowImportance
		iconBg := canvas.NewRectangle(actions.ActionPastelColor(""))
		iconBg.CornerRadius = 6
		iconBg.StrokeColor = theme.ShadowColor()
		iconBg.StrokeWidth = 1
		iconStack := container.NewStack(iconBg, actionIconBtn)
		leftSide := container.NewHBox(
			iconStack,
		)
		displayContainer := container.New(kxlayout.NewRowWrapLayout())
		itemIconsBox := container.NewHBox()
		displayHolder := container.NewCenter(displayContainer)
		itemIconsHolder := container.NewCenter(itemIconsBox)
		scrollContent := container.NewHBox(displayHolder, itemIconsHolder)
		contentScroll := container.NewHScroll(scrollContent)
		contentScroll.SetMinSize(fyne.NewSize(0, treeItemIconSize))
		removeBtn := &widget.Button{Icon: theme.CancelIcon(), Importance: widget.LowImportance}
		border := container.NewBorder(nil, nil, leftSide, removeBtn, contentScroll)

		hlSimple := canvas.NewRectangle(highlightSimpleColor)
		hlSimple.CornerRadius = 6
		hlSimple.Hide()
		hlFill := canvas.NewRectangle(highlightFillColor)
		hlFill.CornerRadius = 6
		hlFill.Hide()
		hlBg := container.New(&fillLayout{}, hlSimple, hlFill)

		// Highlight overlay is drawn on top of the row. canvas.Rectangle is not
		// tappable, so taps still reach the icon/remove buttons beneath it.
		return container.NewStack(border, hlBg)
	}
	mt.UpdateNode = func(uid string, branch bool, obj fyne.CanvasObject) {
		stack := obj.(*fyne.Container)
		c := stack.Objects[0].(*fyne.Container)
		hlBg := stack.Objects[1].(*fyne.Container)
		if mt.consumeHighlightRefresh(uid) && mt.nodeObjectShowsUID(obj, uid) {
			mt.applyHighlightOverlay(uid, hlBg)
			return
		}

		node := mt.Macro.Root.GetAction(uid)
		leftSide := c.Objects[1].(*fyne.Container)
		iconStack := leftSide.Objects[0].(*fyne.Container)
		iconBg := iconStack.Objects[0].(*canvas.Rectangle)
		actionIconBtn := iconStack.Objects[1].(*ttwidget.Button)
		removeButton := c.Objects[2].(*widget.Button)
		contentScroll := c.Objects[0].(*container.Scroll)
		scrollContent, ok := contentScroll.Content.(*fyne.Container)
		if !ok || len(scrollContent.Objects) < 2 {
			return
		}
		displayHolder := scrollContent.Objects[0].(*fyne.Container)
		itemIconsHolder := scrollContent.Objects[1].(*fyne.Container)
		displayContainer := displayHolder.Objects[0].(*fyne.Container)
		itemIconsBox := itemIconsHolder.Objects[0].(*fyne.Container)

		displayContainer.Objects = []fyne.CanvasObject{node.Display()}
		displayContainer.Refresh()
		iconBg.FillColor = macroTreeActionColor(node)
		iconBg.Refresh()
		actionIconBtn.SetIcon(node.Icon())
		actionIconBtn.SetToolTip(node.GetType())
		actionIconBtn.Importance = widget.LowImportance
		actionIconBtn.OnTapped = nil
		if mt.OnOpenActionDialog != nil {
			action := node
			actionIconBtn.OnTapped = func() { mt.OnOpenActionDialog(action) }
		}

		itemIconsBox.Objects = itemIconsBox.Objects[:0]
		if is, ok := node.(*actions.ImageSearch); ok && len(is.Targets) > 0 {
			previewSize := fyne.NewSize(treeItemIconSize, treeItemIconSize)
			for _, target := range is.Targets {
				if path := uiutil.IconPathForTarget(target); path != "" {
					if res := assets.GetFyneResource(path); res != nil {
						img := canvas.NewImageFromResource(res)
						img.SetMinSize(previewSize)
						img.FillMode = canvas.ImageFillContain
						itemIconsBox.Add(img)
					}
				}
			}
		} else if wfp, ok := node.(*actions.FindPixel); ok {
			if col, ok := uiutil.HexToColor(wfp.TargetColor); ok {
				swatch := canvas.NewRectangle(col)
				swatch.SetMinSize(fyne.NewSize(treeItemIconSize, treeItemIconSize))
				itemIconsBox.Add(swatch)
			}
		}
		itemIconsBox.Refresh()

		removeButton.OnTapped = func() {
			node.GetParent().RemoveSubAction(node)
			mt.RefreshItem(uid)
			if len(mt.Macro.Root.SubActions) == 0 || mt.SelectedNode == node.GetUID() {
				mt.SelectedNode = ""
			}
		}
		removeButton.Show()

		mt.markNodeBound(obj, uid)
		mt.applyHighlightOverlay(uid, hlBg)
	}
}

// PasteNode creates a copy of the action from clipboardMap and inserts it into
// the current selection: if the selected node is an advanced action (has children),
// the pasted node is added as its last child; otherwise it is inserted below the
// selected node as a sibling. With no selection, pastes at the end of root.
// Returns true if paste succeeded.
func (mt *MacroTree) PasteNode(clipboardMap map[string]any) bool {
	if clipboardMap == nil {
		return false
	}
	var parent actions.AdvancedActionInterface
	insertIndex := 0
	if mt.SelectedNode != "" {
		selected := mt.Macro.Root.GetAction(mt.SelectedNode)
		if selected == nil {
			return false
		}
		if adv, ok := selected.(actions.AdvancedActionInterface); ok {
			parent = adv
			insertIndex = len(parent.GetSubActions())
		} else {
			parent = selected.GetParent()
			if parent == nil {
				return false
			}
			psa := parent.GetSubActions()
			for i, c := range psa {
				if c.GetUID() == mt.SelectedNode {
					insertIndex = i + 1
					break
				}
			}
		}
	} else {
		parent = mt.Macro.Root
		insertIndex = len(parent.GetSubActions())
	}
	newAction, err := serialize.ViperSerializer.CreateActionFromMap(clipboardMap, parent)
	if err != nil {
		return false
	}
	subActions := parent.GetSubActions()
	newSubs := make([]actions.ActionInterface, 0, len(subActions)+1)
	newSubs = append(newSubs, subActions[:insertIndex]...)
	newSubs = append(newSubs, newAction)
	newSubs = append(newSubs, subActions[insertIndex:]...)
	parent.SetSubActions(newSubs)
	mt.Select(newAction.GetUID())
	mt.SelectedNode = newAction.GetUID()
	mt.Refresh()
	return true
}
