package macro

import (
	"image/color"

	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"

	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

// rowWrapLayout wraps fyne-kx row wrap so tooltip sizing can detect it without
// fragile reflect.TypeOf string checks on an unexported layout type.
type rowWrapLayout struct {
	inner fyne.Layout
}

func newRowWrapLayout() fyne.Layout {
	return &rowWrapLayout{inner: kxlayout.NewRowWrapLayout()}
}

func (l *rowWrapLayout) Layout(objects []fyne.CanvasObject, size fyne.Size) {
	l.inner.Layout(objects, size)
}

func (l *rowWrapLayout) MinSize(objects []fyne.CanvasObject) fyne.Size {
	return l.inner.MinSize(objects)
}

func newRowWrapContainer() *fyne.Container {
	return container.New(newRowWrapLayout())
}

func actionTooltipTypePill(actionType string) fyne.CanvasObject {
	if actionType == "" {
		return nil
	}
	return actiondisplay.NewDisplayPill(actions.ActionTypeLabel(actionType), actionType)
}

// actionTooltipEditTypePill builds the title pill for the edit toolbar with a
// hover tooltip explaining what the action does. The tip surfaces through the
// panel's TooltipSink once BindPillStepperTooltips wires it in rebuildBody.
func actionTooltipEditTypePill(actionType string) fyne.CanvasObject {
	if actionType == "" {
		return nil
	}
	return actiondisplay.NewHoverTipPill(
		actions.ActionTypeLabel(actionType),
		actionType,
		actions.ActionTypeDescription(actionType),
	)
}

func actionTooltipTypeHeader(actionType string) fyne.CanvasObject {
	pill := actionTooltipTypePill(actionType)
	if pill == nil {
		return nil
	}
	return container.NewCenter(pill)
}

var sqyrePrimary = color.NRGBA{R: 0xdc, G: 0x9d, B: 0x2e, A: 0xff}

type pillRow struct {
	box *fyne.Container
}

func newPillRow() *pillRow {
	return &pillRow{box: newRowWrapContainer()}
}

func (r *pillRow) add(obj fyne.CanvasObject) {
	r.box.Add(obj)
}

func (r *pillRow) empty() bool {
	return len(r.box.Objects) == 0
}

func wrapTooltipSection(inner fyne.CanvasObject) fyne.CanvasObject {
	padded := container.NewPadded(inner)
	if activeWire.WrapSqyreFrame != nil {
		return activeWire.WrapSqyreFrame(padded)
	}
	fill := color.NRGBA{R: sqyrePrimary.R, G: sqyrePrimary.G, B: sqyrePrimary.B, A: 13}
	border := canvas.NewRectangle(fill)
	border.StrokeColor = theme.Color(theme.ColorNamePrimary)
	border.StrokeWidth = 1
	border.CornerRadius = 4
	return container.NewStack(border, padded)
}

func joinTooltipSections(sections ...fyne.CanvasObject) fyne.CanvasObject {
	var objects []fyne.CanvasObject
	for _, section := range sections {
		if section != nil {
			objects = append(objects, section)
		}
	}
	if len(objects) == 0 {
		return nil
	}
	return container.NewVBox(objects...)
}

func isRowWrapLayout(l fyne.Layout) bool {
	_, ok := l.(*rowWrapLayout)
	return ok
}

func rowWrapSingleLineWidth(objects []fyne.CanvasObject) float32 {
	padding := theme.Padding()
	var w float32
	first := true
	for _, obj := range objects {
		if !obj.Visible() {
			continue
		}
		if !first {
			w += padding
		}
		first = false
		w += obj.MinSize().Width
	}
	return w
}

func maxRowWrapSingleLineWidth(obj fyne.CanvasObject) float32 {
	var best float32
	var walk func(fyne.CanvasObject)
	walk = func(o fyne.CanvasObject) {
		box, ok := o.(*fyne.Container)
		if !ok {
			return
		}
		if isRowWrapLayout(box.Layout) {
			if w := rowWrapSingleLineWidth(box.Objects); w > best {
				best = w
			}
		}
		for _, child := range box.Objects {
			walk(child)
		}
	}
	walk(obj)
	return best
}

func tooltipSectionChromeWidth() float32 {
	return theme.Padding() * 4
}

func tooltipSectionInnerWidth(outerWidth float32) float32 {
	w := outerWidth - theme.Padding()*2
	if w < 1 {
		return 1
	}
	return w
}

func tooltipSectionChromeHeight() float32 {
	return theme.Padding() * 2
}

func rowWrapHeightAtWidth(box *fyne.Container, width float32) float32 {
	if box == nil || !isRowWrapLayout(box.Layout) {
		return 0
	}
	box.Layout.Layout(box.Objects, fyne.NewSize(width, 0))
	return box.MinSize().Height
}

func findRowWrapContainer(box *fyne.Container) *fyne.Container {
	if box == nil {
		return nil
	}
	if isRowWrapLayout(box.Layout) {
		return box
	}
	for _, child := range box.Objects {
		if c, ok := child.(*fyne.Container); ok {
			if found := findRowWrapContainer(c); found != nil {
				return found
			}
		}
	}
	return nil
}

func isVBoxLike(c *fyne.Container) bool {
	if len(c.Objects) <= 1 {
		return false
	}
	var sum float32
	for i, obj := range c.Objects {
		if i > 0 {
			sum += theme.Padding()
		}
		sum += obj.MinSize().Height
	}
	minH := c.MinSize().Height
	return minH >= sum-theme.Padding() && minH <= sum+theme.Padding()
}

func tooltipSubtreeHeight(obj fyne.CanvasObject, width float32) float32 {
	box, ok := obj.(*fyne.Container)
	if !ok {
		return obj.MinSize().Height
	}
	// Pass through single-child wrappers (centered header, viewParamPillsHolder, etc.)
	// so width-dependent descendants like row-wrapped pill sections size correctly.
	if len(box.Objects) == 1 {
		return tooltipSubtreeHeight(box.Objects[0], width)
	}
	if isVBoxLike(box) {
		return tooltipBodyHeightAtWidth(box, width)
	}
	if rowWrap := findRowWrapContainer(box); rowWrap != nil {
		innerW := tooltipSectionInnerWidth(width)
		return rowWrapHeightAtWidth(rowWrap, innerW) + tooltipSectionChromeHeight()
	}
	return box.MinSize().Height
}

func tooltipBodyHeightAtWidth(body *fyne.Container, width float32) float32 {
	if body == nil || len(body.Objects) == 0 {
		return 0
	}
	return measureVBoxContentHeight(body.Objects, width)
}

func measureVBoxContentHeight(objects []fyne.CanvasObject, width float32) float32 {
	padding := theme.Padding()
	var total float32
	for i, obj := range objects {
		if i > 0 {
			total += padding
		}
		total += tooltipSubtreeHeight(obj, width)
	}
	return total
}
