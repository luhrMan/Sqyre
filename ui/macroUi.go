package ui

import (
	"Squire/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
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

	u.Mui.MacroToolbars.TopToolbar =
		container.NewGridWithColumns(2,
			container.NewHBox(
				u.Mui.constructMacroToolbar(),
				services.MacroActiveIndicator(),
				layout.NewSpacer(),
				widget.NewLabel("Macro Name:"),
			),
			container.NewBorder(nil, nil, nil,
				u.Mui.MacroSelectButton,
				u.Mui.MTabs.MacroNameEntry,
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

	u.Mui.MacroToolbars.BottomToolbar =
		container.NewGridWithRows(2,
			container.NewBorder(
				nil,
				nil,
				nil,
				mousePosition, //right
				u.Mui.MTabs.MacroHotkeyEntry,
			),
			services.MacroProgressBar(),
		)

	macroUi :=
		container.NewBorder(
			u.Mui.MacroToolbars.TopToolbar,
			u.Mui.MacroToolbars.BottomToolbar,
			widget.NewSeparator(),
			nil,
			u.Mui.MTabs,
		)

	return macroUi
}

func (mui *MacroUi) constructMacroToolbar() *widget.Toolbar {
	tb :=
		widget.NewToolbar(
			widget.NewToolbarSpacer(),
			widget.NewToolbarSeparator(),
			widget.NewToolbarAction(theme.RadioButtonIcon(), func() {
				mt := mui.MTabs.SelectedTab()
				mt.UnselectAll()
				ui.ActionTabs.Selected().Content.Refresh()
				mt.SelectedNode = ""
			}),
			widget.NewToolbarAction(theme.MoveDownIcon(), func() {
				mt := mui.MTabs.SelectedTab()
				mt.moveNode(mt.SelectedNode, false)
			}),
			widget.NewToolbarAction(theme.MoveUpIcon(), func() {
				mt := mui.MTabs.SelectedTab()
				mt.moveNode(mt.SelectedNode, true)
			}),
			widget.NewToolbarSeparator(),
			widget.NewToolbarSpacer(),
			widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
				go func() {
					services.Execute(mui.MTabs.SelectedTab().Macro.Root)
				}()
			}),
		)
	return tb
}
