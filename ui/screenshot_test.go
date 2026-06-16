// Screenshot and demo-frame tests for README assets.
//
// Regenerate PNG/GIF source frames:
//
//	SQYRE_UPDATE_SCREENSHOTS=1 ./scripts/generate-docs-media.sh
//
// Verify committed assets match the current UI (CI):
//
//	go test -v ./ui/ -run 'TestDocsScreenshots|TestDemoWorkflowFrames'
package ui_test

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"

	"Sqyre/binders"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/testsupport"
	"Sqyre/ui"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
)

func docsImagesDir(t *testing.T) string {
	t.Helper()
	dir := filepath.Join("..", "docs", "images")
	if err := os.MkdirAll(dir, 0755); err != nil {
		t.Fatalf("mkdir docs/images: %v", err)
	}
	return dir
}

func demoFramesDir(t *testing.T) string {
	t.Helper()
	dir := filepath.Join(docsImagesDir(t), "frames")
	if err := os.MkdirAll(dir, 0755); err != nil {
		t.Fatalf("mkdir frames: %v", err)
	}
	return dir
}

func updateScreenshots() bool {
	return os.Getenv("SQYRE_UPDATE_SCREENSHOTS") == "1"
}

func setupDocsUi(t *testing.T) (*ui.Ui, fyne.Window) {
	t.Helper()
	testsupport.InitDocUIEnv(t)
	a := test.NewApp()
	w := a.NewWindow("Sqyre")
	u := ui.InitializeUi(w)
	u.ConstructUi()
	binders.SetMacroUi()
	binders.SetEditorUi()
	return u, w
}

func writeOrComparePNG(t *testing.T, path string, pngData []byte, minBytes int) {
	t.Helper()
	if minBytes <= 0 {
		minBytes = 5000
	}
	if len(pngData) < minBytes {
		t.Fatalf("screenshot %s too small (%d bytes); likely blank or unlaid-out", path, len(pngData))
	}
	if updateScreenshots() {
		if err := os.WriteFile(path, pngData, 0644); err != nil {
			t.Fatalf("write %s: %v", path, err)
		}
		return
	}
	existing, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("missing golden screenshot %s (run ./scripts/generate-docs-media.sh): %v", path, err)
	}
	if !bytes.Equal(existing, pngData) {
		t.Fatalf("screenshot drift: %s differs from committed file (regenerate with ./scripts/generate-docs-media.sh)", path)
	}
}

func writeFrameFile(t *testing.T, dir, name string, data []byte, minBytes int) {
	t.Helper()
	if minBytes <= 0 {
		minBytes = 5000
	}
	if len(data) < minBytes {
		t.Fatalf("frame %s too small (%d bytes)", name, len(data))
	}
	path := filepath.Join(dir, name)
	if updateScreenshots() {
		if err := os.WriteFile(path, data, 0644); err != nil {
			t.Fatalf("write frame %s: %v", path, err)
		}
		return
	}
	if _, err := os.Stat(path); err != nil {
		t.Fatalf("missing demo frame %s: %v", path, err)
	}
}

func TestDocsScreenshots(t *testing.T) {
	dir := docsImagesDir(t)
	u, w := setupDocsUi(t)
	defer w.Close()

	mainPNG, err := ui.RenderObjectPNG(ui.MacroScreenForScreenshot(u), fyne.NewSize(1000, 500))
	if err != nil {
		t.Fatalf("render main window: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "main-window.png"), mainPNG, 5000)

	panel := ui.ActionDialogPanelForScreenshot(actions.NewWait(500))
	waitPNG, err := ui.RenderObjectPNG(panel, fyne.NewSize(420, 220))
	if err != nil {
		t.Fatalf("render wait dialog panel: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "action-dialog-wait.png"), waitPNG, 3000)

	editorPNG, err := ui.RenderObjectPNG(ui.EditorScreenForScreenshot(u), fyne.NewSize(1000, 500))
	if err != nil {
		t.Fatalf("render data editor: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "data-editor.png"), editorPNG, 5000)

	if updateScreenshots() {
		writeDemoFrames(t, u)
	}
}

func writeDemoFrames(t *testing.T, u *ui.Ui) {
	t.Helper()
	framesDir := demoFramesDir(t)

	mainFrame, err := ui.RenderObjectPNG(ui.MacroScreenForScreenshot(u), fyne.NewSize(1000, 500))
	if err != nil {
		t.Fatalf("capture frame 1: %v", err)
	}
	writeFrameFile(t, framesDir, "demo-macro-001.png", mainFrame, 5000)

	panel := ui.ActionDialogPanelForScreenshot(actions.NewWait(500))
	dialogFrame, err := ui.RenderObjectPNG(panel, fyne.NewSize(420, 220))
	if err != nil {
		t.Fatalf("capture frame 2: %v", err)
	}
	writeFrameFile(t, framesDir, "demo-macro-002.png", dialogFrame, 3000)

	writeFrameFile(t, framesDir, "demo-macro-003.png", mainFrame, 5000)

	mt := u.Mui.MTabs.SelectedTab()
	if mt == nil {
		t.Fatal("no macro tab selected")
	}
	wait := actions.NewWait(0)
	mt.Macro.Root.AddSubAction(wait)
	mt.Refresh()
	mt.Select(wait.GetUID())
	treeFrame, err := ui.RenderObjectPNG(ui.MacroScreenForScreenshot(u), fyne.NewSize(1000, 500))
	if err != nil {
		t.Fatalf("capture frame 4: %v", err)
	}
	writeFrameFile(t, framesDir, "demo-macro-004.png", treeFrame, 5000)
}

func TestDemoWorkflowFrames(t *testing.T) {
	if updateScreenshots() {
		t.Skip("frames written during TestDocsScreenshots update run")
	}
	framesDir := demoFramesDir(t)
	for _, name := range []string{
		"demo-macro-001.png",
		"demo-macro-002.png",
		"demo-macro-003.png",
		"demo-macro-004.png",
	} {
		info, err := os.Stat(filepath.Join(framesDir, name))
		if err != nil {
			t.Fatalf("missing frame %s: %v", name, err)
		}
		min := int64(5000)
		if name == "demo-macro-002.png" {
			min = 3000
		}
		if info.Size() < min {
			t.Fatalf("frame %s too small (%d bytes)", name, info.Size())
		}
	}
}
