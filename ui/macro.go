package ui

import (
	"Squire/internal/assets"
	"Squire/internal/models/serialize"
	"Squire/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
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

func (u *Ui) constructMacroUi() *fyne.Container {
	boundLocXLabel = widget.NewLabelWithData(binding.NewString())
	boundLocYLabel = widget.NewLabelWithData(binding.NewString())
	mui := u.Mui

	// addNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentAddIcon(), func() {
	// 	// Show action selection menu - for now, use the main menu
	// 	// In the future, could show a popup menu here
	// 	// For now, users can use the main menu to add actions
	// })
	unselectNodeButton := ttwidget.NewButtonWithIcon("", theme.RadioButtonIcon(), func() {
		st := mui.MTabs.SelectedTab()
		st.UnselectAll()
		st.SelectedNode = ""
	})

	moveDownNodeButton := ttwidget.NewButtonWithIcon("", assets.ChevronDownIcon, func() {
		st := mui.MTabs.SelectedTab()
		st.moveNode(st.SelectedNode, false)
	})
	moveUpNodeButton := ttwidget.NewButtonWithIcon("", assets.ChevronUpIcon, func() {
		st := mui.MTabs.SelectedTab()
		st.moveNode(st.SelectedNode, true)
	})
	copyNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentCopyIcon(), func() {
		st := mui.MTabs.SelectedTab()
		if st.SelectedNode == "" {
			return
		}
		node := st.Macro.Root.GetAction(st.SelectedNode)
		if node == nil || node.GetParent() == nil {
			return // don't copy root
		}
		m, err := serialize.ActionToMap(node)
		if err != nil {
			return
		}
		copiedNodeMap = m
	})
	pasteNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentPasteIcon(), func() {
		st := mui.MTabs.SelectedTab()
		st.PasteNode(copiedNodeMap)
	})
	playMacroButton := ttwidget.NewButtonWithIcon("", theme.MediaPlayIcon(), func() {
		st := mui.MTabs.SelectedTab()
		go func() {
			services.Execute(st.Macro.Root, st.Macro)
		}()
	})

	// addNodeButton.SetToolTip("add new action node")
	unselectNodeButton.SetToolTip("unselect nodes")
	moveDownNodeButton.SetToolTip("move node down")
	moveUpNodeButton.SetToolTip("move node up")
	copyNodeButton.SetToolTip("copy node")
	pasteNodeButton.SetToolTip("paste node below")
	playMacroButton.SetToolTip("start macro execution")

	mui.MacroToolbars.TopToolbar =
		container.NewGridWithColumns(2,
			container.NewHBox(
				// addNodeButton,
				// layout.NewSpacer(),
				unselectNodeButton,
				moveDownNodeButton,
				moveUpNodeButton,
				copyNodeButton,
				pasteNodeButton,
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
	globaldelaytt.SetToolTip("global delay (ms)")
	mui.MacroToolbars.BottomToolbar =
		container.NewGridWithRows(2,
			container.NewBorder(
				nil,
				nil,
				container.NewHBox(globaldelaytt, mui.MTabs.BoundGlobalDelayEntry),
				mousePosition, //right
				mui.MTabs.MacroHotkeyEntry,
			),
			services.MacroProgressBar(),
		)

	macroUi :=
		container.NewBorder(
			mui.MacroToolbars.TopToolbar,
			mui.MacroToolbars.BottomToolbar,
			nil,
			nil,
			mui.MTabs,
		)

	return macroUi
}
