package editor

import (
	"Sqyre/internal/models"
	"Sqyre/ui/macro"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/widget"

	"Sqyre/ui/macrocxt"
)

// WireDeps supplies the editor shell and callbacks from package ui (avoids import cycle).
type WireDeps struct {
	Window                fyne.Window
	EU                    *EditorUi
	MacroMTabs            func() *macro.MacroTabs
	MacroContext          macrocxt.Provider
	// MacroVariables returns variable names from the currently selected macro (for VarEntry completion).
	MacroVariables      func() []string
	MacroVariableDefs   func() []models.VariableDef
	NavigationVisible     func() bool
	ShowErrorWithEscape   func(err error, parent fyne.Window)
	ShowConfirmWithEscape func(title, message string, callback func(bool), parent fyne.Window)
	ShowInformationWithEscape func(title, message string, parent fyne.Window)
	AddDialogEscapeClose  func(d dialog.Dialog, parent fyne.Window)
	AddPopupEscapeClose   func(pop *widget.PopUp, parent fyne.Window) dialog.Dialog
	ShowRecordingOverlay    func(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func()
	ShowSearchAreaRecordingOverlay func(onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int))
	// WrapTagChip styles tag rows in the Items tab (from ui theme helpers; avoids import cycle).
	WrapTagChip func(inner fyne.CanvasObject) fyne.CanvasObject
}

var activeWire WireDeps

func shell() *EditorUi { return activeWire.EU }

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
	setMasksButtons()
	setMaskSelectionButtons()
	setEditorRecordHandlers()
	setEditorPreviewRefreshHandlers()
	wireIconVariantEditorDialogs(d)
	updateProgramSelectorOptions()
	setupAllDirtyTracking()
	selectFirstProgramInEditorIfAny()
}
