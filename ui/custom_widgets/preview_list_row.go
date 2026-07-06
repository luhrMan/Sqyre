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

// BindPreviewListRow sets the visible label and preview loader on a PreviewListRowTemplate cell.
func BindPreviewListRow(co fyne.CanvasObject, labelText string, load PreviewTooltipLoad) {
	stack := co.(*fyne.Container)
	stack.Objects[0].(*widget.Label).SetText(labelText)
	stack.Objects[1].(*PreviewTooltipHover).SetPreviewLoader(load)
}

// WrapPreviewTooltipHover stacks content with an invisible hover overlay for image preview tooltips.
func WrapPreviewTooltipHover(content fyne.CanvasObject, load PreviewTooltipLoad) fyne.CanvasObject {
	hover := NewPreviewTooltipHover()
	hover.SetPreviewLoader(load)
	return container.NewStack(content, hover)
}
