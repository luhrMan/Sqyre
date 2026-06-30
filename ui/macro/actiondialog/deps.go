package actiondialog

import (
	"Sqyre/internal/models"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/macrocxt"

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

	MacroContext     macrocxt.Provider
	MacroVariables   func() []string
	CurrentMacroName func() string

	// PreviewExpression evaluates a Calculate expression against the current
	// macro's variables and returns a formatted result or an error.
	PreviewExpression func(expr string) (string, error)

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
	if active.MacroContext.CurrentMacro != nil {
		return active.MacroContext.VariableNames()
	}
	return nil
}

func currentMacroName() string {
	if active.CurrentMacroName != nil {
		return active.CurrentMacroName()
	}
	return ""
}

// previewExpression evaluates expr against the current macro for the live
// Calculate preview. Returns ("", nil) when no preview provider is wired.
func previewExpression(expr string) (string, error) {
	if active.PreviewExpression != nil {
		return active.PreviewExpression(expr)
	}
	return "", nil
}

func currentMacro() *models.Macro {
	if active.MacroContext.CurrentMacro != nil {
		return active.MacroContext.CurrentMacro()
	}
	return nil
}

func validateNumericExpression(text string) services.EntryValidation {
	return services.ValidateNumericExpression(text, currentMacro())
}

func validateCalculateExpression(text string) services.EntryValidation {
	return services.ValidateCalculateExpression(text, currentMacro())
}

func validateSetVariableValue(text string) services.EntryValidation {
	return services.ValidateSetVariableValue(text, currentMacro())
}

func validateVariableReferences(text string) services.EntryValidation {
	return services.ValidateVariableReferences(text, currentMacro())
}

// dialogValidatedFields collects validated entries for the active action dialog.
var dialogValidatedFields []*custom_widgets.VarEntryField

func resetDialogValidation() {
	dialogValidatedFields = nil
}

func trackValidatedField(field *custom_widgets.VarEntryField) *custom_widgets.VarEntryField {
	dialogValidatedFields = append(dialogValidatedFields, field)
	return field
}

func newValidatedVarEntry(validate func(text string) services.EntryValidation) *custom_widgets.VarEntryField {
	return trackValidatedField(custom_widgets.NewVarEntryField(macroVarNames, validate))
}

func newValidatedMultiLineVarEntry(validate func(text string) services.EntryValidation) *custom_widgets.VarEntryField {
	return trackValidatedField(custom_widgets.NewMultiLineVarEntryField(macroVarNames, validate))
}

func allDialogFieldsValid() bool {
	for _, f := range dialogValidatedFields {
		if !f.Valid() {
			return false
		}
	}
	return true
}

func wireDialogValidation(onChange func()) {
	for _, f := range dialogValidatedFields {
		f.SetOnValidationChanged(onChange)
	}
}
