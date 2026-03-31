package ui

import (
	"Sqyre/internal/config"
	"Sqyre/ui/editor"
	"Sqyre/ui/macro"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/layout"
	widget "fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
)

var (
	ui             *Ui
	boundLocXLabel *widget.Label
	boundLocYLabel *widget.Label
)

type Ui struct {
	Window   fyne.Window
	MainMenu *fyne.MainMenu
	*editor.EditorUi
	*SettingsUi
	*MainUi
}

type MainUi struct {
	Navigation   *container.Navigation
	Mui          *macro.MacroUi
	ActionDialog dialog.Dialog
}

func GetUi() *Ui { return ui }
func InitializeUi(w fyne.Window) *Ui {
	fyne.CurrentApp().Settings().SetTheme(NewSqyreTheme())
	setupUILogger()
	restoreWindowGeometry(w)
	w.SetCloseIntercept(func() {
		saveWindowGeometry(w)
		if _, ok := fyne.CurrentApp().(desktop.App); ok {
			// System tray is available (see cmd/sqyre systemTraySetup); keep running in background.
			w.Hide()
			return
		}
		onCloseWithoutTray()
		w.Close()
	})
	ui = &Ui{
		Window:   w,
		MainMenu: new(fyne.MainMenu),
		EditorUi: &editor.EditorUi{
			CanvasObject: new(fyne.Container),
			AddButton:    new(widget.Button),
			RemoveButton: new(widget.Button),
			EditorTabs: struct {
				*container.AppTabs
				ProgramsTab    *editor.EditorTab
				ItemsTab       *editor.EditorTab
				PointsTab      *editor.EditorTab
				SearchAreasTab *editor.EditorTab
				MasksTab       *editor.EditorTab
				AutoPicTab     *editor.EditorTab
			}{
				AppTabs: new(container.AppTabs),
				ProgramsTab: &editor.EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				ItemsTab: &editor.EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				PointsTab: &editor.EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				SearchAreasTab: &editor.EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				MasksTab: &editor.EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
				AutoPicTab: &editor.EditorTab{
					Widgets: make(map[string]fyne.CanvasObject),
				},
			},
		},
		SettingsUi: &SettingsUi{},
		MainUi: &MainUi{
			Navigation: new(container.Navigation), // Will be set in ConstructUi
			Mui: &macro.MacroUi{
				MTabs:             macro.NewMacroTabs(),
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

	saveNativeWindowBounds(w, prefs)
}

func (u *Ui) ConstructUi() {
	// construct main screen - action tabs removed, only macro UI-
	u.MainUi.Navigation = container.NewNavigation(u.constructMacroUi())

	// construct editor screen
	editor.ConstructEditorTabs(u.EditorUi, u.Window)
	editor.PrepareToolbarButtons(u.EditorUi)
	u.EditorUi.ActionBar = container.NewHBox(layout.NewSpacer(), u.EditorUi.AddButton, u.EditorUi.RemoveButton)
	u.EditorUi.CanvasObject = container.NewBorder(
		nil,
		u.EditorUi.ActionBar,
		nil,
		nil,
		u.EditorUi.EditorTabs,
	)
	u.EditorUi.RefreshEditorActionBar()
	u.EditorUi.EditorTabs.OnSelected = func(*container.TabItem) {
		u.EditorUi.RefreshEditorActionBar()
	}

	// construct settings screen
	u.constructSettings()

	// construct main menu
	u.Window.SetMainMenu(u.constructMainMenu())

	// Set window content to Navigation container with tooltip layer
	u.Window.SetContent(fynetooltip.AddWindowToolTipLayer(u.MainUi.Navigation, u.Window.Canvas()))

	SetEditorUi()
	SetActionDialogDeps()
	SetMacroUi()

	macro.InitMacroLogPopup(
		func() fyne.Window { return GetUi().Window },
		AddDialogEscapeClose,
		ShowErrorWithEscape,
	)

	toggleMousePos()
}

func toggleMousePos() {
	runMousePositionReadout()
}
