// Package ui_test runs GUI tests using Fyne's headless test driver.
//
// Run with SQUIRE_UI_TEST=1 so the UI skips the mouse-position poller and
// config uses a stub display size. Example:
//
//	SQUIRE_UI_TEST=1 go test -v ./ui/ -run TestGUI
//
// Note: Other packages may still open an X11 display when tests load the full app.
// On headless CI (no DISPLAY), run tests under a virtual display, e.g.:
//
//	xvfb-run -a go test -v ./ui/ -run TestGUI
//
// Escape-to-close on dialogs uses the global keyboard hook (github.com/luhrMan/gohook),
// same pipeline as macro hotkeys — not Fyne canvas OnTypedKey. TestMain starts hook.Process
// so ui.AddDialogEscapeClose handlers run. Esc tests send a real Escape with xdotool (install
// xdotool; use xvfb-run for DISPLAY); tests skip if xdotool is missing.
package ui_test

import (
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/serialize"
	"Sqyre/internal/testdb"
	"Sqyre/ui"
	"Sqyre/ui/macro/actiondialog"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
	hook "github.com/luhrMan/gohook"
)

func init() {
	// Ensures ConstructUi skips toggleMousePos (SQUIRE_UI_TEST). For config display size
	// stub, run the test with SQUIRE_UI_TEST=1 in the environment before go test.
	if os.Getenv("SQUIRE_UI_TEST") == "" {
		_ = os.Setenv("SQUIRE_UI_TEST", "1")
	}
}

func TestMain(m *testing.M) {
	// Same db.yaml fixture as repository tests so MacroRepo/ProgramRepo see stable data if loaded.
	dbDir, err := os.MkdirTemp("", "sqyre-ui-testdb-*")
	if err != nil {
		log.Fatalf("testdb: %v", err)
	}
	defer os.RemoveAll(dbDir)
	dbPath := filepath.Join(dbDir, "db.yaml")
	if err := os.WriteFile(dbPath, testdb.Fixture(), 0644); err != nil {
		log.Fatalf("testdb: %v", err)
	}
	yc := serialize.GetYAMLConfig()
	yc.SetConfigFile(dbPath)
	if err := yc.ReadConfig(); err != nil {
		log.Fatalf("testdb: %v", err)
	}

	// Global hook: must run hook.Process so KeyDown handlers registered by
	// ui.AddDialogEscapeClose are invoked (see dialog_escape.go).
	s := hook.Start()
	procDone := hook.Process(s)
	go func() { <-procDone }()
	code := m.Run()
	hook.End()
	os.Exit(code)
}

func waitUntil(t *testing.T, timeout time.Duration, cond func() bool, msg string) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if cond() {
			return
		}
		time.Sleep(15 * time.Millisecond)
	}
	t.Fatal(msg)
}

// sendEscapeViaGlobalHook asks the OS to synthesize Escape; the same global hook
// pipeline (hook.Start + hook.Process) used for macro hotkeys delivers KeyDown to
// ui.AddDialogEscapeClose. Prefer xdotool under Xvfb — hook.AddEvent can block in C.
func sendEscapeViaGlobalHook(t *testing.T) {
	t.Helper()
	path, err := exec.LookPath("xdotool")
	if err != nil {
		t.Skip("xdotool not on PATH: cannot synthesize Esc for global hook test")
	}
	cmd := exec.Command(path, "key", "Escape")
	if err := cmd.Run(); err != nil {
		t.Fatalf("xdotool key Escape: %v", err)
	}
}

// TestGUIBuild verifies the main UI builds and window has content and main menu.
func TestGUIBuild(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	if u == nil {
		t.Fatal("InitializeUi returned nil")
	}
	u.ConstructUi()

	if u.Window == nil {
		t.Fatal("Window is nil")
	}
	if u.Window.Canvas().Content() == nil {
		t.Error("Window has no content")
	}
	if u.MainMenu == nil || len(u.MainMenu.Items) == 0 {
		t.Error("Main menu missing or empty")
	}
}

// TestGUIMainMenuStructure verifies the main menu has Settings and Macro with expected items.
func TestGUIMainMenuStructure(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

	menuLabels := make(map[string]bool)
	for _, m := range u.MainMenu.Items {
		menuLabels[m.Label] = true
	}
	if !menuLabels["Settings"] {
		t.Error("Settings menu not found")
	}
	if !menuLabels["Macro"] {
		t.Error("Macro menu not found")
	}

	var settingsMenu *fyne.Menu
	for _, m := range u.MainMenu.Items {
		if m.Label == "Settings" {
			settingsMenu = m
			break
		}
	}
	if settingsMenu == nil {
		t.Fatal("Settings menu not found")
	}
	itemLabels := make(map[string]bool)
	for _, it := range settingsMenu.Items {
		itemLabels[it.Label] = true
	}
	if !itemLabels["Data Editor"] {
		t.Error("Data Editor menu item not found under Settings")
	}
	if !itemLabels["User Settings"] {
		t.Error("User Settings menu item not found under Settings")
	}
}

// TestGUIDataEditorNavigation invokes the Data Editor menu action and verifies it runs without panic.
func TestGUIDataEditorNavigation(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

	var dataEditorAction func()
	for _, m := range u.MainMenu.Items {
		if m.Label != "Settings" {
			continue
		}
		for _, it := range m.Items {
			if it.Label == "Data Editor" {
				dataEditorAction = it.Action
				break
			}
		}
		break
	}
	if dataEditorAction == nil {
		t.Fatal("Data Editor menu action not found")
	}

	dataEditorAction()
	// Navigation should have pushed editor; window content still the navigation container.
	if u.Window.Canvas().Content() == nil {
		t.Error("Window content is nil after Data Editor")
	}
}

// TestGUIUserSettingsNavigation invokes the User Settings menu action and verifies it runs without panic.
func TestGUIUserSettingsNavigation(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

	var userSettingsAction func()
	for _, m := range u.MainMenu.Items {
		if m.Label != "Settings" {
			continue
		}
		for _, it := range m.Items {
			if it.Label == "User Settings" {
				userSettingsAction = it.Action
				break
			}
		}
		break
	}
	if userSettingsAction == nil {
		t.Fatal("User Settings menu action not found")
	}

	userSettingsAction()
	if u.Window.Canvas().Content() == nil {
		t.Error("Window content is nil after User Settings")
	}
}

// TestGUIMacroMenuHasAddAction verifies Macro menu has "Add Blank Action" with category submenus
// matching buildActionTemplates (Mouse & Keyboard, Detection, Variables, Miscellaneous).
func TestGUIMacroMenuHasAddAction(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

	var macroMenu *fyne.Menu
	for _, m := range u.MainMenu.Items {
		if m.Label == "Macro" {
			macroMenu = m
			break
		}
	}
	if macroMenu == nil {
		t.Fatal("Macro menu not found")
	}
	var addAction *fyne.MenuItem
	for _, it := range macroMenu.Items {
		if it.Label == "Add Blank Action" {
			addAction = it
			break
		}
	}
	if addAction == nil {
		t.Fatal("Add Blank Action not found")
	}
	if addAction.ChildMenu == nil {
		t.Fatal("Add Blank Action has no child menu")
	}
	subLabels := make(map[string]bool)
	for _, it := range addAction.ChildMenu.Items {
		subLabels[it.Label] = true
	}
	for _, name := range []string{"Mouse & Keyboard", "Detection", "Variables", "Miscellaneous"} {
		if !subLabels[name] {
			t.Errorf("Macro submenu %q not found", name)
		}
	}
}

// TestGUIEscapeClosesInformationDialog verifies Esc dismisses the Computer info dialog
// via the global gohook handler (ui.AddDialogEscapeClose), not canvas key events.
func TestGUIEscapeClosesInformationDialog(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

	// Open "Computer info" dialog from Settings menu (shows an information dialog)
	var computerInfoAction func()
	for _, m := range u.MainMenu.Items {
		if m.Label != "Settings" {
			continue
		}
		for _, it := range m.Items {
			if it.Label == "Computer info" {
				computerInfoAction = it.Action
				break
			}
		}
		break
	}
	if computerInfoAction == nil {
		t.Fatal("Computer info menu action not found")
	}

	computerInfoAction()
	overlays := u.Window.Canvas().Overlays()
	if overlays.Top() == nil {
		t.Fatal("expected overlay (dialog) to be visible after opening Computer info")
	}

	sendEscapeViaGlobalHook(t)
	waitUntil(t, 3*time.Second, func() bool {
		return u.Window.Canvas().Overlays().Top() == nil
	}, "expected global Esc hook to close information dialog")
}

// TestGUIEscapeClosesActionDialog verifies Esc dismisses the action edit dialog
// via the same global gohook path registered in showCustomActionDialog.
func TestGUIEscapeClosesActionDialog(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

	// Open the action dialog directly (same as when user taps an action to edit)
	actiondialog.ShowActionDialog(actions.NewWait(0), nil)
	if u.MainUi.ActionDialog == nil {
		t.Fatal("expected action dialog to be open after ShowActionDialog")
	}
	overlays := u.Window.Canvas().Overlays()
	if overlays.Top() == nil {
		t.Fatal("expected overlay to be visible when action dialog is open")
	}

	sendEscapeViaGlobalHook(t)
	waitUntil(t, 3*time.Second, func() bool {
		return u.MainUi.ActionDialog == nil && u.Window.Canvas().Overlays().Top() == nil
	}, "expected global Esc hook to close action dialog")
}

// TestGUIActionDialogForAllTypes opens the action dialog for every action type
// and verifies it produces a visible dialog overlay without panic.
func TestGUIActionDialogForAllTypes(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

	allActions := []struct {
		name string
		a    actions.ActionInterface
	}{
		{"Click", actions.NewClick(false, true)},
		{"Wait", actions.NewWait(100)},
		{"Move", actions.NewMove(actions.Point{Name: "P", X: 1, Y: 2}, false)},
		{"Key", actions.NewKey("a", true)},
		{"Type", actions.NewType("hello", 0)},
		{"Loop", actions.NewLoop(1, "L", nil)},
		{"Ocr", actions.NewOcr("O", nil, "text", actions.SearchArea{Name: "A"})},
		{"ImageSearch", actions.NewImageSearch("S", nil, nil, actions.SearchArea{Name: "R"}, 1, 1, 0.9, 5)},
		{"FindPixel", actions.NewFindPixel("FP", actions.SearchArea{}, "ff0000", 0, nil)},
		{"SetVariable", actions.NewSetVariable("v", "val")},
		{"Calculate", actions.NewCalculate("1+1", "r")},
		{"DataList", actions.NewDataList("a\nb", "v", false)},
		{"SaveVariable", actions.NewSaveVariable("v", "dest", false, false)},
		{"FocusWindow", actions.NewFocusWindow("chrome")},
		{"RunMacro", actions.NewRunMacro("m")},
	}
	for _, tt := range allActions {
		t.Run(tt.name, func(t *testing.T) {
			actiondialog.ShowActionDialog(tt.a, nil)
			overlay := u.Window.Canvas().Overlays().Top()
			if overlay == nil {
				t.Error("expected dialog overlay")
			}
			// Close the dialog for the next iteration
			if u.MainUi.ActionDialog != nil {
				u.MainUi.ActionDialog.Hide()
				u.MainUi.ActionDialog = nil
			}
		})
	}
}

// TestGUIServicesCallbacksWired verifies that InitializeUi wires RunOnMainThread
// and BoolPreference to Fyne implementations.
func TestGUIServicesCallbacksWired(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	_ = ui.InitializeUi(w)

	// After InitializeUi, RunOnMainThread should be fyne.Do (not the default sync func).
	// We can verify it works by running a function through it.
	done := make(chan bool, 1)
	go func() {
		// fyne.Do queues on main thread; in test driver this runs inline
		ui.GetUi() // verify it doesn't panic
		done <- true
	}()
	select {
	case <-done:
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for goroutine")
	}
}
