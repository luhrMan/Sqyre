package completionentry

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

const minCompletionPopupWidth = 220

type popupGeometry struct {
	size fyne.Size
	pos  fyne.Position
}

func popupGeometryFor(host fyne.CanvasObject, optionCount int, itemHeight float32, measureItemHeight func() float32, labels []string) popupGeometry {
	hostSize := host.Size()
	width := popupWidthFor(host, hostSize.Width, labels)
	size := fyne.NewSize(width, hostSize.Height)

	cnv := fyne.CurrentApp().Driver().CanvasForObject(host)
	if cnv == nil {
		return popupGeometry{size: size, pos: fyne.NewPos(0, size.Height)}
	}
	if itemHeight == 0 && measureItemHeight != nil {
		itemHeight = measureItemHeight()
	}
	canvasSize := cnv.Size()
	hostPos := fyne.CurrentApp().Driver().AbsolutePositionForObject(host)
	padding := 2 * theme.Padding()

	maxWidth := canvasSize.Width - padding*2
	if width > maxWidth {
		width = maxWidth
	}

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

	pos.X = clampPopupX(pos.X, width, canvasSize.Width, padding)
	return popupGeometry{
		size: fyne.NewSize(width, listHeight),
		pos:  pos,
	}
}

func popupWidthFor(host fyne.CanvasObject, hostWidth float32, labels []string) float32 {
	width := hostWidth
	if content := measuredLabelsWidth(host, labels); content > width {
		width = content
	}
	if width < minCompletionPopupWidth {
		width = minCompletionPopupWidth
	}
	return width
}

func measuredLabelsWidth(host fyne.CanvasObject, labels []string) float32 {
	if len(labels) == 0 {
		return 0
	}
	textSize := themeTextSize(host)
	innerPad := theme.Padding() * 4
	maxText := float32(0)
	for _, label := range labels {
		if label == "" {
			continue
		}
		w := fyne.MeasureText(label, textSize, fyne.TextStyle{}).Width
		if w > maxText {
			maxText = w
		}
	}
	return maxText + innerPad
}

func themeTextSize(host fyne.CanvasObject) float32 {
	if w, ok := host.(interface{ Theme() fyne.Theme }); ok {
		return w.Theme().Size(theme.SizeNameText)
	}
	return theme.TextSize()
}

func clampPopupX(x, width, canvasWidth, padding float32) float32 {
	if right := x + width; right > canvasWidth-padding {
		x = canvasWidth - padding - width
	}
	if x < padding {
		x = padding
	}
	return x
}
