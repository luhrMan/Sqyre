package ui

import (
	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
)

// shouldSaveActionDialogOnEnter reports whether a global Enter handler should save
// the action dialog. Enter is ignored while completion is active, recently consumed
// by completion, or while a multi-line text field has focus.
func shouldSaveActionDialogOnEnter(parent fyne.Window) bool {
	if parent == nil {
		return false
	}
	if completionentry.IsCompletionActive() {
		return false
	}
	if completionentry.IsActionDialogEnterSuppressed() {
		return false
	}
	focused := parent.Canvas().Focused()
	if completionentry.IsNavListFocused(focused) {
		return false
	}
	if custom_widgets.IsMultiLineTextFocused(focused) {
		return false
	}
	return true
}
