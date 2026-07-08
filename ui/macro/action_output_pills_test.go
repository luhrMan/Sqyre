package macro

import (
	"testing"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
)

func summaryPillCount(t *testing.T, line fyne.CanvasObject) int {
	t.Helper()
	box, ok := line.(*fyne.Container)
	if !ok {
		t.Fatalf("summary line type = %T, want *fyne.Container", line)
	}
	return len(box.Objects)
}

func TestBuildActionSummaryLine_setVariableShowsNameOnce(t *testing.T) {
	sv := actions.NewSetVariable("count", 5)
	line, _ := buildActionSummaryLine(sv, sv.Params(), macroKnownVariables())
	// One pill for the variable chip + one for the value; the name is not duplicated.
	if got := summaryPillCount(t, line); got != 2 {
		t.Fatalf("set summary pills = %d, want 2 (variable chip + value)", got)
	}
}

func TestBuildActionSummaryLine_ocrAppendsCoordinateOutputs(t *testing.T) {
	ocr := actions.NewOcr("read", "target", actions.CoordinateRef("area"))
	ocr.OutputVariable = "text"
	// Params: Name, Target Text (2). Bindings: foundX, foundY, text (3) appended.
	line, _ := buildActionSummaryLine(ocr, ocr.Params(), macroKnownVariables())
	if got := summaryPillCount(t, line); got != 5 {
		t.Fatalf("ocr summary pills = %d, want 5 (name, target, X, Y, output)", got)
	}
}

func TestBuildActionSummaryLine_nonProducerUnchanged(t *testing.T) {
	wait := actions.NewWait(100)
	line, _ := buildActionSummaryLine(wait, wait.Params(), macroKnownVariables())
	if got := summaryPillCount(t, line); got != 1 {
		t.Fatalf("wait summary pills = %d, want 1", got)
	}
}
