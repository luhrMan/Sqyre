package actiondialog

import (
	"Sqyre/internal/models"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/macrocxt"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/driver/desktop"
)

// Deps supplies window, macro context, and ui callbacks from package ui (avoids import cycle).
type Deps struct {
	Window fyne.Window

	ClearOpenActionDialog      func()
	SetActionDialog            func(dialog.Dialog)
	ClearActionDialogIfCurrent func(d dialog.Dialog)

	MacroContext       macrocxt.Provider
	MacroVariables     func() []string
	MacroVariableDefs  func() []models.VariableDef
	CurrentMacroName   func() string

	PreviewExpression func(expr string) (string, error)

	AddDialogEscapeClose     func(d dialog.Dialog, parent fyne.Window)
	AddActionDialogEnterSave func(d dialog.Dialog, parent fyne.Window, onSave func())
	ShowRecordingOverlay func(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func()
	ShowHotkeyRecordDialog func(parent fyne.Window, stableDuration time.Duration, onRecorded func(keys []string))
	ShowKeyRecordDialog    func(parent fyne.Window, onRecorded func(key string))
}

var active Deps

func SetDeps(d Deps) { active = d }

func macroVarNames() []string {
	return macrocxt.VariableNames(active.MacroContext, active.MacroVariables)
}

func macroVariableDefs() []models.VariableDef {
	return macrocxt.VariableDefs(active.MacroContext, active.MacroVariableDefs)
}

func currentMacroName() string {
	if active.CurrentMacroName != nil {
		return active.CurrentMacroName()
	}
	return ""
}

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

func resolveVariablePreview(text string) string {
	resolved, err := services.ResolveVariables(text, currentMacro())
	if err != nil || resolved == text {
		return ""
	}
	return "→ " + resolved
}

var dialogValidatedFields []*custom_widgets.VarEntryField

func resetDialogValidation() {
	dialogValidatedFields = nil
}

func trackValidatedField(field *custom_widgets.VarEntryField) *custom_widgets.VarEntryField {
	dialogValidatedFields = append(dialogValidatedFields, field)
	return field
}

func newValidatedVarEntry(validate func(text string) services.EntryValidation) *custom_widgets.VarEntryField {
	return trackValidatedField(custom_widgets.NewVarEntryFieldWithDefs(macroVariableDefs, validate))
}

func newValidatedMultiLineVarEntry(validate func(text string) services.EntryValidation) *custom_widgets.VarEntryField {
	return trackValidatedField(custom_widgets.NewMultiLineVarEntryFieldWithDefs(macroVariableDefs, validate))
}

func newReferenceVarEntry() *custom_widgets.VarEntryField {
	f := newValidatedVarEntry(validateVariableReferences)
	f.ResolvePreview = resolveVariablePreview
	return f
}

func newReferenceMultiLineVarEntry() *custom_widgets.VarEntryField {
	f := newValidatedMultiLineVarEntry(validateVariableReferences)
	f.ResolvePreview = resolveVariablePreview
	return f
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
