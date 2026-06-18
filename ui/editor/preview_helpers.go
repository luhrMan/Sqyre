package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/screen"
	"Sqyre/internal/services"
	"fmt"
	"image"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

// editorPreviewPanel shows a screen capture preview or an inline error message (no popup).
type editorPreviewPanel struct {
	image      *canvas.Image
	errorLabel *widget.Label
	errorOverlay *fyne.Container
	container  fyne.CanvasObject
}

func newEditorPreviewPanel() *editorPreviewPanel {
	img := canvas.NewImageFromImage(nil)
	img.FillMode = canvas.ImageFillContain
	img.SetMinSize(fyne.NewSize(400, 300))

	errLbl := widget.NewLabel("")
	errLbl.Wrapping = fyne.TextWrapWord
	errLbl.Alignment = fyne.TextAlignCenter

	// Padded overlay fills the preview bounds so wrapped text flows as a paragraph.
	errOverlay := container.NewPadded(errLbl)
	errOverlay.Hide()
	inner := container.NewStack(img, errOverlay)
	return &editorPreviewPanel{
		image:        img,
		errorLabel:   errLbl,
		errorOverlay: errOverlay,
		container:    wrapEditorPreviewImage(inner),
	}
}

func (p *editorPreviewPanel) setImage(preview image.Image) {
	if p == nil {
		return
	}
	p.errorOverlay.Hide()
	p.image.Show()
	p.image.Image = preview
	p.image.Refresh()
}

func (p *editorPreviewPanel) setError(err error) {
	if p == nil || err == nil {
		return
	}
	p.image.Image = nil
	p.image.Hide()
	p.errorLabel.SetText(err.Error())
	p.errorOverlay.Show()
	p.errorLabel.Refresh()
}

func (p *editorPreviewPanel) clear() {
	if p == nil {
		return
	}
	p.image.Image = nil
	p.image.Show()
	p.errorOverlay.Hide()
	p.errorLabel.SetText("")
	p.image.Refresh()
}

// coordToIntForPreview returns an int for preview/validation; literal ints are used, variable refs yield 0.
func coordToIntForPreview(v any) int {
	switch val := v.(type) {
	case int:
		return val
	case float64:
		return int(val)
	default:
		return 0
	}
}

type searchAreaBounds struct {
	lx, ty, rx, by, w, h int
}

func searchAreaBoundsFrom(sa *models.SearchArea) searchAreaBounds {
	lx := coordToIntForPreview(sa.LeftX)
	ty := coordToIntForPreview(sa.TopY)
	rx := coordToIntForPreview(sa.RightX)
	by := coordToIntForPreview(sa.BottomY)
	return searchAreaBounds{lx: lx, ty: ty, rx: rx, by: by, w: rx - lx, h: by - ty}
}

func resolveSearchAreaBounds(prefix string, sa *models.SearchArea, b searchAreaBounds) (searchAreaBounds, error) {
	lx, ty, rx, by, w, h, err := screen.ValidateSearchAreaRect(b.lx, b.ty, b.rx, b.by)
	if err != nil {
		return searchAreaBounds{}, fmt.Errorf("%s: %w (area: %s)", prefix, err, sa.Name)
	}
	return searchAreaBounds{lx: lx, ty: ty, rx: rx, by: by, w: w, h: h}, nil
}

func captureVirtualDesktop(drawOverlay func(*gocv.Mat, image.Rectangle)) (image.Image, error) {
	vb := screen.VirtualBounds()
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "editor: virtual desktop capture")
		}
	}()

	captureImg, err := robotgo.CaptureImg(vb.Min.X, vb.Min.Y, vb.Dx(), vb.Dy())
	if err != nil {
		return nil, fmt.Errorf("error capturing image: %w", err)
	}
	if captureImg == nil {
		return nil, fmt.Errorf("screen capture returned nil image")
	}

	mat, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		return nil, fmt.Errorf("error converting image to Mat: %w", err)
	}
	defer mat.Close()

	drawEditorPreviewMonitorOutlines(&mat, vb)
	if drawOverlay != nil {
		drawOverlay(&mat, vb)
	}

	return mat.ToImage()
}


func captureCroppedArea(lx, ty, w, h int) (image.Image, error) {
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "editor: cropped area capture")
		}
	}()
	captureImg, err := robotgo.CaptureImg(lx, ty, w, h)
	if err != nil {
		return nil, err
	}
	if captureImg == nil {
		return nil, fmt.Errorf("screen capture returned nil image")
	}
	return captureImg, nil
}
