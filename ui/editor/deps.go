package editor

import (
	"Sqyre/ui/macro"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/driver/desktop"
)

// WireDeps supplies the editor shell and callbacks from package ui (avoids import cycle).
type WireDeps struct {
	Window                fyne.Window
	EU                    *EditorUi
	MacroMTabs            func() *macro.MacroTabs
	// MacroVariables returns variable names from the currently selected macro (for VarEntry completion).
	MacroVariables        func() []string
	NavigationVisible     func() bool
	ShowErrorWithEscape   func(err error, parent fyne.Window)
	ShowConfirmWithEscape func(title, message string, callback func(bool), parent fyne.Window)
	AddDialogEscapeClose  func(d dialog.Dialog, parent fyne.Window)
	ShowRecordingOverlay    func(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func()
	ShowSearchAreaRecordingOverlay func(onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int))
	// WrapTagChip styles tag rows in the Items tab (from ui theme helpers; avoids import cycle).
	WrapTagChip func(inner fyne.CanvasObject) fyne.CanvasObject
}

var activeWire WireDeps

func shell() *EditorUi { return activeWire.EU }

func macroVarNames() []string {
	if activeWire.MacroVariables != nil {
		return activeWire.MacroVariables()
	}
	return nil
}

func wrapTagChip(inner fyne.CanvasObject) fyne.CanvasObject {
	if activeWire.WrapTagChip != nil {
		return activeWire.WrapTagChip(inner)
	}
	return inner
}

// SetEditorUi wires editor lists, forms, and handlers. Call after ConstructEditorTabs.
func SetEditorUi(d WireDeps) {
	activeWire = d
	if d.EU != nil {
		d.EU.win = d.Window
	}
	setEditorLists()
	setEditorForms()
	setEditorButtons()
	setMasksForms()
	setMasksButtons()
	setMaskSelectionButtons()
	setEditorPreviewRefreshButtons()
	updateProgramSelectorOptions()
	setupAllDirtyTracking()
	selectFirstProgramInEditorIfAny()
}
