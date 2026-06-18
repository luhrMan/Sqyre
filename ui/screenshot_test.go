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
	"Sqyre/ui/macro/actiondialog"

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
		{"action-dialog-move.png", actions.NewMove(actions.Point{Name: "center", X: 500, Y: 300}, false), 3000},
		{"action-dialog-click.png", actions.NewClick(false, true), 3000},
		{"action-dialog-key.png", actions.NewKey("ctrl", true), 3000},
		{"action-dialog-type.png", actions.NewType("hello", 50), 3000},
		{"action-dialog-wait.png", actions.NewWait(500), 3000},
		{"action-dialog-focuswindow.png", actions.NewFocusWindow("Notepad"), 3000},
		{"action-dialog-runmacro.png", actions.NewRunMacro("Demo Macro"), 3000},
		{"action-dialog-loop.png", actions.NewLoop(5, "repeat", []actions.ActionInterface{}), 3000},
		{"action-dialog-imagesearch.png", actions.NewImageSearch("find item", []actions.ActionInterface{}, []string{}, actions.SearchArea{Name: "Main area"}, 1, 1, 0.95, 5), 3000},
		{"action-dialog-ocr.png", actions.NewOcr("read text", []actions.ActionInterface{}, "template", actions.SearchArea{Name: "Main area"}), 3000},
		{"action-dialog-findpixel.png", actions.NewFindPixel("find color", actions.SearchArea{Name: "Main area"}, "ffffff", 0, nil), 3000},
		{"action-dialog-setvariable.png", actions.NewSetVariable("counter", "0"), 3000},
		{"action-dialog-calculate.png", actions.NewCalculate("1 + 1", "result"), 3000},
		{"action-dialog-datalist.png", actions.NewDataList("mylist", "value", false), 3000},
		{"action-dialog-savevariable.png", actions.NewSaveVariable("value", "output.txt", false, false), 3000},
	}
}

func captureActionDialogPNG(t *testing.T, action actions.ActionInterface) []byte {
	t.Helper()
	panel := actiondialog.PanelForScreenshot(action)
	size := actiondialog.ScreenshotSizeForAction(action)
	pngData, err := ui.RenderObjectPNG(panel, size)
	if err != nil {
		t.Fatalf("render %s dialog: %v", action.GetType(), err)
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

	pickerPNG, err := ui.RenderObjectPNG(ui.AddActionPickerForScreenshot(), fyne.NewSize(980, 460))
	if err != nil {
		t.Fatalf("render add action picker: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "add-action-picker.png"), pickerPNG, 5000)

	for _, shot := range docActionScreenshots() {
		shot := shot
		t.Run(shot.file, func(t *testing.T) {
			pngData := captureActionDialogPNG(t, shot.action)
			writeOrComparePNG(t, filepath.Join(dir, shot.file), pngData, shot.minBytes)
		})
	}

	editorPNG, err := ui.RenderObjectPNG(ui.EditorScreenForScreenshot(u), fyne.NewSize(1000, 500))
	if err != nil {
		t.Fatalf("render data editor: %v", err)
	}
	writeOrComparePNG(t, filepath.Join(dir, "data-editor.png"), editorPNG, 5000)

	if updateScreenshots() {
		writeDemoFrames(t, u, w)
	}
}

func writeDemoFrames(t *testing.T, u *ui.Ui, w fyne.Window) {
	t.Helper()
	framesDir := demoFramesDir(t)
	_ = w

	mainFrame, err := ui.RenderObjectPNG(u.MainUi.Navigation.Root, fyne.NewSize(1000, 500))
	if err != nil {
		t.Fatalf("capture frame 1: %v", err)
	}
	writeFrameFile(t, framesDir, "demo-macro-001.png", mainFrame, 5000)

	panel := actiondialog.PanelForScreenshot(actions.NewWait(500))
	dialogFrame, err := ui.RenderObjectPNG(panel, actiondialog.ScreenshotSizeForAction(actions.NewWait(500)))
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
	treeFrame, err := ui.RenderObjectPNG(u.MainUi.Navigation.Root, fyne.NewSize(1000, 500))
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
