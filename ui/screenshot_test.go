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

type docScreenshot struct {
	file     string
	action   actions.ActionInterface
	minBytes int
}

func docActionScreenshots() []docScreenshot {
	return []docScreenshot{
		{"action-dialog-move.png", actions.NewMove(actions.NewCoordinateRef("Demo Program", "center"), false), 5000},
		{"action-dialog-click.png", actions.NewClick(actions.ClickButtonLeft, true), 5000},
		{"action-dialog-key.png", actions.NewKey("ctrl", true), 5000},
		{"action-dialog-type.png", actions.NewType("hello", 50), 5000},
		{"action-dialog-wait.png", actions.NewWait(500), 5000},
		{"action-dialog-focuswindow.png", actions.NewFocusWindow("/usr/bin/notepad", "Untitled - Notepad"), 5000},
		{"action-dialog-runmacro.png", actions.NewRunMacro("Demo Macro"), 5000},
		{"action-dialog-loop.png", actions.NewLoop(5, "repeat", []actions.ActionInterface{}), 5000},
		{"action-dialog-imagesearch.png", actions.NewImageSearch("find item", []actions.ActionInterface{}, []string{}, actions.NewCoordinateRef("Demo Program", "Main area"), 1, 1, 0.95, 5), 5000},
		{"action-dialog-ocr.png", actions.NewOcr("read text", "template", actions.NewCoordinateRef("Demo Program", "Main area")), 5000},
		{"action-dialog-findpixel.png", actions.NewFindPixel("find color", actions.NewCoordinateRef("Demo Program", "Main area"), "ffffff", 0), 5000},
		{"action-dialog-setvariable.png", actions.NewSetVariable("counter", "0"), 5000},
		{"action-dialog-calculate.png", actions.NewCalculate("1 + 1", "result"), 5000},
		{"action-dialog-foreachrow.png", actions.NewForEachRow("items", []actions.ListColumn{{Source: "mylist", OutputVar: "value"}}, nil), 5000},
		{"action-dialog-savevariable.png", actions.NewSaveVariable("value", "output.txt", false, false), 5000},
	}
}

func captureActionDialogPNG(t *testing.T, mainPNG []byte, action actions.ActionInterface) []byte {
	t.Helper()
	pngData, err := ui.OverlayActionDialogOnMainPNG(mainPNG, action)
	if err != nil {
		t.Fatalf("render %s dialog on main window: %v", action.GetType(), err)
	}
	return pngData
}

func TestDocsScreenshots(t *testing.T) {
	dir := docsImagesDir(t)
	u, w := setupDocsUi(t)
	defer w.Close()

	mainPNG, err := ui.RenderObjectPNG(u.MainUi.Navigation.Root, fyne.NewSize(1000, 500))
	if err != nil {
		t.Fatalf("render main window: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "main-window.png"), mainPNG, 5000)

	for _, shot := range docActionScreenshots() {
		shot := shot
		t.Run(shot.file, func(t *testing.T) {
			pngData := captureActionDialogPNG(t, mainPNG, shot.action)
			writeOrComparePNG(t, filepath.Join(dir, shot.file), pngData, shot.minBytes)
		})
	}

	pickerPNG, err := ui.RenderObjectPNG(ui.AddActionPickerForScreenshot(), ui.AddActionPickerSize)
	if err != nil {
		t.Fatalf("render add action picker: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "add-action-picker.png"), pickerPNG, 5000)

	editorPNG, err := ui.RenderObjectPNG(ui.EditorScreenForScreenshot(u), fyne.NewSize(1000, 500))
	if err != nil {
		t.Fatalf("render data editor: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "data-editor.png"), editorPNG, 5000)

	if updateScreenshots() {
		writeDemoFrames(t, u, mainPNG)
	}
}

func writeDemoFrames(t *testing.T, u *ui.Ui, mainPNG []byte) {
	t.Helper()
	framesDir := demoFramesDir(t)

	frame1, err := ui.OverlayClickGuide(mainPNG, ui.DemoClickActionIcon)
	if err != nil {
		t.Fatalf("frame 1 click guide: %v", err)
	}
	writeFrameFile(t, framesDir, "demo-macro-001.png", frame1, 5000)

	dialogFrame, err := ui.OverlayActionDialogOnMainPNG(mainPNG, actions.NewWait(500))
	if err != nil {
		t.Fatalf("capture frame 2: %v", err)
	}
	frame2, err := ui.OverlayClickGuide(dialogFrame, ui.DemoClickDialogSave)
	if err != nil {
		t.Fatalf("frame 2 click guide: %v", err)
	}
	writeFrameFile(t, framesDir, "demo-macro-002.png", frame2, 5000)

	pickerFrame, err := ui.OverlayAddActionPickerOnMainPNG(mainPNG)
	if err != nil {
		t.Fatalf("capture frame 3: %v", err)
	}
	frame3, err := ui.OverlayClickGuide(pickerFrame, ui.DemoClickPickerWait)
	if err != nil {
		t.Fatalf("frame 3 click guide: %v", err)
	}
	writeFrameFile(t, framesDir, "demo-macro-003.png", frame3, 5000)

	mt := u.Mui.MTabs.SelectedTab()
	if mt == nil {
		t.Fatal("no macro tab selected")
	}
	wait := actions.NewWait(0)
	mt.Macro.Root.AddSubAction(wait)
	mt.Refresh()
	mt.Select(wait.GetUID())
	treeFrame, err := ui.RenderObjectPNG(u.MainUi.Navigation.Root, fyne.NewSize(1000, 500))
	if err != nil {
		t.Fatalf("capture frame 4: %v", err)
	}
	frame4, err := ui.OverlayClickGuide(treeFrame, ui.DemoClickNewActionRow)
	if err != nil {
		t.Fatalf("frame 4 click guide: %v", err)
	}
	writeFrameFile(t, framesDir, "demo-macro-004.png", frame4, 5000)
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
		if info.Size() < 5000 {
			t.Fatalf("frame %s too small (%d bytes)", name, info.Size())
		}
	}
}
