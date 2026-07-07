package macro

import (
	"image/color"
	"reflect"

	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

var sqyrePrimary = color.NRGBA{R: 0xdc, G: 0x9d, B: 0x2e, A: 0xff}

type pillRow struct {
	box *fyne.Container
}

func newPillRow() *pillRow {
	return &pillRow{box: container.New(kxlayout.NewRowWrapLayout())}
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
	return l != nil && reflect.TypeOf(l).String() == "*layout.rowWrapLayout"
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
	padding := theme.Padding()
	var total float32
	for i, obj := range body.Objects {
		if i > 0 {
			total += padding
		}
		total += tooltipSubtreeHeight(obj, width)
	}
	return total
}
