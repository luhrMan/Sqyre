package ui

import (
	"Squire/internal"
	"Squire/internal/data"
	"log"
	"os"
	"strings"

	_ "fyne.io/x/fyne/widget"
	xwidget "fyne.io/x/fyne/widget"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	widget "fyne.io/fyne/v2/widget"
)

// action settings
var (
	macroName          string
	selectedTreeItem   = ".1"
	time               int
	globalDelay        = 30
	moveX              int
	moveY              int
	spot               string
	button             bool
	key                string
	state              bool
	loopName           string
	count              int = 1
	imageSearchName    string
	searchArea         string
	xSplit             int
	ySplit             int
	imageSearchTargets = data.Items.GetItemsMapAsBool()
	ocrTarget          string
	ocrSearchBox       string
)

func (u *Ui) LoadMainContent() *fyne.Container {
	var p = internal.GetPrograms()

	data.CreateItemMaps()
	u.createDocTabs()
	u.addMacroDocTab((*p[data.DarkAndDarker].Macros)[0])
	u.dt.SelectIndex(0)
	u.createSelect()
	u.dt.OnClosed = func(ti *container.TabItem) {
		delete(u.mtm, ti.Text)
	}
	u.win.SetMainMenu(u.createMainMenu())
	u.actionSettingsTabs()

	macroLayout := container.NewBorder(
		container.NewGridWithColumns(2,
			container.NewHBox(
				u.createMacroToolbar(),
				layout.NewSpacer(),
				widget.NewLabel("Macro Name:"),
			),
			// container.NewBorder(nil, nil, nil, widget.NewButtonWithIcon("", theme.LoginIcon(), func() { u.addMacroDocTab(u.sel.Text) }), u.sel),
		),
		nil,
		widget.NewSeparator(),
		nil,
		u.dt,
	)
	mainLayout := container.NewBorder(nil, nil, u.st.tabs, nil, macroLayout)

	return mainLayout
}
func (u *Ui) createSelect() {
	var macroList []string

	getMacroList := func() []string {
		var list []string
		files, err := os.ReadDir(savedMacrosPath)
		if err != nil {
			log.Fatal(err)
		}
		for _, f := range files {
			list = append(list, strings.TrimSuffix(f.Name(), data.JSON))
		}
		return list
	}

	macroList = getMacroList()
	u.sel = xwidget.NewCompletionEntry(macroList)
	u.sel.ActionItem = widget.NewButtonWithIcon("", theme.ViewRefreshIcon(), func() { macroList = getMacroList() })
	// u.sel.OnSubmitted = func(s string) { u.addMacroDocTab(s) }
	u.sel.OnChanged = func(s string) {
		var matches []string
		userPrefix := strings.ToLower(s)
		for _, listStr := range macroList {
			if len(listStr) < len(s) {
				continue
			}
			listPrefix := strings.ToLower(listStr[:len(s)])
			if userPrefix == listPrefix {
				matches = append(matches, listStr)
			}
		}
		u.sel.SetOptions(matches)
		u.sel.ShowCompletion()
	}
}
