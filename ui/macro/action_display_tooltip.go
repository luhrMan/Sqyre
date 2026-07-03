package macro

import (
	"context"
	"image"
	"image/color"
	"time"

	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/desktopview"

	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
	fynewidget "fyne.io/fyne/v2/widget"
)

const actionDisplayTooltipShowDelay = 500 * time.Millisecond

func actionDisplay(node actions.ActionInterface) fyne.CanvasObject {
	line, extra, actionType := actiondisplay.DisplayFromParams(node.Params())
	loader := actionPreviewLoader(node)
	if loader == nil && len(extra) == 0 {
		return line
	}
	return newActionDisplayTooltipHover(line, extra, actionType, loader)
}

type actionDisplayTooltipHover struct {
	fynewidget.BaseWidget

	content       fyne.CanvasObject
	extra         []actions.Param
	actionType    string
	previewLoader custom_widgets.PreviewTooltipLoad

	tooltipPanel     *actionDisplayTooltipPanel
	pendingCancel    context.CancelFunc
	pendingCtx       context.Context
	captureCancel    context.CancelFunc
	captureCtx       context.Context
	absoluteMousePos fyne.Position
	hovering         bool
}

var _ desktop.Hoverable = (*actionDisplayTooltipHover)(nil)

func newActionDisplayTooltipHover(content fyne.CanvasObject, extra []actions.Param, actionType string, loader custom_widgets.PreviewTooltipLoad) *actionDisplayTooltipHover {
	h := &actionDisplayTooltipHover{
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

func (h *actionDisplayTooltipHover) MouseIn(e *desktop.MouseEvent) {
	if h.previewLoader == nil && len(h.extra) == 0 {
		return
	}
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
	h.cancelPending()
	h.cancelCapture()
	h.hideTooltip()
}

func (h *actionDisplayTooltipHover) MouseMoved(e *desktop.MouseEvent) {
	h.absoluteMousePos = e.AbsolutePosition
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
	panel := newActionDisplayTooltipPanel(h.extra, h.actionType, h.previewLoader != nil)
	if h.previewLoader != nil {
		panel.setPreviewLoading()
	}
	h.placeTooltipPanel(c, layer, panel)
	h.tooltipPanel = panel
}

func (h *actionDisplayTooltipHover) placeTooltipPanel(c fyne.Canvas, layer *custom_widgets.ItemTooltipLayer, panel *actionDisplayTooltipPanel) {
	origin := custom_widgets.ItemTooltipLayerOrigin(layer, c.Overlays().Top())
	size, relPos := actionDisplayTooltipSizeAndPosition(panel, c, h.absoluteMousePos.Subtract(origin))
	panel.Resize(size)
	panel.Move(relPos)
	layer.Container.Objects = []fyne.CanvasObject{panel}
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
				if h.captureCtx != ctx || !h.hovering {
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

	withPreview bool
	pills       *fyne.Container

	img       *canvas.Image
	message   *fynewidget.Label
	caption   *fynewidget.Label
	loading   bool
	showImage bool
}

func newActionDisplayTooltipPanel(extra []actions.Param, actionType string, withPreview bool) *actionDisplayTooltipPanel {
	p := &actionDisplayTooltipPanel{withPreview: withPreview}
	if len(extra) > 0 {
		pills := container.New(kxlayout.NewRowWrapLayout())
		for _, param := range extra {
			if entry := actions.FormatParamEntry(param); entry != "" {
				pills.Add(actiondisplay.NewDisplayPill(entry, actionType))
			}
		}
		if len(pills.Objects) > 0 {
			p.pills = pills
		}
	}
	p.ExtendBaseWidget(p)
	return p
}

func (p *actionDisplayTooltipPanel) previewSize() fyne.Size {
	return fyne.NewSize(config.ImagePreviewMinWidth, config.ImagePreviewMinHeight)
}

func (p *actionDisplayTooltipPanel) MinSize() fyne.Size {
	innerPad := p.Theme().Size(theme.SizeNameInnerPadding)
	var size fyne.Size
	if p.withPreview {
		size = p.previewSize()
		if p.showImage && p.caption != nil && p.caption.Text != "" {
			size.Height += p.caption.MinSize().Height + innerPad/2
		}
	}
	if p.pills != nil {
		pillSize := p.pills.MinSize()
		if size.Width < pillSize.Width {
			size.Width = pillSize.Width
		}
		if p.withPreview {
			size.Height += pillSize.Height + innerPad
		} else {
			size = pillSize
		}
	}
	return size.Add(fyne.NewSquareSize(innerPad * 2))
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

	var sections []fyne.CanvasObject
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
		sections = append(sections, imageStack, p.caption)
	}
	if p.pills != nil {
		sections = append(sections, container.NewPadded(p.pills))
	}
	content := container.NewVBox(sections...)
	return &actionDisplayTooltipPanelRenderer{
		panel:   p,
		bg:      bg,
		content: content,
	}
}

type actionDisplayTooltipPanelRenderer struct {
	panel   *actionDisplayTooltipPanel
	bg      *canvas.Rectangle
	content *fyne.Container
}

func (r *actionDisplayTooltipPanelRenderer) Layout(size fyne.Size) {
	r.bg.Resize(size)
	r.bg.Move(fyne.NewPos(0, 0))
	innerPad := r.panel.Theme().Size(theme.SizeNameInnerPadding)
	innerSize := size.Subtract(fyne.NewSquareSize(innerPad * 2))
	r.content.Resize(innerSize)
	r.content.Move(fyne.NewPos(innerPad, innerPad))
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
	r.content.Refresh()
}

func (r *actionDisplayTooltipPanelRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.bg, r.content}
}

func (r *actionDisplayTooltipPanelRenderer) Destroy() {}

func actionDisplayTooltipSizeAndPosition(panel *actionDisplayTooltipPanel, c fyne.Canvas, mousePos fyne.Position) (fyne.Size, fyne.Position) {
	canvasSize := c.Size()
	canvasPad := theme.Padding()
	size := panel.MinSize()
	maxW := fyne.Min(canvasSize.Width-canvasPad*2, 480)
	if size.Width > maxW {
		panel.Resize(fyne.NewSize(maxW, size.Height))
		size = panel.MinSize()
	}

	pos := mousePos
	if rightEdge := pos.X + size.Width; rightEdge > canvasSize.Width-canvasPad {
		pos.X -= rightEdge - canvasSize.Width + canvasPad
	}
	if pos.X < canvasPad {
		pos.X = canvasPad
	}
	const belowMouseDist = 16
	const aboveMouseDist = 8
	if bottomEdge := pos.Y + size.Height + belowMouseDist; bottomEdge > canvasSize.Height-canvasPad {
		pos.Y -= size.Height + aboveMouseDist
	} else {
		pos.Y += belowMouseDist
	}
	if pos.Y < canvasPad {
		pos.Y = canvasPad
	}
	return size, pos
}
