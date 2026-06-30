package ui

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/config"
	"Sqyre/internal/macrohotkey"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/models/serialize"
	"Sqyre/internal/services"
	"Sqyre/internal/startupprof"
	"Sqyre/ui/custom_widgets"
	"log"
	"os"
	"sync/atomic"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
)

const (
	progressDirs     = 0.15
	progressConfig   = 0.30
	progressMacros   = 0.50
	progressPrograms = 0.65
	progressBuildUI  = 0.78
	progressShell    = 0.88
	progressFinish   = 0.96
	progressComplete = 1.0
)

// BootstrapReporter updates splash status text and progress during startup.
type BootstrapReporter struct {
	label    *widget.Label
	progress *widget.ProgressBar
}

// PaintInitial resets the splash to its starting state. Call from SetOnStarted on the UI thread
// so the splash is visible at 0% before bootstrap work begins.
func (r BootstrapReporter) PaintInitial() {
	r.label.SetText("Starting Sqyre…")
	r.progress.SetValue(0)
}

func (r BootstrapReporter) setStatus(msg string) {
	fyne.Do(func() { r.label.SetText(msg) })
}

func (r BootstrapReporter) setProgress(v float64) {
	fyne.Do(func() { r.progress.SetValue(v) })
}

func (r BootstrapReporter) setStatusDirect(msg string) {
	r.label.SetText(msg)
}

func (r BootstrapReporter) setProgressDirect(v float64) {
	r.progress.SetValue(v)
}

// LoadingScreen is the splash content shown while the app bootstraps.
type LoadingScreen struct {
	root     *fyne.Container
	label    *widget.Label
	progress *widget.ProgressBar
}

// NewLoadingScreen builds the splash layout and callbacks to update status and progress.
func NewLoadingScreen() (*LoadingScreen, BootstrapReporter) {
	s := &LoadingScreen{
		label:    widget.NewLabel("Starting Sqyre…"),
		progress: widget.NewProgressBar(),
	}
	s.progress.SetValue(0)
	s.label.Alignment = fyne.TextAlignCenter
	s.label.TextStyle = fyne.TextStyle{Bold: true}

	icon := canvas.NewImageFromResource(assets.AppIcon)
	icon.SetMinSize(fyne.NewSize(72, 72))
	icon.FillMode = canvas.ImageFillContain

	title := widget.NewLabel("Sqyre")
	title.Alignment = fyne.TextAlignCenter
	title.TextStyle = fyne.TextStyle{Bold: true}

	version := widget.NewLabel("v" + fyne.CurrentApp().Metadata().Version)
	version.Alignment = fyne.TextAlignCenter

	s.root = container.NewCenter(container.NewVBox(
		icon,
		title,
		s.label,
		s.progress,
		version,
	))

	report := BootstrapReporter{
		label:    s.label,
		progress: s.progress,
	}
	return s, report
}

// Content returns the splash widget tree.
func (s *LoadingScreen) Content() fyne.CanvasObject { return s.root }

// NewSplashWindow creates a fixed-size splash window centered on screen.
func NewSplashWindow(a fyne.App) (fyne.Window, BootstrapReporter) {
	screen, report := NewLoadingScreen()

	w := a.NewWindow("Sqyre")
	w.SetIcon(assets.AppIcon)
	w.SetFixedSize(true)
	w.Resize(fyne.NewSize(420, 280))
	w.SetContent(screen.Content())
	w.CenterOnScreen()
	return w, report
}

func setupSystemTray(w fyne.Window) {
	if desk, ok := fyne.CurrentApp().(desktop.App); ok {
		m := fyne.NewMenu("Sqyre",
			fyne.NewMenuItem("Show", func() {
				w.Show()
			}))
		desk.SetSystemTrayMenu(m)
		desk.SetSystemTrayIcon(assets.AppIcon)
	}
}

// PrepareMainWindow creates the hidden main window and registers the system tray.
// Must be called before app.Run(); Fyne requires tray setup before the event loop starts.
func PrepareMainWindow(a fyne.App) fyne.Window {
	w := a.NewWindow("Sqyre")
	w.Resize(fyne.NewSize(1000, 500))
	w.SetIcon(assets.AppIcon)
	w.SetMaster()
	setupSystemTray(w)
	return w
}

// BootstrapDone is true once config is loaded and the UI has finished wiring.
func BootstrapDone() bool { return bootstrapDone.Load() }

var bootstrapDone atomic.Bool

// Bootstrap runs heavy startup off the UI thread, then builds the interface on the UI thread
// so the splash stays visible while data loads and the shell appears before final wiring.
func Bootstrap(mainWindow, splashWindow fyne.Window, report BootstrapReporter) {
	startupprof.Mark("bootstrap goroutine start")

	report.setStatus("Setting up directories…")
	report.setProgress(progressDirs)
	if err := config.InitializeDirectories(); err != nil {
		log.Printf("Warning: Failed to initialize directories: %v", err)
	}
	startupprof.Mark("directories initialized")

	report.setStatus("Loading configuration…")
	report.setProgress(progressConfig)
	if err := serialize.LoadConfig(); err != nil {
		log.Printf("Warning: Failed to read config file: %v", err)
	}
	startupprof.Mark("config loaded")

	report.setStatus("Loading macros…")
	report.setProgress(progressMacros)
	macroRepo := repositories.MacroRepo()
	log.Printf("Initialized MacroRepository with %d macros", macroRepo.Count())
	startupprof.Mark("macros loaded")

	report.setStatus("Loading programs…")
	report.setProgress(progressPrograms)
	programRepo := repositories.ProgramRepo()
	log.Printf("Initialized ProgramRepository with %d programs", programRepo.Count())
	startupprof.Mark("programs loaded")

	report.setStatus("Building interface…")
	report.setProgress(progressBuildUI)
	fyne.DoAndWait(func() {
		InitializeUi(mainWindow)
		report.setProgressDirect(progressShell)
		startupprof.Mark("InitializeUi done")
		u := GetUi()
		u.constructUiShell()
		mainContent := fynetooltip.AddWindowToolTipLayer(u.MainUi.Navigation, mainWindow.Canvas())
		mainWindow.SetContent(custom_widgets.AddWindowItemTooltipLayer(mainContent, mainWindow.Canvas()))
		startupprof.Mark("UI shell built")
		report.setStatusDirect("Finishing setup…")
		report.setProgressDirect(progressFinish)

		u.constructUiFinish()
		if os.Getenv("SQYRE_NO_HOOK") != "1" {
			macrohotkey.FailsafeHotkey()
		}
		report.setProgressDirect(progressComplete)
		bootstrapDone.Store(true)
		splashWindow.Close()
		mainWindow.Show()
		mainWindow.RequestFocus()
		startupprof.Mark("bootstrap done (main window shown)")
		startupprof.Dump()
		services.LogMemoryUsage("startup-idle")
		if startupprof.Enabled() {
			fyne.CurrentApp().Quit()
		}
	})
}
