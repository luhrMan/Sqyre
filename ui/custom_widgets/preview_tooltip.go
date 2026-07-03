package custom_widgets

import (
	"context"
	"image"
	"image/color"
	"time"

	"Sqyre/internal/config"
	"Sqyre/ui/desktopview"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
	fynewidget "fyne.io/fyne/v2/widget"
)

const (
	previewTooltipShowDelay = 500 * time.Millisecond
)

// PreviewTooltipResult is the payload for a coordinate preview tooltip.
type PreviewTooltipResult struct {
	Image   image.Image
	Caption string // coordinate summary shown below the image
}

// PreviewTooltipLoad produces preview image and caption text for a hover tooltip.
type PreviewTooltipLoad func() (PreviewTooltipResult, error)

// PreviewTooltipHover is an invisible overlay for list rows. On hover it shows a
// popup preview image loaded by loadPreview (typically a screen capture).
type PreviewTooltipHover struct {
	fynewidget.BaseWidget

	loadPreview PreviewTooltipLoad

	tooltipPanel     *previewTooltipPanel
	pendingCancel    context.CancelFunc
	pendingCtx       context.Context
	captureCancel    context.CancelFunc
	captureCtx       context.Context
	absoluteMousePos fyne.Position
	hovering         bool
}

var _ desktop.Hoverable = (*PreviewTooltipHover)(nil)

// NewPreviewTooltipHover creates a hover overlay for a list row preview tooltip.
func NewPreviewTooltipHover() *PreviewTooltipHover {
	h := &PreviewTooltipHover{}
	h.ExtendBaseWidget(h)
	return h
}

// SetPreviewLoader sets the function that produces the preview when the tooltip opens.
// Pass nil to disable preview tooltips for this row.
func (h *PreviewTooltipHover) SetPreviewLoader(load PreviewTooltipLoad) {
	h.loadPreview = load
}

func (h *PreviewTooltipHover) MouseIn(e *desktop.MouseEvent) {
	if h.loadPreview == nil {
		return
	}
	h.hovering = true
	h.absoluteMousePos = e.AbsolutePosition
	ctx, cancel := context.WithCancel(context.Background())
	h.pendingCtx = ctx
	h.pendingCancel = cancel
	go func() {
		select {
		case <-time.After(previewTooltipShowDelay):
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
				h.beginCapture()
			})
		}
	}()
}

func (h *PreviewTooltipHover) MouseOut() {
	h.hovering = false
	h.cancelPending()
	h.cancelCapture()
	h.hideTooltip()
}

func (h *PreviewTooltipHover) MouseMoved(e *desktop.MouseEvent) {
	h.absoluteMousePos = e.AbsolutePosition
}

func (h *PreviewTooltipHover) cancelPending() {
	if h.pendingCancel != nil {
		h.pendingCancel()
		h.pendingCancel = nil
		h.pendingCtx = nil
	}
}

func (h *PreviewTooltipHover) cancelCapture() {
	if h.captureCancel != nil {
		h.captureCancel()
		h.captureCancel = nil
		h.captureCtx = nil
	}
}

func (h *PreviewTooltipHover) hideTooltip() {
	if h.tooltipPanel == nil {
		return
	}
	c := fyne.CurrentApp().Driver().CanvasForObject(h)
	if c == nil {
		h.tooltipPanel = nil
		return
	}
	layer := findItemTooltipLayer(c, c.Overlays().Top())
	if layer != nil {
		layer.Container.Objects = nil
		layer.Container.Refresh()
	}
	h.tooltipPanel = nil
}

func (h *PreviewTooltipHover) beginCapture() {
	h.hideTooltip()
	load := h.loadPreview
	if load == nil || !h.hovering {
		return
	}
	c := fyne.CurrentApp().Driver().CanvasForObject(h)
	if c == nil {
		return
	}
	layer := findItemTooltipLayer(c, c.Overlays().Top())
	if layer == nil {
		return
	}
	panel := newPreviewTooltipPanel()
	origin := itemTooltipLayerOrigin(layer, c.Overlays().Top())
	size, relPos := previewTooltipSizeAndPosition(panel, c, h.absoluteMousePos.Subtract(origin))
	panel.Resize(size)
	panel.Move(relPos)
	panel.setLoading()
	layer.Container.Objects = []fyne.CanvasObject{panel}
	layer.Container.Refresh()
	h.tooltipPanel = panel

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
					panel.setError(err.Error())
				} else {
					panel.setImage(result.Image, result.Caption)
				}
				panel.Refresh()
				size, relPos := previewTooltipSizeAndPosition(panel, c, h.absoluteMousePos.Subtract(origin))
				panel.Resize(size)
				panel.Move(relPos)
				layer.Container.Refresh()
			})
		}
	}()
}

type previewTooltipPanel struct {
	fynewidget.BaseWidget

	img       *canvas.Image
	message   *fynewidget.Label
	caption   *fynewidget.Label
	loading   bool
	showImage bool
}

func newPreviewTooltipPanel() *previewTooltipPanel {
	p := &previewTooltipPanel{}
	p.ExtendBaseWidget(p)
	return p
}

func (p *previewTooltipPanel) previewSize() fyne.Size {
	return fyne.NewSize(config.ImagePreviewMinWidth, config.ImagePreviewMinHeight)
}

func (p *previewTooltipPanel) MinSize() fyne.Size {
	innerPad := p.Theme().Size(theme.SizeNameInnerPadding)
	size := p.previewSize()
	if p.showImage && p.caption != nil && p.caption.Text != "" {
		size.Height += p.caption.MinSize().Height + innerPad/2
	}
	return size.Add(fyne.NewSquareSize(innerPad * 2))
}

func (p *previewTooltipPanel) setLoading() {
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

func (p *previewTooltipPanel) setError(msg string) {
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

func (p *previewTooltipPanel) setImage(img image.Image, caption string) {
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

func (p *previewTooltipPanel) CreateRenderer() fyne.WidgetRenderer {
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
	v := fyne.CurrentApp().Settings().ThemeVariant()
	th := p.Theme()
	bg := canvas.NewRectangle(th.Color(theme.ColorNameOverlayBackground, v))
	bg.StrokeColor = th.Color(theme.ColorNameInputBorder, v)
	bg.StrokeWidth = th.Size(theme.SizeNameInputBorder)
	bg.CornerRadius = 4
	imageStack := container.NewStack(
		container.NewMax(p.img),
		container.NewPadded(p.message),
	)
	content := container.NewVBox(imageStack, p.caption)
	return &previewTooltipPanelRenderer{
		panel:   p,
		bg:      bg,
		content: content,
	}
}

type previewTooltipPanelRenderer struct {
	panel   *previewTooltipPanel
	bg      *canvas.Rectangle
	content *fyne.Container
}

func (r *previewTooltipPanelRenderer) Layout(size fyne.Size) {
	r.bg.Resize(size)
	r.bg.Move(fyne.NewPos(0, 0))
	innerPad := r.panel.Theme().Size(theme.SizeNameInnerPadding)
	innerSize := size.Subtract(fyne.NewSquareSize(innerPad * 2))
	r.content.Resize(innerSize)
	r.content.Move(fyne.NewPos(innerPad, innerPad))
	if r.panel.showImage {
		r.panel.img.Show()
		r.panel.message.Hide()
	} else {
		r.panel.img.Hide()
		r.panel.message.Show()
	}
}

func (r *previewTooltipPanelRenderer) MinSize() fyne.Size {
	return r.panel.MinSize()
}

func (r *previewTooltipPanelRenderer) Refresh() {
	th := r.panel.Theme()
	v := fyne.CurrentApp().Settings().ThemeVariant()
	r.bg.FillColor = th.Color(theme.ColorNameOverlayBackground, v)
	r.bg.StrokeColor = th.Color(theme.ColorNameInputBorder, v)
	r.bg.StrokeWidth = th.Size(theme.SizeNameInputBorder)
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
	r.bg.Refresh()
	r.content.Refresh()
}

func (r *previewTooltipPanelRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.bg, r.content}
}

func (r *previewTooltipPanelRenderer) Destroy() {}

func previewTooltipSizeAndPosition(panel *previewTooltipPanel, c fyne.Canvas, mousePos fyne.Position) (fyne.Size, fyne.Position) {
	canvasSize := c.Size()
	canvasPad := theme.Padding()
	size := panel.MinSize()

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

func (h *PreviewTooltipHover) CreateRenderer() fyne.WidgetRenderer {
	return &previewTooltipHoverRenderer{hover: h, hit: canvas.NewRectangle(color.Transparent)}
}

type previewTooltipHoverRenderer struct {
	hover *PreviewTooltipHover
	hit   *canvas.Rectangle
}

func (r *previewTooltipHoverRenderer) Layout(size fyne.Size) {
	r.hit.Resize(size)
}

func (r *previewTooltipHoverRenderer) MinSize() fyne.Size {
	return fyne.NewSize(0, 0)
}

func (r *previewTooltipHoverRenderer) Refresh() {}

func (r *previewTooltipHoverRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.hit}
}

func (r *previewTooltipHoverRenderer) Destroy() {}
