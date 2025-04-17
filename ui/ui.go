package ui

import (
	"Squire/internal/assets"
	"Squire/internal/programs"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	widget "fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

var (
	ui             *Ui
	boundLocXLabel *widget.Label
	boundLocYLabel *widget.Label
)

type Ui struct {
	win fyne.Window

	mui *macroUi
	at  *actionTabs

	p *programs.Program
}

func GetUi() *Ui { return ui }
func InitializeUi(w fyne.Window) *Ui {
	ui = &Ui{
		win: w,
		mui: &macroUi{
			mtabs: &macroTabs{
				DocTabs: container.NewDocTabs(),
				mtMap:   map[string]*MacroTree{},
			},
		},
		at: &actionTabs{AppTabs: &container.AppTabs{}},
	}
	return ui
}
func (u *Ui) ConstructUi() {
	assets.CreateItemMaps()
	u.constructActionSettingsTabs()
	u.win.SetMainMenu(u.createMainMenu())
	u.win.SetContent(
		container.NewBorder(
			nil, nil,
			u.at,
			nil,
			u.mui.constructMacroUi(),
		),
	)
	toggleMousePos()
}

func (u *Ui) SetWindow(w fyne.Window)    { u.win = w }
func (u *Ui) SetCurrentProgram(s string) { u.p = programs.GetPrograms().GetProgram(s) }

func toggleMousePos() {
	locX, locY := robotgo.Location()
	blocX, blocY := binding.BindInt(&locX), binding.BindInt(&locY)
	boundLocXLabel.Bind(binding.IntToString(blocX))
	boundLocYLabel.Bind(binding.IntToString(blocY))
	go func() {
		for {
			robotgo.MilliSleep(100)
			newLocX, newLocY := robotgo.Location()
			if locX == newLocX && locY == newLocY {
				continue
			}
			locX, locY = robotgo.Location()
			blocX.Reload()
			blocY.Reload()
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
