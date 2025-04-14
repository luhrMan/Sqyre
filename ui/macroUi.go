package ui

import (
	"Squire/internal/assets"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
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
	boundLocX = binding.BindInt(&locX)
	boundLocY = binding.BindInt(&locY)
	boundLocXLabel = widget.NewLabelWithData(binding.IntToString(boundLocX))
	boundLocYLabel = widget.NewLabelWithData(binding.IntToString(boundLocY))

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
					action = actions.NewWait(time)
				case "Move":
					action = actions.NewMove(moveX, moveY)
				case "Click":
					action = actions.NewClick(actions.LeftOrRight(button))
				case "Key":
					action = actions.NewKey(key, actions.UpOrDown(state))
				case "Loop":
					action = actions.NewLoop(int(count), loopName, []actions.ActionInterface{})
				case "Image":
					action = actions.NewImageSearch(imageSearchName, []actions.ActionInterface{}, imageSearchTargets, programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea))
				case "OCR":
					action = actions.NewOcr(ocrTarget, []actions.ActionInterface{}, ocrTarget, programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(ocrSearchBox))
				}

				if selectedNode == nil {
					selectedNode = mt.Macro.Root
				}
				if s, ok := selectedNode.(actions.AdvancedActionInterface); ok {
					s.AddSubAction(action)
				} else {
					selectedNode.GetParent().AddSubAction(action)
				}

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
