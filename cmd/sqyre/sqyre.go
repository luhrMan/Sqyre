package main

import (
	"Sqyre/internal/config"
	"Sqyre/internal/macrohotkey"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/internal/startupprof"
	"Sqyre/ui"
	"bufio"
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"strings"
	"time"

	"fyne.io/fyne/v2/app"
	"github.com/gofrs/flock"
)

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
	log.SetOutput(&services.SyncWriter{F: f})
	log.SetFlags(log.Ldate | log.Ltime)
}

func trimLogFileIfNeeded(logPath string, maxLines int) {
	info, err := os.Stat(logPath)
	if err != nil || info.Size() == 0 {
		return
	}
	const avgLineBytes = 128
	if info.Size() <= int64(maxLines*avgLineBytes) {
		data, err := os.ReadFile(logPath)
		if err != nil {
			return
		}
		if strings.Count(string(data), "\n") <= maxLines {
			return
		}
	}
	tailBytes := int64(maxLines * avgLineBytes * 2)
	if tailBytes > info.Size() {
		tailBytes = info.Size()
	}
	f, err := os.Open(logPath)
	if err != nil {
		return
	}
	defer f.Close()
	if _, err := f.Seek(info.Size()-tailBytes, io.SeekStart); err != nil {
		return
	}
	scanner := bufio.NewScanner(f)
	scanner.Buffer(make([]byte, 64*1024), 1024*1024)
	var lines []string
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	if err := scanner.Err(); err != nil || len(lines) <= maxLines {
		return
	}
	toKeep := lines[len(lines)-maxLines:]
	_ = os.WriteFile(logPath, []byte(strings.Join(toKeep, "\n")+"\n"), 0644)
}

func acquireSingleInstance() bool {
	lockPath := filepath.Join(config.GetSqyreDir(), "sqyre.lock")
	if err := os.MkdirAll(config.GetSqyreDir(), 0755); err != nil {
		debugLog("single-instance mkdir: " + err.Error())
		return false
	}
	instanceLock = flock.New(lockPath)
	ok, err := instanceLock.TryLock()
	return err == nil && ok
}

func main() {
	defer func() {
		if r := recover(); r != nil {
			services.LogPanicToFile(r)
		}
	}()

	startupprof.Mark("main() entered")
	debugLog("main started")
	if !acquireSingleInstance() {
		os.Exit(0)
	}

	debugLog("creating Fyne app")
	a := app.NewWithID("Sqyre")
	os.Setenv("FYNE_SCALE", "1")
	a.Settings().SetTheme(ui.NewSqyreTheme())
	startupprof.Mark("fyne app created")

	splash, report := ui.NewSplashWindow(a)
	splash.Show()
	startupprof.Mark("splash window shown")

	mainWindow := ui.PrepareMainWindow(a)

	a.Lifecycle().SetOnStopped(func() {
		ui.SaveOpenMacros()
	})

	a.Lifecycle().SetOnStarted(func() {
		startupprof.Mark("event loop started (splash visible)")
		report.PaintInitial()
		if os.Getenv("SQYRE_NO_HOOK") != "1" {
			services.GoSafe(macrohotkey.StartHook)
		}
		go func() {
			setupLogFile()
			ui.Bootstrap(mainWindow, splash, report)
			debugLog("UI ready")
		}()
	})

	a.Run()

	services.CloseTessClient()

	if ui.BootstrapDone() {
		if err := repositories.ProgramRepo().Save(); err != nil {
			log.Printf("Error saving programs: %v", err)
		}
		if err := repositories.MacroRepo().Save(); err != nil {
			log.Printf("Error saving macros: %v", err)
		}
	}
}
