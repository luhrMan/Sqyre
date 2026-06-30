package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
)

func currentMacro() *models.Macro {
	if activeWire.MacroContext.CurrentMacro != nil {
		return activeWire.MacroContext.CurrentMacro()
	}
	return nil
}

func validateNumericExpression(text string) services.EntryValidation {
	return services.ValidateNumericExpression(text, currentMacro())
}

func newValidatedCoordEntry() *custom_widgets.VarEntryField {
	return custom_widgets.NewVarEntryField(macroVarNames, validateNumericExpression)
}

func tabValidatedFields(tab *EditorTab) []*custom_widgets.VarEntryField {
	if tab == nil || tab.Widgets == nil {
		return nil
	}
	out := make([]*custom_widgets.VarEntryField, 0, 8)
	for _, w := range tab.Widgets {
		if f, ok := w.(*custom_widgets.VarEntryField); ok {
			out = append(out, f)
		}
	}
	return out
}

func allTabFieldsValid(tab *EditorTab) bool {
	for _, f := range tabValidatedFields(tab) {
		if !f.Valid() {
			return false
		}
	}
	return true
}

// RefreshVarEntryInsertButtons re-evaluates + button state on all coordinate fields.
// Editor widgets are built before macro variable wiring, so this must run after SetEditorUi
// and whenever the data editor is shown.
func RefreshVarEntryInsertButtons() {
	eu := shell()
	if eu == nil {
		return
	}
	et := eu.EditorTabs
	for _, tab := range []*EditorTab{
		et.PointsTab, et.SearchAreasTab, et.MasksTab,
	} {
		for _, f := range tabValidatedFields(tab) {
			f.Entry.UpdateInsertButton()
		}
	}
}
