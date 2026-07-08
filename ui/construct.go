package ui

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/services"
	"Sqyre/ui/editor"
	"Sqyre/ui/macro"
	"Sqyre/ui/macrocxt"
	"Sqyre/ui/recording"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/widget"
)

func (u *Ui) constructMacroUi() fyne.CanvasObject {
	boundLocXLabel = widget.NewLabelWithData(binding.NewString())
	boundLocYLabel = widget.NewLabelWithData(binding.NewString())
	return macro.ConstructMacroUi(u.Mui, boundLocXLabel, boundLocYLabel, WrapSqyreFrame)
}

// SaveOpenMacros persists which macro tabs are open (delegates to ui/macro).
func SaveOpenMacros() {
	macro.SaveOpenMacros()
}

// macroContext returns variable metadata for the currently selected macro tab.
func macroContext() macrocxt.Provider {
	u := GetUi()
	return macrocxt.Provider{
		CurrentMacro: func() *models.Macro {
			if u == nil || u.Mui == nil || u.Mui.MTabs == nil {
				return nil
			}
			st := u.Mui.MTabs.SelectedTab()
			if st == nil {
				return nil
			}
			return st.Macro
		},
	}
}

// SetEditorUi wires editor lists, forms, and handlers (implementation in ui/editor).
func SetEditorUi() {
	u := GetUi()
	ctx := macroContext()
	editor.SetEditorUi(editor.WireDeps{
		Window:     u.Window,
		EU:         u.EditorUi,
		MacroMTabs: func() *macro.MacroTabs { return u.Mui.MTabs },
		MacroContext:      ctx,
		MacroVariables:    ctx.VariableNames,
		MacroVariableDefs: ctx.VariableDefs,
		NavigationVisible:              func() bool { return u.MainUi.Navigation.Visible() },
		ShowErrorWithEscape:            ShowErrorWithEscape,
		ShowConfirmWithEscape:          ShowConfirmWithEscape,
		ShowInformationWithEscape:      ShowInformationWithEscape,
		AddDialogEscapeClose:           AddDialogEscapeClose,
		AddPopupEscapeClose:            AddPopupEscapeClose,
		ShowRecordingOverlay:           recording.ShowRecordingOverlay,
		ShowSearchAreaRecordingOverlay: recording.ShowSearchAreaRecordingOverlay,
		WrapTagChip:                    WrapTagChip,
	})
}

// previewExpression validates and evaluates a Calculate expression against the
// currently selected macro's declared and action-produced variables.
func previewExpression(expr string) (string, error) {
	m := macroContext().CurrentMacro()
	if m == nil {
		return "", nil
	}
	return services.PreviewCalculate(expr, m)
}

// SetMacroUi wires macro tab behavior and restores open macros (implementation in ui/macro).
func SetMacroUi() {
	u := GetUi()
	ctx := macroContext()
	macro.SetMacroUi(macro.WireDeps{
		Window:                u.Window,
		Mui:                   u.Mui,
		MacroContext:          ctx,
		MacroVariableDefs:     ctx.VariableDefs,
		RefreshItemsAccordion: editor.RefreshItemsAccordionItems,
		ShowHotkeyRecordDialog: func(parent fyne.Window, stableDuration time.Duration, onRecorded func(keys []string)) {
			recording.ShowHotkeyRecordDialog(parent, stableDuration, AddDialogEscapeClose, onRecorded)
		},
		ShowKeyRecordDialog: recording.ShowKeyRecordDialog,
		ShowErrorWithEscape:   ShowErrorWithEscape,
		AddDialogEscapeClose:  AddDialogEscapeClose,
		AddPopupEscapeClose:   AddPopupEscapeClose,
		ShowConfirmWithEscape: ShowConfirmWithEscape,
		ShowAddActionPicker: func() {
			u.ShowAddActionPicker()
		},
		ShowPointPicker: func(initial actions.CoordinateRef, onSelect func(actions.CoordinateRef), onClosed func()) {
			ShowPointPicker(u.Window, initial, onSelect, onClosed)
		},
		ShowSearchAreaPicker: func(initial actions.CoordinateRef, onSelect func(actions.CoordinateRef), onClosed func()) {
			ShowSearchAreaPicker(u.Window, initial, onSelect, onClosed)
		},
		ShowItemsPicker: func(getTargets func() []string, onChanged func(newTargets []string), onClosed func()) {
			ShowItemsPicker(u.Window, getTargets, onChanged, onClosed)
		},
		PreviewExpression: previewExpression,
		ShowRecordingOverlay: func(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func() {
			return recording.ShowRecordingOverlay(onClosed, onMouseDown)
		},
		RegisterTooltipEnterSave: func(onSave func()) func() {
			return RegisterActionTooltipEnterSave(u.Window, onSave)
		},
		WrapTagChip:    WrapTagChip,
		WrapSqyreFrame: WrapSqyreFrame,
	})
}
