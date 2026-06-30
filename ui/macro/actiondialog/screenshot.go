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

const screenshotDialogPadW = 48
const screenshotDialogPadH = 120

// PanelForScreenshot returns a standalone action edit panel for docs/tests.
func PanelForScreenshot(action actions.ActionInterface) fyne.CanvasObject {
	content, _ := panelContentForAction(action)
	if sz := contentResizeForAction(action); sz.Width > 0 && sz.Height > 0 {
		content.Resize(sz)
	}
	return buildScreenshotPanel(action.GetType(), content)
}

func panelContentForAction(action actions.ActionInterface) (fyne.CanvasObject, func()) {
	switch node := action.(type) {
	case *actions.Wait:
		return createWaitDialogContent(node)
	case *actions.Move:
		return createMoveDialogContent(node)
	case *actions.Click:
		return createClickDialogContent(node)
	case *actions.Key:
		return createKeyDialogContent(node)
	case *actions.Type:
		return createTypeDialogContent(node)
	case *actions.Loop:
		return createLoopDialogContent(node)
	case *actions.ImageSearch:
		return createImageSearchDialogContent(node)
	case *actions.Ocr:
		return createOcrDialogContent(node)
	case *actions.SetVariable:
		return createSetVariableDialogContent(node)
	case *actions.Calculate:
		return createCalculateDialogContent(node)
	case *actions.ForEachRow:
		return createForEachRowDialogContent(node)
	case *actions.SaveVariable:
		return createSaveVariableDialogContent(node)
	case *actions.FindPixel:
		return createFindPixelDialogContent(node)
	case *actions.FocusWindow:
		return createFocusWindowDialogContent(node)
	case *actions.RunMacro:
		return createRunMacroDialogContent(node)
	case *actions.Break:
		return createBreakDialogContent()
	case *actions.Continue:
		return createContinueDialogContent()
	default:
		return widget.NewLabel("Unsupported action type for screenshot: " + action.GetType()), func() {}
	}
}

// contentResizeForAction mirrors ShowActionDialog content sizing so screenshot layout matches the live UI.
func contentResizeForAction(action actions.ActionInterface) fyne.Size {
	switch action.(type) {
	case *actions.Wait:
		return fyne.NewSize(500, 160)
	case *actions.Move:
		return fyne.NewSize(1000, 600)
	case *actions.Click:
		return fyne.NewSize(300, 100)
	case *actions.Key:
		return fyne.NewSize(300, 100)
	case *actions.Type:
		return fyne.NewSize(400, 120)
	case *actions.Loop:
		return fyne.NewSize(600, 100)
	case *actions.Conditional:
		return fyne.NewSize(conditionalDialogWidth, conditionalDialogHeight)
	case *actions.ImageSearch:
		return fyne.NewSize(1000, 1000)
	case *actions.Ocr:
		return fyne.NewSize(700, 680)
	case *actions.ForEachRow:
		return fyne.NewSize(forEachRowDialogWidth, forEachRowDialogHeight)
	case *actions.SetVariable, *actions.SaveVariable:
		return fyne.NewSize(600, 100)
	case *actions.Calculate:
		return fyne.NewSize(640, 360)
	case *actions.FindPixel:
		return fyne.NewSize(800, 500)
	case *actions.FocusWindow:
		return fyne.NewSize(500, 400)
	case *actions.RunMacro:
		return fyne.NewSize(400, 120)
	case *actions.Break, *actions.Continue:
		return fyne.NewSize(400, 100)
	default:
		return fyne.Size{}
	}
}

// ScreenshotSizeOnParent returns the dialog panel size on a parent window (matches live ShowActionDialog sizing).
func ScreenshotSizeOnParent(parent fyne.Size, action actions.ActionInterface) fyne.Size {
	width := parent.Width - 200
	height := parent.Height - 200
	contentSz := contentResizeForAction(action)
	dialogPadding := fyne.NewSize(40, 110)
	contentPreferredSize := fyne.NewSize(
		contentSz.Width+dialogPadding.Width,
		contentSz.Height+dialogPadding.Height,
	)
	if contentPreferredSize.Width < width {
		width = contentPreferredSize.Width
	}
	if contentPreferredSize.Height < height {
		height = contentPreferredSize.Height
	}
	if width < 200 {
		width = 200
	}
	if height < 200 {
		height = 200
	}
	return fyne.NewSize(width, height)
}

// ScreenshotSizeForAction returns a render hint for the full screenshot panel (content + chrome).
func ScreenshotSizeForAction(action actions.ActionInterface) fyne.Size {
	contentSz := contentResizeForAction(action)
	if contentSz.Width <= 0 || contentSz.Height <= 0 {
		return fyne.Size{}
	}
	return fyne.NewSize(contentSz.Width+screenshotDialogPadW, contentSz.Height+screenshotDialogPadH)
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
