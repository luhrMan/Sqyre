package editor

import (
	"Sqyre/internal/capture"
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/panicsafe"
	"Sqyre/internal/screen"
	"Sqyre/internal/vision"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"image"
	"os"
	"path/filepath"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// editorPreviewPanel shows a screen capture preview or an inline error message (no popup).
type editorPreviewPanel struct {
	viewer       *custom_widgets.ZoomableImageView
	errorLabel   *widget.Label
	errorOverlay *fyne.Container
	container    fyne.CanvasObject
}

func newEditorPreviewPanel() *editorPreviewPanel {
	viewer := custom_widgets.NewZoomableImageView()
	viewer.SetMinSize(fyne.NewSize(config.ImagePreviewMinWidth, config.ImagePreviewMinHeight))

	errLbl := widget.NewLabel("")
	errLbl.Wrapping = fyne.TextWrapWord
	errLbl.Alignment = fyne.TextAlignCenter

	// Padded overlay fills the preview bounds so wrapped text flows as a paragraph.
	errOverlay := container.NewPadded(errLbl)
	errOverlay.Hide()
	inner := container.NewStack(viewer, errOverlay)
	return &editorPreviewPanel{
		viewer:       viewer,
		errorLabel:   errLbl,
		errorOverlay: errOverlay,
		container:    wrapEditorPreviewImage(container.NewMax(inner)),
	}
}

func (p *editorPreviewPanel) setImage(preview image.Image) {
	if p == nil {
		return
	}
	p.errorOverlay.Hide()
	p.viewer.Show()
	p.viewer.SetImage(preview)
}

func (p *editorPreviewPanel) setError(err error) {
	if p == nil || err == nil {
		return
	}
	p.viewer.SetImage(nil)
	p.viewer.Hide()
	p.errorLabel.SetText(err.Error())
	p.errorOverlay.Show()
	p.errorLabel.Refresh()
}

func (p *editorPreviewPanel) clear() {
	if p == nil {
		return
	}
	p.viewer.SetImage(nil)
	p.viewer.Show()
	p.errorOverlay.Hide()
	p.errorLabel.SetText("")
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

func captureCroppedArea(lx, ty, w, h int) (image.Image, error) {
	return capture.CaptureRect(lx, ty, w, h)
}

func captureSearchAreaPreview(lx, ty, rx, by int) (image.Image, error) {
	return vision.CaptureSearchAreaPreview(lx, ty, rx, by)
}

func capturePointPreview(px, py int) (image.Image, error) {
	return vision.CapturePointPreview(px, py)
}

func updatePointPreviewPanel(panel *editorPreviewPanel, point *models.Point) {
	if panel == nil {
		return
	}
	previewImg, err := vision.PointPreview(point)
	if err != nil {
		panel.setError(err)
		return
	}
	panel.setImage(previewImg)
}

func updateSearchAreaPreviewPanel(panel *editorPreviewPanel, searchArea *models.SearchArea) {
	if panel == nil {
		return
	}
	previewImg, err := vision.SearchAreaPreview(searchArea)
	if err != nil {
		panel.setError(err)
		return
	}
	panel.setImage(previewImg)
}

func updateMaskPreviewPanel(panel *editorPreviewPanel, programName, maskName string) {
	if panel == nil {
		return
	}
	if programName == "" || maskName == "" {
		panel.clear()
		return
	}
	imgPath := filepath.Join(config.GetMasksPath(), programName, maskName+config.PNG)
	if _, err := os.Stat(imgPath); err != nil {
		panel.clear()
		return
	}
	img, err := vision.ReadColorImage(imgPath)
	if err != nil {
		panel.clear()
		return
	}
	panel.setImage(img)
}

func safeUpdatePointPreviewPanel(panel *editorPreviewPanel, p *models.Point) {
	defer func() {
		if r := recover(); r != nil {
			name := ""
			if p != nil {
				name = p.Name
			}
			panicsafe.LogPanicToFile(r, "Point: Preview update (point: "+name+")")
		}
	}()
	updatePointPreviewPanel(panel, p)
}

// LoadPointPreviewImage captures the editor preview for a point list entry.
func LoadPointPreviewImage(program *models.Program, key string) (custom_widgets.PreviewTooltipResult, error) {
	point, err := ProgramPointRepo(program, config.MainMonitorSizeString).Get(key)
	if err != nil {
		return custom_widgets.PreviewTooltipResult{}, err
	}
	img, caption, err := vision.PointPreviewTooltipCached(point)
	if err != nil {
		return custom_widgets.PreviewTooltipResult{}, err
	}
	return custom_widgets.PreviewTooltipResult{Image: img, Caption: caption}, nil
}

// LoadSearchAreaPreviewImage captures the editor preview for a search area list entry.
func LoadSearchAreaPreviewImage(program *models.Program, key string) (custom_widgets.PreviewTooltipResult, error) {
	sa, err := ProgramSearchAreaRepo(program, config.MainMonitorSizeString).Get(key)
	if err != nil {
		return custom_widgets.PreviewTooltipResult{}, err
	}
	img, caption, err := vision.SearchAreaPreviewTooltipCached(sa)
	if err != nil {
		return custom_widgets.PreviewTooltipResult{}, err
	}
	return custom_widgets.PreviewTooltipResult{Image: img, Caption: caption}, nil
}

func safeUpdateSearchAreaPreviewPanel(panel *editorPreviewPanel, sa *models.SearchArea) {
	defer func() {
		if r := recover(); r != nil {
			name := ""
			if sa != nil {
				name = sa.Name
			}
			panicsafe.LogPanicToFile(r, "SearchArea: Preview update (area: "+name+")")
		}
	}()
	updateSearchAreaPreviewPanel(panel, sa)
}
