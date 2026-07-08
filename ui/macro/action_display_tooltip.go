package macro

import (
	"context"
	"image"
	"image/color"
	"strings"

	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/desktopview"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	fynewidget "fyne.io/fyne/v2/widget"
)

const tooltipGeometryEpsilon float32 = 0.5

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

type actionDisplayHandlers struct {
	onActionSaved func()
}

func actionDisplay(node actions.ActionInterface, handlers actionDisplayHandlers) fyne.CanvasObject {
	return actionDisplayFromParams(node, node.Params(), handlers)
}

func actionDisplayForTree(node actions.ActionInterface, handlers actionDisplayHandlers) fyne.CanvasObject {
	return actionDisplayFromParams(node, actionDisplayParamsForTree(node), handlers)
}

func actionDisplayParamsForTree(node actions.ActionInterface) []actions.Param {
	params := node.Params()
	if _, ok := node.(*actions.ImageSearch); !ok {
		return params
	}
	filtered := make([]actions.Param, 0, len(params))
	for _, p := range params {
		if strings.EqualFold(p.Label, "Items") {
			continue
		}
		filtered = append(filtered, p)
	}
	return filtered
}

func actionDisplayFromParams(node actions.ActionInterface, params []actions.Param, handlers actionDisplayHandlers) fyne.CanvasObject {
	line, extra, actionType := actiondisplay.DisplayFromParams(params, macroKnownVariables())
	loader := actionPreviewLoader(node)
	return newActionDisplayTooltipHover(node, line, extra, actionType, loader, handlers.onActionSaved)
}

type actionDisplayTooltipHover struct {
	fynewidget.BaseWidget

	node          actions.ActionInterface
	onActionSaved func()
	content       fyne.CanvasObject
	extra         []actions.Param
	actionType    string
	previewLoader custom_widgets.PreviewTooltipLoad
	rowBody       *treeRowBody
	keepAliveArea fyne.CanvasObject

	tooltipPanel     *actionDisplayTooltipPanel
	dismissBackdrop  *custom_widgets.TooltipDismissBackdrop
	pendingCancel    context.CancelFunc
	pendingCtx       context.Context
	captureCancel    context.CancelFunc
	captureCtx       context.Context
	absoluteMousePos fyne.Position
	displayHovering  bool
	iconHovering     bool
	rowHovering      bool
	backdropDismissEnabled bool
	previewCache           custom_widgets.PreviewTooltipResult
	previewCacheErr        error
	previewCacheReady      bool

	tooltipMounted    bool
	cachedCanvas      fyne.Canvas
	cachedLayer       *custom_widgets.ItemTooltipLayer
	cachedLayerOrigin fyne.Position
	layerCacheOK      bool

	keepAliveGeometryOK   bool
	cachedSelfOrigin      fyne.Position
	cachedSelfSize        fyne.Size
	cachedKeepAliveOrigin fyne.Position
	cachedKeepAliveSize   fyne.Size

	tooltipPanelGeometryOK bool
	cachedPanelOrigin      fyne.Position
	cachedPanelSize        fyne.Size
}

var _ desktop.Hoverable = (*actionDisplayTooltipHover)(nil)
var _ fyne.Tappable = (*actionDisplayTooltipHover)(nil)
var _ fyne.SecondaryTappable = (*actionDisplayTooltipHover)(nil)

func newActionDisplayTooltipHover(
	node actions.ActionInterface,
	content fyne.CanvasObject,
	extra []actions.Param,
	actionType string,
	loader custom_widgets.PreviewTooltipLoad,
	onActionSaved func(),
) *actionDisplayTooltipHover {
	h := &actionDisplayTooltipHover{
		node:                   node,
		onActionSaved:          onActionSaved,
		content:                content,
		actionType:             actionType,
		previewLoader:          loader,
		backdropDismissEnabled: true,
	}
	if len(extra) > 0 {
		h.extra = append([]actions.Param(nil), extra...)
	}
	h.ExtendBaseWidget(h)
	return h
}

func (h *actionDisplayTooltipHover) bindRowBody(body *treeRowBody) {
	h.rowBody = body
}

func (h *actionDisplayTooltipHover) setTooltipKeepAliveArea(obj fyne.CanvasObject) {
	h.keepAliveArea = obj
}

func (h *actionDisplayTooltipHover) Tapped(pe *fyne.PointEvent) {
	if h.tooltipPanel != nil && !h.tooltipPanel.editing {
		h.hideViewTooltip()
	}
	if h.rowBody != nil {
		h.rowBody.Tapped(pe)
	}
}

func (h *actionDisplayTooltipHover) anyHovering() bool {
	return h.displayHovering || h.iconHovering || h.rowHovering
}

func (h *actionDisplayTooltipHover) MouseIn(e *desktop.MouseEvent) {
	h.displayHovering = true
	h.noteHoverIn(e)
}

func (h *actionDisplayTooltipHover) MouseOut() {
	h.displayHovering = false
	h.noteHoverOut()
}

func (h *actionDisplayTooltipHover) iconMouseIn(e *desktop.MouseEvent) {
	h.iconHovering = true
	h.noteHoverIn(e)
}

func (h *actionDisplayTooltipHover) iconMouseOut() {
	h.iconHovering = false
	h.noteHoverOut()
}

func (h *actionDisplayTooltipHover) iconMouseMoved(e *desktop.MouseEvent) {
	h.trackMouseForTooltip(e)
}

func (h *actionDisplayTooltipHover) rowMouseIn(e *desktop.MouseEvent) {
	h.rowHovering = true
	h.noteHoverIn(e)
}

func (h *actionDisplayTooltipHover) rowMouseOut() {
	h.rowHovering = false
	h.noteHoverOut()
}

func (h *actionDisplayTooltipHover) rowMouseMoved(e *desktop.MouseEvent) {
	h.trackMouseForTooltip(e)
}

func (h *actionDisplayTooltipHover) trackMouseForTooltip(e *desktop.MouseEvent) {
	pos := e.AbsolutePosition
	if h.absoluteMousePos == pos {
		return
	}
	h.absoluteMousePos = pos
	if h.viewTooltipOpen() && !h.pointerInTreeActionSpace(pos) {
		h.cancelCapture()
		h.hideViewTooltip()
		return
	}
	if !h.shouldFollowMouse() {
		return
	}
	h.repositionTooltip()
}

func (h *actionDisplayTooltipHover) pointerInTreeActionSpace(pos fyne.Position) bool {
	return h.pointerInRowKeepAlive(pos)
}

func (h *actionDisplayTooltipHover) shouldFollowMouse() bool {
	if h.tooltipPanel == nil || h.tooltipPinned() {
		return false
	}
	return h.pointerInTreeActionSpace(h.absoluteMousePos)
}

func pointInCachedRect(abs, origin fyne.Position, size fyne.Size) bool {
	return abs.X >= origin.X && abs.Y >= origin.Y &&
		abs.X < origin.X+size.Width && abs.Y < origin.Y+size.Height
}

func (h *actionDisplayTooltipHover) refreshKeepAliveGeometry() {
	driver := fyne.CurrentApp().Driver()
	h.cachedSelfOrigin = driver.AbsolutePositionForObject(h)
	h.cachedSelfSize = h.Size()
	if h.keepAliveArea != nil && h.keepAliveArea.Visible() {
		h.cachedKeepAliveOrigin = driver.AbsolutePositionForObject(h.keepAliveArea)
		h.cachedKeepAliveSize = h.keepAliveArea.Size()
	}
	h.keepAliveGeometryOK = true
}

func (h *actionDisplayTooltipHover) clearKeepAliveGeometryCache() {
	h.keepAliveGeometryOK = false
}

func (h *actionDisplayTooltipHover) refreshTooltipPanelGeometry(origin, relPos fyne.Position, size fyne.Size) {
	h.cachedPanelOrigin = origin.Add(relPos)
	h.cachedPanelSize = size
	h.tooltipPanelGeometryOK = true
}

func (h *actionDisplayTooltipHover) clearTooltipPanelGeometryCache() {
	h.tooltipPanelGeometryOK = false
}

func (h *actionDisplayTooltipHover) pointerInTooltipPanel(pos fyne.Position) bool {
	if h.tooltipPanel == nil || !h.tooltipPanelGeometryOK {
		return false
	}
	return pointInCachedRect(pos, h.cachedPanelOrigin, h.cachedPanelSize)
}

func (h *actionDisplayTooltipHover) pointerInRowKeepAlive(pos fyne.Position) bool {
	if !h.keepAliveGeometryOK {
		h.refreshKeepAliveGeometry()
	}
	if h.keepAliveArea != nil && h.keepAliveArea.Visible() {
		if pointInCachedRect(pos, h.cachedKeepAliveOrigin, h.cachedKeepAliveSize) {
			return true
		}
	}
	return pointInCachedRect(pos, h.cachedSelfOrigin, h.cachedSelfSize)
}

func (h *actionDisplayTooltipHover) hoverCanvas() fyne.Canvas {
	if h.cachedCanvas != nil {
		return h.cachedCanvas
	}
	c := fyne.CurrentApp().Driver().CanvasForObject(h)
	h.cachedCanvas = c
	return c
}

func (h *actionDisplayTooltipHover) isTooltipMounted() bool {
	if h.tooltipPanel == nil {
		return false
	}
	if h.tooltipMounted {
		return true
	}
	c := h.hoverCanvas()
	if c == nil {
		return false
	}
	layer := h.windowTooltipLayer(c)
	if layer == nil {
		return false
	}
	for _, obj := range layer.Container.Objects {
		if obj == h.tooltipPanel {
			h.tooltipMounted = true
			return true
		}
	}
	return false
}

func (h *actionDisplayTooltipHover) clearTooltipLayerCache() {
	h.layerCacheOK = false
	h.cachedCanvas = nil
	h.cachedLayer = nil
	h.clearKeepAliveGeometryCache()
	h.clearTooltipPanelGeometryCache()
}

func (h *actionDisplayTooltipHover) tooltipLayerOn(c fyne.Canvas) (*custom_widgets.ItemTooltipLayer, fyne.Position) {
	if h.layerCacheOK && h.cachedCanvas == c && h.cachedLayer != nil {
		return h.cachedLayer, h.cachedLayerOrigin
	}
	layer := h.windowTooltipLayer(c)
	if layer == nil {
		return nil, fyne.Position{}
	}
	origin := h.windowTooltipOrigin(layer)
	h.cachedCanvas = c
	h.cachedLayer = layer
	h.cachedLayerOrigin = origin
	h.layerCacheOK = true
	return layer, origin
}

func (h *actionDisplayTooltipHover) viewTooltipOpen() bool {
	return h.tooltipPanel != nil && !h.tooltipPanel.editing && h.isTooltipMounted()
}

func (h *actionDisplayTooltipHover) shouldKeepViewTooltip() bool {
	if h.tooltipPinned() || !h.viewTooltipOpen() {
		return true
	}
	return h.pointerInTreeActionSpace(h.absoluteMousePos)
}

func (h *actionDisplayTooltipHover) scheduleViewTooltipDismissCheck() {
	if !h.viewTooltipOpen() {
		return
	}
	fyne.Do(func() {
		if !h.shouldKeepViewTooltip() {
			h.cancelCapture()
			h.hideViewTooltip()
		}
	})
}

func (h *actionDisplayTooltipHover) clearPreviewCache() {
	h.previewCacheReady = false
	h.previewCache = custom_widgets.PreviewTooltipResult{}
	h.previewCacheErr = nil
}

func (h *actionDisplayTooltipHover) applyPreviewCache(panel *actionDisplayTooltipPanel) {
	if !h.previewCacheReady || panel == nil {
		return
	}
	if h.previewCacheErr != nil {
		panel.setPreviewError(h.previewCacheErr.Error())
		return
	}
	panel.setPreviewImage(h.previewCache.Image, h.previewCache.Caption)
}

func (h *actionDisplayTooltipHover) previewCaptureInFlight() bool {
	return h.captureCtx != nil
}

func (h *actionDisplayTooltipHover) noteHoverIn(e *desktop.MouseEvent) {
	h.absoluteMousePos = e.AbsolutePosition
	h.refreshKeepAliveGeometry()
	if h.actionTooltipsSuppressed() {
		h.cancelPending()
		h.hideViewTooltip()
		return
	}
	if actionTooltipEditPinnedByOther(h) {
		h.cancelPending()
		return
	}
	if h.tooltipPinned() {
		return
	}
	if h.tooltipPanel != nil && !h.tooltipPanel.editing {
		if !h.isTooltipMounted() {
			h.openViewTooltip()
			return
		}
		h.repositionTooltip()
		custom_widgets.ActivateTooltipEscapeDismiss(func() { h.hideViewTooltip() })
		return
	}
	h.openViewTooltip()
}

func (h *actionDisplayTooltipHover) openViewTooltip() {
	if h.actionTooltipsSuppressed() || actionTooltipEditPinnedByOther(h) {
		return
	}
	h.cancelPending()
	h.refreshKeepAliveGeometry()
	if h.previewLoader != nil {
		h.beginPreviewCapture()
		return
	}
	h.showTooltipPanel()
}

func (h *actionDisplayTooltipHover) noteHoverOut() {
	h.clearKeepAliveGeometryCache()
	if h.tooltipPinned() {
		return
	}
	if h.anyHovering() && h.pointerInRowKeepAlive(h.absoluteMousePos) {
		return
	}
	h.cancelPending()
	h.cancelCapture()
	h.hideViewTooltip()
}

// hideViewTooltip unmounts a view-mode tooltip but keeps the preview cache so
// re-hovering the same row does not recapture the screen.
func (h *actionDisplayTooltipHover) hideViewTooltip() {
	if h.tooltipPanel == nil || h.tooltipPanel.editing {
		return
	}
	h.cancelPending()
	h.cancelCapture()
	h.clearTooltipPanelGeometryCache()
	h.removeTooltipFromLayer()
	releaseActionViewTooltip(h)
}

func (h *actionDisplayTooltipHover) tooltipPinned() bool {
	return h.tooltipPanel != nil && h.tooltipPanel.editing
}

func (h *actionDisplayTooltipHover) setBackdropDismissEnabled(enabled bool) {
	h.backdropDismissEnabled = enabled
}

func (h *actionDisplayTooltipHover) suspendBackdropDismissForPicker(onClosed func()) func() {
	h.setBackdropDismissEnabled(false)
	if onClosed == nil {
		return func() { h.setBackdropDismissEnabled(true) }
	}
	return func() {
		h.setBackdropDismissEnabled(true)
		onClosed()
	}
}

func (h *actionDisplayTooltipHover) MouseMoved(e *desktop.MouseEvent) {
	h.trackMouseForTooltip(e)
}

func (h *actionDisplayTooltipHover) TappedSecondary(*fyne.PointEvent) {
	h.openTooltipEdit()
}

func (h *actionDisplayTooltipHover) openTooltipEdit() {
	if h.actionTooltipsSuppressed() {
		return
	}
	if h.absoluteMousePos == (fyne.Position{}) {
		h.absoluteMousePos = fyne.CurrentApp().Driver().AbsolutePositionForObject(h)
	}
	h.showTooltipPanel()
	if h.tooltipPanel == nil {
		return
	}
	if !h.tooltipPanel.editing {
		h.tooltipPanel.enterEditMode()
	}
	if h.previewLoader != nil {
		h.startPreviewCapture()
	}
}

func (h *actionDisplayTooltipHover) cancelPending() {
	if h.pendingCancel != nil {
		h.pendingCancel()
		h.pendingCancel = nil
		h.pendingCtx = nil
	}
}

func (h *actionDisplayTooltipHover) cancelCapture() {
	if h.captureCancel != nil {
		h.captureCancel()
		h.captureCancel = nil
		h.captureCtx = nil
		custom_widgets.RevokeActivePreviewCapture()
	}
}

func (h *actionDisplayTooltipHover) windowTooltipLayer(c fyne.Canvas) *custom_widgets.ItemTooltipLayer {
	return custom_widgets.FindItemTooltipLayer(c, nil)
}

func (h *actionDisplayTooltipHover) windowTooltipOrigin(layer *custom_widgets.ItemTooltipLayer) fyne.Position {
	return custom_widgets.ItemTooltipLayerOrigin(layer, nil)
}

func (h *actionDisplayTooltipHover) hideTooltip() {
	if h.tooltipPanel == nil {
		return
	}
	panel := h.tooltipPanel
	h.cancelPending()
	h.cancelCapture()
	h.clearPreviewCache()
	if panel.editing {
		panel.exitEditMode()
	}
	panel.clearPreview()
	panel.deactivateEnterSave()
	h.removeTooltipFromLayer()
	h.tooltipPanel = nil
	releaseActionViewTooltip(h)
}

func (h *actionDisplayTooltipHover) removeTooltipFromLayer() {
	panel := h.tooltipPanel
	if panel == nil {
		return
	}
	c := h.hoverCanvas()
	if c == nil {
		return
	}
	layer := h.windowTooltipLayer(c)
	if layer == nil {
		return
	}
	var remaining []fyne.CanvasObject
	if h.dismissBackdrop != nil {
		remaining = custom_widgets.RemoveLayerObject(layer, h.dismissBackdrop)
	}
	remaining = custom_widgets.RemoveLayerObject(layer, panel)
	if len(remaining) == 0 {
		custom_widgets.DeactivateTooltipEscapeDismiss()
	}
	h.tooltipMounted = false
	h.clearTooltipLayerCache()
}

func (h *actionDisplayTooltipHover) showTooltipPanel() {
	if h.actionTooltipsSuppressed() || actionTooltipEditPinnedByOther(h) {
		return
	}
	h.cancelPending()
	if h.tooltipPanel != nil {
		if h.tooltipPanel.editing {
			h.syncTooltipLayer()
			return
		}
		h.tooltipPanel.refreshViewContent(h)
	} else {
		h.tooltipPanel = newActionDisplayTooltipPanel(h)
	}
	if h.previewLoader != nil {
		switch {
		case h.previewCacheReady:
			h.applyPreviewCache(h.tooltipPanel)
		case !h.previewCaptureInFlight():
			h.tooltipPanel.setPreviewLoading()
		}
	}
	c := h.hoverCanvas()
	if c == nil {
		return
	}
	layer, _ := h.tooltipLayerOn(c)
	if layer == nil {
		return
	}
	if h.tooltipPinned() {
		claimActionEditTooltip(h)
	} else {
		claimActionViewTooltip(h)
	}
	h.placeTooltipPanel(c, layer, h.tooltipPanel)
	custom_widgets.ActivateTooltipEscapeDismiss(func() { h.hideViewTooltip() })
}

func (h *actionDisplayTooltipHover) placeTooltipPanel(c fyne.Canvas, layer *custom_widgets.ItemTooltipLayer, panel *actionDisplayTooltipPanel) {
	h.tooltipPanel = panel
	h.syncTooltipLayer()
}

func (h *actionDisplayTooltipHover) relayoutTooltip() {
	if h.tooltipPanel != nil {
		h.tooltipPanel.invalidateLayoutSize()
	}
	h.syncTooltipLayer()
}

// refreshTooltipLayout recomputes tooltip size/position without rebuilding the overlay layer.
func (h *actionDisplayTooltipHover) refreshTooltipLayout() {
	if h.tooltipPanel != nil {
		h.tooltipPanel.invalidateLayoutSize()
	}
	h.repositionTooltip()
}

func (h *actionDisplayTooltipHover) repositionTooltip() {
	if h.tooltipPanel == nil {
		return
	}
	c := h.hoverCanvas()
	if c == nil {
		return
	}
	_, origin := h.tooltipLayerOn(c)
	h.refreshTooltipPanelGeometry(origin, h.tooltipPanel.Position(), h.tooltipPanel.Size())
	size, relPos := h.tooltipPosition(c, origin)
	curSize := h.tooltipPanel.Size()
	curPos := h.tooltipPanel.Position()
	if fyneSizesClose(curSize, size) && fynePositionsClose(curPos, relPos) {
		return
	}
	if !fyneSizesClose(curSize, size) {
		h.tooltipPanel.Resize(size)
	}
	if !fynePositionsClose(curPos, relPos) {
		h.tooltipPanel.Move(relPos)
	}
	h.refreshTooltipPanelGeometry(origin, relPos, size)
}

func (h *actionDisplayTooltipHover) tooltipPosition(c fyne.Canvas, origin fyne.Position) (fyne.Size, fyne.Position) {
	size := h.tooltipPanel.measureLayoutSize(c)
	mousePos := h.absoluteMousePos.Subtract(origin)
	return size, actionDisplayTooltipPosition(c, mousePos, size)
}

func fyneSizesClose(a, b fyne.Size) bool {
	return abs32(a.Width-b.Width) <= tooltipGeometryEpsilon &&
		abs32(a.Height-b.Height) <= tooltipGeometryEpsilon
}

func fynePositionsClose(a, b fyne.Position) bool {
	return abs32(a.X-b.X) <= tooltipGeometryEpsilon &&
		abs32(a.Y-b.Y) <= tooltipGeometryEpsilon
}

func abs32(v float32) float32 {
	if v < 0 {
		return -v
	}
	return v
}

func actionDisplayTooltipSizeAndPosition(panel *actionDisplayTooltipPanel, c fyne.Canvas, mousePos fyne.Position) (fyne.Size, fyne.Position) {
	size := panel.measureLayoutSize(c)
	return size, actionDisplayTooltipPosition(c, mousePos, size)
}

func actionDisplayTooltipPosition(c fyne.Canvas, mousePos fyne.Position, size fyne.Size) fyne.Position {
	canvasSize := c.Size()
	edgeMarginX := canvasSize.Width * custom_widgets.TooltipEdgeMarginFraction
	edgeMarginY := canvasSize.Height * custom_widgets.TooltipEdgeMarginFraction

	pos := mousePos
	if rightEdge := pos.X + size.Width; rightEdge > canvasSize.Width-edgeMarginX {
		pos.X -= rightEdge - canvasSize.Width + edgeMarginX
	}
	if pos.X < edgeMarginX {
		pos.X = edgeMarginX
	}
	const belowMouseDist = 16
	const aboveMouseDist = 8
	if bottomEdge := pos.Y + size.Height + belowMouseDist; bottomEdge > canvasSize.Height-edgeMarginY {
		pos.Y -= size.Height + aboveMouseDist
	} else {
		pos.Y += belowMouseDist
	}
	if pos.Y < edgeMarginY {
		pos.Y = edgeMarginY
	}
	return pos
}

func (h *actionDisplayTooltipHover) syncTooltipLayer() {
	if h.tooltipPanel == nil {
		return
	}
	c := h.hoverCanvas()
	if c == nil {
		return
	}
	layer, _ := h.tooltipLayerOn(c)
	if layer == nil {
		return
	}
	h.repositionTooltip()

	var objects []fyne.CanvasObject
	if h.tooltipPinned() {
		if h.dismissBackdrop == nil {
			h.dismissBackdrop = custom_widgets.NewTooltipDismissBackdrop(func() {
				if !h.backdropDismissEnabled {
					return
				}
				h.hideTooltip()
			})
		}
		layerSize := layer.Container.Size()
		if layerSize.Width == 0 || layerSize.Height == 0 {
			layerSize = c.Size()
		}
		h.dismissBackdrop.Resize(layerSize)
		h.dismissBackdrop.Move(fyne.NewPos(0, 0))
		objects = append(objects, h.dismissBackdrop)
	}
	objects = append(objects, h.tooltipPanel)
	if layerObjectsChanged(layer.Container.Objects, objects) {
		layer.Container.Objects = objects
		layer.Container.Refresh()
	}
	h.tooltipMounted = true
	if h.tooltipPinned() {
		custom_widgets.ActivateTooltipEscapeDismiss(func() { h.hideTooltip() })
	}
}

func layerObjectsChanged(current, next []fyne.CanvasObject) bool {
	if len(current) != len(next) {
		return true
	}
	for i := range current {
		if current[i] != next[i] {
			return true
		}
	}
	return false
}

func (h *actionDisplayTooltipHover) beginPreviewCapture() {
	if actionTooltipEditPinnedByOther(h) {
		return
	}
	if h.tooltipPanel == nil || !h.isTooltipMounted() {
		h.showTooltipPanel()
	}
	if h.tooltipPanel == nil {
		return
	}
	h.startPreviewCapture()
}

func (h *actionDisplayTooltipHover) startPreviewCapture() {
	h.capturePreview(false)
}

func (h *actionDisplayTooltipHover) capturePreview(force bool) {
	if h.previewLoader == nil || h.tooltipPanel == nil {
		return
	}
	if !force {
		if h.previewCaptureInFlight() {
			return
		}
		if h.previewCacheReady {
			h.applyPreviewCache(h.tooltipPanel)
			h.repositionTooltip()
			return
		}
	} else {
		h.clearPreviewCache()
	}

	panel := h.tooltipPanel
	c := h.hoverCanvas()
	if c == nil {
		return
	}

	h.cancelCapture()
	panel.setPreviewLoading()
	panel.Refresh()
	h.repositionTooltip()

	load := h.previewLoader
	ctx, cancel := context.WithCancel(context.Background())
	h.captureCtx = ctx
	h.captureCancel = cancel
	go func() {
		if !custom_widgets.AcquirePreviewCaptureSlot(ctx) {
			return
		}
		defer custom_widgets.ReleasePreviewCaptureSlot()
		if ctx.Err() != nil {
			return
		}
		result, err := load()
		if ctx.Err() != nil {
			return
		}
		fyne.Do(func() {
			if ctx.Err() != nil || h.captureCtx != ctx || h.tooltipPanel != panel {
				return
			}
			h.previewCache = result
			h.previewCacheErr = err
			h.previewCacheReady = true
			if !h.shouldKeepViewTooltip() {
				return
			}
			h.captureCancel = nil
			h.captureCtx = nil
			if err != nil {
				panel.setPreviewError(err.Error())
			} else {
				panel.setPreviewImage(result.Image, result.Caption)
			}
			panel.Refresh()
			h.repositionTooltip()
		})
	}()
}

func (h *actionDisplayTooltipHover) reloadPreview() {
	if h.previewLoader == nil || h.tooltipPanel == nil {
		return
	}
	h.capturePreview(true)
}

func (h *actionDisplayTooltipHover) exitEditMode() {
	if h.tooltipPanel == nil {
		return
	}
	h.tooltipPanel.exitEditMode()
	h.relayoutTooltip()
}

func (h *actionDisplayTooltipHover) refreshViewPills() {
	if h.tooltipPanel == nil {
		return
	}
	h.extra = nil
	_, extra, _ := actiondisplay.DisplayFromParams(h.node.Params(), macroKnownVariables())
	if len(extra) > 0 {
		h.extra = append([]actions.Param(nil), extra...)
	}
	h.tooltipPanel.refreshViewContent(h)
}

func (h *actionDisplayTooltipHover) CreateRenderer() fyne.WidgetRenderer {
	return &actionDisplayTooltipHoverRenderer{hover: h, hit: canvas.NewRectangle(color.Transparent)}
}

type actionDisplayTooltipHoverRenderer struct {
	hover *actionDisplayTooltipHover
	hit   *canvas.Rectangle
}

func (r *actionDisplayTooltipHoverRenderer) Layout(size fyne.Size) {
	contentSize := r.hover.content.MinSize()
	r.hover.content.Resize(contentSize)
	r.hover.content.Move(fyne.NewPos(0, 0))
	r.hit.Resize(contentSize)
}

func (r *actionDisplayTooltipHoverRenderer) MinSize() fyne.Size {
	return r.hover.content.MinSize()
}

func (r *actionDisplayTooltipHoverRenderer) Refresh() {}

func (r *actionDisplayTooltipHoverRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.hit, r.hover.content}
}

func (r *actionDisplayTooltipHoverRenderer) Destroy() {}

type actionDisplayTooltipPanel struct {
	fynewidget.BaseWidget

	owner       *actionDisplayTooltipHover
	withPreview bool
	actionType  string
	extra       []actions.Param

	editing  bool
	editForm *tooltipEditForm

	enterSaveUnregister func()

	viewParamPills fyne.CanvasObject
	viewParamPillsKey string

	viewParamPillsBodyIndex int
	viewBodyBuilt           bool

	img       *canvas.Image
	message   *fynewidget.Label
	caption   *fynewidget.Label
	loading   bool
	showImage bool

	body *fyne.Container

	hoverTipLayer  *fyne.Container
	activeHoverTip fyne.CanvasObject

	// layoutSize caches tooltip dimensions; row-wrapped target icons make
	// preferredContentWidth/contentSize O(n) and run on every mouse move without this.
	layoutSize          fyne.Size
	layoutCanvasWidth   float32
	layoutSizeOK        bool

	targetIconsSection fyne.CanvasObject
	targetIconsKey     string
}

var _ fyne.SecondaryTappable = (*actionDisplayTooltipPanel)(nil)

func newActionDisplayTooltipPanel(owner *actionDisplayTooltipHover) *actionDisplayTooltipPanel {
	p := &actionDisplayTooltipPanel{
		owner:       owner,
		withPreview: owner.previewLoader != nil,
		actionType:  owner.actionType,
	}
	if len(owner.extra) > 0 {
		p.extra = append([]actions.Param(nil), owner.extra...)
	}
	p.ensureViewParamPills(owner)
	p.body = container.NewVBox()
	p.hoverTipLayer = container.NewWithoutLayout()
	p.rebuildBody()
	p.ExtendBaseWidget(p)
	return p
}

func (p *actionDisplayTooltipPanel) ShowTooltip(text string, absPos fyne.Position) {
	p.HideTooltip()
	if text == "" {
		return
	}
	tip := actiondisplay.NewPillHoverTipPanel(text)
	actiondisplay.PositionPillHoverTip(tip, fyne.CurrentApp().Driver().AbsolutePositionForObject(p), absPos)
	p.hoverTipLayer.Add(tip)
	p.activeHoverTip = tip
	p.hoverTipLayer.Refresh()
}

func (p *actionDisplayTooltipPanel) HideTooltip() {
	if p.hoverTipLayer == nil {
		return
	}
	p.hoverTipLayer.Objects = nil
	p.hoverTipLayer.Refresh()
	p.activeHoverTip = nil
}

func (p *actionDisplayTooltipPanel) ensureViewParamPills(owner *actionDisplayTooltipHover) fyne.CanvasObject {
	if owner == nil {
		return nil
	}
	key := viewParamPillsContentKey(owner.node)
	if p.viewParamPills != nil && p.viewParamPillsKey == key {
		return p.viewParamPills
	}
	p.viewParamPillsKey = key
	p.viewParamPills = viewParamPills(owner.node, owner.actionType)
	return p.viewParamPills
}

func (p *actionDisplayTooltipPanel) refreshViewContent(owner *actionDisplayTooltipHover) {
	p.ensureViewParamPills(owner)
	if !p.editing && p.viewBodyBuilt && p.viewParamPillsBodyIndex >= 0 {
		if p.viewParamPillsBodyIndex < len(p.body.Objects) {
			p.body.Objects[p.viewParamPillsBodyIndex] = p.viewParamPills
		}
		return
	}
	if !p.editing {
		p.rebuildBody()
		p.Refresh()
	}
}

func (p *actionDisplayTooltipPanel) ensureTargetIconsSection(owner *actionDisplayTooltipHover) fyne.CanvasObject {
	if owner == nil {
		return nil
	}
	targets := imageSearchTargetsFromNode(owner.node)
	if len(targets) == 0 {
		p.targetIconsSection = nil
		p.targetIconsKey = ""
		return nil
	}
	key := imageSearchTargetIconsViewKey(targets)
	if p.targetIconsSection != nil && p.targetIconsKey == key {
		return p.targetIconsSection
	}
	p.targetIconsKey = key
	p.targetIconsSection = imageSearchTargetIconsView(targets)
	return p.targetIconsSection
}

func (p *actionDisplayTooltipPanel) enterEditMode() {
	if p.editing || p.owner == nil {
		return
	}
	p.owner.cancelPending()
	p.editing = true
	claimActionEditTooltip(p.owner)
	p.editForm = buildTooltipEditForm(p.owner.node, p.actionType, p.owner)
	p.activateEnterSave()
	p.rebuildBody()
	p.Refresh()
	p.owner.relayoutTooltip()
}

func (p *actionDisplayTooltipPanel) exitEditMode() {
	if !p.editing {
		return
	}
	p.HideTooltip()
	p.deactivateEnterSave()
	p.editing = false
	if p.owner != nil {
		releaseActionEditTooltip(p.owner)
	}
	p.editForm = nil
	p.refreshViewContent(p.owner)
	p.rebuildBody()
	p.Refresh()
}

func (p *actionDisplayTooltipPanel) activateEnterSave() {
	p.deactivateEnterSave()
	if activeWire.RegisterTooltipEnterSave == nil {
		return
	}
	panel := p
	p.enterSaveUnregister = activeWire.RegisterTooltipEnterSave(func() {
		panel.submitEdit()
	})
}

func (p *actionDisplayTooltipPanel) deactivateEnterSave() {
	if p.enterSaveUnregister != nil {
		p.enterSaveUnregister()
		p.enterSaveUnregister = nil
	}
}

func (p *actionDisplayTooltipPanel) submitEdit() {
	if !p.editing || p.editForm == nil || p.owner == nil {
		return
	}
	if err := p.editForm.saveAction(p.owner); err != nil {
		if activeWire.ShowErrorWithEscape != nil && activeWire.Window != nil {
			activeWire.ShowErrorWithEscape(err, activeWire.Window)
		}
		return
	}
	p.owner.exitEditMode()
}

func (p *actionDisplayTooltipPanel) invalidateLayoutSize() {
	p.layoutSizeOK = false
}

func (p *actionDisplayTooltipPanel) measureLayoutSize(c fyne.Canvas) fyne.Size {
	canvasWidth := c.Size().Width
	if p.layoutSizeOK && p.layoutCanvasWidth == canvasWidth {
		return p.layoutSize
	}
	canvasSize := c.Size()
	edgeMarginX := canvasSize.Width * custom_widgets.TooltipEdgeMarginFraction
	maxW := canvasSize.Width - edgeMarginX*2

	natural := p.MinSize()
	width := natural.Width
	if preferred := p.preferredContentWidth(); preferred > width {
		width = preferred
	}
	if width > maxW {
		width = maxW
	}
	p.layoutSize = p.contentSize(width)
	p.layoutCanvasWidth = canvasWidth
	p.layoutSizeOK = true
	return p.layoutSize
}

func (p *actionDisplayTooltipPanel) rebuildBody() {
	p.invalidateLayoutSize()
	p.viewBodyBuilt = false
	p.viewParamPillsBodyIndex = -1
	p.body.Objects = nil
	if p.editing && p.editForm != nil {
		if p.editForm.toolbar != nil {
			p.body.Add(p.editForm.toolbar)
		}
	} else if header := actionTooltipTypeHeader(p.actionType); header != nil {
		p.body.Add(header)
	}
	if p.withPreview {
		if p.img == nil {
			p.img = canvas.NewImageFromImage(nil)
			p.img.FillMode = desktopview.PreviewSnapshotFill
			p.img.SetMinSize(p.previewSize())
		}
		if p.message == nil {
			p.message = fynewidget.NewLabel("Loading preview…")
			p.message.Wrapping = fyne.TextWrapWord
			p.message.Alignment = fyne.TextAlignCenter
		}
		if p.caption == nil {
			p.caption = fynewidget.NewLabel("")
			p.caption.Alignment = fyne.TextAlignCenter
			p.caption.Hide()
		}
		imageStack := container.NewStack(
			container.NewMax(p.img),
			container.NewPadded(p.message),
		)
		previewSection := container.NewVBox(imageStack)
		if p.caption != nil {
			previewSection.Add(p.caption)
		}
		if p.editing && p.editForm != nil && p.editForm.coordEditActions != nil {
			previewSection.Add(p.editForm.coordEditActions)
		}
		p.body.Add(wrapTooltipSection(previewSection))
	}
	if p.owner != nil {
		if p.editing && p.editForm != nil && p.editForm.targetItems != nil {
			p.body.Add(p.editForm.targetItems)
		} else if section := p.ensureTargetIconsSection(p.owner); section != nil {
			p.body.Add(section)
		}
	}
	if p.editing && p.editForm != nil {
		if p.editForm.paramPills != nil {
			p.body.Add(p.editForm.paramPills)
		}
	} else {
		if p.viewParamPills != nil {
			p.viewParamPillsBodyIndex = len(p.body.Objects)
			p.body.Add(p.viewParamPills)
		}
	}
	if p.editing && p.editForm != nil {
		actiondisplay.BindPillStepperTooltips(p.body, p)
	} else {
		p.HideTooltip()
		p.viewBodyBuilt = true
	}
	p.body.Refresh()
}

func (p *actionDisplayTooltipPanel) TappedSecondary(*fyne.PointEvent) {
	if !p.editing {
		p.enterEditMode()
	}
}

func (p *actionDisplayTooltipPanel) previewSize() fyne.Size {
	return fyne.NewSize(config.ImagePreviewMinWidth, config.ImagePreviewMinHeight)
}

func (p *actionDisplayTooltipPanel) MinSize() fyne.Size {
	innerPad := p.Theme().Size(theme.SizeNameInnerPadding)
	size := p.body.MinSize()
	if p.withPreview {
		previewMin := p.previewSize()
		if size.Width < previewMin.Width {
			size.Width = previewMin.Width
		}
		if p.showImage {
			captionHeight := float32(0)
			if p.caption != nil && p.caption.Text != "" {
				captionHeight = p.caption.MinSize().Height
			}
			if p.editing && p.editForm != nil && p.editForm.coordEditActions != nil {
				captionHeight += p.editForm.coordEditActions.MinSize().Height
			}
			if captionHeight > 0 && previewMin.Height > size.Height {
				size.Height = previewMin.Height
			}
		}
	}
	return size.Add(fyne.NewSquareSize(innerPad * 2))
}

func (p *actionDisplayTooltipPanel) preferredContentWidth() float32 {
	innerPad := p.Theme().Size(theme.SizeNameInnerPadding)
	w := maxRowWrapSingleLineWidth(p.body) + tooltipSectionChromeWidth()
	if p.withPreview {
		if previewMin := p.previewSize().Width; previewMin > w {
			w = previewMin
		}
	}
	return w + innerPad*2
}

func (p *actionDisplayTooltipPanel) contentSize(width float32) fyne.Size {
	innerPad := p.Theme().Size(theme.SizeNameInnerPadding)
	innerW := width - innerPad*2
	if innerW < 1 {
		innerW = 1
	}
	contentH := tooltipBodyHeightAtWidth(p.body, innerW)
	if p.withPreview && p.showImage {
		previewMin := p.previewSize()
		if contentH < previewMin.Height {
			contentH = previewMin.Height
		}
	}
	return fyne.NewSize(width, contentH+innerPad*2)
}

func (p *actionDisplayTooltipPanel) clearPreview() {
	p.loading = false
	p.showImage = false
	if p.img != nil {
		p.img.Image = nil
	}
	if p.message != nil {
		p.message.SetText("")
	}
	if p.caption != nil {
		p.caption.SetText("")
		p.caption.Hide()
	}
}

func (p *actionDisplayTooltipPanel) setPreviewLoading() {
	p.loading = true
	p.showImage = false
	if p.img != nil {
		p.img.Image = nil
	}
	if p.message != nil {
		p.message.SetText("Loading preview…")
	}
	if p.caption != nil {
		p.caption.SetText("")
		p.caption.Hide()
	}
}

func (p *actionDisplayTooltipPanel) setPreviewError(msg string) {
	p.loading = false
	p.showImage = false
	if p.message != nil {
		p.message.SetText(msg)
	}
	if p.caption != nil {
		p.caption.SetText("")
		p.caption.Hide()
	}
}

func (p *actionDisplayTooltipPanel) setPreviewImage(img image.Image, caption string) {
	hadCaption := p.caption != nil && p.caption.Text != ""
	p.loading = false
	p.showImage = true
	if p.img != nil {
		p.img.Image = img
	}
	if p.caption != nil {
		p.caption.SetText(caption)
		if caption == "" {
			p.caption.Hide()
		} else {
			p.caption.Show()
		}
	}
	if (hadCaption && caption == "") || (!hadCaption && caption != "") {
		p.invalidateLayoutSize()
	}
}

func (p *actionDisplayTooltipPanel) CreateRenderer() fyne.WidgetRenderer {
	v := fyne.CurrentApp().Settings().ThemeVariant()
	th := p.Theme()
	bg := canvas.NewRectangle(th.Color(theme.ColorNameOverlayBackground, v))
	bg.StrokeColor = th.Color(theme.ColorNameInputBorder, v)
	bg.StrokeWidth = th.Size(theme.SizeNameInputBorder)
	bg.CornerRadius = 4
	bodyStack := container.NewStack(p.body, p.hoverTipLayer)
	return &actionDisplayTooltipPanelRenderer{
		panel: p,
		bg:    bg,
		body:  bodyStack,
	}
}

type actionDisplayTooltipPanelRenderer struct {
	panel *actionDisplayTooltipPanel
	bg    *canvas.Rectangle
	body  *fyne.Container
}

func (r *actionDisplayTooltipPanelRenderer) Layout(size fyne.Size) {
	r.bg.Resize(size)
	r.bg.Move(fyne.NewPos(0, 0))
	innerPad := r.panel.Theme().Size(theme.SizeNameInnerPadding)
	innerSize := size.Subtract(fyne.NewSquareSize(innerPad * 2))
	r.body.Resize(innerSize)
	r.body.Move(fyne.NewPos(innerPad, innerPad))
	layout.NewStackLayout().Layout(r.body.Objects, innerSize)
	layout.NewVBoxLayout().Layout(r.panel.body.Objects, innerSize)
	r.panel.hoverTipLayer.Resize(innerSize)
	if r.panel.withPreview {
		if r.panel.showImage {
			r.panel.img.Show()
			r.panel.message.Hide()
		} else {
			r.panel.img.Hide()
			r.panel.message.Show()
		}
	}
}

func (r *actionDisplayTooltipPanelRenderer) MinSize() fyne.Size {
	return r.panel.MinSize()
}

func (r *actionDisplayTooltipPanelRenderer) Refresh() {
	th := r.panel.Theme()
	v := fyne.CurrentApp().Settings().ThemeVariant()
	r.bg.FillColor = th.Color(theme.ColorNameOverlayBackground, v)
	r.bg.StrokeColor = th.Color(theme.ColorNameInputBorder, v)
	r.bg.StrokeWidth = th.Size(theme.SizeNameInputBorder)
	if r.panel.withPreview {
		if r.panel.showImage {
			r.panel.img.Show()
			r.panel.message.Hide()
			if r.panel.caption.Text != "" {
				r.panel.caption.Show()
			} else {
				r.panel.caption.Hide()
			}
		} else {
			r.panel.img.Hide()
			r.panel.message.Show()
			r.panel.caption.Hide()
		}
		r.panel.img.Refresh()
		r.panel.message.Refresh()
		r.panel.caption.Refresh()
	}
	r.bg.Refresh()
	r.body.Refresh()
}

func (r *actionDisplayTooltipPanelRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.bg, r.body}
}

func (r *actionDisplayTooltipPanelRenderer) Destroy() {}

// actionIconTooltipHover is an invisible overlay on the tree row action icon.
// It shows the same rich action tooltip as hovering the action display.
type actionIconTooltipHover struct {
	fynewidget.BaseWidget

	target *actionDisplayTooltipHover
}

var (
	_ desktop.Hoverable      = (*actionIconTooltipHover)(nil)
	_ fyne.SecondaryTappable = (*actionIconTooltipHover)(nil)
)

func newActionIconTooltipHover() *actionIconTooltipHover {
	h := &actionIconTooltipHover{}
	h.ExtendBaseWidget(h)
	return h
}

func (h *actionIconTooltipHover) bindActionTooltip(target *actionDisplayTooltipHover) {
	h.target = target
}

func (h *actionIconTooltipHover) MouseIn(e *desktop.MouseEvent) {
	if h.target != nil {
		h.target.iconMouseIn(e)
	}
}

func (h *actionIconTooltipHover) MouseOut() {
	if h.target != nil {
		h.target.iconMouseOut()
	}
}

func (h *actionIconTooltipHover) MouseMoved(e *desktop.MouseEvent) {
	if h.target != nil {
		h.target.iconMouseMoved(e)
	}
}

func (h *actionIconTooltipHover) TappedSecondary(*fyne.PointEvent) {
	if h.target != nil {
		h.target.openTooltipEdit()
	}
}

// actionRowTooltipHover is an invisible overlay across the full macro tree action row.
type actionRowTooltipHover struct {
	fynewidget.BaseWidget

	target *actionDisplayTooltipHover
}

var (
	_ desktop.Hoverable      = (*actionRowTooltipHover)(nil)
	_ fyne.SecondaryTappable = (*actionRowTooltipHover)(nil)
)

func newActionRowTooltipHover() *actionRowTooltipHover {
	h := &actionRowTooltipHover{}
	h.ExtendBaseWidget(h)
	return h
}

func (h *actionRowTooltipHover) bindActionTooltip(target *actionDisplayTooltipHover) {
	h.target = target
}

func (h *actionRowTooltipHover) MouseIn(e *desktop.MouseEvent) {
	if h.target != nil {
		h.target.rowMouseIn(e)
	}
}

func (h *actionRowTooltipHover) MouseOut() {
	if h.target != nil {
		h.target.rowMouseOut()
	}
}

func (h *actionRowTooltipHover) MouseMoved(e *desktop.MouseEvent) {
	if h.target != nil {
		h.target.rowMouseMoved(e)
	}
}

func (h *actionRowTooltipHover) TappedSecondary(*fyne.PointEvent) {
	if h.target != nil {
		h.target.openTooltipEdit()
	}
}

func (h *actionRowTooltipHover) CreateRenderer() fyne.WidgetRenderer {
	return &actionRowTooltipHoverRenderer{hover: h, hit: canvas.NewRectangle(color.Transparent)}
}

type actionRowTooltipHoverRenderer struct {
	hover *actionRowTooltipHover
	hit   *canvas.Rectangle
}

func (r *actionRowTooltipHoverRenderer) Layout(size fyne.Size) {
	r.hit.Resize(size)
}

func (r *actionRowTooltipHoverRenderer) MinSize() fyne.Size {
	return fyne.NewSize(0, 0)
}

func (r *actionRowTooltipHoverRenderer) Refresh() {}

func (r *actionRowTooltipHoverRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.hit}
}

func (r *actionRowTooltipHoverRenderer) Destroy() {}

func (h *actionIconTooltipHover) CreateRenderer() fyne.WidgetRenderer {
	return &actionIconTooltipHoverRenderer{hover: h, hit: canvas.NewRectangle(color.Transparent)}
}

type actionIconTooltipHoverRenderer struct {
	hover *actionIconTooltipHover
	hit   *canvas.Rectangle
}

func (r *actionIconTooltipHoverRenderer) Layout(size fyne.Size) {
	r.hit.Resize(size)
}

func (r *actionIconTooltipHoverRenderer) MinSize() fyne.Size {
	return fyne.NewSize(0, 0)
}

func (r *actionIconTooltipHoverRenderer) Refresh() {}

func (r *actionIconTooltipHoverRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.hit}
}

func (r *actionIconTooltipHoverRenderer) Destroy() {}
