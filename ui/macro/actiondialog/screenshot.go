package actiondialog

import (
	"Sqyre/internal/models/actions"
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// PanelForScreenshot returns a standalone action edit panel for docs/tests.
func PanelForScreenshot(action actions.ActionInterface) fyne.CanvasObject {
	var content fyne.CanvasObject
	switch node := action.(type) {
	case *actions.Wait:
		content, _ = createWaitDialogContent(node)
	default:
		content = widget.NewLabel("Unsupported action type for screenshot: " + action.GetType())
	}
	return buildScreenshotPanel(action.GetType(), content)
}

func buildScreenshotPanel(actionType string, content fyne.CanvasObject) fyne.CanvasObject {
	saveButton := ttwidget.NewButton("Save", nil)
	cancelButton := ttwidget.NewButton("Cancel", nil)
	buttons := container.NewHBox(
		layout.NewSpacer(),
		cancelButton,
		saveButton,
	)
	titleLabel := ttwidget.NewLabel("Edit Action - " + actionType)
	titleLabel.TextStyle = fyne.TextStyle{Bold: true}
	dialogContent := container.NewBorder(
		container.NewPadded(titleLabel),
		buttons,
		nil,
		nil,
		content,
	)

	th := fyne.CurrentApp().Settings().Theme()
	v := fyne.CurrentApp().Settings().ThemeVariant()
	panelBg := canvas.NewRectangle(th.Color(theme.ColorNameOverlayBackground, v))
	panelBg.CornerRadius = theme.InputRadiusSize()
	border := canvas.NewRectangle(color.Transparent)
	border.StrokeColor = th.Color(theme.ColorNamePrimary, v)
	border.StrokeWidth = 1
	border.CornerRadius = theme.InputRadiusSize()
	innerPadded := container.NewPadded(container.NewPadded(container.NewPadded(container.NewPadded(dialogContent))))
	return container.NewStack(panelBg, border, innerPadded)
}
