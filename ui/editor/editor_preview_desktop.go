//go:build !js

package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/screen"
	"Sqyre/internal/services"
	"errors"
	"fmt"
	"image"
	"image/color"
	"os"
	"path/filepath"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

const (
	editorPreviewMonitorDash = 10
	editorPreviewMonitorGap  = 6
)

var editorPreviewMonitorOutline = color.RGBA{R: 255, A: 255}

func drawPreviewDottedHLine(mat *gocv.Mat, y, x0, x1 int, c color.RGBA, thick int) {
	if x0 > x1 {
		return
	}
	step := editorPreviewMonitorDash + editorPreviewMonitorGap
	for x := x0; x <= x1; x += step {
		xe := x + editorPreviewMonitorDash - 1
		if xe > x1 {
			xe = x1
		}
		gocv.Line(mat, image.Pt(x, y), image.Pt(xe, y), c, thick)
	}
}

func drawPreviewDottedVLine(mat *gocv.Mat, x, y0, y1 int, c color.RGBA, thick int) {
	if y0 > y1 {
		return
	}
	step := editorPreviewMonitorDash + editorPreviewMonitorGap
	for y := y0; y <= y1; y += step {
		ye := y + editorPreviewMonitorDash - 1
		if ye > y1 {
			ye = y1
		}
		gocv.Line(mat, image.Pt(x, y), image.Pt(x, ye), c, thick)
	}
}

func drawPreviewDottedRectOutline(mat *gocv.Mat, r image.Rectangle, c color.RGBA, thick int) {
	if r.Empty() || r.Dx() <= 0 || r.Dy() <= 0 {
		return
	}
	x0, y0 := r.Min.X, r.Min.Y
	x1, y1 := r.Max.X-1, r.Max.Y-1
	if x1 < x0 || y1 < y0 {
		return
	}
	drawPreviewDottedHLine(mat, y0, x0, x1, c, thick)
	drawPreviewDottedHLine(mat, y1, x0, x1, c, thick)
	drawPreviewDottedVLine(mat, x0, y0, y1, c, thick)
	drawPreviewDottedVLine(mat, x1, y0, y1, c, thick)
}

func drawEditorPreviewMonitorOutlines(mat *gocv.Mat, vb image.Rectangle) {
	const thick = 1
	n := screen.NumDisplays()
	for i := 0; i < n; i++ {
		if !screen.IsMonitorEnabled(i) {
			continue
		}
		b := screen.DisplayBoundsAbs(i)
		inter := b.Intersect(vb)
		if inter.Empty() {
			continue
		}
		rel := image.Rect(inter.Min.X-vb.Min.X, inter.Min.Y-vb.Min.Y, inter.Max.X-vb.Min.X, inter.Max.Y-vb.Min.Y)
		drawPreviewDottedRectOutline(mat, rel, editorPreviewMonitorOutline, thick)
	}
}

func editorOnAutoPicSave(eu *EditorUi) {
	selectedItem := eu.EditorTabs.AutoPicTab.SelectedItem
	if selectedItem == nil {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Cannot save - no search area selected"), eu.win)
		return
	}

	searchArea, ok := selectedItem.(*models.SearchArea)
	if !ok {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Cannot save - selected item is not a search area"), eu.win)
		return
	}

	if searchArea == nil {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Cannot save - search area is nil"), eu.win)
		return
	}

	lx, ty, rx, by, resErr := services.ResolveSearchAreaCoordsForPreview(searchArea.LeftX, searchArea.TopY, searchArea.RightX, searchArea.BottomY)
	if resErr != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Cannot resolve search area coordinates (area: %s): %w", searchArea.Name, resErr), eu.win)
		return
	}
	w := rx - lx
	h := by - ty

	if w <= 0 || h <= 0 {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Cannot save - invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), eu.win)
		return
	}

	vb := screen.VirtualBounds()
	if lx < vb.Min.X || ty < vb.Min.Y || rx > vb.Max.X || by > vb.Max.Y {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Cannot save - search area outside virtual desktop - desktop: (%d,%d)..(%d,%d), area: (%d,%d) to (%d,%d) (area: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, lx, ty, rx, by, searchArea.Name), eu.win)
		return
	}

	var captureImg image.Image
	var err error
	func() {
		defer func() {
			if r := recover(); r != nil {
				services.LogPanicToFile(r, "AutoPic: Screen capture during save (area: "+searchArea.Name+")")
				captureImg = nil
			}
		}()

		captureImg, err = robotgo.CaptureImg(lx, ty, w, h)
		if err != nil {
			activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Error capturing image - %v (area: %s)", err, searchArea.Name), eu.win)
			captureImg = nil
		}
	}()

	if captureImg == nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Cannot save - screen capture returned nil image (area: %s)", searchArea.Name), eu.win)
		return
	}

	timestamp := time.Now().Format("20060102_150405")
	filename := fmt.Sprintf("%s_%s.png", timestamp, searchArea.Name)

	autoPicPath := config.GetAutoPicPath()
	if err := os.MkdirAll(autoPicPath, 0755); err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Error creating AutoPic directory: %v", err), eu.win)
		return
	}

	fullPath := filepath.Join(autoPicPath, filename)
	if fullPath == "" {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Error creating file path"), eu.win)
		return
	}

	func() {
		defer func() {
			if r := recover(); r != nil {
				services.LogPanicToFile(r, "AutoPic: Image save (path: "+fullPath+")")
			}
		}()

		if err := robotgo.SavePng(captureImg, fullPath); err != nil {
			activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Error saving image to %s: %v", fullPath, err), eu.win)
			return
		}

		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Image saved successfully to: %s", fullPath), eu.win)
	}()
}

func editorUpdateAutoPicPreview(eu *EditorUi, searchArea *models.SearchArea) {
	if searchArea == nil {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Cannot update preview - search area is nil"), eu.win)
		return
	}

	lx, ty, rx, by, resErr := services.ResolveSearchAreaCoordsForPreview(searchArea.LeftX, searchArea.TopY, searchArea.RightX, searchArea.BottomY)
	if resErr != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Cannot resolve search area coordinates (area: %s): %w", searchArea.Name, resErr), eu.win)
		eu.clearPreviewImage()
		return
	}
	w := rx - lx
	h := by - ty

	if w <= 0 || h <= 0 {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name), eu.win)
		eu.clearPreviewImage()
		return
	}

	vb := screen.VirtualBounds()
	if lx < vb.Min.X || ty < vb.Min.Y || rx > vb.Max.X || by > vb.Max.Y {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Search area outside virtual desktop - desktop: (%d,%d)..(%d,%d), area: (%d,%d) to (%d,%d) (area: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, lx, ty, rx, by, searchArea.Name), eu.win)
		eu.clearPreviewImage()
		return
	}

	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "AutoPic: Screen capture (area: "+searchArea.Name+")")
			eu.clearPreviewImage()
		}
	}()

	captureImg, err := robotgo.CaptureImg(lx, ty, w, h)
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Error capturing image - %v (area: %s)", err, searchArea.Name), eu.win)
		captureImg = nil
	}

	if captureImg == nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("AutoPic: Screen capture returned nil image (area: %s)", searchArea.Name), eu.win)
		eu.clearPreviewImage()
		return
	}

	if previewImage := eu.EditorTabs.AutoPicTab.previewImage; previewImage != nil {
		previewImage.Image = captureImg
		previewImage.Refresh()
	} else {
		activeWire.ShowErrorWithEscape(errors.New("AutoPic: Preview image widget is nil"), eu.win)
	}
}

func editorUpdateSearchAreaPreview(eu *EditorUi, searchArea *models.SearchArea) {
	eu.EditorTabs.SearchAreasTab.previewImage.Resource = nil
	if searchArea == nil {
		activeWire.ShowErrorWithEscape(errors.New("SearchArea: Cannot update preview - search area is nil"), eu.win)
		return
	}

	lx, ty, rx, by, resErr := services.ResolveSearchAreaCoordsForPreview(searchArea.LeftX, searchArea.TopY, searchArea.RightX, searchArea.BottomY)
	if resErr != nil {
		eu.clearSearchAreaPreviewImage()
		eu.EditorTabs.SearchAreasTab.previewImage.Resource = theme.BrokenImageIcon()
		eu.ErrorPopUp(fmt.Sprintf("SearchArea: Cannot resolve coordinates (area: %s): %v", searchArea.Name, resErr))
		return
	}
	w := rx - lx
	h := by - ty

	if w <= 0 || h <= 0 {
		eu.clearSearchAreaPreviewImage()
		eu.EditorTabs.SearchAreasTab.previewImage.Resource = theme.BrokenImageIcon()
		eu.ErrorPopUp(fmt.Sprintf("SearchArea: Invalid search area dimensions - width: %d, height: %d (area: %s)", w, h, searchArea.Name))
		return
	}

	vb := screen.VirtualBounds()
	if lx < vb.Min.X || ty < vb.Min.Y || rx > vb.Max.X || by > vb.Max.Y {
		activeWire.ShowErrorWithEscape(fmt.Errorf("SearchArea: Search area outside virtual desktop - desktop: (%d,%d)..(%d,%d), area: (%d,%d) to (%d,%d) (area: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, lx, ty, rx, by, searchArea.Name), eu.win)
		eu.clearSearchAreaPreviewImage()
		return
	}

	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "SearchArea: Screen capture (area: "+searchArea.Name+")")
			eu.clearSearchAreaPreviewImage()
		}
	}()

	captureImg, err := robotgo.CaptureImg(vb.Min.X, vb.Min.Y, vb.Dx(), vb.Dy())
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("SearchArea: Error capturing image - %v (area: %s)", err, searchArea.Name), eu.win)
		captureImg = nil
	}

	if captureImg == nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("SearchArea: Screen capture returned nil image (area: %s)", searchArea.Name), eu.win)
		eu.clearSearchAreaPreviewImage()
		return
	}

	mat, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("SearchArea: Error converting image to Mat - %v (area: %s)", err, searchArea.Name), eu.win)
		eu.clearSearchAreaPreviewImage()
		return
	}
	defer mat.Close()

	drawEditorPreviewMonitorOutlines(&mat, vb)

	rect := image.Rect(lx-vb.Min.X, ty-vb.Min.Y, rx-vb.Min.X, by-vb.Min.Y)
	redColor := color.RGBA{R: 255, A: 255}
	gocv.Rectangle(&mat, rect, redColor, 2)

	previewImg, err := mat.ToImage()
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("SearchArea: Error converting Mat to image - %v (area: %s)", err, searchArea.Name), eu.win)
		eu.clearSearchAreaPreviewImage()
		return
	}

	if previewImage := eu.EditorTabs.SearchAreasTab.previewImage; previewImage != nil {
		previewImage.Image = previewImg
		previewImage.Refresh()
	} else {
		activeWire.ShowErrorWithEscape(errors.New("SearchArea: Preview image widget is nil"), eu.win)
	}
}

func editorUpdatePointPreview(eu *EditorUi, point *models.Point) {
	if point == nil {
		activeWire.ShowErrorWithEscape(errors.New("Point: Cannot update preview - point is nil"), eu.win)
		return
	}

	px := pointCoordToIntForPreview(point.X)
	py := pointCoordToIntForPreview(point.Y)

	vb := screen.VirtualBounds()
	if px < vb.Min.X || py < vb.Min.Y || px > vb.Max.X || py > vb.Max.Y {
		activeWire.ShowErrorWithEscape(fmt.Errorf("Point: Point outside virtual desktop - desktop: (%d,%d)..(%d,%d), point: (%d,%d) (point: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, px, py, point.Name), eu.win)
		eu.clearPointPreviewImage()
		return
	}

	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r, "Point: Screen capture (point: "+point.Name+")")
			eu.clearPointPreviewImage()
		}
	}()

	captureImg, err := robotgo.CaptureImg(vb.Min.X, vb.Min.Y, vb.Dx(), vb.Dy())
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("Point: Error capturing image - %v (point: %s)", err, point.Name), eu.win)
		captureImg = nil
	}
	if captureImg == nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("Point: Screen capture returned nil image (point: %s)", point.Name), eu.win)
		eu.clearPointPreviewImage()
		return
	}

	mat, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("Point: Error converting image to Mat - %v (point: %s)", err, point.Name), eu.win)
		eu.clearPointPreviewImage()
		return
	}
	defer mat.Close()

	drawEditorPreviewMonitorOutlines(&mat, vb)

	center := image.Point{X: px - vb.Min.X, Y: py - vb.Min.Y}
	redColor := color.RGBA{R: 255, A: 255}

	gocv.Circle(&mat, center, 8, redColor, 2)
	gocv.Line(&mat, image.Point{X: center.X - 15, Y: center.Y}, image.Point{X: center.X + 15, Y: center.Y}, redColor, 2)
	gocv.Line(&mat, image.Point{X: center.X, Y: center.Y - 15}, image.Point{X: center.X, Y: center.Y + 15}, redColor, 2)

	previewImg, err := mat.ToImage()
	if err != nil {
		activeWire.ShowErrorWithEscape(fmt.Errorf("Point: Error converting Mat to image - %v (point: %s)", err, point.Name), eu.win)
		eu.clearPointPreviewImage()
		return
	}

	if previewImage := eu.EditorTabs.PointsTab.previewImage; previewImage != nil {
		previewImage.Image = previewImg
		fyne.DoAndWait(func() {
			previewImage.Refresh()
		})
	} else {
		activeWire.ShowErrorWithEscape(errors.New("Point: Preview image widget is nil"), eu.win)
	}
}

func editorUpdateMaskPreview(eu *EditorUi, programName, maskName string) {
	masksPath := config.GetMasksPath()
	imgPath := filepath.Join(masksPath, programName, maskName+config.PNG)

	if _, err := os.Stat(imgPath); err != nil {
		eu.ClearMaskPreviewImage()
		return
	}

	mat := gocv.IMRead(imgPath, gocv.IMReadColor)
	if mat.Empty() {
		eu.ClearMaskPreviewImage()
		return
	}
	defer mat.Close()

	img, err := mat.ToImage()
	if err != nil {
		eu.ClearMaskPreviewImage()
		return
	}

	if previewImage := eu.EditorTabs.MasksTab.previewImage; previewImage != nil {
		previewImage.Image = img
		previewImage.Refresh()
	}
}
