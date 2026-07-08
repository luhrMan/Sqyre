package macro

import (
	"context"
	"image/color"

	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/driver/desktop"
	fynewidget "fyne.io/fyne/v2/widget"
)

type actionDisplayTooltipHover struct {
	fynewidget.BaseWidget

	node             actions.ActionInterface
	onActionSaved    func()
	content          fyne.CanvasObject
	extra            []actions.Param
	actionType       string
	previewLoader    custom_widgets.PreviewTooltipLoad
	rowBody          *treeRowBody
	keepAliveArea    fyne.CanvasObject
	keepAliveExclude fyne.CanvasObject

	tooltipPanel           *actionDisplayTooltipPanel
	dismissBackdrop        *custom_widgets.TooltipDismissBackdrop
	pendingCancel          context.CancelFunc
	pendingCtx             context.Context
	captureCancel          context.CancelFunc
	captureCtx             context.Context
	absoluteMousePos       fyne.Position
	displayHovering        bool
	iconHovering           bool
	rowHovering            bool
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

// selectRow selects this action's tree row, keeping tree selection in sync when
// the action enters edit mode (e.g. via right-click, which does not tap the row).
func (h *actionDisplayTooltipHover) selectRow() {
	if h.rowBody == nil || h.rowBody.tree == nil || h.rowBody.uid == "" {
		return
	}
	h.rowBody.tree.Select(h.rowBody.uid)
}

func (h *actionDisplayTooltipHover) setTooltipKeepAliveArea(obj fyne.CanvasObject) {
	h.keepAliveArea = obj
}

func (h *actionDisplayTooltipHover) setTooltipKeepAliveExclude(obj fyne.CanvasObject) {
	h.keepAliveExclude = obj
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

func (h *actionDisplayTooltipHover) shouldFollowMouse() bool {
	if h.tooltipPanel == nil || h.tooltipPinned() {
		return false
	}
	return h.pointerInTreeActionSpace(h.absoluteMousePos)
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
