package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/validation"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/fieldvalidation"
	"Sqyre/ui/macrocxt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

var macroValidators = fieldvalidation.MacroContext{
	CurrentMacro: currentMacro,
	VariableDefs: macroVariableDefs,
}

func currentMacro() *models.Macro {
	if activeWire.MacroContext.CurrentMacro != nil {
		return activeWire.MacroContext.CurrentMacro()
	}
	return nil
}

func macroVarNames() []string {
	return macrocxt.VariableNames(activeWire.MacroContext, activeWire.MacroVariables)
}

func macroVariableDefs() []models.VariableDef {
	return macrocxt.VariableDefs(activeWire.MacroContext, activeWire.MacroVariableDefs)
}

func newValidatedCoordEntry() *custom_widgets.VarEntryField {
	return custom_widgets.NewVarEntryFieldWithDefs(macroVariableDefs, macroValidators.ValidateNumericExpression)
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
	return tabExtraValid(tab)
}

func tabExtraValid(tab *EditorTab) bool {
	if tab == nil || tab.Widgets == nil {
		return true
	}
	et := shell().EditorTabs
	w := tab.Widgets

	hasName := tab == et.ProgramsTab || tab == et.ItemsTab || tab == et.PointsTab ||
		tab == et.SearchAreasTab || tab == et.MasksTab || tab == et.CollectionsTab
	if hasName {
		nameEntry, ok := w["Name"].(*widget.Entry)
		if !ok || validation.ValidateEntityName(nameEntry.Text) != nil {
			return false
		}
	}

	switch tab {
	case et.ItemsTab:
		return validation.ValidateItemGridFields(
			custom_widgets.EntryText(w["Cols"]),
			custom_widgets.EntryText(w["Rows"]),
			custom_widgets.EntryText(w["StackMax"]),
		) == nil
	case et.SearchAreasTab:
		return validation.ValidateSearchAreaSave(searchAreaFromWidgets(w)) == nil
	case et.CollectionsTab:
		return validateCollectionForSave(w) == nil
	}
	return true
}

func validateEntityNameForSave(name string) error {
	return validation.ValidateEntityName(name)
}

func validateItemGridForSave(w map[string]fyne.CanvasObject) error {
	return validation.ValidateItemGridFields(
		custom_widgets.EntryText(w["Cols"]),
		custom_widgets.EntryText(w["Rows"]),
		custom_widgets.EntryText(w["StackMax"]),
	)
}

func validateSearchAreaForSave(w map[string]fyne.CanvasObject) error {
	return validation.ValidateSearchAreaSave(searchAreaFromWidgets(w))
}
