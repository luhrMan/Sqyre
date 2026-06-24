// Package ui_test runs GUI tests using Fyne's headless test driver.
//
// Headless runs (no X11 display) should use ./scripts/test.sh or:
//
//	GOFLAGS="-tags=gocv_specific_modules,nohook" SQUIRE_UI_TEST=1 go test ./...
//
// Full hook/display tests (Esc via gohook, screenshot golden files) need xvfb:
//
//	./scripts/test-ui.sh
package ui_test

import (
	"log"
	"os"
	"path/filepath"
	"testing"
	"time"

	"Sqyre/internal/models/serialize"
	"Sqyre/internal/testdb"
	"Sqyre/ui"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
)

func init() {
	if os.Getenv("SQUIRE_UI_TEST") == "" {
		_ = os.Setenv("SQUIRE_UI_TEST", "1")
	}
	if os.Getenv("SQYRE_NO_HOOK") == "" {
		_ = os.Setenv("SQYRE_NO_HOOK", "1")
	}
}

var uiTestDBDir string

func TestMain(m *testing.M) {
	initUITestDB()
	defer os.RemoveAll(uiTestDBDir)
	os.Exit(m.Run())
}

func initUITestDB() {
	dbDir, err := os.MkdirTemp("", "sqyre-ui-testdb-*")
	if err != nil {
		log.Fatalf("testdb: %v", err)
	}
	uiTestDBDir = dbDir
	dbPath := filepath.Join(dbDir, "db.yaml")
	if err := os.WriteFile(dbPath, testdb.Fixture(), 0644); err != nil {
		log.Fatalf("testdb: %v", err)
	}
	yc := serialize.GetYAMLConfig()
	yc.SetConfigFile(dbPath)
	if err := yc.ReadConfig(); err != nil {
		log.Fatalf("testdb: %v", err)
	}
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
