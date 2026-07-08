package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
)

// OverlayTopRight places topRight over the top-right corner of content.
func OverlayTopRight(content, topRight fyne.CanvasObject) fyne.CanvasObject {
	if topRight == nil {
		return content
	}
	return container.NewBorder(
		container.NewHBox(layout.NewSpacer(), topRight),
		nil, nil, nil,
		content,
	)
}

// StackTopRight overlays topRight on a full-size transparent layer (for image stacks).
func StackTopRight(topRight fyne.CanvasObject) fyne.CanvasObject {
	if topRight == nil {
		return nil
	}
	return container.NewBorder(
		container.NewHBox(layout.NewSpacer(), topRight),
		nil, nil, nil,
		nil,
	)
}
