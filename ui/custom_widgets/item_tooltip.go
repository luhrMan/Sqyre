package custom_widgets

import (
	"context"
	"image/color"
	"strings"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
	fynewidget "fyne.io/fyne/v2/widget"
)

const (
	itemTooltipShowDelay = 750 * time.Millisecond
	itemTooltipMaxWidth  = 600
)

// ItemTooltipLabel is an invisible hover target for item grid cells. It shows a rich tooltip
// with the item name and optional tags (tags use caption size, italic, and primary color).
type ItemTooltipLabel struct {
	fynewidget.BaseWidget

	itemName string
	tags     []string

	tooltipPanel     *itemTooltipPanel
	pendingCancel    context.CancelFunc
	pendingCtx       context.Context
	absoluteMousePos fyne.Position
}

var _ desktop.Hoverable = (*ItemTooltipLabel)(nil)

// NewItemTooltipLabel creates a hover overlay for an item grid cell.
func NewItemTooltipLabel() *ItemTooltipLabel {
	l := &ItemTooltipLabel{}
	l.ExtendBaseWidget(l)
	return l
}

// SetItem sets the tooltip content for this cell.
func (l *ItemTooltipLabel) SetItem(name string, tags []string) {
	l.itemName = name
	if len(tags) == 0 {
		l.tags = nil
		return
	}
	l.tags = append([]string(nil), tags...)
}

func (l *ItemTooltipLabel) MouseIn(e *desktop.MouseEvent) {
	if l.itemName == "" {
		return
	}
	l.absoluteMousePos = e.AbsolutePosition
	ctx, cancel := context.WithCancel(context.Background())
	l.pendingCtx = ctx
	l.pendingCancel = cancel
	go func() {
		select {
		case <-time.After(itemTooltipShowDelay):
		case <-ctx.Done():
			return
		}
		select {
		case <-ctx.Done():
			return
		default:
			fyne.Do(func() {
				if l.pendingCtx != ctx {
					return
				}
				l.pendingCtx = nil
				l.pendingCancel = nil
				l.showTooltip()
			})
		}
	}()
}

func (l *ItemTooltipLabel) MouseOut() {
	l.cancelPending()
	l.hideTooltip()
}

func (l *ItemTooltipLabel) MouseMoved(e *desktop.MouseEvent) {
	l.absoluteMousePos = e.AbsolutePosition
}

func (l *ItemTooltipLabel) cancelPending() {
	if l.pendingCancel != nil {
		l.pendingCancel()
		l.pendingCancel = nil
		l.pendingCtx = nil
	}
}

func (l *ItemTooltipLabel) hideTooltip() {
	if l.tooltipPanel == nil {
		return
	}
	c := fyne.CurrentApp().Driver().CanvasForObject(l)
	if c == nil {
		l.tooltipPanel = nil
		return
	}
	layer := findItemTooltipLayer(c, c.Overlays().Top())
	if layer != nil {
		layer.Container.Objects = nil
		layer.Container.Refresh()
	}
	l.tooltipPanel = nil
}

func (l *ItemTooltipLabel) showTooltip() {
	l.hideTooltip()
	c := fyne.CurrentApp().Driver().CanvasForObject(l)
	if c == nil || l.itemName == "" {
		return
	}
	layer := findItemTooltipLayer(c, c.Overlays().Top())
	if layer == nil {
		return
	}
	panel := newItemTooltipPanel(l.itemName, l.tags)
	origin := itemTooltipLayerOrigin(layer, c.Overlays().Top())
	size, relPos := itemTooltipSizeAndPosition(panel, c, l.absoluteMousePos.Subtract(origin))
	panel.Resize(size)
	panel.Move(relPos)
	layer.Container.Objects = []fyne.CanvasObject{panel}
	layer.Container.Refresh()
	l.tooltipPanel = panel
}

// itemTooltipPanel renders item name + tags; sizing follows fyne-tooltip's approach.
type itemTooltipPanel struct {
	fynewidget.BaseWidget

	name string
	tags []string

	richtext *fynewidget.RichText
}

func newItemTooltipPanel(name string, tags []string) *itemTooltipPanel {
	p := &itemTooltipPanel{name: name, tags: tags}
	p.ExtendBaseWidget(p)
	return p
}

func (p *itemTooltipPanel) MinSize() fyne.Size {
	return fyne.NewSize(0, 0)
}

func (p *itemTooltipPanel) Resize(size fyne.Size) {
	p.updateRichText()
	p.richtext.Resize(size)
	p.BaseWidget.Resize(size)
}

func (p *itemTooltipPanel) textMinSize() fyne.Size {
	p.updateRichText()
	innerPad := p.Theme().Size(theme.SizeNameInnerPadding)
	return p.richtext.MinSize().Subtract(fyne.NewSquareSize(2 * innerPad)).Add(fyne.NewSize(2, 8))
}

func (p *itemTooltipPanel) nonWrappingTextWidth() float32 {
	th := p.Theme()
	innerPad := th.Size(theme.SizeNameInnerPadding)
	nameW := fyne.MeasureText(p.name, th.Size(theme.SizeNameText), fyne.TextStyle{Bold: true}).Width
	w := nameW
	if len(p.tags) > 0 {
		tagStyle := fyne.TextStyle{Italic: true}
		tagSize := th.Size(theme.SizeNameCaptionText)
		for _, tag := range p.tags {
			tagW := fyne.MeasureText(tag, tagSize, tagStyle).Width
			w = fyne.Max(w, tagW)
		}
	}
	return w + innerPad*2
}

func (p *itemTooltipPanel) updateRichText() {
	if p.richtext == nil {
		p.richtext = fynewidget.NewRichText()
		p.richtext.Wrapping = fyne.TextWrapWord
	}
	segments := []fynewidget.RichTextSegment{
		&fynewidget.TextSegment{
			Text: p.name,
			Style: fynewidget.RichTextStyle{
				SizeName:  theme.SizeNameText,
				TextStyle: fyne.TextStyle{Bold: true},
			},
		},
	}
	if len(p.tags) > 0 {
		segments = append(segments, &fynewidget.TextSegment{
			Text: "\n" + strings.Join(p.tags, "\n"),
			Style: fynewidget.RichTextStyle{
				SizeName:  theme.SizeNameCaptionText,
				ColorName: theme.ColorNamePrimary,
				TextStyle: fyne.TextStyle{Italic: true},
			},
		})
	}
	p.richtext.Segments = segments
}

func (p *itemTooltipPanel) CreateRenderer() fyne.WidgetRenderer {
	p.updateRichText()
	v := fyne.CurrentApp().Settings().ThemeVariant()
	th := p.Theme()
	bg := canvas.NewRectangle(th.Color(theme.ColorNameOverlayBackground, v))
	bg.StrokeColor = th.Color(theme.ColorNameInputBorder, v)
	bg.StrokeWidth = th.Size(theme.SizeNameInputBorder)
	return &itemTooltipPanelRenderer{
		panel:    p,
		bg:       bg,
		richtext: p.richtext,
	}
}

type itemTooltipPanelRenderer struct {
	panel    *itemTooltipPanel
	bg       *canvas.Rectangle
	richtext *fynewidget.RichText
}

func (r *itemTooltipPanelRenderer) Layout(size fyne.Size) {
	r.bg.Resize(size)
	r.bg.Move(fyne.NewPos(0, 0))
	innerPad := r.panel.Theme().Size(theme.SizeNameInnerPadding)
	r.richtext.Resize(size)
	r.richtext.Move(fyne.NewPos(0, -innerPad+3))
}

func (r *itemTooltipPanelRenderer) MinSize() fyne.Size {
	return r.panel.textMinSize()
}

func (r *itemTooltipPanelRenderer) Refresh() {
	th := r.panel.Theme()
	v := fyne.CurrentApp().Settings().ThemeVariant()
	r.bg.FillColor = th.Color(theme.ColorNameOverlayBackground, v)
	r.bg.StrokeColor = th.Color(theme.ColorNameInputBorder, v)
	r.bg.StrokeWidth = th.Size(theme.SizeNameInputBorder)
	r.panel.updateRichText()
	r.richtext.Refresh()
	r.bg.Refresh()
}

func (r *itemTooltipPanelRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.bg, r.richtext}
}

func (r *itemTooltipPanelRenderer) Destroy() {}

func itemTooltipSizeAndPosition(panel *itemTooltipPanel, c fyne.Canvas, mousePos fyne.Position) (fyne.Size, fyne.Position) {
	canvasSize := c.Size()
	canvasPad := theme.Padding()

	w := fyne.Min(panel.nonWrappingTextWidth(), fyne.Min(canvasSize.Width-canvasPad*2, itemTooltipMaxWidth))
	panel.Resize(fyne.NewSize(w, 1))
	h := panel.textMinSize().Height
	size := fyne.NewSize(w, h)

	pos := mousePos
	if rightEdge := pos.X + w; rightEdge > canvasSize.Width-canvasPad {
		pos.X -= rightEdge - canvasSize.Width + canvasPad
	}
	const belowMouseDist = 16
	const aboveMouseDist = 8
	if bottomEdge := pos.Y + h + belowMouseDist; bottomEdge > canvasSize.Height-canvasPad {
		pos.Y -= h + aboveMouseDist
	} else {
		pos.Y += belowMouseDist
	}
	return size, pos
}

func (l *ItemTooltipLabel) CreateRenderer() fyne.WidgetRenderer {
	return &itemTooltipRenderer{label: l, hit: canvas.NewRectangle(color.Transparent)}
}

type itemTooltipRenderer struct {
	label *ItemTooltipLabel
	hit   *canvas.Rectangle
}

func (r *itemTooltipRenderer) Layout(size fyne.Size) {
	r.hit.Resize(size)
}

func (r *itemTooltipRenderer) MinSize() fyne.Size {
	return fyne.NewSize(0, 0)
}

func (r *itemTooltipRenderer) Refresh() {}

func (r *itemTooltipRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.hit}
}

func (r *itemTooltipRenderer) Destroy() {}
