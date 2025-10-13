package ui

import (
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"Squire/internal/utils"

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
		Toolbar1 *fyne.Container
		Toolbar2 *fyne.Container
	}
}

func (u *Ui) constructMacroUi() *fyne.Container {
	boundLocXLabel = widget.NewLabelWithData(binding.NewString())
	boundLocYLabel = widget.NewLabelWithData(binding.NewString())

	u.Mui.MacroToolbars.Toolbar1 =
		container.NewGridWithColumns(2,
			container.NewHBox(
				u.Mui.constructMacroToolbar(),
				utils.MacroActiveIndicator(),
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

	u.Mui.MacroToolbars.Toolbar2 =
		container.NewGridWithRows(2,
			container.NewBorder(
				nil,
				nil,
				nil,
				mousePosition, //right
				u.Mui.MTabs.MacroHotkeyEntry,
			),
			utils.MacroProgressBar(),
		)

	macroUi :=
		container.NewBorder(
			u.Mui.MacroToolbars.Toolbar1,
			u.Mui.MacroToolbars.Toolbar2,
			widget.NewSeparator(),
			nil,
			u.Mui.MTabs,
		)

	return macroUi
}

func (mui *MacroUi) constructMacroToolbar() *widget.Toolbar {
	tb :=
		widget.NewToolbar(
			widget.NewToolbarAction(theme.ContentAddIcon(), func() {
				var action actions.ActionInterface
				mt := mui.MTabs.SelectedTab()
				selectedNode := mt.Macro.Root.GetAction(mt.SelectedNode)
				if selectedNode == nil {
					selectedNode = mt.Macro.Root
				}
				switch ui.ActionTabs.Selected().Text {
				case "Wait":
					time, _ := GetUi().ActionTabs.BoundWait.GetValue("Time")
					action = actions.NewWait(time.(int))
				case "Move":
					name, _ := GetUi().ActionTabs.BoundPoint.GetValue("Name")
					x, _ := GetUi().ActionTabs.BoundPoint.GetValue("X")
					y, _ := GetUi().ActionTabs.BoundPoint.GetValue("Y")
					action = actions.NewMove(coordinates.Point{Name: name.(string), X: x.(int), Y: y.(int)})
				case "Click":
					button, _ := GetUi().ActionTabs.BoundClick.GetValue("Button")
					action = actions.NewClick(button.(string))
				case "Key":
					key, _ := GetUi().ActionTabs.BoundKey.GetValue("Key")
					state, _ := GetUi().ActionTabs.BoundKey.GetValue("State")
					action = actions.NewKey(key.(string), state.(string))
				case "Loop":
					name, _ := GetUi().ActionTabs.BoundAdvancedAction.GetValue("Name")
					count, _ := GetUi().ActionTabs.BoundLoop.GetValue("Count")
					subactions := []actions.ActionInterface{}
					action = actions.NewLoop(count.(int), name.(string), subactions)
				case "Image":
					name, _ := GetUi().ActionTabs.BoundAdvancedAction.GetValue("Name")
					subactions := []actions.ActionInterface{}
					targets, _ := GetUi().ActionTabs.BoundImageSearch.GetValue("Targets")
					searchArea, _ := GetUi().ActionTabs.BoundSearchArea.GetValue("Name")
					action = actions.NewImageSearch(
						name.(string),
						subactions,
						targets.([]string),
						searchArea.(coordinates.SearchArea),
						// binders.GetProgram(config.DarkAndDarker).Coordinates[config.MainMonitorSizeString].GetSearchArea(searchArea.(string))
					)
				case "OCR":
					name, _ := GetUi().ActionTabs.BoundAdvancedAction.GetValue("Name")
					target, _ := GetUi().ActionTabs.BoundOcr.GetValue("Target")
					subactions := []actions.ActionInterface{}
					searchArea, _ := GetUi().ActionTabs.BoundSearchArea.GetValue("Name")
					action = actions.NewOcr(
						name.(string),
						subactions,
						target.(string),
						searchArea.(coordinates.SearchArea),
						// binders.GetProgram(config.DarkAndDarker).Coordinates[config.MainMonitorSizeString].GetSearchArea(searchArea.(string))
					)
				}

				if selectedNode == nil {
					selectedNode = mt.Macro.Root
				}
				if s, ok := selectedNode.(actions.AdvancedActionInterface); ok {
					s.AddSubAction(action)
				} else {
					selectedNode.GetParent().AddSubAction(action)
				}
				mt.Select(action.GetUID())
				mt.RefreshItem(action.GetUID())
			}),
			widget.NewToolbarSpacer(),
			widget.NewToolbarSeparator(),
			widget.NewToolbarAction(theme.RadioButtonIcon(), func() {
				mt := mui.MTabs.SelectedTab()
				mt.UnselectAll()
				ui.ActionTabs.Selected().Content.Refresh()
				mt.SelectedNode = ""
				// unbindAll()
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
				mt := mui.MTabs.SelectedTab()
				mt.Macro.ExecuteActionTree()
			}),
		)
	return tb
}
