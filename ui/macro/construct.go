package macro

import (
	"image/color"
	"log"

	"Sqyre/internal/assets"
	"Sqyre/internal/models/repositories"
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
		if !st.PasteNode(copiedNodeMap) {
			return
		}
		if err := repositories.MacroRepo().Set(st.Macro.Name, st.Macro); err != nil {
			log.Printf("failed to save macro after paste: %v", err)
		}
	})
	expandAllBtn := ttwidget.NewButtonWithIcon("", assets.DoubleDownChevronIcon, func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.OpenAllBranches()
	})
	collapseAllBtn := ttwidget.NewButtonWithIcon("", assets.DoubleUpChevronIcon, func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.CloseAllBranches()
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
	highlightPump := newHighlightPump(mui)
	services.SetHighlightCallback(highlightPump.handle)
	services.SetMacroRunningCallback(func(running bool) {
		if running {
			playMacroButton.Disable()
			startLogPump()
			highlightPump.startTicker()
		} else {
			playMacroButton.Enable()
			stopLogPump()
			highlightPump.stopTicker()
		}
	})

	unselectNodeButton.SetToolTip("unselect nodes")
	moveDownNodeButton.SetToolTip("move node down")
	moveUpNodeButton.SetToolTip("move node up")
	copyNodeButton.SetToolTip("copy node")
	pasteNodeButton.SetToolTip("paste node below")
	expandAllBtn.SetToolTip("expand all branches")
	collapseAllBtn.SetToolTip("collapse all branches")
	playMacroButton.SetToolTip("start macro execution")

	mui.MacroToolbars.TopToolbar =
		container.NewGridWithColumns(2,
			container.NewHBox(
				unselectNodeButton,
				moveDownNodeButton,
				moveUpNodeButton,
				copyNodeButton,
				pasteNodeButton,
				expandAllBtn,
				collapseAllBtn,
				layout.NewSpacer(),
				layout.NewSpacer(),
				layout.NewSpacer(),
				playMacroButton,
				services.MacroActiveIndicator(),
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
	globaldelaytt.SetToolTip("delay between actions (ms)")
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
