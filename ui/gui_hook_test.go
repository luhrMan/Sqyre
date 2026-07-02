//go:build !nohook

package ui_test

import (
	"context"
	"os/exec"
	"sync"
	"testing"
	"time"

	"Sqyre/internal/models/actions"
	"Sqyre/ui"
	"Sqyre/ui/macro/actiondialog"

	"fyne.io/fyne/v2/test"
	hook "github.com/luhrMan/gohook"
)

var startUITestHookOnce sync.Once

// ensureUITestHook starts the global hook processor for Esc synthesis tests only.
// Do not call from init: gohook polls X11 and can stall Fyne screenshot tests under xvfb.
func ensureUITestHook(t *testing.T) {
	t.Helper()
	startUITestHookOnce.Do(func() {
		s := hook.Start()
		procDone := hook.Process(s)
		go func() { <-procDone }()
	})
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
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, path, "key", "Escape")
	if err := cmd.Run(); err != nil {
		t.Fatalf("xdotool key Escape: %v", err)
	}
}

// TestGUIEscapeClosesInformationDialog verifies Esc dismisses the Computer info dialog
// via the global gohook handler (ui.AddDialogEscapeClose), not canvas key events.
func TestGUIEscapeClosesInformationDialog(t *testing.T) {
	ensureUITestHook(t)
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

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
	ensureUITestHook(t)
	a := test.NewApp()
	w := a.NewWindow("")
	defer w.Close()

	u := ui.InitializeUi(w)
	u.ConstructUi()

	actiondialog.ShowActionDialog(actions.NewWait(0), nil, nil)
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
