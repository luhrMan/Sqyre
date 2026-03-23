package ui

import (
	"fyne.io/fyne/v2"
)

// iconTopRightButtonLayout lays out the first object to fill the container and places the
// second as a square in the top-right corner. Side length is fraction×min(width, height).
type iconTopRightButtonLayout struct {
	fraction float32
}

func (l *iconTopRightButtonLayout) Layout(objects []fyne.CanvasObject, size fyne.Size) {
	if len(objects) < 2 {
		return
	}
	bg, btn := objects[0], objects[1]
	bg.Resize(size)
	bg.Move(fyne.NewPos(0, 0))

	side := min(size.Width, size.Height) * l.fraction
	if side < 1 {
		side = 1
	}
	bs := fyne.NewSize(side, side)
	btn.Resize(bs)
	btn.Move(fyne.NewPos(size.Width-bs.Width, 0))
}

func (l *iconTopRightButtonLayout) MinSize(objects []fyne.CanvasObject) fyne.Size {
	if len(objects) < 1 {
		return fyne.NewSize(0, 0)
	}
	return objects[0].MinSize()
}
