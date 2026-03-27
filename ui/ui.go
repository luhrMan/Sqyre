package ui

import (
	"path/filepath"

	"Sqyre/internal/config"
	"Sqyre/internal/logger"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/layout"
	widget "fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
	"github.com/go-vgo/robotgo"
)

var (
	ui             *Ui
	boundLocXLabel *widget.Label
	boundLocYLabel *widget.Label
)

type Ui struct {
	Window   fyne.Window
	MainMenu *fyne.MainMenu
	*EditorUi
	*SettingsUi
	*MainUi
}

type MainUi struct {
	Navigation   *container.Navigation
	Mui          *MacroUi
	ActionDialog dialog.Dialog
}

func GetUi() *Ui { return ui }
func InitializeUi(w fyne.Window) *Ui {
	fyne.CurrentApp().Settings().SetTheme(NewSqyreTheme())
	logger.SetLogFile(filepath.Join(config.GetSqyreDir(), "sqyre.log"))
	restoreWindowGeometry(w)
	w.SetCloseIntercept(func() {
		saveWindowGeometry(w)
		if _, ok := fyne.CurrentApp().(desktop.App); ok {
			// System tray is available (see cmd/sqyre systemTraySetup); keep running in background.
			w.Hide()
			return
		}
		services.LogMatProfile()
		w.Close()
	})
	ui = &Ui{
		Window:   w,
		MainMenu: new(fyne.MainMenu),
		EditorUi: &EditorUi{
			CanvasObject: new(fyne.Container),
			AddButton:    new(widget.Button),
			RemoveButton: new(widget.Button),
			EditorTabs: struct {
				*container.AppTabs
				ProgramsTab    *EditorTab
				ItemsTab       *EditorTab
				PointsTab      *EditorTab
				SearchAreasTab *EditorTab
				MasksTab       *EditorTab
				AutoPicTab     *EditorTab
			}{
				AppTabs: new(container.AppTabs),
				ProgramsTab: &EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				ItemsTab: &EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				PointsTab: &EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				SearchAreasTab: &EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				MasksTab: &EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				AutoPicTab: &EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
			},
		},
		SettingsUi: &SettingsUi{},
		MainUi: &MainUi{
			Navigation: new(container.Navigation), // Will be set in ConstructUi
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
			ActionDialog: nil, // set when a dialog is shown; Esc handler checks for nil before Hide()
		},
	}
	return ui
}

func restoreWindowGeometry(w fyne.Window) {
	prefs := fyne.CurrentApp().Preferences()
	savedWidth := prefs.IntWithFallback(config.PrefWindowWidth, 1000)
	savedHeight := prefs.IntWithFallback(config.PrefWindowHeight, 1000)
	if savedWidth > 0 && savedHeight > 0 {
		w.Resize(fyne.NewSize(float32(savedWidth), float32(savedHeight)))
	}
}

func saveWindowGeometry(w fyne.Window) {
	prefs := fyne.CurrentApp().Preferences()

	// Persist content size from Fyne.
	size := w.Canvas().Size()
	if size.Width > 0 && size.Height > 0 {
		prefs.SetInt(config.PrefWindowWidth, int(size.Width))
		prefs.SetInt(config.PrefWindowHeight, int(size.Height))
	}

	// Persist desktop window bounds (x, y, w, h) from current process window.
	pid := robotgo.GetPid()
	x, y, width, height := robotgo.GetBounds(pid)
	if width > 0 && height > 0 {
		prefs.SetInt(config.PrefWindowX, x)
		prefs.SetInt(config.PrefWindowY, y)
		prefs.SetInt(config.PrefWindowWidth, width)
		prefs.SetInt(config.PrefWindowHeight, height)
	}
}

func (u *Ui) ConstructUi() {
	// construct main screen - action tabs removed, only macro UI-
	u.MainUi.Navigation = container.NewNavigation(u.constructMacroUi())

	// construct editor screen
	u.constructEditorTabs()
	u.constructAddButton()
	u.constructRemoveButton()
	u.EditorUi.ActionBar = container.NewHBox(layout.NewSpacer(), u.EditorUi.AddButton, u.EditorUi.RemoveButton)
	u.EditorUi.CanvasObject = container.NewBorder(
		nil,
		u.EditorUi.ActionBar,
		nil,
		nil,
		ui.EditorUi.EditorTabs,
	)
	u.refreshEditorActionBar()
	u.EditorUi.EditorTabs.OnSelected = func(*container.TabItem) {
		u.refreshEditorActionBar()
	}

	// construct settings screen
	u.constructSettings()

	// construct main menu
	u.Window.SetMainMenu(u.constructMainMenu())

	// Set window content to Navigation container with tooltip layer
	u.Window.SetContent(fynetooltip.AddWindowToolTipLayer(u.MainUi.Navigation, u.Window.Canvas()))

	toggleMousePos()
}

// widget.NewSelect(repositories.ProgramRepo().GetAllAsStringSlice(), func(s string) {}),
func toggleMousePos() {
	locX, locY := robotgo.Location()
	blocX, blocY := binding.BindInt(&locX), binding.BindInt(&locY)
	boundLocXLabel.Bind(binding.IntToString(blocX))
	boundLocYLabel.Bind(binding.IntToString(blocY))
	services.GoSafe(func() {
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
	})
}
