package actiondialog

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/services"
	"Sqyre/internal/validation"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/fieldvalidation"
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

	MacroContext      macrocxt.Provider
	MacroVariables    func() []string
	MacroVariableDefs func() []models.VariableDef
	CurrentMacroName  func() string

	PreviewExpression func(expr string) (string, error)

	AddDialogEscapeClose     func(d dialog.Dialog, parent fyne.Window)
	AddActionDialogEnterSave func(d dialog.Dialog, parent fyne.Window, onSave func())
	ShowErrorWithEscape        func(err error, parent fyne.Window)
	ShowRecordingOverlay       func(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func()
	ShowHotkeyRecordDialog     func(parent fyne.Window, stableDuration time.Duration, onRecorded func(keys []string))
	ShowKeyRecordDialog        func(parent fyne.Window, onRecorded func(key string))
}

var active Deps

var macroValidators fieldvalidation.MacroContext

func SetDeps(d Deps) {
	active = d
	macroValidators = fieldvalidation.MacroContext{
		CurrentMacro: currentMacro,
		VariableDefs: macroVariableDefs,
	}
}

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
	return macroValidators.ValidateNumericExpression(text)
}

func validateCalculateExpression(text string) services.EntryValidation {
	return macroValidators.ValidateCalculateExpression(text)
}

func validateSetVariableValue(text string) services.EntryValidation {
	return macroValidators.ValidateSetVariableValue(text)
}

func validateVariableReferences(text string) services.EntryValidation {
	return macroValidators.ValidateVariableReferences(text)
}

func resolveVariablePreview(text string) string {
	return macroValidators.ResolveVariablePreview(text)
}

var (
	dialogValidatedFields  []*custom_widgets.VarEntryField
	dialogValidityChecks   []func() bool
	dialogValidationNotify func()
)

func resetDialogValidation() {
	dialogValidatedFields = nil
	dialogValidityChecks = nil
	dialogValidationNotify = nil
}

func trackValidatedField(field *custom_widgets.VarEntryField) *custom_widgets.VarEntryField {
	dialogValidatedFields = append(dialogValidatedFields, field)
	return field
}

func trackDialogValidityCheck(fn func() bool) {
	dialogValidityChecks = append(dialogValidityChecks, fn)
}

func notifyDialogValidationChanged() {
	if dialogValidationNotify != nil {
		dialogValidationNotify()
	}
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
	for _, check := range dialogValidityChecks {
		if !check() {
			return false
		}
	}
	return true
}

func wireDialogValidation(onChange func()) {
	dialogValidationNotify = onChange
	for _, f := range dialogValidatedFields {
		f.SetOnValidationChanged(onChange)
	}
}

func showActionDialogError(err error) {
	if err == nil {
		return
	}
	if active.ShowErrorWithEscape != nil && active.Window != nil {
		active.ShowErrorWithEscape(err, active.Window)
		return
	}
	dialog.ShowError(err, active.Window)
}

func validateActionBeforeSave(action actions.ActionInterface) error {
	return validation.ValidateAction(action, currentMacro())
}
