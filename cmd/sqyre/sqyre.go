package main

import (
	"Squire/binders"
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models/repositories"
	"Squire/internal/models/serialize"
	"Squire/internal/services"
	"Squire/ui"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime/debug"
	"slices"
	"strings"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/driver/desktop"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
	"github.com/go-vgo/robotgo"
	"github.com/gofrs/flock"
	hook "github.com/luhrMan/gohook"
)

// instanceLock is held for the process lifetime so only one instance runs.
var instanceLock *flock.Flock

func debugLog(msg string) {
	dir := os.TempDir()
	if d := os.Getenv("APPDATA"); d != "" {
		dir = filepath.Join(d, "Sqyre")
		_ = os.MkdirAll(dir, 0755)
	}
	f, err := os.OpenFile(filepath.Join(dir, "sqyre-debug.log"), os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		return
	}
	defer f.Close()
	fmt.Fprintf(f, "[%s] %s\n", time.Now().Format("15:04:05.000"), msg)
}

const maxLogLines = 10000

func setupLogFile() {
	logPath := filepath.Join(config.GetSqyreDir(), "sqyre.log")
	trimLogFileIfNeeded(logPath, maxLogLines)
	f, err := os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		return
	}
	log.SetOutput(f)
	log.SetFlags(log.Ldate | log.Ltime)
}

// logCrashAndRepanic writes the panic value and stack trace to sqyre.log and returns.
// Called from a deferred recover in main() so the program continues instead of exiting.
func logCrashAndRepanic(r interface{}) {
	logPath := filepath.Join(config.GetSqyreDir(), "sqyre.log")
	f, err := os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		log.Printf("panic (log file unavailable): %v\n%s", r, debug.Stack())
		return
	}
	defer f.Close()
	ts := time.Now().Format("2006-01-02 15:04:05.000")
	fmt.Fprintf(f, "\n[%s] panic recovered: %v\n", ts, r)
	f.Write(debug.Stack())
	fmt.Fprintf(f, "\n")
}

// trimLogFileIfNeeded keeps only the last maxLines in the file so sqyre.log does not grow unbounded.
func trimLogFileIfNeeded(logPath string, maxLines int) {
	content, err := os.ReadFile(logPath)
	if err != nil {
		if os.IsNotExist(err) {
			return
		}
		return
	}
	lines := strings.Split(string(content), "\n")
	if len(lines) <= maxLines {
		return
	}
	toKeep := lines[len(lines)-maxLines:]
	if err := os.WriteFile(logPath, []byte(strings.Join(toKeep, "\n")+"\n"), 0644); err != nil {
		return
	}
}

func init() {
	debugLog("init started")

	// Single-instance check: if another Sqyre is already running, exit immediately.
	lockPath := filepath.Join(config.GetSqyreDir(), "sqyre.lock")
	if err := os.MkdirAll(config.GetSqyreDir(), 0755); err != nil {
		debugLog("single-instance mkdir: " + err.Error())
		os.Exit(1)
	}
	instanceLock = flock.New(lockPath)
	ok, err := instanceLock.TryLock()
	if err != nil || !ok {
		os.Exit(0)
	}

	// Append logs to ~/.sqyre/sqyre.log (file kept open for process lifetime)
	setupLogFile()

	// Initialize directory structure first
	if err := config.InitializeDirectories(); err != nil {
		debugLog("init dirs failed: " + err.Error())
		log.Printf("Warning: Failed to initialize directories: %v", err)
	}
	debugLog("directories OK")

	if os.Getenv("SQYRE_NO_HOOK") != "1" {
		go services.StartHook()
	}

	// Initialize YAML config with proper file path
	yamlDb := serialize.GetYAMLConfig()
	yamlDb.SetConfigFile(config.GetDbPath())
	if err := yamlDb.ReadConfig(); err != nil {
		log.Printf("Warning: Failed to read config file: %v", err)
	}

	serialize.Decode() // read db.yaml data and save into GO structs

	// Initialize repositories - they will load data from db.yaml
	macroRepo := repositories.MacroRepo()
	log.Printf("Initialized MacroRepository with %d macros", macroRepo.Count())

	programRepo := repositories.ProgramRepo()
	log.Printf("Initialized ProgramRepository with %d programs", programRepo.Count())

	debugLog("creating Fyne app")
	a := app.NewWithID("Sqyre")
	os.Setenv("FYNE_SCALE", "1")

	w := a.NewWindow("Sqyre")

	w.Resize(fyne.NewSize(1000, 500))
	w.SetIcon(assets.AppIcon)
	w.SetMaster()

	if os.Getenv("SQYRE_NO_HOOK") != "1" {
		hook.Register(hook.KeyDown, []string{"esc"}, func(e hook.Event) {
			if !isWindowWithTitleActive("sqyre") {
				return
			}
			// Capture refs once to avoid TOCTOU: dialog/mainUi can be set nil on main thread
			u := ui.GetUi()
			if u == nil {
				return
			}
			var actionDialog dialog.Dialog
			var mainUi *ui.MainUi
			if u.MainUi != nil {
				actionDialog = u.MainUi.ActionDialog
				mainUi = u.MainUi
			}
			fyne.Do(func() {
				if actionDialog != nil {
					actionDialog.Hide()
				}
				if mainUi != nil && mainUi.Navigation.Root != nil && !mainUi.Navigation.Root.Visible() {
					log.Println("showing main ui")
					mainUi.Navigation.Back()
				}
			})
		})
	}

	systemTraySetup(w)

	//Initialize ui 		(provide an object for each property of ui)
	ui.InitializeUi(w)
	// construct the initialized 	(add widgets to ui)
	ui.GetUi().ConstructUi()
	setUi()

	w.SetContent(fynetooltip.AddWindowToolTipLayer(ui.GetUi().MainUi.Navigation, w.Canvas()))
	w.RequestFocus()
	debugLog("init done")
}

func main() {
	defer func() {
		if r := recover(); r != nil {
			logCrashAndRepanic(r)
		}
	}()
	debugLog("main started")
	if os.Getenv("SQYRE_NO_HOOK") != "1" {
		services.FailsafeHotkey()
	}
	// Get the Dark and Darker program and set up the item-corner mask
	// program, err := repositories.ProgramRepo().Get("dark and darker")
	// if err != nil {
	// 	log.Printf("Warning: Could not load %s program: %v", "dark and darker", err)
	// } else {
	// 	program.GetMasks()["item-corner"] = func(f ...any) *gocv.Mat {
	// 		rows, cols, x, y :=
	// 			f[0].(int), f[1].(int), f[2].(int), f[3].(int)
	// 		roi :=
	// 			image.Rect(
	// 				// (cols/x)/2,
	// 				// (rows/y)/2,
	// 				(cols/x)-cols,
	// 				(rows/y)-rows,
	// 				cols,
	// 				rows,
	// 			)

	// 		cmask := gocv.NewMatWithSize(rows, cols, gocv.MatTypeCV8U)
	// 		cmask.SetTo(gocv.NewScalar(255, 255, 255, 0))

	// 		region := cmask.Region(roi)
	// 		defer region.Close()
	// 		region.SetTo(gocv.NewScalar(0, 0, 0, 0))

	// 		return &cmask
	// 	}
	// }
	// mask, _ := program.ItemRepo().Get("Ancient Scroll")
	// gocv.IMWrite(config.GetMetaPath()+"mask.png", *program.GetMasks()["item-corner"](162, 108, mask.GridSize[0], mask.GridSize[1]))
	// mask, _ := program.ItemRepo().Get("Bandage")
	// gocv.IMWrite(config.GetMetaPath()+"mask.png", *program.GetMasks()["item-corner"](54, 54, mask.GridSize[0], mask.GridSize[1]))
	ui.GetUi().Window.ShowAndRun()

	services.CloseTessClient()

	// Save all repositories on shutdown
	if err := repositories.ProgramRepo().Save(); err != nil {
		log.Printf("Error saving programs: %v", err)
	}
	if err := repositories.MacroRepo().Save(); err != nil {
		log.Printf("Error saving macros: %v", err)
	}
}

func setUi() {
	binders.SetMacroUi()
	binders.SetEditorUi()
}

func systemTraySetup(w fyne.Window) {
	if desk, ok := fyne.CurrentApp().(desktop.App); ok {
		m := fyne.NewMenu("Sqyre",
			fyne.NewMenuItem("Show", func() {
				w.Show()
			}))
		desk.SetSystemTrayMenu(m)
		desk.SetSystemTrayIcon(assets.AppIcon)
	}

	w.SetCloseIntercept(func() {
		w.Hide()
	})
}

func isWindowWithTitleActive(targetTitle string) bool {
	pids, err := robotgo.FindIds(targetTitle)
	if err != nil || len(pids) == 0 {
		return false
	}
	log.Println(pids)
	currentPid := robotgo.GetPid()
	log.Println(currentPid)
	return slices.Contains(pids, currentPid)
}
