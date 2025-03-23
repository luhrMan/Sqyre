package ui

import (
	"Squire/internal/data"

	_ "fyne.io/x/fyne/widget"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
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
	ocrName            string
	ocrTarget          string
	ocrSearchBox       string
)

func (u *Ui) LoadMainContent() *fyne.Container {
	data.CreateItemMaps()
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
