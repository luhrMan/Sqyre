package macro

import (
	"image/color"

	"Sqyre/internal/assets"
	"Sqyre/internal/models/serialize"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// copiedNodeMap is the clipboard for macro tree copy/paste (nil when empty).
var copiedNodeMap map[string]any

type MacroUi struct {
	MTabs             *MacroTabs
	MacroSelectButton *widget.Button
	MacroToolbars     struct {
		TopToolbar    *fyne.Container
		BottomToolbar *fyne.Container
	}
}

// WrapSqyreFrame matches ui.WrapSqyreFrame; injected to avoid importing package ui.
type WrapSqyreFrameFunc func(inner fyne.CanvasObject) fyne.CanvasObject

// ConstructMacroUi builds the macro toolbar and tab strip. boundLocX/Y labels are bound to mouse position elsewhere.
func ConstructMacroUi(mui *MacroUi, boundLocXLabel, boundLocYLabel *widget.Label, wrapFrame WrapSqyreFrameFunc) *fyne.Container {
	unselectNodeButton := ttwidget.NewButtonWithIcon("", theme.RadioButtonIcon(), func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.UnselectAll()
		st.SelectedNode = ""
	})

	moveDownNodeButton := ttwidget.NewButtonWithIcon("", assets.ChevronDownIcon, func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.moveNode(st.SelectedNode, false)
	})
	moveUpNodeButton := ttwidget.NewButtonWithIcon("", assets.ChevronUpIcon, func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.moveNode(st.SelectedNode, true)
	})
	copyNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentCopyIcon(), func() {
		st := mui.MTabs.SelectedTab()
		if st == nil || st.SelectedNode == "" {
			return
		}
		node := st.Macro.Root.GetAction(st.SelectedNode)
		if node == nil || node.GetParent() == nil {
			return
		}
		m, err := serialize.ActionToMap(node)
		if err != nil {
			return
		}
		copiedNodeMap = m
	})
	pasteNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentPasteIcon(), func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.PasteNode(copiedNodeMap)
	})
	playMacroButton := ttwidget.NewButtonWithIcon("", theme.MediaPlayIcon(), func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		go func() {
			services.ExecuteMacroWithLogging(st.Macro)
		}()
	})
	services.SetMacroRunningCallback(func(running bool) {
		if running {
			playMacroButton.Disable()
		} else {
			playMacroButton.Enable()
		}
	})

	unselectNodeButton.SetToolTip("unselect nodes")
	moveDownNodeButton.SetToolTip("move node down")
	moveUpNodeButton.SetToolTip("move node up")
	copyNodeButton.SetToolTip("copy node")
	pasteNodeButton.SetToolTip("paste node below")
	playMacroButton.SetToolTip("start macro execution")

	macroActivity := &widget.Activity{}
	services.SetActivityReporter(macroActivity)

	mui.MacroToolbars.TopToolbar =
		container.NewGridWithColumns(2,
			container.NewHBox(
				unselectNodeButton,
				moveDownNodeButton,
				moveUpNodeButton,
				copyNodeButton,
				pasteNodeButton,
				layout.NewSpacer(),
				layout.NewSpacer(),
				layout.NewSpacer(),
				playMacroButton,
				macroActivity,
				widget.NewLabel("Macro Name:"),
			),
			container.NewBorder(nil, nil, nil,
				mui.MacroSelectButton,
				mui.MTabs.MacroNameEntry,
			),
		)

	mousePosition :=
		container.NewHBox(
			container.NewBorder(nil, nil,
				widget.NewLabel("X: "), nil,
				boundLocXLabel,
			),
			container.NewBorder(nil, nil,
				widget.NewLabel("Y: "), nil,
				boundLocYLabel,
			),
		)

	globaldelaytt := ttwidget.NewIcon(theme.HistoryIcon())
	globaldelaytt.SetToolTip("global delay (ms)")
	bottomLeftContent := container.NewHBox(globaldelaytt, mui.MTabs.BoundGlobalDelayEntry, mousePosition)
	bottomLeft := wrapFrame(container.NewPadded(bottomLeftContent))
	bottomFiller := canvas.NewRectangle(color.Transparent)
	bottomRightContent := container.NewHBox(
		widget.NewLabel("Hotkey:"),
		mui.MTabs.MacroHotkeyLabel,
		widget.NewLabel("Trigger:"),
		mui.MTabs.HotkeyTriggerRadio,
		mui.MTabs.MacroHotkeyRecordBtn,
	)
	bottomRight := wrapFrame(container.NewPadded(bottomRightContent))
	mui.MacroToolbars.BottomToolbar = container.NewBorder(nil, nil, bottomLeft, bottomRight, bottomFiller)

	return container.NewBorder(
		mui.MacroToolbars.TopToolbar,
		mui.MacroToolbars.BottomToolbar,
		nil,
		nil,
		mui.MTabs,
	)
}
