package macro

import (
	"errors"

	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var macroLogPopup dialog.Dialog

var (
	macroLogGetWindow            func() fyne.Window
	macroLogAddDialogEscapeClose func(d dialog.Dialog, parent fyne.Window)
	macroLogShowErrorWithEscape  func(err error, parent fyne.Window)
)

// InitMacroLogPopup registers the macro log popup and panic UI with services.
// Call once from package ui after the main window exists (avoids ui↔macro import cycle).
func InitMacroLogPopup(
	getWindow func() fyne.Window,
	addDialogEscapeClose func(d dialog.Dialog, parent fyne.Window),
	showErrorWithEscape func(err error, parent fyne.Window),
) {
	macroLogGetWindow = getWindow
	macroLogAddDialogEscapeClose = addDialogEscapeClose
	macroLogShowErrorWithEscape = showErrorWithEscape

	services.SetShowMacroLogPopupFunc(ShowMacroLogPopup)
	services.OnPanicNotifyUser = func(message string) {
		fyne.Do(func() {
			w := macroLogGetWindow()
			if w != nil && macroLogShowErrorWithEscape != nil {
				macroLogShowErrorWithEscape(errors.New(message), w)
			}
		})
	}
}

// ShowMacroLogPopup displays a popup showing the currently running macro and its log output.
// It is shown when a macro starts and can be closed by the user.
func ShowMacroLogPopup(macroName string) {
	w := macroLogGetWindow()
	if w == nil {
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
		if c := fyne.CurrentApp().Clipboard(); c != nil {
			c.SetContent(logEntry.Text)
		}
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

	popup := dialog.NewCustomWithoutButtons("Macro Log", content, w)
	canvasSize := w.Canvas().Size()
	popupSize := fyne.NewSize(canvasSize.Width*0.75, canvasSize.Height*0.75)
	popup.Resize(popupSize)
	if macroLogAddDialogEscapeClose != nil {
		macroLogAddDialogEscapeClose(popup, w)
	}

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
