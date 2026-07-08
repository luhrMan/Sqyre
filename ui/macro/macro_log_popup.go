package macro

import (
	"errors"

	"Sqyre/internal/models/repositories"
	"Sqyre/internal/panicsafe"
	"Sqyre/internal/services"
	"Sqyre/ui/dialogs"

	"fyne.io/fyne/v2"
)

var macroLogGetWindow func() fyne.Window

// InitMacroLogPopup wires the execution-log view and panic UI with services.
// Call once from package ui after the main window exists (avoids ui↔macro import cycle).
func InitMacroLogPopup(getWindow func() fyne.Window) {
	macroLogGetWindow = getWindow

	services.SetShowMacroLogPopupFunc(ActivateMacroLog)
	panicsafe.OnPanicNotifyUser = func(message string) {
		fyne.Do(func() {
			if w := macroLogGetWindow(); w != nil {
				dialogs.ShowErrorWithEscape(errors.New(message), w)
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
