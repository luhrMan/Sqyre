//go:build !js

package actiondialog

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/screen"
	"Sqyre/internal/services"
	"image"
	"image/color"

	"fyne.io/fyne/v2/canvas"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

func refreshMovePointPreview(pointPreviewImage *canvas.Image, point *actions.Point) {
	if point == nil {
		pointPreviewImage.Image = nil
		pointPreviewImage.Refresh()
		return
	}

	px := pointCoordToInt(point.X)
	py := pointCoordToInt(point.Y)

	vb := screen.VirtualBounds()
	if px < vb.Min.X || py < vb.Min.Y || px > vb.Max.X || py > vb.Max.Y {
		pointPreviewImage.Image = nil
		pointPreviewImage.Refresh()
		return
	}

	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "Action dialog: point preview capture")
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
		}
	}()

	captureImg, err := robotgo.CaptureImg(vb.Min.X, vb.Min.Y, vb.Dx(), vb.Dy())
	if err != nil || captureImg == nil {
		pointPreviewImage.Image = nil
		pointPreviewImage.Refresh()
		return
	}

	mat, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		pointPreviewImage.Image = nil
		pointPreviewImage.Refresh()
		return
	}
	defer mat.Close()

	center := image.Point{X: px - vb.Min.X, Y: py - vb.Min.Y}
	redColor := color.RGBA{R: 255, A: 255}

	gocv.Circle(&mat, center, 8, redColor, 2)
	gocv.Line(&mat, image.Point{X: center.X - 15, Y: center.Y}, image.Point{X: center.X + 15, Y: center.Y}, redColor, 2)
	gocv.Line(&mat, image.Point{X: center.X, Y: center.Y - 15}, image.Point{X: center.X, Y: center.Y + 15}, redColor, 2)

	previewImg, err := mat.ToImage()
	if err != nil {
		pointPreviewImage.Image = nil
		pointPreviewImage.Refresh()
		return
	}

	pointPreviewImage.Image = previewImg
	pointPreviewImage.Refresh()
}
