package editor

import (
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/dialogs"
)

func wireIconVariantEditorDialogs(d WireDeps) {
	if d.EU == nil || d.EU.EditorTabs.ItemsTab == nil {
		return
	}
	iconEditor, ok := d.EU.EditorTabs.ItemsTab.Widgets["iconVariantEditor"].(*custom_widgets.IconVariantEditor)
	if !ok {
		return
	}
	iconEditor.SetDialogDeps(dialogs.ShowErrorWithEscape, dialogs.ShowConfirmWithEscape, dialogs.ShowInformationWithEscape)
}
