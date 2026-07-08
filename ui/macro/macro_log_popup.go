package macro

import (
	"errors"

	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
)

var (
	macroLogGetWindow            func() fyne.Window
	macroLogAddDialogEscapeClose func(d dialog.Dialog, parent fyne.Window)
	macroLogShowErrorWithEscape  func(err error, parent fyne.Window)
)

// InitMacroLogPopup wires the execution-log view and panic UI with services.
// Call once from package ui after the main window exists (avoids ui↔macro import cycle).
func InitMacroLogPopup(
	getWindow func() fyne.Window,
	addDialogEscapeClose func(d dialog.Dialog, parent fyne.Window),
	showErrorWithEscape func(err error, parent fyne.Window),
) {
	macroLogGetWindow = getWindow
	macroLogAddDialogEscapeClose = addDialogEscapeClose
	macroLogShowErrorWithEscape = showErrorWithEscape

	services.SetShowMacroLogPopupFunc(ActivateMacroLog)
	services.OnPanicNotifyUser = func(message string) {
		fyne.Do(func() {
			w := macroLogGetWindow()
			if w != nil && macroLogShowErrorWithEscape != nil {
				macroLogShowErrorWithEscape(errors.New(message), w)
			}
		})
	}
}

// ActivateMacroLog binds the running macro's log capture to its tab's Log view.
// It selects the macro's tab (so its actions and execution highlight are visible)
// but leaves the inner tab on Actions so highlighting can be seen. If the macro
// isn't open in a tab (e.g. hotkey-triggered while closed) it is opened first.
// Called on the UI thread when a macro starts.
func ActivateMacroLog(macroName string) {
	mtabs := activeWire.Mui.MTabs
	if mtabs == nil {
		return
	}

	content := contentForMacro(mtabs, macroName)
	if content == nil {
		// Macro isn't open in a tab; open it so the log/live variables and the
		// action highlight are visible during execution.
		m, err := repositories.MacroRepo().Get(macroName)
		if err != nil || m == nil {
			return
		}
		// openMacroTabNoHotkey avoids re-registering the hotkey that may have
		// just triggered this run (AddMacroTab would re-register it).
		openMacroTabForLog(m)
		content = contentForMacro(mtabs, macroName)
		if content == nil {
			return
		}
	}

	for _, item := range mtabs.Items {
		if item.Text == macroName {
			mtabs.Select(item)
			break
		}
	}
	content.BindLog(macroName)
}

func contentForMacro(mtabs *MacroTabs, macroName string) *MacroTabContent {
	for _, item := range mtabs.Items {
		if item.Text != macroName {
			continue
		}
		return ensureMacroTabContent(item.Content)
	}
	return nil
}
