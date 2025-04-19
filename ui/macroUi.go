package ui

import (
	"Squire/internal/assets"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"Squire/internal/utils"
	"log"
	"slices"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type macroUi struct {
	mtabs *macroTabs

	mtoolbars struct {
		tb1 *fyne.Container
		tb2 *fyne.Container
	}
}

func (mui *macroUi) constructMacroUi() *fyne.Container {
	boundLocXLabel = widget.NewLabelWithData(binding.NewString())
	boundLocYLabel = widget.NewLabelWithData(binding.NewString())

	mui.mtabs.constructTabs()

	mui.mtoolbars.tb1 =
		container.NewGridWithColumns(2,
			container.NewHBox(
				mui.constructMacroToolbar(),
				&mui.mtabs.isExecuting,
				layout.NewSpacer(),
				widget.NewLabel("Macro Name:"),
			),
			container.NewBorder(nil, nil, nil,
				mui.constructMacroSelect(),
				mui.mtabs.macroNameEntry,
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

	mui.mtoolbars.tb2 =
		container.NewGridWithRows(2,
			container.NewBorder(
				nil,
				nil,
				nil,
				mousePosition, //right
				mui.mtabs.macroHotkeyEntry,
			),
			utils.MacroProgressBar(),
		)

	macroUi :=
		container.NewBorder(
			mui.mtoolbars.tb1,
			mui.mtoolbars.tb2,
			widget.NewSeparator(),
			nil,
			mui.mtabs,
		)

	return macroUi
}

func GetMacrosAsStringSlice() []string {
	keys := make([]string, len(programs.CurrentProgram().Macros))

	i := 0
	for _, k := range programs.CurrentProgram().Macros {
		keys[i] = k.Name
		i++
	}
	return keys
}

func (mui *macroUi) constructMacroSelect() *widget.Button {
	return widget.NewButtonWithIcon("",
		theme.FolderOpenIcon(),
		func() {
			title := "Open Macro"
			for _, w := range fyne.CurrentApp().Driver().AllWindows() {
				if w.Title() == title {
					w.RequestFocus()
					return
				}
			}
			w := fyne.CurrentApp().NewWindow(title)
			w.SetIcon(assets.AppIcon)
			mui.mtabs.boundMacroListWidget = widget.NewList(
				func() int {
					return len(programs.CurrentProgram().Macros)
				},
				func() fyne.CanvasObject {
					return widget.NewLabel("template")
				},
				func(id widget.ListItemID, co fyne.CanvasObject) {
					k := GetMacrosAsStringSlice()
					label := co.(*widget.Label)
					slices.Sort(k)
					v := k[id]
					label.SetText(v)
					label.Importance = widget.MediumImportance
					for _, d := range mui.mtabs.Items {
						if d.Text == v {
							label.Importance = widget.SuccessImportance
						}
					}
					label.Refresh()
				},
			)
			mui.mtabs.boundMacroListWidget.OnSelected =
				func(id widget.ListItemID) {
					k := GetMacrosAsStringSlice()
					slices.Sort(k)
					macroName := k[id]
					mui.mtabs.addTab(macroName)
					mui.mtabs.boundMacroListWidget.RefreshItem(id)
					mui.mtabs.boundMacroListWidget.UnselectAll()
				}
			w.SetContent(
				container.NewAdaptiveGrid(1,
					mui.mtabs.boundMacroListWidget,
				),
			)
			w.Resize(fyne.NewSize(300, 500))
			w.Show()
		},
	)
}

func (mui *macroUi) constructMacroToolbar() *widget.Toolbar {
	tb :=
		widget.NewToolbar(
			widget.NewToolbarAction(theme.ContentAddIcon(), func() {
				var action actions.ActionInterface
				mt, err := ui.mui.mtabs.selectedTab()
				if err != nil {
					log.Println(err)
					return
				}
				selectedNode := mt.Macro.Root.GetAction(selectedTreeItem)
				if selectedNode == nil {
					selectedNode = mt.Macro.Root
				}
				switch ui.at.Selected().Text {
				case "Wait":
					time, _ := GetUi().at.boundWait.GetValue("Time")
					action = actions.NewWait(time.(int))
				case "Move":
					name, _ := GetUi().at.boundPoint.GetValue("Name")
					x, _ := GetUi().at.boundPoint.GetValue("X")
					y, _ := GetUi().at.boundPoint.GetValue("Y")
					action = actions.NewMove(coordinates.Point{Name: name.(string), X: x.(int), Y: y.(int)})
				case "Click":
					button, _ := GetUi().at.boundClick.GetValue("Button")
					action = actions.NewClick(button.(string))
				case "Key":
					key, _ := GetUi().at.boundKey.GetValue("Key")
					state, _ := GetUi().at.boundKey.GetValue("State")
					action = actions.NewKey(key.(string), state.(string))
				case "Loop":
					name, _ := GetUi().at.boundAdvancedAction.GetValue("Name")
					count, _ := GetUi().at.boundLoop.GetValue("Count")
					subactions := []actions.ActionInterface{}
					action = actions.NewLoop(count.(int), name.(string), subactions)
				case "Image":
					name, _ := GetUi().at.boundAdvancedAction.GetValue("Name")
					subactions := []actions.ActionInterface{}
					targets, _ := GetUi().at.boundImageSearch.GetValue("Targets")
					searchArea, _ := GetUi().at.boundSearchArea.GetValue("Name")
					action = actions.NewImageSearch(
						name.(string),
						subactions,
						targets.([]string),
						programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea.(string)))
				case "OCR":
					name, _ := GetUi().at.boundAdvancedAction.GetValue("Name")
					target, _ := GetUi().at.boundOcr.GetValue("Target")
					subactions := []actions.ActionInterface{}
					searchArea, _ := GetUi().at.boundSearchArea.GetValue("Name")
					action = actions.NewOcr(
						name.(string),
						subactions,
						target.(string),
						programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea.(string)))
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
				mt, err := ui.mui.mtabs.selectedTab()
				if err != nil {
					log.Println(err)
					return
				}
				mt.UnselectAll()
				selectedTreeItem = ""
				unbindAll()
			}),
			widget.NewToolbarAction(theme.MoveDownIcon(), func() {
				mt, err := ui.mui.mtabs.selectedTab()
				if err != nil {
					log.Println(err)
					return
				}

				mt.moveNode(selectedTreeItem, false)
			}),
			widget.NewToolbarAction(theme.MoveUpIcon(), func() {
				mt, err := ui.mui.mtabs.selectedTab()
				if err != nil {
					log.Println(err)
					return
				}

				mt.moveNode(selectedTreeItem, true)
			}),
			widget.NewToolbarSeparator(),
			widget.NewToolbarSpacer(),
			widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
				mt, err := ui.mui.mtabs.selectedTab()
				if err != nil {
					log.Println(err)
					return
				}

				mui.mtabs.isExecuting.Show()
				mui.mtabs.isExecuting.Start()
				mt.Macro.ExecuteActionTree()
				mui.mtabs.isExecuting.Stop()
				mui.mtabs.isExecuting.Hide()
			}),
		)
	return tb
}
