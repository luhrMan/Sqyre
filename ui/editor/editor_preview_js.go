//go:build js

package editor

import (
	"errors"

	"Sqyre/internal/models"
)

func editorOnAutoPicSave(eu *EditorUi) {
	activeWire.ShowErrorWithEscape(errors.New("Saving AutoPic screenshots requires the desktop app."), eu.win)
}

func editorUpdateAutoPicPreview(eu *EditorUi, searchArea *models.SearchArea) {
	_ = searchArea
	eu.clearPreviewImage()
}

func editorUpdateSearchAreaPreview(eu *EditorUi, searchArea *models.SearchArea) {
	_ = searchArea
	eu.EditorTabs.SearchAreasTab.previewImage.Resource = nil
	eu.clearSearchAreaPreviewImage()
}

func editorUpdatePointPreview(eu *EditorUi, point *models.Point) {
	_ = point
	eu.clearPointPreviewImage()
}

func editorUpdateMaskPreview(eu *EditorUi, programName, maskName string) {
	_ = programName
	_ = maskName
	eu.ClearMaskPreviewImage()
}
