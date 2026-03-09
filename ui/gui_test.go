// Package ui_test runs GUI tests using Fyne's headless test driver.
//
// Run with SQUIRE_UI_TEST=1 so the UI skips robotgo for mouse position and
// config uses a stub display size. Example:
//
//	SQUIRE_UI_TEST=1 go test -v ./ui/ -run TestGUI
//
// Note: The robotgo dependency may still open an X11 display when the package
// is loaded. On headless CI (no DISPLAY), run tests under a virtual display, e.g.:
//
//	xvfb-run -a go test -v ./ui/ -run TestGUI
package ui_test

import (
	"os"
	"testing"

	"Sqyre/internal/models/actions"
	"Sqyre/ui"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
)

func init() {
	// Set so ConstructUi skips robotgo (toggleMousePos). For config display size
	// stub, run the test with SQUIRE_UI_TEST=1 in the environment before go test.
	if os.Getenv("SQUIRE_UI_TEST") == "" {
		_ = os.Setenv("SQUIRE_UI_TEST", "1")
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

// TestGUIMacroMenuHasAddAction verifies Macro menu has "Add Blank Action" with Basic/Advanced/Variable submenus.
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
	for _, name := range []string{"Basic", "Advanced", "Variable"} {
		if !subLabels[name] {
			t.Errorf("Macro submenu %q not found", name)
		}
	}
}

// TestGUIEscapeClosesInformationDialog verifies that pressing Esc closes the top-most
// popup/dialog (e.g. the Computer Information dialog opened from Settings).
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

	// Simulate Esc key to close the top overlay
	onTypedKey := u.Window.Canvas().OnTypedKey()
	if onTypedKey == nil {
		t.Fatal("window canvas has no OnTypedKey handler; Esc-to-close not implemented")
	}
	onTypedKey(&fyne.KeyEvent{Name: fyne.KeyEscape})

	if overlays.Top() != nil {
		t.Error("expected overlay to be closed after Esc; top overlay still present")
	}
}

// TestGUIEscapeClosesActionDialog verifies that pressing Esc closes the action edit
// dialog when it is the top-most layer.
func TestGUIEscapeClosesActionDialog(t *testing.T) {
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

	// Open the action dialog directly (same as when user taps an action to edit)
	ui.ShowActionDialog(actions.NewWait(0), nil)
	if u.MainUi.ActionDialog == nil {
		t.Fatal("expected action dialog to be open after ShowActionDialog")
	}
	overlays := u.Window.Canvas().Overlays()
	if overlays.Top() == nil {
		t.Fatal("expected overlay to be visible when action dialog is open")
	}

	// Simulate Esc to close the action dialog
	onTypedKey := u.Window.Canvas().OnTypedKey()
	if onTypedKey == nil {
		t.Fatal("window canvas has no OnTypedKey handler; Esc-to-close not implemented")
	}
	onTypedKey(&fyne.KeyEvent{Name: fyne.KeyEscape})

	if u.MainUi.ActionDialog != nil {
		t.Error("expected ActionDialog to be nil after Esc")
	}
	if overlays.Top() != nil {
		t.Error("expected overlay to be closed after Esc; top overlay still present")
	}
}
