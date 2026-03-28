package actiondialog

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/driver/desktop"
)

// Deps supplies window, macro context, and ui callbacks from package ui (avoids import cycle).
type Deps struct {
	Window fyne.Window

	ClearOpenActionDialog      func()
	SetActionDialog            func(d dialog.Dialog)
	ClearActionDialogIfCurrent func(d dialog.Dialog)

	MacroVariables   func() []string
	CurrentMacroName func() string

	AddDialogEscapeClose func(d dialog.Dialog, parent fyne.Window)
	ShowRecordingOverlay func(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func()
}

var active Deps

// SetDeps wires the action dialog shell. Call from ui during ConstructUi before SetMacroUi.
func SetDeps(d Deps) { active = d }

func macroVarNames() []string {
	if active.MacroVariables != nil {
		return active.MacroVariables()
	}
	return nil
}

func currentMacroName() string {
	if active.CurrentMacroName != nil {
		return active.CurrentMacroName()
	}
	return ""
}
