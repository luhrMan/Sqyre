package macro

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/serialize"
	"Sqyre/internal/uiutil"
	"image/color"
	"log"
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

// OnOpenActionDialog is called when the user taps an action's icon or double-clicks
// its row to edit it. If non-nil, the tree will open the action dialog from this callback.
type OnOpenActionDialogFunc func(action actions.ActionInterface)

// treeRowBody is the tappable center of each action row. Single click selects the
// row; double click opens the action editor dialog.
type treeRowBody struct {
	widget.BaseWidget
	tree   *MacroTree
	uid    string
	scroll *container.Scroll
}

func newTreeRowBody(scroll *container.Scroll) *treeRowBody {
	b := &treeRowBody{scroll: scroll}
	b.ExtendBaseWidget(b)
	return b
}

func (b *treeRowBody) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(b.scroll)
}

func (b *treeRowBody) MinSize() fyne.Size {
	if b.scroll == nil {
		return fyne.NewSize(0, treeItemIconSize)
	}
	return b.scroll.MinSize()
}

func (b *treeRowBody) Tapped(*fyne.PointEvent) {
	if b.tree == nil || b.uid == "" {
		return
	}
	now := time.Now()
	if b.uid == b.tree.lastRowTapUID && now.Sub(b.tree.lastRowTapTime) < treeRowDoubleClickInterval {
		b.tree.lastRowTapUID = ""
		b.openActionDialog()
		return
	}
	b.tree.lastRowTapUID = b.uid
	b.tree.lastRowTapTime = now

	b.tree.Select(b.uid)
	canvas := fyne.CurrentApp().Driver().CanvasForObject(b.tree)
	if canvas != nil && canvas.Focused() != b.tree {
		if !fyne.CurrentDevice().IsMobile() {
			canvas.Focus(b.tree)
		}
	}
}

func (b *treeRowBody) openActionDialog() {
	if b.tree == nil || b.tree.executing || b.tree.OnOpenActionDialog == nil || b.uid == "" {
		return
	}
	if b.tree.Macro == nil || b.tree.Macro.Root == nil {
		return
	}
	node := b.tree.Macro.Root.GetAction(b.uid)
	if node != nil {
		b.tree.OnOpenActionDialog(node)
	}
}

func (b *treeRowBody) TappedSecondary(pe *fyne.PointEvent) {
	showMacroTreeActionContextMenu(b.tree, b, pe, b.uid)
}

var (
	_ fyne.Tappable          = (*treeRowBody)(nil)
	_ fyne.SecondaryTappable = (*treeRowBody)(nil)
)

type MacroTree struct {
	widget.Tree
	Macro              *models.Macro
	SelectedNode       string
	OnOpenActionDialog OnOpenActionDialogFunc
	// onShowAddActionPicker opens the new-action picker (Ctrl+A when tree focused).
	onShowAddActionPicker func()

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
	// suppressBranchOpenScroll skips OnBranchOpened auto-scroll during
	// programmatic expansion (execution highlight, GoToAction, etc.).
	suppressBranchOpenScroll int

	// OnTreeChanged is invoked after the tree structure is mutated by the user
	// (drag-and-drop reorder or move up/down) so the macro can be persisted.
	OnTreeChanged func()

	// OnHistoryChanged is invoked when undo/redo availability changes.
	OnHistoryChanged func()

	history         *treeHistory
	applyingHistory bool

	// executing is true while a macro runs; drag-and-drop is disabled then.
	executing bool

	// Drag-and-drop reorder state. dragVisible is the flattened list of visible
	// row UIDs captured when the drag begins (rebuilt on auto-expand and preview).
	// dragTreeTop is the canvas-absolute Y of the tree content's top (content
	// Y 0 at scroll 0); it stays fixed during the drag. dragLastPointerY is the
	// most recent pointer Y in canvas coordinates, used to re-resolve the drop
	// target while auto-scrolling without a new pointer event.
	dragActive         bool
	dragSrcUID         string
	dragTreeTop        float32
	dragLastPointerY   float32
	dragVisible        []string
	dropParent         actions.AdvancedActionInterface
	dropTargetUID      string
	dropMode           dropMode
	dropValid           bool
	dragOrigin          dragOrigin
	dropIndicatorKey    string
	dropGhostContentKey string

	// Debounced live preview: after the pointer rests on a valid drop target the
	// dragged action is temporarily inserted so sibling rows shift aside.
	dragPreviewInTree   bool
	dragPreviewKey      string
	dragPreviewTimer    *time.Timer
	dragUndoSnapshot    treeSnapshot
	dragUndoSnapshotOK  bool

	// Edge auto-scroll state. autoScrollDir is -1 (up), 0 (idle), or 1 (down).
	// autoScrollStop signals the ticker goroutine to exit.
	autoScrollDir  int
	autoScrollStop chan struct{}

	// Drop placement overlay shown while dragging. Nil in headless tests.
	dropOverlay    *fyne.Container
	dropGhost      *fyne.Container
	dropGhostInset *canvas.Rectangle
	dropGhostRow   *fyne.Container

	// autoExpand state: when a drag dwells over a collapsed branch, it is
	// expanded so the user can drop inside it.
	autoExpandUID   string
	autoExpandTimer *time.Timer
	// dragStartOpenBranches records branches open when a drag begins.
	dragStartOpenBranches map[string]struct{}
	// dragAutoOpenedBranches tracks branches opened during drag so they can
	// collapse again when the pointer leaves.
	dragAutoOpenedBranches map[string]struct{}

	// rowContentCache stores display widgets keyed by action UID.
	rowContentCache map[string]cachedRowContent
	// highlightOverlays maps action UIDs to row overlay widgets so execution
	// highlights can update without RefreshItem / tree relayout.
	highlightOverlays map[string]highlightRow
	// preExecClosedBranches records branches collapsed before a macro run so
	// they can be restored when execution finishes.
	preExecClosedBranches map[string]struct{}
	// execFullyExpanded is true while all branches are held open for execution.
	execFullyExpanded bool

	// lastRowTapUID/lastRowTapTime detect double-clicks without fyne.DoubleTappable,
	// which delays single taps by the driver double-tap interval (~300ms).
	lastRowTapUID  string
	lastRowTapTime time.Time
}

const (
	collapseDebounceMs          = 150
	treeItemIconSize            = 24
	treeRowDoubleClickInterval  = 300 * time.Millisecond
)
// cachedRowContent holds reusable display widgets for a tree row so highlight
// refreshes and branch open/close do not rebuild pills and PNG thumbnails.
type cachedRowContent struct {
	display   fyne.CanvasObject
	itemIcons fyne.CanvasObject
}

type highlightRow struct {
	stack *fyne.Container
	hlBg  *fyne.Container
}

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
	t := &MacroTree{fills: map[string]float64{}, history: newTreeHistory()}
	t.ExtendBaseWidget(t)
	t.Macro = m
	t.setTree()

	return t
}

// Select marks uid selected without scrolling when the row is already visible.
func (mt *MacroTree) Select(uid widget.TreeNodeID) {
	if uid == "" {
		mt.UnselectAll()
		mt.SelectedNode = ""
		return
	}
	scrollY, ok := treeScrollOffsetY(&mt.Tree)
	inView := ok && mt.isRowInViewport(string(uid))
	mt.Tree.Select(uid)
	mt.SelectedNode = string(uid)
	if inView && ok {
		mt.ScrollToOffset(scrollY)
	}
}

var _ fyne.Shortcutable = (*MacroTree)(nil)

// TypedShortcut handles keyboard shortcuts while the macro tree has focus.
func (mt *MacroTree) TypedShortcut(shortcut fyne.Shortcut) {
	handleMacroTreeShortcut(mt, shortcut)
}

// FocusGained forwards focus to the embedded tree so keyboard navigation works.
func (mt *MacroTree) FocusGained() {
	mt.Tree.FocusGained()
}

// FocusLost forwards focus loss to the embedded tree.
func (mt *MacroTree) FocusLost() {
	mt.Tree.FocusLost()
}

// RecordMutation saves the current tree state on the undo stack. Call before
// any direct mutation that does not go through tree helpers that record already.
func (mt *MacroTree) RecordMutation() {
	mt.recordMutation()
}

func (mt *MacroTree) recordMutation() {
	if mt.applyingHistory || mt.Macro == nil || mt.Macro.Root == nil {
		return
	}
	if mt.history == nil {
		mt.history = newTreeHistory()
	}
	mt.history.push(mt.Macro.Root, mt.SelectedNode)
	mt.notifyHistoryChanged()
}

func (mt *MacroTree) notifyHistoryChanged() {
	if mt.OnHistoryChanged != nil {
		mt.OnHistoryChanged()
	}
}

func (mt *MacroTree) CanUndo() bool {
	return mt.history != nil && mt.history.canUndo()
}

func (mt *MacroTree) CanRedo() bool {
	return mt.history != nil && mt.history.canRedo()
}

func (mt *MacroTree) Undo() bool {
	if !mt.CanUndo() {
		return false
	}
	current, err := snapshotTree(mt.Macro.Root, mt.SelectedNode)
	if err != nil {
		log.Printf("tree undo: snapshot current failed: %v", err)
		return false
	}
	prev, ok := mt.history.popUndo()
	if !ok {
		return false
	}
	mt.history.pushRedo(current)
	if err := mt.applySnapshot(prev); err != nil {
		log.Printf("tree undo: restore failed: %v", err)
		return false
	}
	return true
}

func (mt *MacroTree) Redo() bool {
	if !mt.CanRedo() {
		return false
	}
	current, err := snapshotTree(mt.Macro.Root, mt.SelectedNode)
	if err != nil {
		log.Printf("tree redo: snapshot current failed: %v", err)
		return false
	}
	next, ok := mt.history.popRedo()
	if !ok {
		return false
	}
	mt.history.pushUndo(current)
	if err := mt.applySnapshot(next); err != nil {
		log.Printf("tree redo: restore failed: %v", err)
		return false
	}
	return true
}

func (mt *MacroTree) applySnapshot(snap treeSnapshot) error {
	if mt.Macro == nil {
		return nil
	}
	root, err := restoreTreeRoot(snap.rootMap)
	if err != nil {
		return err
	}
	mt.applyingHistory = true
	defer func() { mt.applyingHistory = false }()

	view := mt.captureViewState()

	mt.Macro.Root = root
	mt.clearRowCache()
	mt.SelectedNode = snap.selectedUID
	if snap.selectedUID != "" && mt.Macro.Root.GetAction(snap.selectedUID) == nil {
		mt.SelectedNode = ""
	}
	mt.flushNodeCache()
	mt.restoreViewState(view)
	mt.selectPreservingScroll(mt.SelectedNode)
	if mt.OnTreeChanged != nil {
		mt.OnTreeChanged()
	}
	mt.notifyHistoryChanged()
	return nil
}

// Refresh rebuilds the tree and clears cached row widgets (e.g. after edits).
func (mt *MacroTree) Refresh() {
	scrollY, ok := treeScrollOffsetY(&mt.Tree)
	mt.clearRowCache()
	mt.Tree.Refresh()
	if ok {
		mt.ScrollToOffset(scrollY)
		if !mt.dragActive {
			mt.scheduleClampScroll()
		}
	}
}

func (mt *MacroTree) clearRowCache() {
	mt.rowContentCache = nil
	mt.highlightOverlays = nil
	mt.nodeBoundUID = nil
}

func (mt *MacroTree) invalidateRowCache(uid string) {
	if mt.rowContentCache != nil && uid != "" {
		delete(mt.rowContentCache, uid)
	}
}

func (mt *MacroTree) cachedRowContent(node actions.ActionInterface) cachedRowContent {
	uid := node.GetUID()
	if mt.rowContentCache != nil {
		if cached, ok := mt.rowContentCache[uid]; ok {
			return cached
		}
	}
	entry := cachedRowContent{display: node.Display()}
	if is, ok := node.(*actions.ImageSearch); ok && len(is.Targets) > 0 {
		previewSize := fyne.NewSize(treeItemIconSize, treeItemIconSize)
		box := container.NewHBox()
		for _, target := range is.Targets {
			if path := uiutil.IconPathForTarget(target); path != "" {
				if res := assets.GetFyneResource(path); res != nil {
					img := canvas.NewImageFromResource(res)
					img.SetMinSize(previewSize)
					img.FillMode = canvas.ImageFillContain
					box.Add(img)
				}
			}
		}
		if len(box.Objects) > 0 {
			entry.itemIcons = box
		}
	} else if wfp, ok := node.(*actions.FindPixel); ok {
		if col, ok := uiutil.HexToColor(wfp.TargetColor); ok {
			swatch := canvas.NewRectangle(col)
			swatch.SetMinSize(fyne.NewSize(treeItemIconSize, treeItemIconSize))
			entry.itemIcons = swatch
		}
	}
	if mt.rowContentCache == nil {
		mt.rowContentCache = map[string]cachedRowContent{}
	}
	mt.rowContentCache[uid] = entry
	return entry
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
		mt.refreshHighlightOverlay(old)
	}
	if uid != "" {
		if !mt.execFullyExpanded {
			mt.openAncestorBranches(uid)
		}
		mt.refreshHighlightOverlay(uid)
		targetUID := uid
		fyne.Do(func() {
			if mt.cursorUID == targetUID {
				mt.scrollToIfNeeded(targetUID)
			}
		})
	} else {
		mt.lastScrollUID = ""
	}
	if !mt.executing {
		mt.scheduleCollapseStale()
	}
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
		if !mt.execFullyExpanded {
			mt.openAncestorBranches(uid)
		}
		if !mt.executing {
			mt.scheduleCollapseStale()
		}
		targetUID := uid
		fyne.Do(func() {
			if _, ok := mt.fills[targetUID]; ok {
				mt.scrollToIfNeeded(targetUID)
			}
		})
	}
	mt.refreshHighlightOverlay(uid)
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
		mt.refreshHighlightOverlay(uid)
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
		mt.refreshHighlightOverlay(k)
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
	mt.suppressBranchOpenScroll++
	defer func() { mt.suppressBranchOpenScroll-- }()
	for i := len(ancestors) - 1; i >= 0; i-- {
		a := ancestors[i]
		if !mt.IsBranchOpen(a) {
			mt.OpenBranch(a)
			mt.trackExecOpened(a)
		}
	}
}

// OpenAllBranches expands every branch in the macro tree.
func (mt *MacroTree) OpenAllBranches() {
	mt.stopCollapseDebounce()
	mt.execOpenedBranches = nil
	mt.suppressBranchOpenScroll++
	defer func() { mt.suppressBranchOpenScroll-- }()
	mt.Tree.OpenAllBranches()
}

// CloseAllBranches collapses every branch in the macro tree.
func (mt *MacroTree) CloseAllBranches() {
	mt.stopCollapseDebounce()
	mt.execOpenedBranches = nil
	mt.Tree.CloseAllBranches()
	mt.scheduleClampScroll()
}

// GoToAction selects uid, expands ancestor branches, and scrolls it into view.
func (mt *MacroTree) GoToAction(uid string) {
	if uid == "" || mt.Macro == nil || mt.Macro.Root == nil {
		return
	}
	if mt.Macro.Root.GetAction(uid) == nil {
		return
	}
	mt.Select(uid)
	mt.SelectedNode = uid
	mt.lastScrollUID = ""
	mt.revealNode(uid)
}

// revealNode expands ancestor branches and scrolls so uid is visible.
func (mt *MacroTree) revealNode(uid string) {
	mt.openAncestorBranches(uid)
	mt.scrollToIfNeeded(uid)
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

func (mt *MacroTree) registerHighlightOverlay(uid string, stack *fyne.Container, hlBg *fyne.Container) {
	if mt.highlightOverlays == nil {
		mt.highlightOverlays = map[string]highlightRow{}
	}
	mt.highlightOverlays[uid] = highlightRow{stack: stack, hlBg: hlBg}
	mt.markNodeBound(stack, uid)
}

// refreshHighlightOverlay updates the execution highlight on uid when its row
// overlay is already bound, avoiding RefreshItem and tree relayout.
func (mt *MacroTree) refreshHighlightOverlay(uid string) {
	if uid == "" {
		return
	}
	if row, ok := mt.highlightOverlays[uid]; ok && mt.nodeObjectShowsUID(row.stack, uid) {
		mt.applyHighlightOverlay(uid, row.hlBg)
		return
	}
	mt.markHighlightRefresh(uid)
	mt.RefreshItem(uid)
}

func rgbaEqual(a, b color.Color) bool {
	if a == nil && b == nil {
		return true
	}
	if a == nil || b == nil {
		return false
	}
	ar, ag, ab, aa := a.RGBA()
	br, bg, bb, ba := b.RGBA()
	return ar == br && ag == bg && ab == bb && aa == ba
}

func (mt *MacroTree) applyHighlightOverlay(uid string, hlBg *fyne.Container) {
	fl := hlBg.Layout.(*fillLayout)
	hlSimple := hlBg.Objects[0].(*canvas.Rectangle)
	hlFill := hlBg.Objects[1].(*canvas.Rectangle)

	var wantFrac float64
	var wantSimpleVisible, wantFillVisible bool
	var wantSimpleColor color.Color

	switch {
	case mt.dragActive && uid == mt.dragSrcUID:
		wantSimpleVisible = true
		wantSimpleColor = dragSourceColor
	default:
		if frac, ok := mt.fills[uid]; ok {
			wantFrac = frac
			wantFillVisible = true
		} else if uid == mt.cursorUID {
			wantSimpleVisible = true
			wantSimpleColor = highlightSimpleColor
		}
	}

	simpleVisible := hlSimple.Visible()
	fillVisible := hlFill.Visible()
	if fl.fraction == wantFrac &&
		simpleVisible == wantSimpleVisible &&
		fillVisible == wantFillVisible &&
		(!wantSimpleVisible || rgbaEqual(hlSimple.FillColor, wantSimpleColor)) {
		return
	}

	fl.fraction = wantFrac
	if wantFillVisible {
		hlFill.Show()
		hlSimple.Hide()
	} else if wantSimpleVisible {
		hlSimple.FillColor = wantSimpleColor
		hlSimple.Show()
		hlFill.Hide()
	} else {
		hlSimple.Hide()
		hlFill.Hide()
	}
	hlSimple.Refresh()
	hlFill.Refresh()
}

func (mt *MacroTree) scheduleCollapseStale() {
	if mt.executing {
		return
	}
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
	if mt.executing || mt.execOpenedBranches == nil {
		return
	}
	keep := mt.branchesToKeepOpen()
	closed := false
	for uid := range mt.execOpenedBranches {
		if keep[uid] {
			continue
		}
		if mt.IsBranchOpen(uid) {
			mt.Tree.CloseBranch(uid)
			closed = true
		}
		mt.untrackExecOpenedBranch(uid)
	}
	if closed {
		mt.scheduleClampScroll()
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

	moved := false
	if up && index > 0 {
		mt.recordMutation()
		psa[index-1], psa[index] = psa[index], psa[index-1]
		mt.Select(psa[index-1].GetUID())
		moved = true
	} else if !up && index < len(psa)-1 {
		mt.recordMutation()
		psa[index], psa[index+1] = psa[index+1], psa[index]
		mt.Select(psa[index+1].GetUID())
		moved = true
	}
	mt.Refresh()
	if moved && mt.OnTreeChanged != nil {
		mt.OnTreeChanged()
	}
}

// DeleteSelectedAction removes the currently selected action from the tree.
func (mt *MacroTree) DeleteSelectedAction() bool {
	if mt == nil || mt.SelectedNode == "" || mt.Macro == nil || mt.Macro.Root == nil {
		return false
	}
	node := mt.Macro.Root.GetAction(mt.SelectedNode)
	return mt.deleteAction(node)
}

func (mt *MacroTree) deleteAction(node actions.ActionInterface) bool {
	if node == nil || node.GetParent() == nil {
		return false
	}
	uid := node.GetUID()
	mt.recordMutation()
	mt.invalidateRowCache(uid)
	node.GetParent().RemoveSubAction(node)
	mt.RefreshItem(uid)
	if len(mt.Macro.Root.SubActions) == 0 || mt.SelectedNode == uid {
		mt.SelectedNode = ""
	}
	if mt.OnTreeChanged != nil {
		mt.OnTreeChanged()
	}
	return true
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
	mt.CreateNode = func(branch bool) fyne.CanvasObject {
		actionIconBtn := ttwidget.NewButtonWithIcon("", theme.ErrorIcon(), nil)
		actionIconBtn.Importance = widget.LowImportance
		iconBg := canvas.NewRectangle(actions.ActionPastelColor(""))
		iconBg.CornerRadius = 6
		iconBg.StrokeColor = theme.ShadowColor()
		iconBg.StrokeWidth = 1
		iconStack := container.NewStack(iconBg, actionIconBtn)
		dh := newDragHandle()
		dh.tree = mt
		leftSide := container.NewHBox(
			dh,
			iconStack,
		)
		displayContainer := container.New(kxlayout.NewRowWrapLayout())
		itemIconsBox := container.NewHBox()
		displayHolder := container.NewCenter(displayContainer)
		itemIconsHolder := container.NewCenter(itemIconsBox)
		scrollContent := container.NewHBox(displayHolder, itemIconsHolder)
		contentScroll := container.NewHScroll(scrollContent)
		contentScroll.SetMinSize(fyne.NewSize(0, treeItemIconSize))
		rowBody := newTreeRowBody(contentScroll)
		rowBody.tree = mt
		removeBtn := &widget.Button{Icon: theme.CancelIcon(), Importance: widget.LowImportance}
		border := container.NewBorder(nil, nil, leftSide, removeBtn, rowBody)

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
			mt.registerHighlightOverlay(uid, stack, hlBg)
			mt.applyHighlightOverlay(uid, hlBg)
			return
		}

		node := mt.Macro.Root.GetAction(uid)
		if node == nil {
			// Can occur transiently during a node-cache flush (sentinel root).
			return
		}
		leftSide := c.Objects[1].(*fyne.Container)
		dh := leftSide.Objects[0].(*dragHandle)
		dh.tree = mt
		dh.uid = uid
		iconStack := leftSide.Objects[1].(*fyne.Container)
		iconBg := iconStack.Objects[0].(*canvas.Rectangle)
		actionIconBtn := iconStack.Objects[1].(*ttwidget.Button)
		removeButton := c.Objects[2].(*widget.Button)
		rowBody := c.Objects[0].(*treeRowBody)
		rowBody.tree = mt
		rowBody.uid = uid
		contentScroll := rowBody.scroll
		scrollContent, ok := contentScroll.Content.(*fyne.Container)
		if !ok || len(scrollContent.Objects) < 2 {
			return
		}
		displayHolder := scrollContent.Objects[0].(*fyne.Container)
		itemIconsHolder := scrollContent.Objects[1].(*fyne.Container)
		displayContainer := displayHolder.Objects[0].(*fyne.Container)
		itemIconsBox := itemIconsHolder.Objects[0].(*fyne.Container)

		rowContent := mt.cachedRowContent(node)
		displayContainer.Objects = []fyne.CanvasObject{rowContent.display}
		if mt.executing {
			itemIconsBox.Objects = nil
		} else if rowContent.itemIcons != nil {
			itemIconsBox.Objects = []fyne.CanvasObject{rowContent.itemIcons}
		} else {
			itemIconsBox.Objects = nil
		}
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
		itemIconsBox.Refresh()

		removeButton.OnTapped = func() {
			mt.deleteAction(node)
		}
		removeButton.Show()

		mt.registerHighlightOverlay(uid, stack, hlBg)
		mt.applyHighlightOverlay(uid, hlBg)
	}
	mt.Tree.OnBranchOpened = func(uid widget.TreeNodeID) {
		if mt.suppressBranchOpenScroll > 0 || mt.dragActive {
			return
		}
		target := uid
		if children := mt.ChildUIDs(uid); len(children) > 0 {
			target = children[0]
		}
		scrollUID := target
		fyne.Do(func() {
			if mt.suppressBranchOpenScroll > 0 || mt.dragActive {
				return
			}
			mt.ScrollTo(scrollUID)
		})
	}
	mt.Tree.OnBranchClosed = func(widget.TreeNodeID) {
		mt.scheduleClampScroll()
	}
}

// insertLocationBelowSelection returns the parent and index at which a new action
// should be inserted relative to the current selection. With no selection, or when
// root is selected, appends to the end of root. When a branch is selected, inserts
// as its first child. Otherwise inserts directly below the selected leaf.
func (mt *MacroTree) insertLocationBelowSelection() (actions.AdvancedActionInterface, int, bool) {
	if mt.Macro == nil || mt.Macro.Root == nil {
		return nil, 0, false
	}
	root := actions.AdvancedActionInterface(mt.Macro.Root)
	if mt.SelectedNode == "" {
		return root, len(root.GetSubActions()), true
	}
	selected := mt.Macro.Root.GetAction(mt.SelectedNode)
	if selected == nil || selected.GetUID() == mt.Macro.Root.GetUID() {
		return root, len(root.GetSubActions()), true
	}
	if branch, ok := selected.(actions.AdvancedActionInterface); ok {
		return branch, 0, true
	}
	parent := selected.GetParent()
	if parent == nil {
		return root, len(root.GetSubActions()), true
	}
	insertIndex := len(parent.GetSubActions())
	for i, c := range parent.GetSubActions() {
		if c.GetUID() == mt.SelectedNode {
			insertIndex = i + 1
			break
		}
	}
	return parent, insertIndex, true
}

func (mt *MacroTree) insertActionAt(parent actions.AdvancedActionInterface, insertIndex int, action actions.ActionInterface) {
	action.SetParent(parent)
	subActions := parent.GetSubActions()
	newSubs := make([]actions.ActionInterface, 0, len(subActions)+1)
	newSubs = append(newSubs, subActions[:insertIndex]...)
	newSubs = append(newSubs, action)
	newSubs = append(newSubs, subActions[insertIndex:]...)
	parent.SetSubActions(newSubs)
}

// InsertActionBelowSelection inserts action relative to the current selection.
// Branches receive the action as their first child; leaves receive it as the next
// sibling. With no selection, appends to the end of root. Returns false when the
// macro tree has no root.
func (mt *MacroTree) InsertActionBelowSelection(action actions.ActionInterface) bool {
	parent, insertIndex, ok := mt.insertLocationBelowSelection()
	if !ok {
		return false
	}
	mt.recordMutation()
	mt.insertActionAt(parent, insertIndex, action)
	return true
}

// PasteNode creates a copy of the action from clipboardMap and inserts it using
// the same placement rules as InsertActionBelowSelection. Returns true if paste
// succeeded.
func (mt *MacroTree) PasteNode(clipboardMap map[string]any) bool {
	if clipboardMap == nil {
		return false
	}
	parent, insertIndex, ok := mt.insertLocationBelowSelection()
	if !ok {
		return false
	}
	newAction, err := serialize.ViperSerializer.CreateActionFromMap(clipboardMap, parent)
	if err != nil {
		return false
	}
	mt.recordMutation()
	mt.insertActionAt(parent, insertIndex, newAction)
	mt.Select(newAction.GetUID())
	mt.SelectedNode = newAction.GetUID()
	mt.Refresh()
	return true
}
