package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"fmt"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func formatMacroDelayTooltip(m *models.Macro) string {
	if m == nil {
		return "Action delays (ms)"
	}
	var parts []string
	if m.GlobalDelay > 0 {
		parts = append(parts, fmt.Sprintf("Global: %d ms", m.GlobalDelay))
	}
	if m.KeyboardDelay > 0 {
		parts = append(parts, fmt.Sprintf("Keyboard: %d ms", m.KeyboardDelay))
	}
	if m.MouseDelay > 0 {
		parts = append(parts, fmt.Sprintf("Mouse: %d ms", m.MouseDelay))
	}
	if len(parts) == 0 {
		return "Action delays (ms)"
	}
	return strings.Join(parts, "\n")
}

func updateMacroDelayButton(mtabs *MacroTabs, m *models.Macro) {
	if mtabs == nil || mtabs.MacroDelayBtn == nil {
		return
	}
	mtabs.MacroDelayBtn.SetToolTip(formatMacroDelayTooltip(m))
	mtabs.MacroDelayBtn.Refresh()
}

func showMacroDelayPopup(mtabs *MacroTabs) {
	if mtabs == nil || mtabs.MacroDelayBtn == nil {
		return
	}
	mt := mtabs.SelectedTab()
	if mt == nil || mt.Macro == nil {
		return
	}
	anchor := mtabs.MacroDelayBtn
	holder := fyne.CurrentApp().Driver().CanvasForObject(anchor)
	if holder == nil {
		return
	}

	globalRow := container.NewBorder(nil, nil,
		widget.NewLabel("Global (ms):"), nil,
		mtabs.BoundGlobalDelayEntry,
	)
	keyboardRow := container.NewBorder(nil, nil,
		widget.NewLabel("Keyboard (ms):"), nil,
		mtabs.BoundKeyboardDelayEntry,
	)
	mouseRow := container.NewBorder(nil, nil,
		widget.NewLabel("Mouse (ms):"), nil,
		mtabs.BoundMouseDelayEntry,
	)
	content := container.NewPadded(container.NewVBox(
		widget.NewLabel("Delay between actions"),
		globalRow,
		keyboardRow,
		mouseRow,
	))
	popup := widget.NewPopUp(content, holder)
	mtabs.macroDelayPopup = popup

	min := content.MinSize().Add(fyne.NewSize(theme.Padding()*4, theme.Padding()*4))
	popup.Resize(min)

	pos := fyne.CurrentApp().Driver().AbsolutePositionForObject(anchor)
	popup.ShowAtPosition(pos.Add(fyne.NewPos(0, anchor.Size().Height)))
}

func persistMacroDelays(mtabs *MacroTabs) {
	mt := mtabs.SelectedTab()
	if mt == nil || mt.Macro == nil {
		return
	}
	if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
		return
	}
	updateMacroDelayButton(mtabs, mt.Macro)
}

func wireMacroDelayHandlers(mtabs *MacroTabs) {
	if mtabs == nil {
		return
	}
	if mtabs.MacroDelayBtn != nil {
		mtabs.MacroDelayBtn.Importance = widget.LowImportance
		mtabs.MacroDelayBtn.OnTapped = func() { showMacroDelayPopup(mtabs) }
	}
	mtabs.BoundGlobalDelayEntry.OnChanged = func(gd int) {
		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}
		mt.Macro.GlobalDelay = gd
		persistMacroDelays(mtabs)
	}
	mtabs.BoundKeyboardDelayEntry.OnChanged = func(kd int) {
		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}
		mt.Macro.KeyboardDelay = kd
		persistMacroDelays(mtabs)
	}
	mtabs.BoundMouseDelayEntry.OnChanged = func(md int) {
		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}
		mt.Macro.MouseDelay = md
		persistMacroDelays(mtabs)
	}
}
