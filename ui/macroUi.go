package ui

import (
	"Squire/internal/assets"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"Squire/internal/utils"
	"log"

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
	// boundLocX = binding.BindInt(&locX)
	// boundLocY = binding.BindInt(&locY)
	// boundLocXLabel = widget.NewLabelWithData(binding.IntToString(boundLocX))
	// boundLocYLabel = widget.NewLabelWithData(binding.IntToString(boundLocY))
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
				mui.mtabs.boundMacroNameEntry,
			),
		)

	macroHotkey :=
		container.NewHBox(
			mui.mtabs.macroHotkeySelect1,
			mui.mtabs.macroHotkeySelect2,
			mui.mtabs.macroHotkeySelect3,
			widget.NewButtonWithIcon("", theme.DocumentSaveIcon(),
				func() {
					macroHotkey = []string{
						mui.mtabs.macroHotkeySelect1.Selected,
						mui.mtabs.macroHotkeySelect2.Selected,
						mui.mtabs.macroHotkeySelect3.Selected,
					}
					mt, err := mui.mtabs.selectedTab()
					if err != nil {
						log.Println(err)
						return
					}
					mt.UnregisterHotkey()
					mt.Macro.Hotkey = macroHotkey
					mui.mtabs.boundMacroHotkey.Reload()
					mt.RegisterHotkey()
				},
			),
		)
	macroGlobalDelay :=
		container.NewHBox(
			widget.NewLabel("Global Delay (ms)"), mui.mtabs.boundGlobalDelayEntry)

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
				macroHotkey,      //right
				mousePosition,    //left
				macroGlobalDelay, //middle
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
			boundMacroListWidget := widget.NewListWithData(
				mui.mtabs.boundMacroList,
				func() fyne.CanvasObject {
					return widget.NewLabel("template")
				},
				func(di binding.DataItem, co fyne.CanvasObject) {
					co.(*widget.Label).Bind(di.(binding.String))
				},
			)
			boundMacroListWidget.OnSelected =
				func(id widget.ListItemID) {
					if ui.p.GetMacroByName(macroList[id]) == nil {
						ui.p.AddMacro(macroList[id], globalDelay)
					}
					mui.mtabs.addTab(ui.p.GetMacroByName(macroList[id]))
					boundMacroListWidget.UnselectAll()
				}
			w.SetContent(
				container.NewAdaptiveGrid(1,
					boundMacroListWidget,
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
				mt, err := mui.mtabs.GetTabTree()
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
					action = actions.NewClick(actions.LeftOrRight(button))
				case "Key":
					action = actions.NewKey(key, actions.UpOrDown(state))
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
				mt.Refresh()
			}),
			widget.NewToolbarSpacer(),
			widget.NewToolbarSeparator(),
			widget.NewToolbarAction(theme.RadioButtonIcon(), func() {
				t, err := mui.mtabs.GetTabTree()
				if err != nil {
					log.Println(err)
					return
				}
				t.UnselectAll()
				selectedTreeItem = ""
				unbindAll()
			}),
			widget.NewToolbarAction(theme.MoveDownIcon(), func() {
				t, err := mui.mtabs.GetTabTree()
				if err != nil {
					log.Println(err)
					return
				}
				t.moveNode(selectedTreeItem, false)
			}),
			widget.NewToolbarAction(theme.MoveUpIcon(), func() {
				t, err := mui.mtabs.GetTabTree()
				if err != nil {
					log.Println(err)
					return
				}
				t.moveNode(selectedTreeItem, true)
			}),
			widget.NewToolbarSeparator(),
			widget.NewToolbarSpacer(),
			widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
				t, err := mui.mtabs.GetTabTree()
				if err != nil {
					log.Println(err)
					return
				}
				mui.mtabs.isExecuting.Show()
				mui.mtabs.isExecuting.Start()
				t.Macro.ExecuteActionTree()
				mui.mtabs.isExecuting.Stop()
				mui.mtabs.isExecuting.Hide()
			}),
		)
	return tb
}
