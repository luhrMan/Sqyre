package ui

import (
	"errors"

	"Sqyre/internal/services"

	"github.com/go-vgo/robotgo"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var macroLogPopup dialog.Dialog

// ShowMacroLogPopup displays a popup showing the currently running macro and its log output.
// It is shown when a macro starts and can be closed by the user.
func ShowMacroLogPopup(macroName string) {
	u := GetUi()
	if u == nil || u.Window == nil {
		return
	}

	// Close existing popup if any
	if macroLogPopup != nil {
		macroLogPopup.Hide()
	}

	titleLabel := widget.NewLabel("Macro: " + macroName)
	titleLabel.TextStyle = fyne.TextStyle{Bold: true}

	logEntry := widget.NewMultiLineEntry()
	logEntry.Disable()
	logEntry.Wrapping = fyne.TextWrapOff

	// Initial content from buffer (in case macro already produced logs)
	logEntry.SetText(services.GetMacroLogBuffer())

	scrollContainer := container.NewScroll(logEntry)

	copyBtn := widget.NewButtonWithIcon("Copy", theme.ContentCopyIcon(), func() {
		robotgo.WriteAll(logEntry.Text)
	})
	copyBtn.Importance = widget.MediumImportance

	closeBtn := widget.NewButtonWithIcon("Close", theme.CancelIcon(), func() {
		if macroLogPopup != nil {
			macroLogPopup.Hide()
			macroLogPopup = nil
		}
	})
	closeBtn.Importance = widget.HighImportance

	buttonBar := container.NewHBox(layout.NewSpacer(), copyBtn, closeBtn)

	content := container.NewBorder(
		titleLabel,
		buttonBar,
		nil,
		nil,
		scrollContainer,
	)

	popup := dialog.NewCustomWithoutButtons("Macro Log", content, u.Window)
	canvasSize := u.Window.Canvas().Size()
	popupSize := fyne.NewSize(canvasSize.Width*0.75, canvasSize.Height*0.75)
	popup.Resize(popupSize)
	AddDialogEscapeClose(popup, u.Window)

	macroLogPopup = popup

	// Register callback to append new log lines in real-time
	onLine := func(line string) {
		if macroLogPopup == nil {
			return
		}
		prev := logEntry.Text
		if prev != "" {
			prev += "\n"
		}
		logEntry.SetText(prev + line)
		scrollContainer.ScrollToBottom()
	}

	services.StartMacroLogCapture(macroName, onLine)
	popup.Show()
}

// HideMacroLogPopup closes the macro log popup if it is open.
func HideMacroLogPopup() {
	if macroLogPopup != nil {
		macroLogPopup.Hide()
		macroLogPopup = nil
	}
}

func init() {
	services.SetShowMacroLogPopupFunc(ShowMacroLogPopup)
	// Notify user of any recovered panic (from any goroutine); run on UI thread.
	services.OnPanicNotifyUser = func(message string) {
		fyne.Do(func() {
			u := GetUi()
			if u != nil && u.Window != nil {
				ShowErrorWithEscape(errors.New(message), u.Window)
			}
		})
	}
}
