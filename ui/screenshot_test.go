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

	"Sqyre/internal/testsupport"
	"Sqyre/ui"
	"Sqyre/ui/macro"
	"Sqyre/ui/screenshot"
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
	ui.ResetGlobalsForTesting()
	u := ui.InitializeUi(w)
	u.ConstructUi()
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

var docsMainSize = fyne.NewSize(1000, 500)

func TestDocsScreenshots(t *testing.T) {
	dir := docsImagesDir(t)
	u, w := setupDocsUi(t)
	defer w.Close()

	mt := u.Mui.MTabs.SelectedTab()
	if mt == nil {
		t.Fatal("no macro tab selected")
	}
	// Show nested steps (image-search / loop children) in the tree summary.
	mt.OpenAllBranches()
	mt.Refresh()

	mainPNG, err := screenshot.RenderObjectPNG(u.MainUi.Navigation.Root, docsMainSize)
	if err != nil {
		t.Fatalf("render main window: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "main-window.png"), mainPNG, 5000)

	pickerPNG, err := screenshot.RenderObjectPNG(ui.AddActionPickerForScreenshot(), ui.AddActionPickerSize)
	if err != nil {
		t.Fatalf("render add action picker: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "add-action-picker.png"), pickerPNG, 5000)

	editorPNG, err := screenshot.RenderObjectPNG(ui.EditorScreenForScreenshot(u), docsMainSize)
	if err != nil {
		t.Fatalf("render data editor: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "data-editor.png"), editorPNG, 5000)

	if updateScreenshots() {
		writeDemoFrames(t, u, mt)
	}
}

// rowClickGuideOnMain renders the populated main window and draws a click guide
// centered on the tree row for uid, using real tree geometry.
func rowClickGuideOnMain(t *testing.T, u *ui.Ui, mt *macro.MacroTree, uid string) []byte {
	t.Helper()
	var center fyne.Position
	pngData, _, err := screenshot.RenderObjectPNGWithAnchors(u.MainUi.Navigation.Root, docsMainSize, func() []fyne.Position {
		pos, ok := mt.RowCenterForScreenshot(uid)
		if !ok {
			t.Fatalf("row %s not visible for click guide", uid)
		}
		center = pos
		return []fyne.Position{pos}
	})
	if err != nil {
		t.Fatalf("render main window with row anchor: %v", err)
	}
	frame, err := screenshot.OverlayClickGuide(pngData, screenshot.ClickGuideAt(center))
	if err != nil {
		t.Fatalf("row click guide: %v", err)
	}
	return frame
}

func writeDemoFrames(t *testing.T, u *ui.Ui, mt *macro.MacroTree) {
	t.Helper()
	framesDir := demoFramesDir(t)

	subs := mt.Macro.Root.GetSubActions()
	if len(subs) < 3 {
		t.Fatalf("demo macro has too few actions (%d)", len(subs))
	}
	firstUID := subs[0].GetUID()
	// The image-search branch is the third action (see buildDemoMacroActions).
	branchUID := subs[2].GetUID()

	// Frame 1: introduce the macro by pointing at its first action row.
	frame1 := rowClickGuideOnMain(t, u, mt, firstUID)
	writeFrameFile(t, framesDir, "demo-macro-001.png", frame1, 5000)

	// Frame 2: the add-action picker, click guide anchored on the Click tile.
	pickerContent, clickTile := ui.AddActionPickerWithTargetForScreenshot("Click")
	if clickTile == nil {
		t.Fatal("Click tile not found in picker")
	}
	var tileCenter fyne.Position
	pickerPNG, _, err := screenshot.RenderObjectPNGWithAnchors(pickerContent, docsMainSize, func() []fyne.Position {
		tileCenter = screenshot.AnchorCenter(clickTile)
		return []fyne.Position{tileCenter}
	})
	if err != nil {
		t.Fatalf("render picker for frame 2: %v", err)
	}
	mainPNG, err := screenshot.RenderObjectPNG(u.MainUi.Navigation.Root, docsMainSize)
	if err != nil {
		t.Fatalf("render main for frame 2: %v", err)
	}
	composite, offset, err := screenshot.CompositePickerOverDimmedMain(mainPNG, pickerPNG)
	if err != nil {
		t.Fatalf("composite picker for frame 2: %v", err)
	}
	frame2, err := screenshot.OverlayClickGuide(composite, screenshot.ClickGuide{
		X: offset.X + int(tileCenter.X),
		Y: offset.Y + int(tileCenter.Y),
	})
	if err != nil {
		t.Fatalf("frame 2 click guide: %v", err)
	}
	writeFrameFile(t, framesDir, "demo-macro-002.png", frame2, 5000)

	// Frame 3: point at the image-search branch to show nested detection steps.
	mt.Select(branchUID)
	frame3 := rowClickGuideOnMain(t, u, mt, branchUID)
	writeFrameFile(t, framesDir, "demo-macro-003.png", frame3, 5000)
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
	} {
		info, err := os.Stat(filepath.Join(framesDir, name))
		if err != nil {
			t.Fatalf("missing frame %s: %v", name, err)
		}
		if info.Size() < 5000 {
			t.Fatalf("frame %s too small (%d bytes)", name, info.Size())
		}
	}
}
