package ui

import (
	"Sqyre/internal/models/actions"
	"Sqyre/ui/editor"
	"Sqyre/ui/macro"
	"Sqyre/ui/macro/actiondialog"
	"Sqyre/ui/recording"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
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

// SetEditorUi wires editor lists, forms, and handlers (implementation in ui/editor).
func SetEditorUi() {
	u := GetUi()
	editor.SetEditorUi(editor.WireDeps{
		Window:     u.Window,
		EU:         u.EditorUi,
		MacroMTabs: func() *macro.MacroTabs { return u.Mui.MTabs },
		MacroVariables: func() []string {
			st := u.Mui.MTabs.SelectedTab()
			if st == nil || st.Macro == nil {
				return nil
			}
			return st.Macro.CollectDefinedVariables()
		},
		NavigationVisible:              func() bool { return u.MainUi.Navigation.Visible() },
		ShowErrorWithEscape:            ShowErrorWithEscape,
		ShowConfirmWithEscape:          ShowConfirmWithEscape,
		AddDialogEscapeClose:           AddDialogEscapeClose,
		ShowRecordingOverlay:           recording.ShowRecordingOverlay,
		ShowSearchAreaRecordingOverlay: recording.ShowSearchAreaRecordingOverlay,
		WrapTagChip:                    WrapTagChip,
	})
}

// SetActionDialogDeps wires the macro action editor dialog (implementation in ui/macro/actiondialog).
func SetActionDialogDeps() {
	u := GetUi()
	actiondialog.SetDeps(actiondialog.Deps{
		Window: u.Window,
		ClearOpenActionDialog: func() {
			if u.MainUi != nil {
				u.MainUi.ActionDialog = nil
			}
		},
		SetActionDialog: func(d dialog.Dialog) {
			if u.MainUi != nil {
				u.MainUi.ActionDialog = d
			}
		},
		ClearActionDialogIfCurrent: func(d dialog.Dialog) {
			if u.MainUi != nil && u.MainUi.ActionDialog == d {
				u.MainUi.ActionDialog = nil
			}
		},
		MacroVariables: func() []string {
			st := u.Mui.MTabs.SelectedTab()
			if st == nil || st.Macro == nil {
				return nil
			}
			return st.Macro.CollectDefinedVariables()
		},
		CurrentMacroName: func() string {
			st := u.Mui.MTabs.SelectedTab()
			if st == nil || st.Macro == nil {
				return ""
			}
			return st.Macro.Name
		},
		AddDialogEscapeClose: AddDialogEscapeClose,
		ShowRecordingOverlay: recording.ShowRecordingOverlay,
	})
}

// SetMacroUi wires macro tab behavior and restores open macros (implementation in ui/macro).
func SetMacroUi() {
	u := GetUi()
	macro.SetMacroUi(macro.WireDeps{
		Window:                u.Window,
		Mui:                   u.Mui,
		RefreshItemsAccordion: editor.RefreshItemsAccordionItems,
		ShowHotkeyRecordDialog: func(parent fyne.Window, stableDuration time.Duration, onRecorded func(keys []string)) {
			recording.ShowHotkeyRecordDialog(parent, stableDuration, AddDialogEscapeClose, onRecorded)
		},
		ShowErrorWithEscape:   ShowErrorWithEscape,
		AddDialogEscapeClose:  AddDialogEscapeClose,
		ShowConfirmWithEscape: ShowConfirmWithEscape,
		ShowActionDialog: func(action actions.ActionInterface, onSave func(actions.ActionInterface)) {
			actiondialog.ShowActionDialog(action, onSave)
		},
	})
}
