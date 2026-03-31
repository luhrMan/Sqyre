//go:build js

package actiondialog

import (
	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2/canvas"
)

func refreshMovePointPreview(pointPreviewImage *canvas.Image, point *actions.Point) {
	_ = point
	pointPreviewImage.Image = nil
	pointPreviewImage.Refresh()
}
