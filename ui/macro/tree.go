package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
	"Sqyre/internal/uiutil"
	"image/color"
	"log"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// treeRowBody is the tappable center of each action row. Single click selects the
// row; double click opens the action tooltip editor.
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

func (b *treeRowBody) Tapped(pe *fyne.PointEvent) {
	if b.tree == nil || b.uid == "" {
		return
	}
	now := time.Now()
	if b.uid == b.tree.lastRowTapUID && now.Sub(b.tree.lastRowTapTime) < treeRowDoubleClickInterval {
		b.tree.lastRowTapUID = ""
		var cursor fyne.Position
		if pe != nil {
			cursor = pe.AbsolutePosition
		}
		b.openActionEdit(cursor)
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

func (b *treeRowBody) openActionEdit(cursor fyne.Position) {
	if b.tree == nil || b.tree.executing || b.uid == "" {
		return
	}
	b.tree.editActionAt(b.uid, cursor)
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
	rowContentCache *rowContentLRUCache
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
	return actiondisplay.ActionPastelColorForApp(action.GetType())
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

// RefreshVisibleRowDisplays clears cached row widgets and re-binds visible rows
// without a full tree rebuild. Use when display colors or labels changed but
// structure did not.
func (mt *MacroTree) RefreshVisibleRowDisplays() {
	if mt.Macro == nil {
		return
	}
	scrollY, ok := treeScrollOffsetY(&mt.Tree)
	mt.clearRowCache()
	for _, uid := range mt.visibleRowUIDs() {
		mt.RefreshItem(uid)
	}
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
		mt.rowContentCache.delete(uid)
	}
}

func (mt *MacroTree) cachedRowContent(node actions.ActionInterface) cachedRowContent {
	uid := node.GetUID()
	if mt.rowContentCache != nil {
		if cached, ok := mt.rowContentCache.get(uid); ok {
			return cached
		}
	}
	entry := cachedRowContent{display: actionDisplayForTree(node, actionDisplayHandlers{
		onActionSaved: func() {
			mt.RecordMutation()
			mt.invalidateRowCache(uid)
			mt.RefreshItem(uid)
			if mt.OnTreeChanged != nil {
				mt.OnTreeChanged()
			}
		},
	})}
	if is, ok := node.(*actions.ImageSearch); ok && len(is.Targets) > 0 {
		entry.itemIcons = imageSearchRowTargetIcons(is.Targets)
	} else if wfp, ok := node.(*actions.FindPixel); ok {
		if col, ok := uiutil.HexToColor(wfp.TargetColor); ok {
			swatch := canvas.NewRectangle(col)
			swatch.SetMinSize(fyne.NewSize(treeItemIconSize, treeItemIconSize))
			entry.itemIcons = swatch
		}
	}
	if mt.rowContentCache == nil {
		mt.rowContentCache = newRowContentLRUCache()
	}
	mt.rowContentCache.put(uid, entry)
	return entry
}

// EditAction scrolls to the action row and opens its tooltip in edit mode
// anchored to the row (used by the icon button and keyboard, which have no cursor).
func (mt *MacroTree) EditAction(uid string) {
	mt.editActionAt(uid, fyne.Position{})
}

// editActionAt opens the action's tooltip in edit mode. A non-zero cursor anchors
// the tooltip to the pointer (double-click); a zero cursor anchors it to the row.
func (mt *MacroTree) editActionAt(uid string, cursor fyne.Position) {
	if mt == nil || mt.executing || uid == "" || mt.Macro == nil || mt.Macro.Root == nil {
		return
	}
	node := mt.Macro.Root.GetAction(uid)
	if node == nil {
		return
	}
	mt.ScrollTo(uid)
	mt.Select(uid)
	fyne.Do(func() {
		if mt.Macro.Root.GetAction(uid) == nil {
			return
		}
		mt.RefreshItem(uid)
		hover, ok := mt.cachedRowContent(node).display.(*actionDisplayTooltipHover)
		if !ok || hover == nil {
			return
		}
		if cursor != (fyne.Position{}) {
			hover.absoluteMousePos = cursor
		} else {
			hover.absoluteMousePos = fyne.CurrentApp().Driver().AbsolutePositionForObject(hover)
		}
		hover.openTooltipEdit()
	})
}

// SetCursor moves the single "currently executing" highlight to uid (or clears
// it when uid is empty). Must be called on the Fyne UI thread.
