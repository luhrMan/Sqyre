package macro

import (
	"context"
	"image"
	"image/color"
	"strings"
	"time"

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

const (
	actionDisplayTooltipShowDelay        = 500 * time.Millisecond
	actionDisplayTooltipEdgeMarginFraction float32 = 0.10
)

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

	tooltipPanel     *actionDisplayTooltipPanel
	dismissBackdrop  *custom_widgets.TooltipDismissBackdrop
	pendingCancel    context.CancelFunc
	pendingCtx       context.Context
	captureCancel    context.CancelFunc
	captureCtx       context.Context
	absoluteMousePos fyne.Position
	hovering         bool
	panelHovering    bool
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
		node:          node,
		onActionSaved: onActionSaved,
		content:       content,
		actionType:    actionType,
		previewLoader: loader,
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

func (h *actionDisplayTooltipHover) Tapped(pe *fyne.PointEvent) {
	if h.tooltipPanel != nil && !h.tooltipPanel.editing {
		h.hideTooltip()
	}
	if h.rowBody != nil {
		h.rowBody.Tapped(pe)
	}
}

func (h *actionDisplayTooltipHover) MouseIn(e *desktop.MouseEvent) {
	h.hovering = true
	h.absoluteMousePos = e.AbsolutePosition
	ctx, cancel := context.WithCancel(context.Background())
	h.pendingCtx = ctx
	h.pendingCancel = cancel
	go func() {
		select {
		case <-time.After(actionDisplayTooltipShowDelay):
		case <-ctx.Done():
			return
		}
		select {
		case <-ctx.Done():
			return
		default:
			fyne.Do(func() {
				if h.pendingCtx != ctx || !h.hovering {
					return
				}
				h.pendingCtx = nil
				h.pendingCancel = nil
				if h.previewLoader != nil {
					h.beginPreviewCapture()
				} else {
					h.showTooltipPanel()
				}
			})
		}
	}()
}

func (h *actionDisplayTooltipHover) MouseOut() {
	h.hovering = false
	if h.tooltipPinned() || h.panelHovering {
		return
	}
	h.cancelPending()
	h.cancelCapture()
	h.hideTooltip()
}

func (h *actionDisplayTooltipHover) tooltipPinned() bool {
	return h.tooltipPanel != nil && h.tooltipPanel.editing
}

func (h *actionDisplayTooltipHover) MouseMoved(e *desktop.MouseEvent) {
	h.absoluteMousePos = e.AbsolutePosition
}

func (h *actionDisplayTooltipHover) TappedSecondary(*fyne.PointEvent) {
	if h.tooltipPanel != nil && !h.tooltipPanel.editing {
		h.tooltipPanel.enterEditMode()
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
	}
}

func (h *actionDisplayTooltipHover) hideTooltip() {
	if h.tooltipPanel == nil {
		return
	}
	custom_widgets.DeactivateTooltipEscapeDismiss()
	h.cancelPending()
	h.cancelCapture()
	h.panelHovering = false
	h.tooltipPanel.deactivateEnterSave()
	c := fyne.CurrentApp().Driver().CanvasForObject(h)
	if c == nil {
		h.tooltipPanel = nil
		return
	}
	layer := custom_widgets.FindItemTooltipLayer(c, c.Overlays().Top())
	if layer != nil {
		layer.Container.Objects = nil
		layer.Container.Refresh()
	}
	h.tooltipPanel = nil
}

func (h *actionDisplayTooltipHover) showTooltipPanel() {
	h.hideTooltip()
	c := fyne.CurrentApp().Driver().CanvasForObject(h)
	if c == nil {
		return
	}
	layer := custom_widgets.FindItemTooltipLayer(c, c.Overlays().Top())
	if layer == nil {
		return
	}
	panel := newActionDisplayTooltipPanel(h)
	if h.previewLoader != nil {
		panel.setPreviewLoading()
	}
	h.placeTooltipPanel(c, layer, panel)
	h.tooltipPanel = panel
	custom_widgets.ActivateTooltipEscapeDismiss(func() { h.hideTooltip() })
}

func (h *actionDisplayTooltipHover) placeTooltipPanel(c fyne.Canvas, layer *custom_widgets.ItemTooltipLayer, panel *actionDisplayTooltipPanel) {
	h.tooltipPanel = panel
	h.updateTooltipLayer(c, layer)
}

func (h *actionDisplayTooltipHover) relayoutTooltip() {
	if h.tooltipPanel == nil {
		return
	}
	c := fyne.CurrentApp().Driver().CanvasForObject(h)
	if c == nil {
		return
	}
	layer := custom_widgets.FindItemTooltipLayer(c, c.Overlays().Top())
	if layer == nil {
		return
	}
	h.updateTooltipLayer(c, layer)
}

func (h *actionDisplayTooltipHover) updateTooltipLayer(c fyne.Canvas, layer *custom_widgets.ItemTooltipLayer) {
	if h.tooltipPanel == nil {
		layer.Container.Objects = nil
		layer.Container.Refresh()
		return
	}
	origin := custom_widgets.ItemTooltipLayerOrigin(layer, c.Overlays().Top())
	size, relPos := actionDisplayTooltipSizeAndPosition(h.tooltipPanel, c, h.absoluteMousePos.Subtract(origin))
	h.tooltipPanel.Resize(size)
	h.tooltipPanel.Move(relPos)

	var objects []fyne.CanvasObject
	if h.tooltipPinned() {
		if h.dismissBackdrop == nil {
			h.dismissBackdrop = custom_widgets.NewTooltipDismissBackdrop(func() {
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
	layer.Container.Objects = objects
	layer.Container.Refresh()
}

func (h *actionDisplayTooltipHover) beginPreviewCapture() {
	h.showTooltipPanel()
	load := h.previewLoader
	if load == nil || !h.hovering {
		return
	}
	panel := h.tooltipPanel
	if panel == nil {
		return
	}
	c := fyne.CurrentApp().Driver().CanvasForObject(h)
	if c == nil {
		return
	}
	layer := custom_widgets.FindItemTooltipLayer(c, c.Overlays().Top())
	if layer == nil {
		return
	}
	origin := custom_widgets.ItemTooltipLayerOrigin(layer, c.Overlays().Top())

	ctx, cancel := context.WithCancel(context.Background())
	h.captureCtx = ctx
	h.captureCancel = cancel
	go func() {
		result, err := load()
		select {
		case <-ctx.Done():
			return
		default:
			fyne.Do(func() {
				if h.captureCtx != ctx || (!h.hovering && !h.tooltipPinned()) {
					return
				}
				h.captureCancel = nil
				h.captureCtx = nil
				if h.tooltipPanel != panel {
					return
				}
				if err != nil {
					panel.setPreviewError(err.Error())
				} else {
					panel.setPreviewImage(result.Image, result.Caption)
				}
				panel.Refresh()
				size, relPos := actionDisplayTooltipSizeAndPosition(panel, c, h.absoluteMousePos.Subtract(origin))
				panel.Resize(size)
				panel.Move(relPos)
				layer.Container.Refresh()
			})
		}
	}()
}

func (h *actionDisplayTooltipHover) reloadPreview() {
	if h.previewLoader == nil || h.tooltipPanel == nil {
		return
	}
	panel := h.tooltipPanel
	c := fyne.CurrentApp().Driver().CanvasForObject(h)
	if c == nil {
		return
	}
	layer := custom_widgets.FindItemTooltipLayer(c, c.Overlays().Top())
	if layer == nil {
		return
	}
	origin := custom_widgets.ItemTooltipLayerOrigin(layer, c.Overlays().Top())

	h.cancelCapture()
	panel.setPreviewLoading()
	panel.Refresh()
	size, relPos := actionDisplayTooltipSizeAndPosition(panel, c, h.absoluteMousePos.Subtract(origin))
	panel.Resize(size)
	panel.Move(relPos)
	layer.Container.Refresh()

	load := h.previewLoader
	ctx, cancel := context.WithCancel(context.Background())
	h.captureCtx = ctx
	h.captureCancel = cancel
	go func() {
		result, err := load()
		select {
		case <-ctx.Done():
			return
		default:
			fyne.Do(func() {
				if h.captureCtx != ctx || h.tooltipPanel != panel {
					return
				}
				if !h.hovering && !h.tooltipPinned() {
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
				size, relPos := actionDisplayTooltipSizeAndPosition(panel, c, h.absoluteMousePos.Subtract(origin))
				panel.Resize(size)
				panel.Move(relPos)
				layer.Container.Refresh()
			})
		}
	}()
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

	img       *canvas.Image
	message   *fynewidget.Label
	caption   *fynewidget.Label
	loading   bool
	showImage bool

	body *fyne.Container

	hoverTipLayer  *fyne.Container
	activeHoverTip fyne.CanvasObject
}

var _ desktop.Hoverable = (*actionDisplayTooltipPanel)(nil)
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
	p.viewParamPills = viewParamPills(owner.node, owner.actionType)
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

func (p *actionDisplayTooltipPanel) refreshViewContent(owner *actionDisplayTooltipHover) {
	p.viewParamPills = viewParamPills(owner.node, owner.actionType)
	if !p.editing {
		p.rebuildBody()
		p.Refresh()
	}
}

func (p *actionDisplayTooltipPanel) enterEditMode() {
	if p.editing || p.owner == nil {
		return
	}
	p.editing = true
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

func (p *actionDisplayTooltipPanel) rebuildBody() {
	p.body.Objects = nil
	if p.editing && p.editForm != nil {
		if p.editForm.toolbar != nil {
			p.body.Add(p.editForm.toolbar)
		}
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
		if targets := imageSearchTargetsFromNode(p.owner.node); len(targets) > 0 {
			if p.editing && p.editForm != nil && p.editForm.targetItems != nil {
				p.body.Add(p.editForm.targetItems)
			} else if view := imageSearchTargetIconsView(targets); view != nil {
				p.body.Add(view)
			}
		}
	}
	if p.editing && p.editForm != nil {
		if p.editForm.paramPills != nil {
			p.body.Add(p.editForm.paramPills)
		}
	} else {
		if p.viewParamPills != nil {
			p.body.Add(p.viewParamPills)
		}
	}
	if p.editing && p.editForm != nil {
		actiondisplay.BindPillStepperTooltips(p.body, p)
	} else {
		p.HideTooltip()
	}
	p.body.Refresh()
}

func (p *actionDisplayTooltipPanel) MouseIn(*desktop.MouseEvent) {
	if p.owner != nil {
		p.owner.panelHovering = true
	}
}

func (p *actionDisplayTooltipPanel) MouseOut() {
	if p.owner == nil {
		return
	}
	p.owner.panelHovering = false
	if !p.owner.hovering && !p.editing {
		p.owner.cancelPending()
		p.owner.cancelCapture()
		p.owner.hideTooltip()
	}
}

func (p *actionDisplayTooltipPanel) MouseMoved(*desktop.MouseEvent) {}

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

func (p *actionDisplayTooltipPanel) setPreviewLoading() {
	p.loading = true
	p.showImage = false
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

func actionDisplayTooltipSizeAndPosition(panel *actionDisplayTooltipPanel, c fyne.Canvas, mousePos fyne.Position) (fyne.Size, fyne.Position) {
	canvasSize := c.Size()
	edgeMarginX := canvasSize.Width * actionDisplayTooltipEdgeMarginFraction
	edgeMarginY := canvasSize.Height * actionDisplayTooltipEdgeMarginFraction
	maxW := canvasSize.Width - edgeMarginX*2

	natural := panel.MinSize()
	width := natural.Width
	if preferred := panel.preferredContentWidth(); preferred > width {
		width = preferred
	}
	if width > maxW {
		width = maxW
	}
	size := panel.contentSize(width)

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
	return size, pos
}
