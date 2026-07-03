package editor

import (
	"Sqyre/ui/custom_widgets"
)

func wireIconVariantEditorDialogs(d WireDeps) {
	if d.EU == nil || d.EU.EditorTabs.ItemsTab == nil {
		return
	}
	editor, ok := d.EU.EditorTabs.ItemsTab.Widgets["iconVariantEditor"].(*custom_widgets.IconVariantEditor)
	if !ok {
		return
	}
	editor.SetDialogDeps(d.ShowErrorWithEscape, d.ShowConfirmWithEscape, d.ShowInformationWithEscape)
}
