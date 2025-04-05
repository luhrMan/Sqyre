package ui

import (
	"Squire/internal/assets"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/internal/utils"
	"errors"
	"log"
	"sort"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

type macroUi struct {
	mtabs *macroTabs

	boundMacroName      binding.String
	boundMacroNameEntry *widget.Entry

	boundMacroHotkey   binding.ExternalStringList
	macroHotkeySelect1 *widget.Select
	macroHotkeySelect2 *widget.Select
	macroHotkeySelect3 *widget.Select

	boundGlobalDelay      binding.Int
	boundGlobalDelayEntry *widget.Entry

	mtoolbars struct {
		tb1 *fyne.Container
		tb2 *fyne.Container
	}
}

func (mui *macroUi) constructMacroUi() *fyne.Container {
	mui.constructMacroSettings()
	mui.constructTabs()

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
				mui.boundMacroNameEntry,
			),
		)

	macroHotkey :=
		container.NewHBox(
			mui.macroHotkeySelect1,
			mui.macroHotkeySelect2,
			mui.macroHotkeySelect3,
			widget.NewButtonWithIcon("", theme.DocumentSaveIcon(),
				func() {
					macroHotkey = []string{
						mui.macroHotkeySelect1.Selected,
						mui.macroHotkeySelect2.Selected,
						mui.macroHotkeySelect3.Selected,
					}
					mt, err := mui.mtabs.selectedTab()
					if err != nil {
						log.Println(err)
						return
					}
					mt.Macro.Hotkey = macroHotkey
					mui.boundMacroHotkey.Reload()
					mui.mtabs.ReRegisterHotkeys()
				},
			),
		)
	macroGlobalDelay :=
		container.NewHBox(
			widget.NewLabel("Global Delay (ms)"), mui.boundGlobalDelayEntry)

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
					var t []string
					for i, item := range itemsBoolList {
						if item {
							t = append(t, i)
						}
					}
					action = actions.NewImageSearch(imageSearchName, []actions.ActionInterface{}, t, programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea))
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

				mt.Tree.Refresh()
			}),
			widget.NewToolbarSpacer(),
			widget.NewToolbarSeparator(),
			widget.NewToolbarAction(theme.RadioButtonIcon(), func() {
				t, err := mui.mtabs.GetTabTree()
				if err != nil {
					log.Println(err)
					return
				}
				t.Tree.UnselectAll()
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

func (mui *macroUi) constructMacroSettings() {
	boundLocX = binding.BindInt(&locX)
	boundLocY = binding.BindInt(&locY)
	boundLocXLabel = widget.NewLabelWithData(binding.IntToString(boundLocX))
	boundLocYLabel = widget.NewLabelWithData(binding.IntToString(boundLocY))

	boundMacro := binding.NewUntyped()
	boundMacro.Set(ui.p.Macros[0])
	mui.mtabs.boundMacroList = binding.BindStringList(&macroList)
	for _, m := range ui.p.Macros {
		mui.mtabs.boundMacroList.Append(m.Name)
	}
	mui.mtabs.boundMacroList.AddListener(binding.NewDataListener(func() {
		ml, err := mui.mtabs.boundMacroList.Get()
		if err != nil {
			log.Println(err)
			return
		}
		sort.Strings(ml)
	}))
	mui.boundMacroName = binding.BindString(&macroName)
	mui.boundMacroNameEntry = widget.NewEntryWithData(mui.boundMacroName)
	mui.boundMacroNameEntry.OnSubmitted = func(string) {
		t, err := mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}
		for _, m := range ui.p.Macros {
			if m.Name == macroName {
				dialog.ShowError(errors.New("macro name already exists"), ui.win)
				return
			}
		}
		delete(mui.mtabs.mtMap, t.Macro.Name)
		mui.mtabs.boundMacroList.Remove(t.Macro.Name)
		mui.mtabs.SetTreeMapKeyValue(macroName, t)
		// u.mtMap[macroName] = t
		t.Macro.Name = macroName
		mui.mtabs.Selected().Text = macroName
		mui.mtabs.boundMacroList.Append(macroName)

		mui.mtabs.Refresh()
	}
	macroHotkey = []string{"1", "2", "3"}
	mui.boundMacroHotkey = binding.BindStringList(&macroHotkey)
	mui.macroHotkeySelect1 = &widget.Select{Options: []string{"ctrl"}}
	mui.macroHotkeySelect2 = &widget.Select{Options: []string{"", "shift"}}
	mui.macroHotkeySelect3 = &widget.Select{Options: []string{"1", "2", "3", "4", "5"}}

	mui.macroHotkeySelect1.SetSelectedIndex(0)

	mui.boundGlobalDelay = binding.BindInt(&globalDelay)
	mui.boundGlobalDelayEntry = widget.NewEntryWithData(binding.IntToString(mui.boundGlobalDelay))
	mui.boundGlobalDelay.AddListener(binding.NewDataListener(func() {
		t, err := mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}
		t.Macro.GlobalDelay = globalDelay
		robotgo.MouseSleep = globalDelay
		robotgo.KeySleep = globalDelay
	}))

}
