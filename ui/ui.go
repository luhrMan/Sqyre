package ui

import (
	"Squire/internal/assets"
	"Squire/internal/programs"
	"Squire/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	widget "fyne.io/fyne/v2/widget"
)

var ui *Ui

type Ui struct {
	win fyne.Window

	mtMap map[string]*MacroTree

	dt *container.DocTabs
	st *settingsTabs

	p *programs.Program
}

func GetUi() *Ui { return ui }
func InitializeUi(w fyne.Window) *Ui {
	ui = &Ui{
		win:   w,
		mtMap: map[string]*MacroTree{},
	}
	return ui
}
func (u *Ui) ConstructUi() {
	u.st = &settingsTabs{tabs: &container.AppTabs{}}

	assets.CreateItemMaps()
	u.actionSettingsTabs()
	u.createDocTabs()
	u.win.SetMainMenu(u.createMainMenu())
	u.win.SetContent(u.constructMainLayout())
}

func (u *Ui) constructMainLayout() *fyne.Container {
	macroLayout := container.NewBorder(
		container.NewGridWithColumns(2,
			container.NewHBox(
				u.createMacroToolbar(),
				layout.NewSpacer(),
				widget.NewLabel("Macro Name:"),
			),
			container.NewBorder(nil, nil, nil,
				u.createMacroSelect(), u.st.boundMacroNameEntry),
		),
		nil,
		widget.NewSeparator(),
		nil,
		u.dt,
	)
	mainLayout := container.NewBorder(nil, nil, u.st.tabs, nil, macroLayout)

	return mainLayout
}

func (u *Ui) SetWindow(w fyne.Window)                { u.win = w }
func (u *Ui) SetCurrentProgram(s string)             { u.p = programs.GetPrograms().GetProgram(s) }
func (u *Ui) AddMacroTree(key string, mt *MacroTree) { u.mtMap[key] = mt }
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
			boundMacroListWidget := widget.NewListWithData(
				u.st.boundMacroList,
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
		// theme.LoginIcon(),
		// func() {
		// 	if u.sel.Text == "" {
		// 		return
		// 	}
		// 	if u.p.GetMacroByName(u.sel.Text) == nil {
		// 		u.p.AddMacro(u.sel.Text, globalDelay)
		// 	}
		// 	u.addMacroDocTab(u.p.GetMacroByName(u.sel.Text))
		// },
	)
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

type settingsTabs struct {
	tabs                  *container.AppTabs
	boundMacroList        binding.StringList
	boundMacroName        binding.String
	boundMacroNameEntry   *widget.Entry
	boundGlobalDelay      binding.Int
	boundGlobalDelayEntry *widget.Entry
	waitTab
	moveTab
	clickTab
	keyTab
	loopTab
	imageSearchTab
	ocrTab
}

// settingsTabs indexes
const (
	waittab = iota
	movetab
	clicktab
	keytab
	looptab
	imagesearchtab
	ocrtab
)

type waitTab struct {
	boundTime binding.Int

	boundTimeSlider *widget.Slider
	boundTimeEntry  *widget.Entry
}

type moveTab struct {
	boundMoveX binding.Int
	boundMoveY binding.Int
	boundSpot  binding.String

	boundMoveXSlider *widget.Slider
	boundMoveYSlider *widget.Slider
	boundMoveXEntry  *widget.Entry
	boundMoveYEntry  *widget.Entry
	boundSpotSelect  *widget.Select
}

type clickTab struct {
	boundButton binding.Bool

	boundButtonToggle *custom_widgets.Toggle
}

type keyTab struct {
	boundKey   binding.String
	boundState binding.Bool

	boundKeySelect   *widget.Select
	boundStateToggle *custom_widgets.Toggle
}

type loopTab struct {
	boundLoopName binding.String
	boundCount    binding.Int

	boundLoopNameEntry *widget.Entry
	boundCountSlider   *widget.Slider
	boundCountLabel    *widget.Label
}

type imageSearchTab struct {
	boundImageSearchName    binding.String
	boundImageSearchArea    binding.String
	boundImageSearchTargets binding.StringList
	boundXSplit             binding.Int
	boundYSplit             binding.Int

	boundImageSearchNameEntry  *widget.Entry
	boundImageSearchAreaSelect *widget.Select
	boundXSplitSlider          *widget.Slider
	boundXSplitEntry           *widget.Entry
}

type ocrTab struct {
	boundOCRTarget     binding.String
	boundOCRSearchArea binding.String

	boundOCRTargetEntry      *widget.Entry
	boundOCRSearchAreaSelect *widget.Select
}
