package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
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
			CanvasObject:    new(fyne.Container),
			NavButton:       new(widget.Button),
			AddButton:       new(widget.Button),
			RemoveButton:    new(widget.Button),
			ProgramSelector: new(widget.SelectEntry),
			EditorTabs: struct {
				*container.AppTabs
				ItemsTab       *EditorTab
				PointsTab      *EditorTab
				SearchAreasTab *EditorTab
			}{
				AppTabs: new(container.AppTabs),
				ItemsTab: &EditorTab{
					Widgets: make(map[string]fyne.Widget),
				},
				PointsTab: &EditorTab{
					Widgets: make(map[string]fyne.Widget),
				},
				SearchAreasTab: &EditorTab{
					Widgets: make(map[string]fyne.Widget),
				},
			},
		},
		MainUi: &MainUi{
			CanvasObject: new(fyne.Container),
			Mui: &MacroUi{
				MTabs:             NewMacroTabs(),
				MacroSelectButton: new(widget.Button),
				MacroToolbars: struct {
					TopToolbar    *fyne.Container
					BottomToolbar *fyne.Container
				}{
					TopToolbar:    new(fyne.Container),
					BottomToolbar: new(fyne.Container),
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
		container.NewBorder(
			nil, nil,
			ui.EditorUi.NavButton,
			container.NewHBox(ui.EditorUi.AddButton, layout.NewSpacer(), ui.EditorUi.RemoveButton),
			layout.NewSpacer(), ui.EditorUi.ProgramSelector,
		),
		nil,
		nil,
		ui.EditorUi.EditorTabs,
	)
	u.constructEditorTabs()
	u.constructNavButton()
	u.constructAddButton()
	u.constructRemoveButton()

	// construct main menu
	u.Window.SetMainMenu(u.constructMainMenu())

	toggleMousePos()
}

// widget.NewSelect(repositories.ProgramRepo().GetAllAsStringSlice(), func(s string) {}),
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
