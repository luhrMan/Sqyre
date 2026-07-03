package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// PreviewListRowTemplate returns a list cell with an invisible hover overlay for image previews.
func PreviewListRowTemplate() fyne.CanvasObject {
	return container.NewStack(
		widget.NewLabel("template"),
		NewPreviewTooltipHover(),
	)
}

// BindPreviewListRow sets the visible label, preview loader, and optional right-click edit handler.
func BindPreviewListRow(co fyne.CanvasObject, labelText string, load PreviewTooltipLoad, onEdit PreviewTooltipEditFunc) {
	stack := co.(*fyne.Container)
	hover := stack.Objects[1].(*PreviewTooltipHover)
	stack.Objects[0].(*widget.Label).SetText(labelText)
	hover.SetPreviewLoader(load)
	hover.SetOnEdit(onEdit)
}

// WrapPreviewTooltipHover stacks content with an invisible hover overlay for image preview tooltips.
func WrapPreviewTooltipHover(content fyne.CanvasObject, load PreviewTooltipLoad, onEdit PreviewTooltipEditFunc) fyne.CanvasObject {
	hover := NewPreviewTooltipHover()
	hover.SetPreviewLoader(load)
	hover.SetOnEdit(onEdit)
	return container.NewStack(content, hover)
}
