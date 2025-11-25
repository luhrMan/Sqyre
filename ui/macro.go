package ui

import (
	"Squire/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

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

	addNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentAddIcon(), func() {

	})
	unselectNodeButton := ttwidget.NewButtonWithIcon("", theme.RadioButtonIcon(), func() {
		st := mui.MTabs.SelectedTab()
		st.UnselectAll()
		ui.ActionTabs.Selected().Content.Refresh()
		st.SelectedNode = ""
	})

	moveDownNodeButton := ttwidget.NewButtonWithIcon("", theme.MoveDownIcon(), func() {
		st := mui.MTabs.SelectedTab()
		st.moveNode(st.SelectedNode, false)
	})
	// upIcon := fyne.NewStaticResource("custom_arrowup", assets.CustomArrowUpIcon())
	moveUpNodeButton := ttwidget.NewButtonWithIcon("", theme.MoveUpIcon(), func() {
		st := mui.MTabs.SelectedTab()
		st.moveNode(st.SelectedNode, true)
	})
	playMacroButton := ttwidget.NewButtonWithIcon("", theme.MediaPlayIcon(), func() {
		st := mui.MTabs.SelectedTab()
		go func() {
			services.Execute(st.Macro.Root)
		}()
	})

	addNodeButton.SetToolTip("add new action node")
	unselectNodeButton.SetToolTip("unselect nodes")
	moveDownNodeButton.SetToolTip("move node down")
	moveUpNodeButton.SetToolTip("move node up")
	playMacroButton.SetToolTip("start macro execution")

	mui.MacroToolbars.TopToolbar =
		container.NewGridWithColumns(2,
			container.NewHBox(
				addNodeButton,
				layout.NewSpacer(),
				unselectNodeButton,
				moveDownNodeButton,
				moveUpNodeButton,
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
			widget.NewSeparator(),
			nil,
			mui.MTabs,
		)

	return macroUi
}
