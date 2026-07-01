package completionentry

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type popupGeometry struct {
	size fyne.Size
	pos  fyne.Position
}

func popupGeometryFor(host fyne.CanvasObject, optionCount int, itemHeight float32, measureItemHeight func() float32) popupGeometry {
	cnv := fyne.CurrentApp().Driver().CanvasForObject(host)
	if cnv == nil {
		size := host.Size()
		return popupGeometry{size: size, pos: fyne.NewPos(0, size.Height)}
	}
	if itemHeight == 0 && measureItemHeight != nil {
		itemHeight = measureItemHeight()
	}
	canvasSize := cnv.Size()
	hostSize := host.Size()
	hostPos := fyne.CurrentApp().Driver().AbsolutePositionForObject(host)
	padding := 2 * theme.Padding()

	listHeight := float32(optionCount)*(itemHeight+2*theme.Padding()+theme.SeparatorThicknessSize()) + 2*theme.Padding()
	spaceBelow := canvasSize.Height - hostPos.Y - hostSize.Height - padding
	spaceAbove := hostPos.Y - padding
	if spaceBelow < 0 {
		spaceBelow = 0
	}
	if spaceAbove < 0 {
		spaceAbove = 0
	}

	showAbove := listHeight > spaceBelow && spaceAbove > spaceBelow
	var maxHeight float32
	var pos fyne.Position
	if showAbove {
		maxHeight = spaceAbove
		if listHeight > maxHeight {
			listHeight = maxHeight
		}
		pos = fyne.NewPos(hostPos.X, hostPos.Y-listHeight)
	} else {
		maxHeight = spaceBelow
		if listHeight > maxHeight {
			listHeight = maxHeight
		}
		pos = hostPos.Add(fyne.NewPos(0, hostSize.Height))
	}
	return popupGeometry{
		size: fyne.NewSize(hostSize.Width, listHeight),
		pos:  pos,
	}
}
