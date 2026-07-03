package ui

import (
	"path/filepath"
	"sync"
	"time"

	"Sqyre/internal/config"
	"Sqyre/internal/logger"
	"Sqyre/internal/screen"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/editor"
	"Sqyre/ui/macro"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/driver/desktop"
	widget "fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
)

var (
	ui             *Ui
	uiInitMu       sync.Mutex
	boundLocXLabel *widget.Label
	boundLocYLabel *widget.Label
	mousePosStop   chan struct{}
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
	overlayKind  overlayKind
}

func GetUi() *Ui { return ui }
func InitializeUi(w fyne.Window) *Ui {
	uiInitMu.Lock()
	defer uiInitMu.Unlock()
	ApplyAppearanceFromPrefs()
	services.SetRunOnUIThread(fyne.Do)
	services.SetRunOnUIThreadAndWait(fyne.DoAndWait)
	config.BoolPreference = func(key string, fallback bool) bool {
		return fyne.CurrentApp().Preferences().BoolWithFallback(key, fallback)
	}
	config.StringPreference = func(key string) string {
		return fyne.CurrentApp().Preferences().String(key)
	}
	screen.EnabledMonitorsString = func() string {
		return config.PrefString(config.PrefEnabledMonitors)
	}
	logger.SetLogFile(filepath.Join(config.GetSqyreDir(), "sqyre.log"))
	restoreWindowGeometry(w)
	w.SetCloseIntercept(func() {
		saveWindowGeometry(w)
		if _, ok := fyne.CurrentApp().(desktop.App); ok {
			// System tray is available (see cmd/sqyre systemTraySetup); keep running in background.
			stopMousePosPolling()
			w.Hide()
			return
		}
		services.LogMatProfile()
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
	savedHeight := prefs.IntWithFallback(config.PrefWindowHeight, 500)
	if savedWidth > 0 && savedHeight > 0 {
		w.Resize(clampWindowSize(fyne.NewSize(float32(savedWidth), float32(savedHeight))))
	}
}

func saveWindowGeometry(w fyne.Window) {
	FlushAppearancePrefs()
	prefs := fyne.CurrentApp().Preferences()

	// Persist content size from Fyne.
	size := w.Canvas().Size()
	if size.Width > 0 && size.Height > 0 {
		prefs.SetInt(config.PrefWindowWidth, int(size.Width))
		prefs.SetInt(config.PrefWindowHeight, int(size.Height))
	}

	if config.IsUITestMode() {
		return
	}

	// Persist desktop window bounds (x, y, w, h) from current process window.
	x, y, width, height := screen.ProcessWindowBounds()
	if width > 0 && height > 0 {
		prefs.SetInt(config.PrefWindowX, x)
		prefs.SetInt(config.PrefWindowY, y)
		prefs.SetInt(config.PrefWindowWidth, width)
		prefs.SetInt(config.PrefWindowHeight, height)
	}
}

var editorUiWired *editor.EditorUi

// EnsureDataEditor builds the data editor UI on first open and wires its handlers.
func EnsureDataEditor() {
	u := GetUi()
	editor.EnsureBuilt(u.EditorUi, u.Window)
	if editorUiWired != u.EditorUi {
		SetEditorUi()
		editorUiWired = u.EditorUi
	}
}

// ResetGlobalsForTesting clears package-level UI singletons between tests.
func ResetGlobalsForTesting() {
	uiInitMu.Lock()
	defer uiInitMu.Unlock()
	stopMousePosPolling()
	ui = nil
	editorUiWired = nil
	editor.ResetBuiltForTesting()
}

func (u *Ui) ConstructUi() {
	u.constructUiShell()
	u.constructUiFinish()
	if config.IsUITestMode() {
		bootstrapDone.Store(true)
	}
}

func (u *Ui) constructUiShell() {
	u.MainUi.Navigation = container.NewNavigation(u.constructMacroUi())
	mainContent := fynetooltip.AddWindowToolTipLayer(u.MainUi.Navigation, u.Window.Canvas())
	u.Window.SetContent(custom_widgets.AddWindowItemTooltipLayer(mainContent, u.Window.Canvas()))
}

func (u *Ui) constructUiFinish() {
	u.constructSettings()
	u.wireNavigation()

	u.Window.SetMainMenu(u.constructMainMenu())

	SetActionDialogDeps()
	SetMacroUi()

	macro.InitMacroLogPopup(
		func() fyne.Window { return GetUi().Window },
		AddDialogEscapeClose,
		ShowErrorWithEscape,
	)

	if config.IsUITestMode() {
		EnsureDataEditor()
	}

	if !config.IsUITestMode() {
		toggleMousePos()
	}
}

func toggleMousePos() {
	locX, locY := screen.Location()
	blocX, blocY := binding.BindInt(&locX), binding.BindInt(&locY)
	boundLocXLabel.Bind(binding.IntToString(blocX))
	boundLocYLabel.Bind(binding.IntToString(blocY))
	stop := make(chan struct{})
	mousePosStop = stop
	services.GoSafe(func() {
		for {
			select {
			case <-stop:
				return
			default:
			}
			time.Sleep(100 * time.Millisecond)
			newLocX, newLocY := screen.Location()
			if locX == newLocX && locY == newLocY {
				continue
			}
			locX, locY = screen.Location()
			blocX.Reload()
			blocY.Reload()
		}
	})
}

func stopMousePosPolling() {
	if mousePosStop != nil {
		close(mousePosStop)
		mousePosStop = nil
	}
}

func startMousePosPolling() {
	if mousePosStop != nil {
		return
	}
	toggleMousePos()
}
