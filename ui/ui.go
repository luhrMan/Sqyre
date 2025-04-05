package ui

import (
	"Squire/internal/assets"
	"Squire/internal/programs"
	"Squire/internal/utils"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	widget "fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

var (
	ui             *Ui
	locX           int
	locY           int
	boundLocX      binding.ExternalInt
	boundLocY      binding.ExternalInt
	boundLocXLabel *widget.Label
	boundLocYLabel *widget.Label
)

type Ui struct {
	win fyne.Window

	mtMap map[string]*MacroTree

	dt *container.DocTabs
	at *actionTabs
	ms *macroSettings

	p *programs.Program
}

func GetUi() *Ui { return ui }
func InitializeUi(w fyne.Window) *Ui {
	ui = &Ui{
		win:   w,
		mtMap: map[string]*MacroTree{},
		at:    &actionTabs{AppTabs: &container.AppTabs{}},
		ms:    &macroSettings{},
	}
	return ui
}
func (u *Ui) ConstructUi() {
	toggleMousePos()
	// u.at = &actionTabs{tabs: &container.AppTabs{}}
	assets.CreateItemMaps()
	u.actionSettingsTabs()
	u.createDocTabs()
	u.win.SetMainMenu(u.createMainMenu())
	u.win.SetContent(u.constructMainLayout())
}

func (u *Ui) constructMainLayout() *fyne.Container {
	macroToolbar :=
		container.NewGridWithColumns(2,
			container.NewHBox(
				u.createMacroToolbar(),
				&u.ms.isExecuting,
				layout.NewSpacer(),
				widget.NewLabel("Macro Name:"),
			),
			container.NewBorder(nil, nil, nil,
				u.createMacroSelect(),
				u.ms.boundMacroNameEntry,
			),
		)
	macroHotkey :=
		container.NewHBox(
			u.ms.macroHotkeySelect1,
			u.ms.macroHotkeySelect2,
			u.ms.macroHotkeySelect3,
			widget.NewButtonWithIcon("", theme.DocumentSaveIcon(),
				func() {
					macroHotkey = []string{
						u.ms.macroHotkeySelect1.Selected,
						u.ms.macroHotkeySelect2.Selected,
						u.ms.macroHotkeySelect3.Selected,
					}
					mt, err := u.selectedMacroTab()
					if err != nil {
						log.Println(err)
						return
					}
					mt.Macro.Hotkey = macroHotkey
					u.ms.boundMacroHotkey.Reload()
					ReRegisterMacroHotkeys()
				},
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
	macroGlobalDelay :=
		container.NewHBox(widget.NewLabel("Global Delay (ms)"), u.ms.boundGlobalDelayEntry)

	macroBottom :=
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

	macroLayout :=
		container.NewBorder(
			macroToolbar,
			macroBottom,
			widget.NewSeparator(),
			nil,
			u.dt,
		)
	mainLayout := container.NewBorder(nil, nil, u.at, nil, macroLayout)
	return mainLayout
}

func (u *Ui) SetWindow(w fyne.Window)                           { u.win = w }
func (u *Ui) SetCurrentProgram(s string)                        { u.p = programs.GetPrograms().GetProgram(s) }
func (u *Ui) SetMacroTreeMapKeyValue(key string, mt *MacroTree) { u.mtMap[key] = mt }
func (u *Ui) createMacroSelect() *widget.Button {
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
				u.ms.boundMacroList,
				func() fyne.CanvasObject {
					return widget.NewLabel("template")
				},
				func(di binding.DataItem, co fyne.CanvasObject) {
					co.(*widget.Label).Bind(di.(binding.String))
				},
			)
			boundMacroListWidget.OnSelected =
				func(id widget.ListItemID) {
					if u.p.GetMacroByName(macroList[id]) == nil {
						u.p.AddMacro(macroList[id], globalDelay)
					}
					u.addMacroDocTab(u.p.GetMacroByName(macroList[id]))
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

func toggleMousePos() {
	locX, locY = robotgo.Location()
	go func() {
		for {
			robotgo.MilliSleep(100)
			newLocX, newLocY := robotgo.Location()
			if locX == newLocX && locY == newLocY {
				continue
			}
			locX, locY = robotgo.Location()
			boundLocX.Reload()
			boundLocY.Reload()
		}
	}()
}

// func (u *Ui) createSelect() {
// 	var macroList []string
// 	for _, m := range u.p.Macros {
// 		macroList = append(macroList, m.Name)
// 	}
// 	u.sel = xwidget.NewCompletionEntry(macroList)
// 	// u.sel.ActionItem = widget.NewButtonWithIcon("", theme.ViewRefreshIcon(), func() { macroList = getMacroList() })
// 	u.sel.OnSubmitted = func(s string) { u.addMacroDocTab(u.p.GetMacroByName(s)) }
// 	u.sel.OnChanged = func(s string) {
// 		var matches []string
// 		userPrefix := strings.ToLower(s)
// 		for _, listStr := range macroList {
// 			if len(listStr) < len(s) {
// 				continue
// 			}
// 			listPrefix := strings.ToLower(listStr[:len(s)])
// 			if userPrefix == listPrefix {
// 				matches = append(matches, listStr)
// 			}
// 		}
// 		u.sel.SetOptions(matches)
// 		u.sel.ShowCompletion()
// 	}
// }

type macroSettings struct {
	isExecuting           widget.Activity
	boundMacroList        binding.StringList
	boundMacroName        binding.String
	boundMacroNameEntry   *widget.Entry
	boundGlobalDelay      binding.Int
	boundGlobalDelayEntry *widget.Entry
	boundMacroHotkey      binding.ExternalStringList
	macroHotkeySelect1    *widget.Select
	macroHotkeySelect2    *widget.Select
	macroHotkeySelect3    *widget.Select
}
