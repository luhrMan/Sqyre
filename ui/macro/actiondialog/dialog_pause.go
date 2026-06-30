package actiondialog

import (
	"Sqyre/internal/macrohotkey"
	"Sqyre/internal/models/actions"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

func createPauseDialogContent(action *actions.Pause) (fyne.CanvasObject, func()) {
	messageEntry := widget.NewEntry()
	messageEntry.SetPlaceHolder("Optional note shown while paused")
	messageEntry.SetText(action.Message)

	keyLabel := widget.NewLabel(macrohotkey.FormatContinueKey(action.ContinueKey))
	if keyLabel.Text == "" {
		keyLabel.SetText("(not set)")
	}
	keyLabel.TextStyle = fyne.TextStyle{Monospace: true}

	var recordBtn *widget.Button
	recordBtn = widget.NewButtonWithIcon("Record continue key", theme.MediaRecordIcon(), func() {
		if active.ShowHotkeyRecordDialog == nil || active.Window == nil {
			return
		}
		active.ShowHotkeyRecordDialog(active.Window, 1*time.Second, func(keys []string) {
			action.ContinueKey = append([]string(nil), keys...)
			keyLabel.SetText(macrohotkey.FormatContinueKey(keys))
			if keyLabel.Text == "" {
				keyLabel.SetText("(not set)")
			}
		})
	})

	passThroughCheck := ttwidget.NewCheck("Pass continue key to focused app", nil)
	passThroughCheck.SetChecked(action.PassThrough)

	content := widget.NewForm(
		formHint("Message:", messageEntry, "Optional text shown in the log and pause banner while waiting."),
		formHint("Continue key:", container.NewVBox(
			keyLabel,
			recordBtn,
		), "Hold the key combination you want to press to resume the macro. Recorded the same way as macro hotkeys."),
		formHint("", passThroughCheck, "When unchecked, Sqyre tries to suppress the continue key so it does not reach other applications (best effort; on Linux an X11 grab is used when available). When checked, the key behaves normally for the focused app."),
	)

	saveFunc := func() {
		action.Message = messageEntry.Text
		action.PassThrough = passThroughCheck.Checked
	}

	return content, saveFunc
}

func validatePauseAction(action *actions.Pause) error {
	return macrohotkey.ValidateContinueKeyForUI(action.ContinueKey)
}
