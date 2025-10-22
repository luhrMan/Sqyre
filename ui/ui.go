package ui

import (
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
	Window fyne.Window
	*EditorUi
	*MainUi
}

type MainUi struct {
	fyne.CanvasObject
	Mui        *MacroUi
	ActionTabs *ActionTabs
}

func GetUi() *Ui { return ui }
func InitializeUi(w fyne.Window) *Ui {
	ui = &Ui{
		Window: w,
		EditorUi: &EditorUi{
			NavButton: &widget.Button{},
			EditorTabs: struct {
				*container.AppTabs
				ItemsTab       *EditorTab
				PointsTab      *EditorTab
				SearchAreasTab *EditorTab
			}{
				AppTabs: &container.AppTabs{},
				ItemsTab: &EditorTab{
					BindableWidgets: make(map[string]fyne.Widget),
				},
				PointsTab: &EditorTab{
					BindableWidgets: make(map[string]fyne.Widget),
				},
				SearchAreasTab: &EditorTab{
					BindableWidgets: make(map[string]fyne.Widget),
				},
			},
		},
		MainUi: &MainUi{
			Mui: &MacroUi{
				MTabs:             NewMacroTabs(),
				MacroSelectButton: &widget.Button{},
				MacroToolbars: struct {
					TopToolbar    *fyne.Container
					BottomToolbar *fyne.Container
				}{
					TopToolbar:    &fyne.Container{},
					BottomToolbar: &fyne.Container{},
				},
			},
			ActionTabs: newActionTabs(),
		},
	}
	return ui
}
func (u *Ui) ConstructUi() {
	// construct main screen
	u.MainUi.CanvasObject =
		container.NewHSplit(
			u.constructActionTabs(), u.constructMacroUi(),
		)
	u.MainUi.CanvasObject.(*container.Split).SetOffset(0.3)

	// construct editor screen
	u.EditorUi.CanvasObject = container.NewBorder(
		nil,
		container.NewHBox(ui.EditorUi.NavButton),
		nil,
		nil,
		ui.EditorUi.EditorTabs,
	)
	u.constructEditorTabs()
	u.constructNavButton()

	// construct main menu
	u.Window.SetMainMenu(u.constructMainMenu())

	toggleMousePos()
}

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
