package actiondialog

import (
	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// PanelForScreenshot returns a standalone action edit panel for docs/tests.
func PanelForScreenshot(action actions.ActionInterface) fyne.CanvasObject {
	content, _ := panelContentForAction(action)
	title := "Edit Action - " + action.GetType()
	footer := container.NewHBox(
		layout.NewSpacer(),
		ttwidget.NewButton("Cancel", nil),
		ttwidget.NewButton("Save", nil),
	)
	return buildActionDialogPanel(title, wrapActionDialogContent(action, content), footer)
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
	case *actions.Conditional:
		return createConditionalDialogContent(node)
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

// ScreenshotSizeOnParent returns the dialog panel size on a parent window (matches live ShowActionDialog sizing).
func ScreenshotSizeOnParent(parent fyne.Size, action actions.ActionInterface, panel fyne.CanvasObject) fyne.Size {
	return actionDialogSize(parent, action, panel.MinSize())
}
